use super::*;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use systems_modeler_core::{
    Action, ActionKind, ActivityEdge, ActivityEdgeId, ActivityEdgeKind, ActivityEndpoint, ActivityId,
    ActivityNode, ActivityNodeId, ActivityNodeKind, ActivityRepository, ObjectNode, ObjectNodeKind,
    ObjectNodeOrdering,
};
use systems_modeler_persistence::{load_activity_repository, save_activity_repository};

const ACTIVITY_DIAGRAM_METADATA_KEY: &str = "activity-diagrams";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityDiagramNode {
    pub id: String,
    pub activity_node_id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityDiagramEdge {
    pub id: String,
    pub activity_edge_id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub points: Vec<DiagramPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityDiagram {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub activity_id: String,
    pub nodes: Vec<ActivityDiagramNode>,
    pub edges: Vec<ActivityDiagramEdge>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActivityWorkspaceSnapshot {
    pub repository: ActivityRepository,
    pub diagrams: Vec<ActivityDiagram>,
}

pub struct ActivityWorkspaceState {
    repository: Mutex<ActivityRepository>,
    diagrams: Mutex<Vec<ActivityDiagram>>,
}

impl Default for ActivityWorkspaceState {
    fn default() -> Self {
        Self {
            repository: Mutex::new(ActivityRepository::default()),
            diagrams: Mutex::new(Vec::new()),
        }
    }
}

fn parse_activity_id(value: &str) -> Result<ActivityId, String> {
    uuid::Uuid::parse_str(value)
        .map(ActivityId)
        .map_err(|_| format!("invalid activity id: {value}"))
}

fn parse_activity_node_id(value: &str) -> Result<ActivityNodeId, String> {
    uuid::Uuid::parse_str(value)
        .map(ActivityNodeId)
        .map_err(|_| format!("invalid activity node id: {value}"))
}

fn activity_node_size(kind: &ActivityNodeKind) -> (f64, f64) {
    match kind {
        ActivityNodeKind::Initial
        | ActivityNodeKind::ActivityFinal
        | ActivityNodeKind::FlowFinal => (24.0, 24.0),
        ActivityNodeKind::Decision { .. } | ActivityNodeKind::Merge => (28.0, 28.0),
        ActivityNodeKind::Fork | ActivityNodeKind::Join { .. } => (90.0, 12.0),
        ActivityNodeKind::Object(_) | ActivityNodeKind::ActivityParameter(_) => (130.0, 48.0),
        ActivityNodeKind::Action(_) => (150.0, 72.0),
    }
}

fn node_rect(node: &ActivityDiagramNode) -> routing::RouteRect {
    routing::RouteRect {
        x: node.x,
        y: node.y,
        width: node.width,
        height: node.height,
    }
}

fn validate_activity_diagrams(
    repository: &ActivityRepository,
    diagrams: &[ActivityDiagram],
) -> Result<(), String> {
    let mut diagram_ids = HashSet::new();
    let mut presentation_ids = HashSet::new();
    let mut presentation_edge_ids = HashSet::new();
    for diagram in diagrams {
        if uuid::Uuid::parse_str(&diagram.id).is_err() || !diagram_ids.insert(&diagram.id) {
            return Err(format!("invalid or duplicate Activity diagram id: {}", diagram.id));
        }
        let activity_id = parse_activity_id(&diagram.activity_id)?;
        let activity = repository
            .activities
            .get(&activity_id)
            .ok_or_else(|| format!("Activity diagram references missing activity: {activity_id}"))?;
        for node in &diagram.nodes {
            if uuid::Uuid::parse_str(&node.id).is_err() || !presentation_ids.insert(&node.id) {
                return Err(format!("invalid or duplicate Activity presentation node: {}", node.id));
            }
            let semantic_id = parse_activity_node_id(&node.activity_node_id)?;
            if !activity.nodes.iter().any(|candidate| candidate.id == semantic_id) {
                return Err(format!("Activity presentation references missing node: {semantic_id}"));
            }
        }
        for edge in &diagram.edges {
            if uuid::Uuid::parse_str(&edge.id).is_err() || !presentation_edge_ids.insert(&edge.id) {
                return Err(format!("invalid or duplicate Activity presentation edge: {}", edge.id));
            }
            let semantic_edge_id = uuid::Uuid::parse_str(&edge.activity_edge_id)
                .map(ActivityEdgeId)
                .map_err(|_| format!("invalid Activity edge id: {}", edge.activity_edge_id))?;
            if !activity.edges.iter().any(|candidate| candidate.id == semantic_edge_id) {
                return Err(format!("Activity presentation references missing edge: {semantic_edge_id}"));
            }
            if !diagram.nodes.iter().any(|node| node.id == edge.source_node_id)
                || !diagram.nodes.iter().any(|node| node.id == edge.target_node_id)
            {
                return Err("Activity presentation edge references a missing presentation node".into());
            }
            if edge.points.len() < 2 || !routing::route_is_clear(&edge.points, &[]) {
                return Err(format!("Activity presentation edge has invalid route: {}", edge.id));
            }
        }
    }
    Ok(())
}

pub fn save_activity_workspace_metadata(
    database: &mut ProjectDatabase,
    project: &Project,
    repository: &ActivityRepository,
    diagrams: &[ActivityDiagram],
) -> Result<(), String> {
    repository.validate(project).map_err(|error| error.to_string())?;
    validate_activity_diagrams(repository, diagrams)?;
    save_activity_repository(database, project, repository).map_err(|error| error.to_string())?;
    let payload = serde_json::to_string(diagrams).map_err(|error| error.to_string())?;
    database
        .save_metadata(project.id, ACTIVITY_DIAGRAM_METADATA_KEY, &payload)
        .map_err(|error| error.to_string())
}

pub fn load_activity_workspace_metadata(
    database: &ProjectDatabase,
    project: &Project,
) -> Result<(ActivityRepository, Vec<ActivityDiagram>), String> {
    let repository = load_activity_repository(database, project).map_err(|error| error.to_string())?;
    let diagrams = match database
        .load_metadata(project.id, ACTIVITY_DIAGRAM_METADATA_KEY)
        .map_err(|error| error.to_string())?
    {
        Some(payload) => serde_json::from_str::<Vec<ActivityDiagram>>(&payload)
            .map_err(|error| format!("invalid saved Activity presentation data: {error}"))?,
        None => Vec::new(),
    };
    validate_activity_diagrams(&repository, &diagrams)?;
    Ok((repository, diagrams))
}

#[tauri::command]
pub fn activity_snapshot(
    state: tauri::State<'_, ActivityWorkspaceState>,
) -> Result<ActivityWorkspaceSnapshot, String> {
    let repository = state
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?;
    let diagrams = state
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?;
    Ok(ActivityWorkspaceSnapshot {
        repository: repository.clone(),
        diagrams: diagrams.clone(),
    })
}

#[tauri::command]
pub fn reset_activity_workspace(
    state: tauri::State<'_, ActivityWorkspaceState>,
) -> Result<(), String> {
    *state
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")? = ActivityRepository::default();
    state
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?
        .clear();
    Ok(())
}

#[tauri::command]
pub fn create_activity_diagram(
    owner_id: String,
    context_id: Option<String>,
    name: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity_state: tauri::State<'_, ActivityWorkspaceState>,
) -> Result<String, String> {
    let owner_id = parse_element_id(&owner_id)?;
    let context_id = context_id.as_deref().map(parse_element_id).transpose()?;
    let project_guard = workspace.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_ref().ok_or("no project open")?;
    let mut repository = activity_state
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?;
    let activity_id = repository
        .create_activity(project, owner_id, context_id, name.clone())
        .map_err(|error| error.to_string())?;
    let diagram_id = DiagramId::new();
    activity_state
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?
        .push(ActivityDiagram {
            id: diagram_id.to_string(),
            name,
            owner_id: owner_id.to_string(),
            activity_id: activity_id.to_string(),
            nodes: Vec::new(),
            edges: Vec::new(),
        });
    Ok(diagram_id.to_string())
}

fn make_activity_node(kind: &str, name: String) -> Result<ActivityNode, String> {
    let kind = match kind {
        "Initial" => ActivityNodeKind::Initial,
        "ActivityFinal" => ActivityNodeKind::ActivityFinal,
        "FlowFinal" => ActivityNodeKind::FlowFinal,
        "Decision" => ActivityNodeKind::Decision { decision_input: None },
        "Merge" => ActivityNodeKind::Merge,
        "Fork" => ActivityNodeKind::Fork,
        "Join" => ActivityNodeKind::Join {
            join_specification: None,
        },
        "OpaqueAction" => ActivityNodeKind::Action(Action {
            kind: ActionKind::Opaque { body: String::new() },
            pins: Vec::new(),
        }),
        "ObjectNode" => ActivityNodeKind::Object(ObjectNode {
            kind: ObjectNodeKind::Object,
            type_id: None,
            multiplicity: Multiplicity::ONE,
            ordering: ObjectNodeOrdering::Unordered,
            selection: None,
        }),
        "CentralBufferNode" => ActivityNodeKind::Object(ObjectNode {
            kind: ObjectNodeKind::CentralBuffer,
            type_id: None,
            multiplicity: Multiplicity::ONE,
            ordering: ObjectNodeOrdering::Unordered,
            selection: None,
        }),
        "DataStoreNode" => ActivityNodeKind::Object(ObjectNode {
            kind: ObjectNodeKind::DataStore,
            type_id: None,
            multiplicity: Multiplicity::ONE,
            ordering: ObjectNodeOrdering::Unordered,
            selection: None,
        }),
        _ => return Err(format!("unsupported Activity node kind: {kind}")),
    };
    Ok(ActivityNode {
        id: ActivityNodeId::new(),
        name,
        kind,
        partition_id: None,
        structured_node_id: None,
    })
}

#[tauri::command]
pub fn add_activity_node(
    diagram_id: String,
    kind: String,
    name: String,
    x: f64,
    y: f64,
    activity_state: tauri::State<'_, ActivityWorkspaceState>,
) -> Result<String, String> {
    let mut diagrams = activity_state
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("Activity diagram not found")?;
    let activity_id = parse_activity_id(&diagram.activity_id)?;
    let node = make_activity_node(&kind, name)?;
    let node_id = node.id;
    let (width, height) = activity_node_size(&node.kind);
    let mut repository = activity_state
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?;
    repository
        .activities
        .get_mut(&activity_id)
        .ok_or("Activity not found")?
        .nodes
        .push(node);
    let presentation_id = uuid::Uuid::new_v4().to_string();
    diagram.nodes.push(ActivityDiagramNode {
        id: presentation_id,
        activity_node_id: node_id.to_string(),
        x,
        y,
        width,
        height,
    });
    Ok(node_id.to_string())
}

#[tauri::command]
pub fn add_activity_edge(
    diagram_id: String,
    kind: String,
    source_activity_node_id: String,
    target_activity_node_id: String,
    guard: Option<String>,
    weight: Option<String>,
    activity_state: tauri::State<'_, ActivityWorkspaceState>,
) -> Result<String, String> {
    let source_id = parse_activity_node_id(&source_activity_node_id)?;
    let target_id = parse_activity_node_id(&target_activity_node_id)?;
    if source_id == target_id {
        return Err("Activity flow cannot connect a node to itself".into());
    }
    let edge_kind = match kind.as_str() {
        "ControlFlow" => ActivityEdgeKind::ControlFlow,
        "ObjectFlow" => ActivityEdgeKind::ObjectFlow,
        _ => return Err(format!("unsupported Activity edge kind: {kind}")),
    };
    let mut diagrams = activity_state
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("Activity diagram not found")?;
    let activity_id = parse_activity_id(&diagram.activity_id)?;
    let source_presentation = diagram
        .nodes
        .iter()
        .find(|node| node.activity_node_id == source_activity_node_id)
        .cloned()
        .ok_or("source Activity node is not presented on this diagram")?;
    let target_presentation = diagram
        .nodes
        .iter()
        .find(|node| node.activity_node_id == target_activity_node_id)
        .cloned()
        .ok_or("target Activity node is not presented on this diagram")?;

    let mut repository = activity_state
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?;
    let activity = repository
        .activities
        .get_mut(&activity_id)
        .ok_or("Activity not found")?;
    if !activity.nodes.iter().any(|node| node.id == source_id)
        || !activity.nodes.iter().any(|node| node.id == target_id)
    {
        return Err("Activity flow endpoint is not owned by this Activity".into());
    }
    let edge_id = ActivityEdgeId::new();
    activity.edges.push(ActivityEdge {
        id: edge_id,
        name: String::new(),
        kind: edge_kind,
        source: ActivityEndpoint::Node(source_id),
        target: ActivityEndpoint::Node(target_id),
        guard,
        weight,
        selection: None,
        transformation: None,
        interrupting_region_id: None,
    });

    let obstacles: Vec<_> = diagram
        .nodes
        .iter()
        .filter(|node| node.id != source_presentation.id && node.id != target_presentation.id)
        .map(node_rect)
        .collect();
    let lane_index = diagram
        .edges
        .iter()
        .filter(|edge| {
            edge.source_node_id == source_presentation.id
                && edge.target_node_id == target_presentation.id
        })
        .count();
    let points = routing::orthogonal_route(routing::RouteRequest {
        source: node_rect(&source_presentation),
        target: node_rect(&target_presentation),
        obstacles: &obstacles,
        lane_index,
    });
    diagram.edges.push(ActivityDiagramEdge {
        id: uuid::Uuid::new_v4().to_string(),
        activity_edge_id: edge_id.to_string(),
        source_node_id: source_presentation.id,
        target_node_id: target_presentation.id,
        points,
    });
    Ok(edge_id.to_string())
}

#[tauri::command]
pub fn save_activity_workspace(
    path: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity_state: tauri::State<'_, ActivityWorkspaceState>,
) -> Result<String, String> {
    let path = normalize_project_path(&path)?;
    let project_guard = workspace.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_ref().ok_or("no project open")?;
    let repository = activity_state
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?;
    let diagrams = activity_state
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?;
    let mut database = ProjectDatabase::open(&path).map_err(|error| error.to_string())?;
    save_activity_workspace_metadata(&mut database, project, &repository, &diagrams)?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn load_activity_workspace(
    path: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity_state: tauri::State<'_, ActivityWorkspaceState>,
) -> Result<(), String> {
    let path = normalize_project_path(&path)?;
    let database = ProjectDatabase::open(&path).map_err(|error| error.to_string())?;
    let project_guard = workspace.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_ref().ok_or("open the project before loading Activity metadata")?;
    let (repository, diagrams) = load_activity_workspace_metadata(&database, project)?;
    *activity_state
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")? = repository;
    *activity_state
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")? = diagrams;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_route_reuses_shared_obstacle_router() {
        let source = ActivityDiagramNode {
            id: "s".into(),
            activity_node_id: ActivityNodeId::new().to_string(),
            x: 0.0,
            y: 80.0,
            width: 120.0,
            height: 60.0,
        };
        let target = ActivityDiagramNode {
            id: "t".into(),
            activity_node_id: ActivityNodeId::new().to_string(),
            x: 420.0,
            y: 80.0,
            width: 120.0,
            height: 60.0,
        };
        let obstacle = routing::RouteRect {
            x: 190.0,
            y: 60.0,
            width: 120.0,
            height: 100.0,
        };
        let route = routing::orthogonal_route(routing::RouteRequest {
            source: node_rect(&source),
            target: node_rect(&target),
            obstacles: &[obstacle],
            lane_index: 0,
        });
        assert!(routing::route_is_clear(&route, &[obstacle]));
        assert!(route.windows(2).all(|segment| {
            (segment[0].x - segment[1].x).abs() < f64::EPSILON
                || (segment[0].y - segment[1].y).abs() < f64::EPSILON
        }));
    }
}
