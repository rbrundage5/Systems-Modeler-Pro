use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use systems_modeler_core::{
    AggregationKind, BehaviorRepository, DiagramId, ElementId, ElementKind, Multiplicity, Project,
    Relationship, RelationshipId, RelationshipKind,
};
use systems_modeler_persistence::ProjectDatabase;

const BDD_METADATA_KEY: &str = "bdd-diagrams";

#[derive(Debug, Clone, Serialize)]
pub struct ElementSnapshot {
    pub id: String,
    pub external_id: String,
    pub kind: String,
    pub name: String,
    pub owner_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssociationEndSnapshot {
    pub id: String,
    pub classifier_id: String,
    pub role_name: String,
    pub multiplicity: String,
    pub navigable: bool,
    pub aggregation: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelationshipSnapshot {
    pub id: String,
    pub external_id: String,
    pub kind: String,
    pub source_id: String,
    pub target_id: String,
    pub association_ends: Vec<AssociationEndSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectSnapshot {
    pub id: String,
    pub name: String,
    pub root_id: String,
    pub elements: Vec<ElementSnapshot>,
    pub relationships: Vec<RelationshipSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DiagramPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramNode {
    pub id: String,
    pub element_id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramEdge {
    pub id: String,
    pub relationship_id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub points: Vec<DiagramPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BddDiagram {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub nodes: Vec<DiagramNode>,
    #[serde(default)]
    pub edges: Vec<DiagramEdge>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceSnapshot {
    pub project: Option<ProjectSnapshot>,
    pub diagrams: Vec<BddDiagram>,
    pub ibd_diagrams: Vec<ibd::IbdDiagram>,
    pub current_file: Option<String>,
}

pub struct WorkspaceState {
    project: Mutex<Option<Project>>,
    diagrams: Mutex<Vec<BddDiagram>>,
    ibd_diagrams: Mutex<Vec<ibd::IbdDiagram>>,
    behavior: Mutex<BehaviorRepository>,
    behavior_diagrams: Mutex<Vec<behavior_workspace::BehaviorDiagram>>,
    current_file: Mutex<Option<String>>,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self {
            project: Mutex::new(None),
            diagrams: Mutex::new(Vec::new()),
            ibd_diagrams: Mutex::new(Vec::new()),
            behavior: Mutex::new(BehaviorRepository::default()),
            behavior_diagrams: Mutex::new(Vec::new()),
            current_file: Mutex::new(None),
        }
    }
}

fn parse_element_id(value: &str) -> Result<ElementId, String> {
    uuid::Uuid::parse_str(value)
        .map(ElementId)
        .map_err(|_| format!("invalid element id: {value}"))
}

fn parse_diagram_id(value: &str) -> Result<DiagramId, String> {
    uuid::Uuid::parse_str(value)
        .map(DiagramId)
        .map_err(|_| format!("invalid diagram id: {value}"))
}

fn parse_relationship_id(value: &str) -> Result<RelationshipId, String> {
    uuid::Uuid::parse_str(value)
        .map(RelationshipId)
        .map_err(|_| format!("invalid relationship id: {value}"))
}

fn normalize_project_path(value: &str) -> Result<PathBuf, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("project path is required".into());
    }
    let mut path = PathBuf::from(trimmed);
    if path.extension().is_none() {
        path.set_extension("smproj");
    }
    Ok(path)
}

fn aggregation_name(value: AggregationKind) -> &'static str {
    match value {
        AggregationKind::None => "none",
        AggregationKind::Shared => "shared",
        AggregationKind::Composite => "composite",
    }
}

fn relationship_display_kind(relationship: &Relationship) -> &'static str {
    if relationship.kind == RelationshipKind::Association {
        if relationship
            .association_ends
            .iter()
            .any(|end| end.aggregation == AggregationKind::Composite)
        {
            return "Composition";
        }
        if relationship
            .association_ends
            .iter()
            .any(|end| end.aggregation == AggregationKind::Shared)
        {
            return "Aggregation";
        }
        return "Association";
    }
    match relationship.kind {
        RelationshipKind::Dependency => "Dependency",
        RelationshipKind::Association => "Association",
        RelationshipKind::Composition => "Composition",
        RelationshipKind::Generalization => "Generalization",
        RelationshipKind::Realization => "Realization",
        RelationshipKind::Connector => "Connector",
        RelationshipKind::ItemFlow => "ItemFlow",
    }
}

fn snapshot_project(project: &Project) -> ProjectSnapshot {
    let mut elements: Vec<_> = project
        .elements
        .values()
        .map(|element| ElementSnapshot {
            id: element.id.to_string(),
            external_id: element.external_id.clone(),
            kind: format!("{:?}", element.kind),
            name: element.name.clone(),
            owner_id: element.owner_id.map(|id| id.to_string()),
        })
        .collect();
    elements.sort_by(|a, b| a.name.cmp(&b.name));

    let mut relationships: Vec<_> = project
        .relationships
        .values()
        .map(|relationship| RelationshipSnapshot {
            id: relationship.id.to_string(),
            external_id: relationship.external_id.clone(),
            kind: relationship_display_kind(relationship).to_string(),
            source_id: relationship.source_id.to_string(),
            target_id: relationship.target_id.to_string(),
            association_ends: relationship
                .association_ends
                .iter()
                .map(|end| AssociationEndSnapshot {
                    id: end.id.to_string(),
                    classifier_id: end.classifier_id.to_string(),
                    role_name: end.role_name.clone(),
                    multiplicity: end.multiplicity.notation(),
                    navigable: end.navigable,
                    aggregation: aggregation_name(end.aggregation).to_string(),
                })
                .collect(),
        })
        .collect();
    relationships.sort_by(|a, b| a.id.cmp(&b.id));

    ProjectSnapshot {
        id: project.id.to_string(),
        name: project.name.clone(),
        root_id: project.root_id.to_string(),
        elements,
        relationships,
    }
}

fn validate_loaded_diagrams(project: &Project, diagrams: &[BddDiagram]) -> Result<(), String> {
    let mut diagram_ids = HashSet::new();
    let mut node_ids = HashSet::new();
    let mut edge_ids = HashSet::new();
    for diagram in diagrams {
        parse_diagram_id(&diagram.id)?;
        if !diagram_ids.insert(&diagram.id) {
            return Err(format!("duplicate diagram id: {}", diagram.id));
        }
        let owner_id = parse_element_id(&diagram.owner_id)?;
        let owner = project.element(owner_id).map_err(|error| error.to_string())?;
        if !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {
            return Err(format!("BDD owner is not a Model or Package: {}", diagram.owner_id));
        }
        for node in &diagram.nodes {
            if uuid::Uuid::parse_str(&node.id).is_err() {
                return Err(format!("invalid diagram node id: {}", node.id));
            }
            if !node_ids.insert(&node.id) {
                return Err(format!("duplicate diagram node id: {}", node.id));
            }
            let element_id = parse_element_id(&node.element_id)?;
            project.element(element_id).map_err(|error| error.to_string())?;
        }
        for edge in &diagram.edges {
            if uuid::Uuid::parse_str(&edge.id).is_err() {
                return Err(format!("invalid diagram edge id: {}", edge.id));
            }
            if !edge_ids.insert(&edge.id) {
                return Err(format!("duplicate diagram edge id: {}", edge.id));
            }
            let relationship_id = parse_relationship_id(&edge.relationship_id)?;
            let relationship = project.relationship(relationship_id).map_err(|error| error.to_string())?;
            if matches!(relationship.kind, RelationshipKind::Connector | RelationshipKind::ItemFlow) {
                return Err("Connector and ItemFlow presentations belong on an IBD, not a BDD".into());
            }
            let source = diagram.nodes.iter().find(|node| node.id == edge.source_node_id).ok_or_else(|| format!("edge source node not found: {}", edge.source_node_id))?;
            let target = diagram.nodes.iter().find(|node| node.id == edge.target_node_id).ok_or_else(|| format!("edge target node not found: {}", edge.target_node_id))?;
            if source.element_id != relationship.source_id.to_string() || target.element_id != relationship.target_id.to_string() {
                return Err(format!("diagram edge endpoints do not match semantic relationship: {}", edge.relationship_id));
            }
            if edge.points.len() < 2 {
                return Err(format!("diagram edge has no usable route: {}", edge.id));
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn workspace_snapshot(state: tauri::State<'_, WorkspaceState>) -> Result<WorkspaceSnapshot, String> {
    let project = state.project.lock().map_err(|_| "project lock poisoned")?;
    let diagrams = state.diagrams.lock().map_err(|_| "diagram lock poisoned")?;
    let ibd_diagrams = state.ibd_diagrams.lock().map_err(|_| "IBD lock poisoned")?;
    let current_file = state.current_file.lock().map_err(|_| "project path lock poisoned")?;
    Ok(WorkspaceSnapshot {
        project: project.as_ref().map(snapshot_project),
        diagrams: diagrams.clone(),
        ibd_diagrams: ibd_diagrams.clone(),
        current_file: current_file.clone(),
    })
}

#[tauri::command]
pub fn new_project(name: String, state: tauri::State<'_, WorkspaceState>) -> Result<(), String> {
    *state.project.lock().map_err(|_| "project lock poisoned")? = Some(Project::new(name));
    state.diagrams.lock().map_err(|_| "diagram lock poisoned")?.clear();
    state.ibd_diagrams.lock().map_err(|_| "IBD lock poisoned")?.clear();
    *state.behavior.lock().map_err(|_| "behavior lock poisoned")? = BehaviorRepository::default();
    state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")?.clear();
    *state.current_file.lock().map_err(|_| "project path lock poisoned")? = None;
    Ok(())
}

#[tauri::command]
pub fn save_project_file(path: String, state: tauri::State<'_, WorkspaceState>) -> Result<String, String> {
    let path = normalize_project_path(&path)?;
    let project = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project.as_ref().ok_or("no project open")?;
    let diagrams = state.diagrams.lock().map_err(|_| "diagram lock poisoned")?;
    let ibd_diagrams = state.ibd_diagrams.lock().map_err(|_| "IBD lock poisoned")?;
    project.validate().map_err(|error| format!("project validation failed: {error}"))?;
    validate_loaded_diagrams(project, &diagrams)?;
    ibd::validate_ibd_diagrams(project, &ibd_diagrams)?;
    let mut database = ProjectDatabase::open(&path).map_err(|error| error.to_string())?;
    database.save_project(project).map_err(|error| error.to_string())?;
    let diagram_payload = serde_json::to_string(&*diagrams).map_err(|error| error.to_string())?;
    database.save_metadata(project.id, BDD_METADATA_KEY, &diagram_payload).map_err(|error| error.to_string())?;
    ibd::save_ibd_metadata(&mut database, project, &ibd_diagrams)?;
    let behavior = state.behavior.lock().map_err(|_| "behavior lock poisoned")?;
    let behavior_diagrams = state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")?;
    behavior_workspace::save_behavior_metadata(&mut database, project, &behavior, &behavior_diagrams)?;
    let saved_path = path.to_string_lossy().into_owned();
    *state.current_file.lock().map_err(|_| "project path lock poisoned")? = Some(saved_path.clone());
    Ok(saved_path)
}

#[tauri::command]
pub fn save_current_project(state: tauri::State<'_, WorkspaceState>) -> Result<String, String> {
    let path = state.current_file.lock().map_err(|_| "project path lock poisoned")?.clone().ok_or("project has not been saved yet; use Save As")?;
    save_project_file(path, state)
}

#[tauri::command]
pub fn open_project_file(path: String, state: tauri::State<'_, WorkspaceState>) -> Result<String, String> {
    let path = normalize_project_path(&path)?;
    if !path.exists() {
        return Err(format!("project file does not exist: {}", path.display()));
    }
    let database = ProjectDatabase::open(&path).map_err(|error| error.to_string())?;
    let project = database.load_first_project().map_err(|error| error.to_string())?;
    project.validate().map_err(|error| format!("saved project validation failed: {error}"))?;
    let diagrams = match database.load_metadata(project.id, BDD_METADATA_KEY).map_err(|error| error.to_string())? {
        Some(payload) => serde_json::from_str::<Vec<BddDiagram>>(&payload).map_err(|error| format!("invalid saved BDD presentation data: {error}"))?,
        None => Vec::new(),
    };
    validate_loaded_diagrams(&project, &diagrams)?;
    let ibd_diagrams = ibd::load_ibd_metadata(&database, &project)?;
    let (behavior, behavior_diagrams) = behavior_workspace::load_behavior_metadata(&database, &project)?;
    let opened_path = path.to_string_lossy().into_owned();
    *state.project.lock().map_err(|_| "project lock poisoned")? = Some(project);
    *state.diagrams.lock().map_err(|_| "diagram lock poisoned")? = diagrams;
    *state.ibd_diagrams.lock().map_err(|_| "IBD lock poisoned")? = ibd_diagrams;
    *state.behavior.lock().map_err(|_| "behavior lock poisoned")? = behavior;
    *state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")? = behavior_diagrams;
    *state.current_file.lock().map_err(|_| "project path lock poisoned")? = Some(opened_path.clone());
    Ok(opened_path)
}

#[tauri::command]
pub fn create_package(owner_id: String, name: String, state: tauri::State<'_, WorkspaceState>) -> Result<String, String> {
    create_element(ElementKind::Package, owner_id, name, state)
}

#[tauri::command]
pub fn create_block(owner_id: String, name: String, state: tauri::State<'_, WorkspaceState>) -> Result<String, String> {
    create_element(ElementKind::Block, owner_id, name, state)
}

fn create_element(kind: ElementKind, owner_id: String, name: String, state: tauri::State<'_, WorkspaceState>) -> Result<String, String> {
    let owner_id = parse_element_id(&owner_id)?;
    let mut project = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project.as_mut().ok_or("no project open")?;
    project.create_element(kind, name, owner_id).map(|id| id.to_string()).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn rename_element(element_id: String, name: String, state: tauri::State<'_, WorkspaceState>) -> Result<(), String> {
    let element_id = parse_element_id(&element_id)?;
    let mut project = state.project.lock().map_err(|_| "project lock poisoned")?;
    project.as_mut().ok_or("no project open")?.rename_element(element_id, name).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_bdd(owner_id: String, name: String, state: tauri::State<'_, WorkspaceState>) -> Result<String, String> {
    let owner_id = parse_element_id(&owner_id)?;
    {
        let project = state.project.lock().map_err(|_| "project lock poisoned")?;
        let project = project.as_ref().ok_or("no project open")?;
        let owner = project.element(owner_id).map_err(|error| error.to_string())?;
        if !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {
            return Err("BDD owner must be a Model or Package in this workflow".into());
        }
    }
    let id = DiagramId::new();
    state.diagrams.lock().map_err(|_| "diagram lock poisoned")?.push(BddDiagram {
        id: id.to_string(), name, owner_id: owner_id.to_string(), nodes: Vec::new(), edges: Vec::new(),
    });
    Ok(id.to_string())
}

#[tauri::command]
pub fn place_element_on_bdd(diagram_id: String, element_id: String, x: f64, y: f64, state: tauri::State<'_, WorkspaceState>) -> Result<String, String> {
    let diagram_id = parse_diagram_id(&diagram_id)?;
    let element_id = parse_element_id(&element_id)?;
    {
        let project = state.project.lock().map_err(|_| "project lock poisoned")?;
        let element = project.as_ref().ok_or("no project open")?.element(element_id).map_err(|error| error.to_string())?;
        if element.kind != ElementKind::Block {
            return Err("only Blocks can be presented on this legacy BDD command".into());
        }
    }
    let mut diagrams = state.diagrams.lock().map_err(|_| "diagram lock poisoned")?;
    let diagram = diagrams.iter_mut().find(|diagram| diagram.id == diagram_id.to_string()).ok_or("diagram not found")?;
    if diagram.nodes.iter().any(|node| node.element_id == element_id.to_string()) {
        return Err("this Block is already presented on the BDD".into());
    }
    let node_id = uuid::Uuid::new_v4().to_string();
    diagram.nodes.push(DiagramNode { id: node_id.clone(), element_id: element_id.to_string(), x, y, width: 180.0, height: 105.0 });
    Ok(node_id)
}

fn supported_relationship_kind(value: &str) -> Result<&'static str, String> {
    match value {
        "Association" => Ok("Association"),
        "Aggregation" => Ok("Aggregation"),
        "Composition" => Ok("Composition"),
        "Generalization" => Ok("Generalization"),
        "Dependency" => Ok("Dependency"),
        "Realization" => Ok("Realization"),
        _ => Err(format!("unsupported BDD relationship kind: {value}")),
    }
}

fn semantic_duplicate(project: &Project, kind: &str, source_id: ElementId, target_id: ElementId) -> bool {
    project.relationships.values().any(|relationship| {
        relationship.source_id == source_id && relationship.target_id == target_id && relationship_display_kind(relationship) == kind
    })
}

#[tauri::command]
pub fn create_bdd_relationship(diagram_id: String, kind: String, source_element_id: String, target_element_id: String, state: tauri::State<'_, WorkspaceState>) -> Result<String, String> {
    let kind = supported_relationship_kind(&kind)?;
    let source_id = parse_element_id(&source_element_id)?;
    let target_id = parse_element_id(&target_element_id)?;
    if source_id == target_id { return Err(format!("{kind} cannot connect a Block to itself")); }
    let mut project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_mut().ok_or("no project open")?;
    let source = project.element(source_id).map_err(|error| error.to_string())?;
    let target = project.element(target_id).map_err(|error| error.to_string())?;
    if source.kind != ElementKind::Block || target.kind != ElementKind::Block { return Err(format!("{kind} requires Block endpoints on a BDD")); }
    if semantic_duplicate(project, kind, source_id, target_id) { return Err(format!("an equivalent {kind} already exists")); }
    let mut diagrams = state.diagrams.lock().map_err(|_| "diagram lock poisoned")?;
    let diagram = diagrams.iter_mut().find(|diagram| diagram.id == diagram_id).ok_or("diagram not found")?;
    let source_node = diagram.nodes.iter().find(|node| node.element_id == source_element_id).cloned().ok_or("source Block must be presented on the selected BDD")?;
    let target_node = diagram.nodes.iter().find(|node| node.element_id == target_element_id).cloned().ok_or("target Block must be presented on the selected BDD")?;
    let owner_id = Some(parse_element_id(&diagram.owner_id)?);
    let relationship_id = match kind {
        "Association" => project.create_association(owner_id, vec![Project::association_end(source_id, "", Multiplicity::ONE, true, AggregationKind::None), Project::association_end(target_id, "", Multiplicity::ONE, true, AggregationKind::None)]).map_err(|error| error.to_string())?,
        "Aggregation" => project.create_association(owner_id, vec![Project::association_end(source_id, "", Multiplicity::ONE, true, AggregationKind::Shared), Project::association_end(target_id, "", Multiplicity::ONE, true, AggregationKind::None)]).map_err(|error| error.to_string())?,
        "Composition" => project.create_association(owner_id, vec![Project::association_end(source_id, "", Multiplicity::ONE, true, AggregationKind::Composite), Project::association_end(target_id, "", Multiplicity::ONE, true, AggregationKind::None)]).map_err(|error| error.to_string())?,
        "Generalization" => project.create_relationship(RelationshipKind::Generalization, source_id, target_id, owner_id).map_err(|error| error.to_string())?,
        "Dependency" => project.create_relationship(RelationshipKind::Dependency, source_id, target_id, owner_id).map_err(|error| error.to_string())?,
        "Realization" => project.create_relationship(RelationshipKind::Realization, source_id, target_id, owner_id).map_err(|error| error.to_string())?,
        _ => unreachable!(),
    };
    let points = route_relationship(&source_node, &target_node, &diagram.nodes);
    let edge_id = uuid::Uuid::new_v4().to_string();
    diagram.edges.push(DiagramEdge { id: edge_id, relationship_id: relationship_id.to_string(), source_node_id: source_node.id, target_node_id: target_node.id, points });
    Ok(relationship_id.to_string())
}

fn route_relationship(source: &DiagramNode, target: &DiagramNode, nodes: &[DiagramNode]) -> Vec<DiagramPoint> {
    let obstacles: Vec<routing::RouteRect> = nodes.iter().filter(|node| node.id != source.id && node.id != target.id).map(|node| routing::RouteRect { x: node.x, y: node.y, width: node.width, height: node.height }).collect();
    routing::orthogonal_route(routing::RouteRequest {
        source: routing::RouteRect { x: source.x, y: source.y, width: source.width, height: source.height },
        target: routing::RouteRect { x: target.x, y: target.y, width: target.width, height: target.height },
        obstacles: &obstacles,
        lane_index: 0,
    })
}
