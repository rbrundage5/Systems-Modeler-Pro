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

fn rect_overlaps_corridor(
    rect: routing::RouteRect,
    source: routing::RouteRect,
    target: routing::RouteRect,
) -> bool {
    const CORRIDOR_PADDING: f64 = 72.0;
    let left = source.x.min(target.x) - CORRIDOR_PADDING;
    let right = (source.x + source.width)
        .max(target.x + target.width)
        + CORRIDOR_PADDING;
    let top = source.y.min(target.y) - CORRIDOR_PADDING;
    let bottom = (source.y + source.height)
        .max(target.y + target.height)
        + CORRIDOR_PADDING;
    rect.x + rect.width >= left
        && rect.x <= right
        && rect.y + rect.height >= top
        && rect.y <= bottom
}

fn route_semantic_edge(
    diagram: &activity_workspace::ActivityDiagram,
    activity: &Activity,
    edge: &ActivityEdge,
    lane_index: usize,
) -> Result<Vec<DiagramPoint>, String> {
    let source_owner = endpoint_owner(activity, edge.source)?.to_string();
    let target_owner = endpoint_owner(activity, edge.target)?.to_string();
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
        .filter(|rect| rect_overlaps_corridor(*rect, source_rect, target_rect))
        .collect();
    Ok(routing::orthogonal_route(routing::RouteRequest {
        source: source_rect,
        target: target_rect,
        obstacles: &obstacles,
        lane_index,
    }))
}

fn reroute_diagram(
    diagram: &mut activity_workspace::ActivityDiagram,
    activity: &Activity,
) -> Result<(), String> {
    let snapshot = diagram.clone();
    for (index, presentation) in diagram.edges.iter_mut().enumerate() {
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
        presentation.points = route_semantic_edge(&snapshot, activity, semantic, lane_index)?;
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
    reroute_diagram(diagram, activity)
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
    reroute_diagram(diagram, activity)
}

#[tauri::command]
pub fn route_activity_diagram(
    diagram_id: String,
    activity_state: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
) -> Result<(), String> {
    let mut diagrams = activity_state
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("Activity diagram not found")?;
    let activity_id = activity_workspace::parse_activity_id(&diagram.activity_id)?;
    let repository = activity_state
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?;
    let activity = repository
        .activities
        .get(&activity_id)
        .ok_or("Activity not found")?;
    reroute_diagram(diagram, activity)
}