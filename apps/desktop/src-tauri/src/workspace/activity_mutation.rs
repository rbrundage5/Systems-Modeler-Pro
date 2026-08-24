use super::*;
use std::collections::HashSet;
use systems_modeler_core::{
    Activity, ActivityEdge, ActivityEdgeId, ActivityEndpoint, ActivityNodeId, ActivityNodeKind,
    PinId,
};

fn parse_edge_id(value: &str) -> Result<ActivityEdgeId, String> {
    uuid::Uuid::parse_str(value)
        .map(ActivityEdgeId)
        .map_err(|_| format!("invalid Activity edge id: {value}"))
}

fn endpoint_owner(
    activity: &Activity,
    endpoint: ActivityEndpoint,
) -> Result<ActivityNodeId, String> {
    match endpoint {
        ActivityEndpoint::Node(id) => activity
            .nodes
            .iter()
            .any(|node| node.id == id)
            .then_some(id)
            .ok_or_else(|| "Activity edge references a missing node".to_string()),
        ActivityEndpoint::Pin(pin_id) => activity
            .nodes
            .iter()
            .find(|node| {
                matches!(
                    &node.kind,
                    ActivityNodeKind::Action(action)
                        if action.pins.iter().any(|pin| pin.id == pin_id)
                )
            })
            .map(|node| node.id)
            .ok_or_else(|| "Activity edge references a missing pin".to_string()),
    }
}

fn parse_endpoint(activity: &Activity, token: &str) -> Result<ActivityEndpoint, String> {
    if let Some(value) = token.strip_prefix("pin:") {
        let pin_id = uuid::Uuid::parse_str(value)
            .map(PinId)
            .map_err(|_| format!("invalid Activity pin id: {value}"))?;
        endpoint_owner(activity, ActivityEndpoint::Pin(pin_id))?;
        Ok(ActivityEndpoint::Pin(pin_id))
    } else {
        let node_id = activity_workspace::parse_activity_node_id(token)?;
        endpoint_owner(activity, ActivityEndpoint::Node(node_id))?;
        Ok(ActivityEndpoint::Node(node_id))
    }
}

fn route_rect(node: &activity_workspace::ActivityDiagramNode) -> routing::RouteRect {
    routing::RouteRect {
        x: node.x,
        y: node.y,
        width: node.width,
        height: node.height,
    }
}

fn member_bounds(
    diagram: &activity_workspace::ActivityDiagram,
    member_ids: &HashSet<String>,
    fallback_index: usize,
) -> routing::RouteRect {
    let members: Vec<_> = diagram
        .nodes
        .iter()
        .filter(|node| member_ids.contains(&node.activity_node_id))
        .collect();
    if members.is_empty() {
        return routing::RouteRect {
            x: 70.0 + fallback_index as f64 * 34.0,
            y: 70.0 + fallback_index as f64 * 28.0,
            width: 360.0,
            height: 220.0,
        };
    }
    let left = members
        .iter()
        .map(|node| node.x)
        .fold(f64::INFINITY, f64::min)
        - 36.0;
    let top = members
        .iter()
        .map(|node| node.y)
        .fold(f64::INFINITY, f64::min)
        - 54.0;
    let right = members
        .iter()
        .map(|node| node.x + node.width)
        .fold(f64::NEG_INFINITY, f64::max)
        + 36.0;
    let bottom = members
        .iter()
        .map(|node| node.y + node.height)
        .fold(f64::NEG_INFINITY, f64::max)
        + 36.0;
    routing::RouteRect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    }
}

fn activity_region_obstacles(
    diagram: &activity_workspace::ActivityDiagram,
    activity: &Activity,
    source_owner: Option<ActivityNodeId>,
    target_owner: Option<ActivityNodeId>,
) -> Vec<routing::RouteRect> {
    let mut obstacles = Vec::new();
    for (index, partition) in activity.partitions.iter().enumerate() {
        let members: HashSet<_> = activity
            .nodes
            .iter()
            .filter(|node| node.partition_id == Some(partition.id))
            .map(|node| node.id.to_string())
            .collect();
        let related = source_owner.is_some_and(|id| members.contains(&id.to_string()))
            || target_owner.is_some_and(|id| members.contains(&id.to_string()));
        if !related {
            obstacles.push(member_bounds(diagram, &members, index));
        }
    }
    let offset = activity.partitions.len();
    for (index, structured) in activity.structured_nodes.iter().enumerate() {
        let members: HashSet<_> = activity
            .nodes
            .iter()
            .filter(|node| node.structured_node_id == Some(structured.id))
            .map(|node| node.id.to_string())
            .collect();
        let related = source_owner.is_some_and(|id| members.contains(&id.to_string()))
            || target_owner.is_some_and(|id| members.contains(&id.to_string()));
        if !related {
            obstacles.push(member_bounds(diagram, &members, offset + index));
        }
    }
    obstacles
}

fn route_semantic_edge(
    diagram: &activity_workspace::ActivityDiagram,
    activity: &Activity,
    edge: &ActivityEdge,
    lane_index: usize,
    reserved_routes: &[Vec<DiagramPoint>],
    allow_shared_departure: bool,
    additional_obstacles: &[routing::RouteRect],
    bounds: Option<routing::RouteRect>,
) -> Result<Vec<DiagramPoint>, String> {
    let source_owner_id = endpoint_owner(activity, edge.source)?;
    let target_owner_id = endpoint_owner(activity, edge.target)?;
    let source_owner = source_owner_id.to_string();
    let target_owner = target_owner_id.to_string();
    let source = diagram
        .nodes
        .iter()
        .find(|node| node.activity_node_id == source_owner)
        .ok_or("source Activity endpoint owner is not presented on this diagram")?;
    let target = diagram
        .nodes
        .iter()
        .find(|node| node.activity_node_id == target_owner)
        .ok_or("target Activity endpoint owner is not presented on this diagram")?;
    let source_rect = route_rect(source);
    let target_rect = route_rect(target);
    let obstacles: Vec<_> = diagram
        .nodes
        .iter()
        .filter(|node| node.id != source.id && node.id != target.id)
        .map(route_rect)
        .chain(activity_region_obstacles(
            diagram,
            activity,
            Some(source_owner_id),
            Some(target_owner_id),
        ))
        .chain(additional_obstacles.iter().copied())
        .collect();
    routing::orthogonal_route(routing::RouteRequest {
        source: source_rect,
        target: target_rect,
        obstacles: &obstacles,
        lane_index,
        reserved_routes,
        allow_shared_departure,
        bounds,
    })
}

fn reroute_diagram(
    diagram: &mut activity_workspace::ActivityDiagram,
    activity: &Activity,
    bounds: Option<routing::RouteRect>,
) -> Result<(), String> {
    let snapshot = diagram.clone();
    let mut reserved_routes = Vec::new();
    let mut reserved_labels = Vec::new();
    let all_regions = activity_region_obstacles(&snapshot, activity, None, None);
    let all_obstacles: Vec<_> = snapshot
        .nodes
        .iter()
        .map(route_rect)
        .chain(all_regions)
        .collect();
    let mut routed = Vec::new();
    for (index, presentation) in snapshot.edges.iter().enumerate() {
        let semantic = activity
            .edges
            .iter()
            .find(|edge| edge.id.to_string() == presentation.activity_edge_id)
            .ok_or("Activity presentation edge references missing semantic edge")?;
        // Separate flows that actually branch from or converge on the same
        // endpoint. Unrelated flows can reuse lane zero, avoiding the large
        // perimeter detours caused by a diagram-global monotonically growing
        // lane index.
        let lane_index = snapshot.edges[..index]
            .iter()
            .filter(|candidate| {
                candidate.source_node_id == presentation.source_node_id
                    || candidate.target_node_id == presentation.target_node_id
                    || candidate.source_node_id == presentation.target_node_id
                    || candidate.target_node_id == presentation.source_node_id
            })
            .count();
        let allow_shared_departure = snapshot.edges[..index]
            .iter()
            .any(|edge| edge.source_node_id == presentation.source_node_id);
        let points = route_semantic_edge(
            &snapshot,
            activity,
            semantic,
            lane_index,
            &reserved_routes,
            allow_shared_departure,
            &reserved_labels,
            bounds,
        )?;
        let label_obstacles: Vec<_> = all_obstacles
            .iter()
            .copied()
            .chain(reserved_labels.iter().copied())
            .collect();
        let label_anchor = routing::route_label_anchor_avoiding(
            &points,
            &label_obstacles,
            &reserved_routes,
            bounds,
        )?;
        reserved_routes.push(points.clone());
        reserved_labels.push(routing::label_rect(label_anchor));
        routed.push((presentation.id.clone(), points, label_anchor));
    }
    for (id, points, label_anchor) in routed {
        let presentation = diagram
            .edges
            .iter_mut()
            .find(|edge| edge.id == id)
            .ok_or("Activity edge presentation not found")?;
        presentation.points = points;
        presentation.label_anchor = Some(label_anchor);
    }
    Ok(())
}

fn pin_ids_for_node(activity: &Activity, node_id: ActivityNodeId) -> HashSet<PinId> {
    activity
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .and_then(|node| match &node.kind {
            ActivityNodeKind::Action(action) => {
                Some(action.pins.iter().map(|pin| pin.id).collect())
            }
            _ => None,
        })
        .unwrap_or_default()
}

fn endpoint_is_incident(
    endpoint: ActivityEndpoint,
    node_id: ActivityNodeId,
    pin_ids: &HashSet<PinId>,
) -> bool {
    match endpoint {
        ActivityEndpoint::Node(id) => id == node_id,
        ActivityEndpoint::Pin(id) => pin_ids.contains(&id),
    }
}

#[tauri::command]
pub fn delete_activity_item(
    diagram_id: String,
    item_kind: String,
    item_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity_state: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
) -> Result<(), String> {
    let project_guard = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_ref().ok_or("no project open")?;
    let mut diagrams = activity_state
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("Activity diagram not found")?;
    let activity_id = activity_workspace::parse_activity_id(&diagram.activity_id)?;
    let original_diagram = diagram.clone();
    let mut repository = activity_state
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?;
    let original_activity = repository
        .activities
        .get(&activity_id)
        .cloned()
        .ok_or("Activity not found")?;

    {
        let activity = repository
            .activities
            .get_mut(&activity_id)
            .ok_or("Activity not found")?;
        match item_kind.as_str() {
            "edge" => {
                let edge_id = parse_edge_id(&item_id)?;
                let before = activity.edges.len();
                activity.edges.retain(|edge| edge.id != edge_id);
                if before == activity.edges.len() {
                    return Err("Activity edge not found".into());
                }
                diagram
                    .edges
                    .retain(|edge| edge.activity_edge_id != edge_id.to_string());
            }
            "node" => {
                let node_id = activity_workspace::parse_activity_node_id(&item_id)?;
                if !activity.nodes.iter().any(|node| node.id == node_id) {
                    return Err("Activity node not found".into());
                }
                let pins = pin_ids_for_node(activity, node_id);
                let incident: HashSet<_> = activity
                    .edges
                    .iter()
                    .filter(|edge| {
                        endpoint_is_incident(edge.source, node_id, &pins)
                            || endpoint_is_incident(edge.target, node_id, &pins)
                    })
                    .map(|edge| edge.id)
                    .collect();
                let incident_strings: HashSet<_> =
                    incident.iter().map(ToString::to_string).collect();
                activity.edges.retain(|edge| !incident.contains(&edge.id));
                activity.nodes.retain(|node| node.id != node_id);
                let removed_presentations: HashSet<_> = diagram
                    .nodes
                    .iter()
                    .filter(|node| node.activity_node_id == node_id.to_string())
                    .map(|node| node.id.clone())
                    .collect();
                diagram
                    .nodes
                    .retain(|node| node.activity_node_id != node_id.to_string());
                diagram.edges.retain(|edge| {
                    !incident_strings.contains(&edge.activity_edge_id)
                        && !removed_presentations.contains(&edge.source_node_id)
                        && !removed_presentations.contains(&edge.target_node_id)
                });
            }
            _ => return Err(format!("unsupported Activity delete kind: {item_kind}")),
        }
    }

    if let Err(error) = repository
        .validate(project)
        .map_err(|error| error.to_string())
    {
        repository.activities.insert(activity_id, original_activity);
        *diagram = original_diagram;
        return Err(error);
    }
    let activity = repository
        .activities
        .get(&activity_id)
        .ok_or("Activity not found")?;
    reroute_diagram(diagram, activity, None)
}

#[tauri::command]
pub fn reconnect_activity_edge(
    diagram_id: String,
    activity_edge_id: String,
    source_endpoint: String,
    target_endpoint: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity_state: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
) -> Result<(), String> {
    let edge_id = parse_edge_id(&activity_edge_id)?;
    let project_guard = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_ref().ok_or("no project open")?;
    let mut diagrams = activity_state
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("Activity diagram not found")?;
    let activity_id = activity_workspace::parse_activity_id(&diagram.activity_id)?;
    let original_diagram = diagram.clone();
    let mut repository = activity_state
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?;
    let original_activity = repository
        .activities
        .get(&activity_id)
        .cloned()
        .ok_or("Activity not found")?;

    let (source, target) = {
        let activity = repository
            .activities
            .get(&activity_id)
            .ok_or("Activity not found")?;
        let source = parse_endpoint(activity, &source_endpoint)?;
        let target = parse_endpoint(activity, &target_endpoint)?;
        if source == target
            || endpoint_owner(activity, source)? == endpoint_owner(activity, target)?
        {
            return Err("Activity flow requires distinct source and target nodes".into());
        }
        (source, target)
    };
    {
        let activity = repository
            .activities
            .get_mut(&activity_id)
            .ok_or("Activity not found")?;
        let edge = activity
            .edges
            .iter_mut()
            .find(|edge| edge.id == edge_id)
            .ok_or("Activity edge not found")?;
        edge.source = source;
        edge.target = target;
    }

    if let Err(error) = repository
        .validate(project)
        .map_err(|error| error.to_string())
    {
        repository.activities.insert(activity_id, original_activity);
        *diagram = original_diagram;
        return Err(error);
    }
    let activity = repository
        .activities
        .get(&activity_id)
        .ok_or("Activity not found")?;
    let source_owner = endpoint_owner(activity, source)?.to_string();
    let target_owner = endpoint_owner(activity, target)?.to_string();
    let source_presentation_id = diagram
        .nodes
        .iter()
        .find(|node| node.activity_node_id == source_owner)
        .ok_or("source Activity endpoint owner is not presented on this diagram")?
        .id
        .clone();
    let target_presentation_id = diagram
        .nodes
        .iter()
        .find(|node| node.activity_node_id == target_owner)
        .ok_or("target Activity endpoint owner is not presented on this diagram")?
        .id
        .clone();
    let presentation = diagram
        .edges
        .iter_mut()
        .find(|edge| edge.activity_edge_id == edge_id.to_string())
        .ok_or("Activity edge is not presented on this diagram")?;
    presentation.source_node_id = source_presentation_id;
    presentation.target_node_id = target_presentation_id;
    reroute_diagram(diagram, activity, None)
}

#[tauri::command]
pub fn route_activity_diagram(
    diagram_id: String,
    activity_state: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
) -> Result<(), String> {
    route_activity_with_bounds(&diagram_id, &activity_state, None)?;
    Ok(())
}

fn activity_presentation_changed(
    left: &activity_workspace::ActivityDiagram,
    right: &activity_workspace::ActivityDiagram,
) -> bool {
    left.nodes.len() != right.nodes.len()
        || left.edges.len() != right.edges.len()
        || left.nodes.iter().zip(&right.nodes).any(|(left, right)| {
            left.id != right.id
                || left.x != right.x
                || left.y != right.y
                || left.width != right.width
                || left.height != right.height
        })
        || left.edges.iter().zip(&right.edges).any(|(left, right)| {
            left.id != right.id
                || left.points != right.points
                || left.label_anchor != right.label_anchor
        })
}

pub(super) fn route_activity_with_bounds(
    diagram_id: &str,
    activity_state: &activity_workspace::ActivityWorkspaceState,
    bounds: Option<routing::RouteRect>,
) -> Result<bool, String> {
    let mut diagrams = activity_state
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?;
    let index = diagrams
        .iter()
        .position(|diagram| diagram.id == diagram_id)
        .ok_or("Activity diagram not found")?;
    let original = diagrams[index].clone();
    let mut candidate = original.clone();
    let activity_id = activity_workspace::parse_activity_id(&candidate.activity_id)?;
    let repository = activity_state
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?;
    let activity = repository
        .activities
        .get(&activity_id)
        .ok_or("Activity not found")?;
    reroute_diagram(&mut candidate, activity, bounds)?;
    let changed = activity_presentation_changed(&original, &candidate);
    if changed {
        diagrams[index] = candidate;
    }
    Ok(changed)
}

pub fn layout_activity_diagram(
    diagram_id: String,
    activity_state: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
) -> Result<(), String> {
    layout_activity_with_bounds(&diagram_id, &activity_state, None)?;
    Ok(())
}

pub(super) fn layout_activity_with_bounds(
    diagram_id: &str,
    activity_state: &activity_workspace::ActivityWorkspaceState,
    bounds: Option<routing::RouteRect>,
) -> Result<bool, String> {
    let mut diagrams = activity_state
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?;
    let index = diagrams
        .iter()
        .position(|diagram| diagram.id == diagram_id)
        .ok_or("Activity diagram not found")?;
    let original = diagrams[index].clone();
    let mut candidate = original.clone();
    let edges: Vec<_> = candidate
        .edges
        .iter()
        .map(|edge| (edge.source_node_id.clone(), edge.target_node_id.clone()))
        .collect();
    let positions = super::layout::hierarchical_positions_sized(
        candidate
            .nodes
            .iter()
            .map(|node| super::layout::LayoutNode {
                id: node.id.clone(),
                width: node.width,
                height: node.height,
            }),
        &edges,
        systems_modeler_core::PreferredFlowDirection::TopToBottom,
    );
    for node in &mut candidate.nodes {
        if let Some((x, y)) = positions.get(&node.id) {
            node.x = *x;
            node.y = *y;
        }
    }
    let activity_id = activity_workspace::parse_activity_id(&candidate.activity_id)?;
    let repository = activity_state
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?;
    let activity = repository
        .activities
        .get(&activity_id)
        .ok_or("Activity not found")?;
    reroute_diagram(&mut candidate, activity, bounds)?;
    let changed = activity_presentation_changed(&original, &candidate);
    if changed {
        diagrams[index] = candidate;
    }
    Ok(changed)
}
