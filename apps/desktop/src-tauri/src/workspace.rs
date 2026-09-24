use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use systems_modeler_core::{
    AggregationKind, BehaviorRepository, DiagramId, ElementId, ElementKind,
    Multiplicity, Project, RelationshipId, RelationshipKind, VisibilityKind,
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
    pub qualified_name: String,
    pub packageable: bool,
    pub namespace: bool,
    pub visibility: String,
    pub documentation: String,
    pub requirement_id: Option<String>,
    pub requirement_text: Option<String>,
    pub extension_points: Vec<String>,
    pub use_case_specification: String,
    pub represented_classifier_id: Option<String>,
    pub constraint_expression: String,
    pub quantity_dimension: Option<String>,
    pub unit_symbol: Option<String>,
    pub unit_scale_to_base: f64,
    pub type_id: Option<String>,
    pub multiplicity: Option<String>,
    pub default_value: Option<String>,
    pub is_derived: bool,
    pub is_read_only: bool,
    pub quantity_kind_external_id: Option<String>,
    pub unit_external_id: Option<String>,
    pub applied_stereotypes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssociationEndSnapshot {
    pub id: String,
    pub property_id: Option<String>,
    pub decoration_side: Option<String>,
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
    pub name: String,
    pub owner_id: Option<String>,
    pub source_id: String,
    pub target_id: String,
    pub documentation: String,
    pub visibility: String,
    pub alias: Option<String>,
    pub association_ends: Vec<AssociationEndSnapshot>,
    pub extension_condition: Option<String>,
    pub extension_location: Option<String>,
    pub binding: Option<BindingConnectorSnapshot>,
    pub applied_stereotypes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BindingEndpointSnapshot {
    pub role_id: String,
    pub parameter_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BindingConnectorSnapshot {
    pub source: BindingEndpointSnapshot,
    pub target: BindingEndpointSnapshot,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectSnapshot {
    pub id: String,
    pub name: String,
    pub root_id: String,
    pub elements: Vec<ElementSnapshot>,
    pub relationships: Vec<RelationshipSnapshot>,
    pub profiles: systems_modeler_core::ProfileRepository,
}

pub use systems_modeler_core::structural_presentation::{
    BddDiagram, ConstraintParameterPresentation, DiagramEdge, DiagramNode, DiagramPoint,
    UseCaseSubjectBoundary, parametric_endpoint_matches, relationship_display_kind,
    validate_structural_diagrams as validate_loaded_diagrams,
};

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceSnapshot {
    pub project: Option<ProjectSnapshot>,
    pub diagrams: Vec<BddDiagram>,
    pub ibd_diagrams: Vec<ibd::IbdDiagram>,
    pub behavior_repository: BehaviorRepository,
    pub behavior_diagrams: Vec<behavior_workspace::BehaviorDiagram>,
    pub current_file: Option<String>,
}

pub struct WorkspaceState {
    project: Mutex<Option<Project>>,
    diagrams: Mutex<Vec<BddDiagram>>,
    ibd_diagrams: Mutex<Vec<ibd::IbdDiagram>>,
    behavior: Mutex<BehaviorRepository>,
    behavior_diagrams: Mutex<Vec<behavior_workspace::BehaviorDiagram>>,
    current_file: Mutex<Option<String>>,
    reqif_exchange: Mutex<reqif_interchange::ReqifExchangeState>,
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
            reqif_exchange: Mutex::new(reqif_interchange::ReqifExchangeState::default()),
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

fn visibility_name(value: VisibilityKind) -> &'static str {
    match value {
        VisibilityKind::Public => "public",
        VisibilityKind::Private => "private",
    }
}

fn qualified_element_name(project: &Project, element_id: ElementId) -> String {
    let mut names = Vec::new();
    let mut current = Some(element_id);
    let mut visited = HashSet::new();
    while let Some(id) = current {
        if !visited.insert(id) {
            break;
        }
        let Ok(element) = project.element(id) else {
            break;
        };
        names.push(element.name.clone());
        current = element.owner_id;
    }
    names.reverse();
    names.join("::")
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
            qualified_name: qualified_element_name(project, element.id),
            packageable: element.is_packageable(),
            namespace: element.is_namespace(),
            visibility: visibility_name(element.visibility).to_string(),
            documentation: element.documentation.clone(),
            requirement_id: element.requirement_id.clone(),
            requirement_text: element.requirement_text.clone(),
            extension_points: element.extension_points.clone(),
            use_case_specification: element.use_case_specification.clone(),
            represented_classifier_id: element.represented_classifier_id.map(|id| id.to_string()),
            constraint_expression: element.constraint_expression.clone(),
            quantity_dimension: element.quantity_dimension.clone(),
            unit_symbol: element.unit_symbol.clone(),
            unit_scale_to_base: element.unit_scale_to_base,
            type_id: element.type_id.map(|id| id.to_string()),
            multiplicity: element.multiplicity.map(|value| value.notation()),
            default_value: element.default_value.clone(),
            is_derived: element.is_derived,
            is_read_only: element.is_read_only,
            quantity_kind_external_id: element.quantity_kind_external_id.clone(),
            unit_external_id: element.unit_external_id.clone(),
            applied_stereotypes: element.applied_stereotypes.clone(),
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
            name: relationship.name.clone(),
            owner_id: relationship.owner_id.map(|id| id.to_string()),
            source_id: relationship.source_id.to_string(),
            target_id: relationship.target_id.to_string(),
            documentation: relationship.documentation.clone(),
            visibility: visibility_name(relationship.visibility).to_string(),
            alias: relationship.alias.clone(),
            association_ends: relationship
                .association_ends
                .iter()
                .enumerate()
                .map(|(index, end)| association_end_snapshot(relationship, index, end))
                .collect(),
            extension_condition: relationship.extension_condition.clone(),
            extension_location: relationship.extension_location.clone(),
            binding: relationship.binding.as_ref().map(|binding| BindingConnectorSnapshot {
                source: BindingEndpointSnapshot {
                    role_id: binding.source.role_id.to_string(),
                    parameter_id: binding.source.parameter_id.map(|id| id.to_string()),
                },
                target: BindingEndpointSnapshot {
                    role_id: binding.target.role_id.to_string(),
                    parameter_id: binding.target.parameter_id.map(|id| id.to_string()),
                },
            }),
            applied_stereotypes: relationship.applied_stereotypes.clone(),
        })
        .collect();
    relationships.sort_by(|a, b| a.id.cmp(&b.id));

    ProjectSnapshot {
        id: project.id.to_string(),
        name: project.name.clone(),
        root_id: project.root_id.to_string(),
        elements,
        relationships,
        profiles: project.profiles.clone(),
    }
}


#[tauri::command]
pub fn workspace_snapshot(state: tauri::State<'_, WorkspaceState>) -> Result<WorkspaceSnapshot, String> {
    let project = state.project.lock().map_err(|_| "project lock poisoned")?;
    let diagrams = state.diagrams.lock().map_err(|_| "diagram lock poisoned")?;
    let ibd_diagrams = state.ibd_diagrams.lock().map_err(|_| "IBD lock poisoned")?;
    let behavior_repository = state.behavior.lock().map_err(|_| "behavior lock poisoned")?;
    let behavior_diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let current_file = state.current_file.lock().map_err(|_| "project path lock poisoned")?;
    Ok(WorkspaceSnapshot {
        project: project.as_ref().map(snapshot_project),
        diagrams: diagrams.clone(),
        ibd_diagrams: ibd_diagrams.clone(),
        behavior_repository: behavior_repository.clone(),
        behavior_diagrams: behavior_diagrams.clone(),
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
    *state.reqif_exchange.lock().map_err(|_| "ReqIF exchange lock poisoned")? = reqif_interchange::ReqifExchangeState::default();
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
    let reqif_exchange = state.reqif_exchange.lock().map_err(|_| "ReqIF exchange lock poisoned")?;
    reqif_runtime::save_reqif_metadata(&mut database, project, &reqif_exchange)?;
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
    let reqif_exchange = reqif_runtime::load_reqif_metadata(&database, &project)?;
    let opened_path = path.to_string_lossy().into_owned();
    *state.project.lock().map_err(|_| "project lock poisoned")? = Some(project);
    *state.diagrams.lock().map_err(|_| "diagram lock poisoned")? = diagrams;
    *state.ibd_diagrams.lock().map_err(|_| "IBD lock poisoned")? = ibd_diagrams;
    *state.behavior.lock().map_err(|_| "behavior lock poisoned")? = behavior;
    *state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")? = behavior_diagrams;
    *state.reqif_exchange.lock().map_err(|_| "ReqIF exchange lock poisoned")? = reqif_exchange;
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
        id: id.to_string(), name, owner_id: owner_id.to_string(), family: "bdd".into(), semantic_context_id: None, subject_boundary: None, nodes: Vec::new(), edges: Vec::new(),
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
    diagram.nodes.push(DiagramNode { id: node_id.clone(), element_id: element_id.to_string(), x, y, width: 180.0, height: 105.0, actor_notation: None, parameter_presentations: Vec::new() });
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
    bdd_elements::create_bdd_relationship_in_state(diagram_id, kind, source_element_id, target_element_id, &state)
}

fn association_end_snapshot(
    relationship: &systems_modeler_core::Relationship,
    index: usize,
    end: &systems_modeler_core::AssociationEnd,
) -> AssociationEndSnapshot {
    let canonical = relationship.association_ends.iter().any(|end| end.property_id.is_some());
    AssociationEndSnapshot {
        id: end.id.to_string(),
        property_id: end.property_id.map(|id| id.to_string()),
        decoration_side: (end.aggregation != AggregationKind::None).then(|| {
            if (index == 0) != canonical { "source" } else { "target" }.to_owned()
        }),
        classifier_id: end.classifier_id.to_string(),
        role_name: end.role_name.clone(),
        multiplicity: end.multiplicity.notation(),
        navigable: end.navigable,
        aggregation: aggregation_name(end.aggregation).to_string(),
    }
}

fn route_relationship(
    source: &DiagramNode,
    target: &DiagramNode,
    nodes: &[DiagramNode],
) -> Result<Vec<DiagramPoint>, String> {
    route_relationship_at_lane(source, target, nodes, 0)
}

fn route_relationship_at_lane(
    source: &DiagramNode,
    target: &DiagramNode,
    nodes: &[DiagramNode],
    lane_index: usize,
) -> Result<Vec<DiagramPoint>, String> {
    let obstacles: Vec<routing::RouteRect> = nodes.iter().filter(|node| node.id != source.id && node.id != target.id).map(|node| routing::RouteRect { x: node.x, y: node.y, width: node.width, height: node.height }).collect();
    routing::orthogonal_route(routing::RouteRequest {
        source: routing::RouteRect { x: source.x, y: source.y, width: source.width, height: source.height },
        target: routing::RouteRect { x: target.x, y: target.y, width: target.width, height: target.height },
        obstacles: &obstacles,
        lane_index,
        reserved_routes: &[],
        allow_shared_departure: false,
        bounds: None,
    })
}




fn bdd_presentation_changed(left: &BddDiagram, right: &BddDiagram) -> bool {
    left.nodes.len() != right.nodes.len()
        || left.edges.len() != right.edges.len()
        || left.nodes.iter().zip(&right.nodes).any(|(left, right)| {
            left.id != right.id
                || left.x != right.x
                || left.y != right.y
                || left.width != right.width
                || left.height != right.height
                || left.actor_notation != right.actor_notation
        })
        || left.edges.iter().zip(&right.edges).any(|(left, right)| {
            left.id != right.id
                || left.points != right.points
                || left.label_anchor != right.label_anchor
        })
        || left.subject_boundary != right.subject_boundary
}

pub(super) fn route_bdd_with_bounds(
    diagram_id: &str,
    state: &WorkspaceState,
    bounds: Option<routing::RouteRect>,
) -> Result<bool, String> {
    let mut diagrams = state
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("BDD or Requirement Diagram not found")?;
    let routed = routed_bdd_edges(diagram, bounds)?;
    let changed = diagram.edges.iter().zip(&routed).any(|(left, right)| {
        left.points != right.points || left.label_anchor != right.label_anchor
    });
    if changed {
        diagram.edges = routed;
    }
    Ok(changed)
}

pub(super) fn layout_bdd_with_bounds(
    diagram_id: &str,
    state: &WorkspaceState,
    bounds: Option<routing::RouteRect>,
) -> Result<bool, String> {
    let project = state
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone();
    let mut diagrams = state
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?;
    let index = diagrams
        .iter()
        .position(|diagram| diagram.id == diagram_id)
        .ok_or("BDD or Requirement Diagram not found")?;
    let original = diagrams[index].clone();
    let mut candidate = original.clone();
    let edges: Vec<_> = candidate
        .edges
        .iter()
        .map(|edge| {
            let source = candidate
                .nodes
                .iter()
                .find(|node| node.id == edge.source_node_id);
            let target = candidate
                .nodes
                .iter()
                .find(|node| node.id == edge.target_node_id);
            let actor_to_use_case = candidate.family == "use-case"
                && source
                    .and_then(|node| parse_element_id(&node.element_id).ok())
                    .and_then(|id| project.as_ref()?.element(id).ok())
                    .is_some_and(|element| element.kind == ElementKind::UseCase)
                && target
                    .and_then(|node| parse_element_id(&node.element_id).ok())
                    .and_then(|id| project.as_ref()?.element(id).ok())
                    .is_some_and(|element| element.kind == ElementKind::Actor);
            if actor_to_use_case {
                (edge.target_node_id.clone(), edge.source_node_id.clone())
            } else {
                (edge.source_node_id.clone(), edge.target_node_id.clone())
            }
        })
        .collect();
    let flow = systems_modeler_core::supported_diagram_families()
        .get(&systems_modeler_core::DiagramFamilyId(candidate.family.clone()))
        .map(|family| family.preferred_flow.clone())
        .unwrap_or(systems_modeler_core::PreferredFlowDirection::TopToBottom);
    let positions = layout::hierarchical_positions_sized(
        candidate.nodes.iter().map(|node| layout::LayoutNode {
            id: node.id.clone(),
            width: node.width,
            height: node.height,
        }),
        &edges,
        flow,
    );
    for node in &mut candidate.nodes {
        if let Some((x, y)) = positions.get(&node.id) {
            node.x = *x;
            node.y = *y;
        }
    }
    if let Some(project) = project.as_ref() {
        use_cases::fit_use_case_subject_boundary(&mut candidate, project, true);
    }
    candidate.edges = routed_bdd_edges(&candidate, bounds)?;
    let changed = bdd_presentation_changed(&original, &candidate);
    if changed {
        diagrams[index] = candidate;
    }
    Ok(changed)
}

pub use systems_modeler_core::structural_presentation::geometry::routed_bdd_edges;
#[cfg(test)]
use systems_modeler_core::structural_presentation::geometry::diagram_node_route_rect;
