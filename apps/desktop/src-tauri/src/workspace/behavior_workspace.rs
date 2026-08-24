use super::{DiagramPoint, WorkspaceState, parse_element_id};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use systems_modeler_core::behavior::{
    BehaviorRepository, CombinedFragment, Event, ExecutionSpecification, InteractionOperand,
    InteractionOperator, Lifeline, LifelineId, Message, MessageId, MessageSignature, MessageSort,
    Occurrence, OccurrenceId, PseudostateKind, Region, RegionId, State, StateInvariant,
    StateMachineId, Transition, TransitionId, TransitionKind, Trigger, Vertex, VertexId,
    VertexKind,
};
use systems_modeler_core::{ElementId, ElementKind, Project};
use systems_modeler_persistence::ProjectDatabase;

pub const BEHAVIOR_METADATA_KEY: &str = "behavior-repository";
pub const BEHAVIOR_DIAGRAM_METADATA_KEY: &str = "behavior-diagrams";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BehaviorDiagramKind {
    StateMachine,
    Sequence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateNodePresentation {
    pub vertex_id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

fn default_lifeline_timeline_start_y() -> f64 {
    102.0
}

fn default_lifeline_timeline_end_y() -> f64 {
    840.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifelinePresentation {
    pub lifeline_id: String,
    pub x: f64,
    #[serde(default = "default_lifeline_timeline_start_y")]
    pub timeline_start_y: f64,
    #[serde(default = "default_lifeline_timeline_end_y")]
    pub timeline_end_y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorEdgePresentation {
    pub semantic_id: String,
    pub points: Vec<DiagramPoint>,
    #[serde(default)]
    pub label_anchor: Option<DiagramPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorPresentationCopy {
    pub id: String,
    pub semantic_id: String,
    pub kind: String,
    pub offset_x: f64,
    pub offset_y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorDiagram {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub context_id: String,
    pub kind: BehaviorDiagramKind,
    pub semantic_id: String,
    #[serde(default)]
    pub state_nodes: Vec<StateNodePresentation>,
    #[serde(default)]
    pub lifelines: Vec<LifelinePresentation>,
    #[serde(default)]
    pub edge_routes: Vec<BehaviorEdgePresentation>,
    #[serde(default)]
    pub hidden_semantic_ids: Vec<String>,
    #[serde(default)]
    pub presentation_copies: Vec<BehaviorPresentationCopy>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BehaviorWorkspaceSnapshot {
    pub repository: BehaviorRepository,
    pub diagrams: Vec<BehaviorDiagram>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LifelineCandidate {
    pub label: String,
    pub property_path: Vec<String>,
}

fn parse_uuid(value: &str) -> Result<uuid::Uuid, String> {
    uuid::Uuid::parse_str(value).map_err(|_| format!("invalid behavior id: {value}"))
}

fn state_machine_id(value: &str) -> Result<StateMachineId, String> {
    parse_uuid(value).map(StateMachineId)
}
fn region_id(value: &str) -> Result<RegionId, String> {
    parse_uuid(value).map(RegionId)
}
fn vertex_id(value: &str) -> Result<VertexId, String> {
    parse_uuid(value).map(VertexId)
}
fn lifeline_id(value: &str) -> Result<LifelineId, String> {
    parse_uuid(value).map(LifelineId)
}

fn diagram_owner(project: &Project, context_id: ElementId) -> Result<ElementId, String> {
    let context = project
        .element(context_id)
        .map_err(|error| error.to_string())?;
    if !matches!(
        context.kind,
        ElementKind::Block | ElementKind::AssociationBlock | ElementKind::InterfaceBlock
    ) {
        return Err(
            "State Machine and Sequence diagrams require a Block-like classifier context".into(),
        );
    }
    Ok(context.owner_id.unwrap_or(project.root_id))
}

fn root_region_id(
    repository: &BehaviorRepository,
    machine_id: StateMachineId,
) -> Result<RegionId, String> {
    repository
        .state_machines
        .get(&machine_id)
        .and_then(|machine| machine.regions.first())
        .map(|region| region.id)
        .ok_or_else(|| "state machine has no root Region".into())
}

fn find_region_mut(regions: &mut [Region], wanted: RegionId) -> Option<&mut Region> {
    for region in regions {
        if region.id == wanted {
            return Some(region);
        }
        for vertex in &mut region.vertices {
            if let VertexKind::State(state) = &mut vertex.kind
                && let Some(found) = find_region_mut(&mut state.regions, wanted)
            {
                return Some(found);
            }
        }
    }
    None
}

fn find_vertex_mut(regions: &mut [Region], wanted: VertexId) -> Option<&mut Vertex> {
    for region in regions {
        for vertex in &mut region.vertices {
            if vertex.id == wanted {
                return Some(vertex);
            }
            if let VertexKind::State(state) = &mut vertex.kind
                && let Some(found) = find_vertex_mut(&mut state.regions, wanted)
            {
                return Some(found);
            }
        }
    }
    None
}

fn find_vertex(regions: &[Region], wanted: VertexId) -> Option<&Vertex> {
    for region in regions {
        for vertex in &region.vertices {
            if vertex.id == wanted {
                return Some(vertex);
            }
            if let VertexKind::State(state) = &vertex.kind
                && let Some(found) = find_vertex(&state.regions, wanted)
            {
                return Some(found);
            }
        }
    }
    None
}

fn collect_transition_endpoints(
    regions: &[Region],
    output: &mut Vec<(String, String, String, bool)>,
) {
    for region in regions {
        output.extend(region.transitions.iter().map(|transition| {
            (
                transition.id.to_string(),
                transition.source_id.to_string(),
                transition.target_id.to_string(),
                transition.trigger.is_some()
                    || transition
                        .guard
                        .as_deref()
                        .is_some_and(|guard| !guard.trim().is_empty())
                    || transition
                        .effect
                        .as_deref()
                        .is_some_and(|effect| !effect.trim().is_empty()),
            )
        }));
        for vertex in &region.vertices {
            if let VertexKind::State(state) = &vertex.kind {
                collect_transition_endpoints(&state.regions, output);
            }
        }
    }
}

fn collect_vertex_ancestors(
    regions: &[Region],
    ancestors: &[String],
    output: &mut BTreeMap<String, Vec<String>>,
) {
    for region in regions {
        for vertex in &region.vertices {
            output.insert(vertex.id.to_string(), ancestors.to_vec());
            if let VertexKind::State(state) = &vertex.kind {
                let mut nested_ancestors = ancestors.to_vec();
                nested_ancestors.push(vertex.id.to_string());
                collect_vertex_ancestors(&state.regions, &nested_ancestors, output);
            }
        }
    }
}

fn vertex_kind(value: &str) -> Result<VertexKind, String> {
    Ok(match value {
        "State" => VertexKind::State(State::default()),
        "FinalState" => VertexKind::FinalState,
        "Initial" => VertexKind::Pseudostate(PseudostateKind::Initial),
        "Choice" => VertexKind::Pseudostate(PseudostateKind::Choice),
        "Junction" => VertexKind::Pseudostate(PseudostateKind::Junction),
        "Fork" => VertexKind::Pseudostate(PseudostateKind::Fork),
        "Join" => VertexKind::Pseudostate(PseudostateKind::Join),
        "ShallowHistory" => VertexKind::Pseudostate(PseudostateKind::ShallowHistory),
        "DeepHistory" => VertexKind::Pseudostate(PseudostateKind::DeepHistory),
        "EntryPoint" => VertexKind::Pseudostate(PseudostateKind::EntryPoint),
        "ExitPoint" => VertexKind::Pseudostate(PseudostateKind::ExitPoint),
        "Terminate" => VertexKind::Pseudostate(PseudostateKind::Terminate),
        _ => return Err(format!("unsupported state vertex kind: {value}")),
    })
}

fn transition_kind(value: &str) -> Result<TransitionKind, String> {
    match value {
        "External" => Ok(TransitionKind::External),
        "Internal" => Ok(TransitionKind::Internal),
        "Local" => Ok(TransitionKind::Local),
        _ => Err(format!("unsupported transition kind: {value}")),
    }
}

fn event_from_input(
    event_kind: Option<String>,
    event_reference_id: Option<String>,
    event_expression: Option<String>,
) -> Result<Option<Trigger>, String> {
    let Some(kind) = event_kind.filter(|value| value != "None") else {
        return Ok(None);
    };
    let event = match kind.as_str() {
        "Signal" => Event::Signal {
            signal_id: parse_element_id(
                event_reference_id
                    .as_deref()
                    .ok_or("Signal trigger requires a Signal")?,
            )?,
        },
        "Call" => Event::Call {
            operation_id: parse_element_id(
                event_reference_id
                    .as_deref()
                    .ok_or("Call trigger requires an Operation")?,
            )?,
        },
        "Time" => Event::Time {
            expression: event_expression.unwrap_or_default(),
            is_relative: true,
        },
        "Change" => Event::Change {
            expression: event_expression.unwrap_or_default(),
        },
        "AnyReceive" => Event::AnyReceive,
        _ => return Err(format!("unsupported trigger event: {kind}")),
    };
    Ok(Some(Trigger { event }))
}

fn message_sort(value: &str) -> Result<MessageSort, String> {
    match value {
        "SynchCall" => Ok(MessageSort::SynchCall),
        "AsynchCall" => Ok(MessageSort::AsynchCall),
        "AsynchSignal" => Ok(MessageSort::AsynchSignal),
        "Reply" => Ok(MessageSort::Reply),
        "Create" => Ok(MessageSort::Create),
        "Delete" => Ok(MessageSort::Delete),
        "Lost" => Ok(MessageSort::Lost),
        "Found" => Ok(MessageSort::Found),
        _ => Err(format!("unsupported message sort: {value}")),
    }
}

fn interaction_operator(value: &str) -> Result<InteractionOperator, String> {
    match value {
        "alt" => Ok(InteractionOperator::Alt),
        "opt" => Ok(InteractionOperator::Opt),
        "loop" => Ok(InteractionOperator::Loop),
        "break" => Ok(InteractionOperator::Break),
        "par" => Ok(InteractionOperator::Par),
        "critical" => Ok(InteractionOperator::Critical),
        "neg" => Ok(InteractionOperator::Neg),
        "assert" => Ok(InteractionOperator::Assert),
        "strict" => Ok(InteractionOperator::Strict),
        "seq" => Ok(InteractionOperator::Seq),
        "ignore" => Ok(InteractionOperator::Ignore),
        "consider" => Ok(InteractionOperator::Consider),
        _ => Err(format!("unsupported combined fragment operator: {value}")),
    }
}

fn collect_lifeline_candidates(
    project: &Project,
    classifier_id: ElementId,
    path: &mut Vec<ElementId>,
    labels: &mut Vec<String>,
    output: &mut Vec<LifelineCandidate>,
    depth: usize,
) {
    if depth > 6 {
        return;
    }
    let mut features: Vec<_> = project
        .children(classifier_id)
        .filter(|element| {
            matches!(
                element.kind,
                ElementKind::PartProperty | ElementKind::ReferenceProperty
            )
        })
        .collect();
    features.sort_by(|a, b| a.name.cmp(&b.name));
    for feature in features {
        path.push(feature.id);
        labels.push(feature.name.clone());
        output.push(LifelineCandidate {
            label: labels.join("."),
            property_path: path.iter().map(ToString::to_string).collect(),
        });
        if let Some(type_id) = feature.type_id {
            collect_lifeline_candidates(project, type_id, path, labels, output, depth + 1);
        }
        path.pop();
        labels.pop();
    }
}

fn next_occurrence_order(messages: &[Message]) -> u32 {
    messages
        .iter()
        .flat_map(|message| [message.send_event.as_ref(), message.receive_event.as_ref()])
        .flatten()
        .map(|occurrence| occurrence.order)
        .max()
        .unwrap_or(0)
        .saturating_add(10)
}

#[tauri::command]
pub fn behavior_snapshot(
    state: tauri::State<'_, WorkspaceState>,
) -> Result<BehaviorWorkspaceSnapshot, String> {
    Ok(BehaviorWorkspaceSnapshot {
        repository: state
            .behavior
            .lock()
            .map_err(|_| "behavior lock poisoned")?
            .clone(),
        diagrams: state
            .behavior_diagrams
            .lock()
            .map_err(|_| "behavior diagram lock poisoned")?
            .clone(),
    })
}

#[tauri::command]
pub fn create_state_machine_diagram(
    context_id: String,
    name: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let context_id = parse_element_id(&context_id)?;
    let project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_ref().ok_or("no project open")?;
    let owner_id = diagram_owner(project, context_id)?;
    let mut repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    let semantic_id = repository
        .create_state_machine(project, context_id, name.clone())
        .map_err(|error| error.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?
        .push(BehaviorDiagram {
            id: id.clone(),
            name,
            owner_id: owner_id.to_string(),
            context_id: context_id.to_string(),
            kind: BehaviorDiagramKind::StateMachine,
            semantic_id: semantic_id.to_string(),
            state_nodes: Vec::new(),
            lifelines: Vec::new(),
            edge_routes: Vec::new(),
            hidden_semantic_ids: Vec::new(),
            presentation_copies: Vec::new(),
        });
    Ok(id)
}

#[tauri::command]
pub fn create_sequence_diagram(
    context_id: String,
    name: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let context_id = parse_element_id(&context_id)?;
    let project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_ref().ok_or("no project open")?;
    let owner_id = diagram_owner(project, context_id)?;
    let mut repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    let semantic_id = repository
        .create_interaction(project, context_id, name.clone())
        .map_err(|error| error.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?
        .push(BehaviorDiagram {
            id: id.clone(),
            name,
            owner_id: owner_id.to_string(),
            context_id: context_id.to_string(),
            kind: BehaviorDiagramKind::Sequence,
            semantic_id: semantic_id.to_string(),
            state_nodes: Vec::new(),
            lifelines: Vec::new(),
            edge_routes: Vec::new(),
            hidden_semantic_ids: Vec::new(),
            presentation_copies: Vec::new(),
        });
    Ok(id)
}

#[tauri::command]
pub fn add_state_vertex(
    diagram_id: String,
    region_id_value: Option<String>,
    kind: String,
    name: String,
    x: f64,
    y: f64,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let mut diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    if diagram.kind != BehaviorDiagramKind::StateMachine {
        return Err("active behavior diagram is not a State Machine".into());
    }
    let machine_id = state_machine_id(&diagram.semantic_id)?;
    let mut repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    let target_region = match region_id_value {
        Some(value) => region_id(&value)?,
        None => root_region_id(&repository, machine_id)?,
    };
    let machine = repository
        .state_machines
        .get_mut(&machine_id)
        .ok_or("State Machine not found")?;
    let region = find_region_mut(&mut machine.regions, target_region).ok_or("Region not found")?;
    let parsed_kind = vertex_kind(&kind)?;
    if matches!(
        parsed_kind,
        VertexKind::Pseudostate(PseudostateKind::Initial)
    ) && region.vertices.iter().any(|vertex| {
        matches!(
            vertex.kind,
            VertexKind::Pseudostate(PseudostateKind::Initial)
        )
    }) {
        return Err("A Region can have only one Initial pseudostate".into());
    }
    let id = VertexId::new();
    region.vertices.push(Vertex {
        id,
        name,
        kind: parsed_kind,
    });
    let (width, height) = if kind == "State" {
        (150.0, 80.0)
    } else {
        (24.0, 24.0)
    };
    diagram.state_nodes.push(StateNodePresentation {
        vertex_id: id.to_string(),
        x,
        y,
        width,
        height,
    });
    Ok(id.to_string())
}

#[tauri::command]
pub fn add_state_region(
    diagram_id: String,
    state_vertex_id: String,
    name: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    let machine_id = state_machine_id(&diagram.semantic_id)?;
    drop(diagrams);
    let mut repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    let machine = repository
        .state_machines
        .get_mut(&machine_id)
        .ok_or("State Machine not found")?;
    let vertex = find_vertex_mut(&mut machine.regions, vertex_id(&state_vertex_id)?)
        .ok_or("State not found")?;
    let VertexKind::State(state_semantic) = &mut vertex.kind else {
        return Err("Regions can only be owned by a State".into());
    };
    let id = RegionId::new();
    state_semantic.regions.push(Region {
        id,
        name,
        vertices: Vec::new(),
        transitions: Vec::new(),
    });
    Ok(id.to_string())
}

#[tauri::command]
pub fn update_state_behaviors(
    diagram_id: String,
    state_vertex_id: String,
    entry: Option<String>,
    do_activity: Option<String>,
    exit: Option<String>,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    let machine_id = state_machine_id(&diagram.semantic_id)?;
    drop(diagrams);
    let mut repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    let machine = repository
        .state_machines
        .get_mut(&machine_id)
        .ok_or("State Machine not found")?;
    let vertex = find_vertex_mut(&mut machine.regions, vertex_id(&state_vertex_id)?)
        .ok_or("State not found")?;
    let VertexKind::State(state_semantic) = &mut vertex.kind else {
        return Err("selected vertex is not a State".into());
    };
    state_semantic.entry = entry.filter(|value| !value.trim().is_empty());
    state_semantic.do_activity = do_activity.filter(|value| !value.trim().is_empty());
    state_semantic.exit = exit.filter(|value| !value.trim().is_empty());
    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Stable named-field Tauri IPC boundary.
pub fn add_state_transition(
    diagram_id: String,
    region_id_value: Option<String>,
    source_vertex_id: String,
    target_vertex_id: String,
    kind: String,
    event_kind: Option<String>,
    event_reference_id: Option<String>,
    event_expression: Option<String>,
    guard: Option<String>,
    effect: Option<String>,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    let machine_id = state_machine_id(&diagram.semantic_id)?;
    drop(diagrams);
    let mut repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    let root_region = match region_id_value {
        Some(value) => region_id(&value)?,
        None => root_region_id(&repository, machine_id)?,
    };
    let machine = repository
        .state_machines
        .get_mut(&machine_id)
        .ok_or("State Machine not found")?;
    let source_id = vertex_id(&source_vertex_id)?;
    let target_id = vertex_id(&target_vertex_id)?;
    let source =
        find_vertex(&machine.regions, source_id).ok_or("transition source vertex not found")?;
    find_vertex(&machine.regions, target_id).ok_or("transition target vertex not found")?;
    let trigger = event_from_input(event_kind, event_reference_id, event_expression)?;
    let guard = guard.filter(|value| !value.trim().is_empty());
    if matches!(
        source.kind,
        VertexKind::Pseudostate(PseudostateKind::Initial)
    ) && (trigger.is_some() || guard.is_some())
    {
        return Err("Initial transition must be triggerless and guardless. Connect Initial directly to its first State, then place triggers/guards on later Transitions.".into());
    }
    if matches!(source.kind, VertexKind::FinalState) {
        return Err("Final State cannot have outgoing Transitions".into());
    }
    let id = TransitionId::new();
    let transition = Transition {
        id,
        source_id,
        target_id,
        kind: transition_kind(&kind)?,
        trigger,
        guard,
        effect: effect.filter(|value| !value.trim().is_empty()),
    };
    let region = find_region_mut(&mut machine.regions, root_region).ok_or("Region not found")?;
    region.transitions.push(transition);
    Ok(id.to_string())
}

#[tauri::command]
pub fn move_state_vertex(
    diagram_id: String,
    state_vertex_id: String,
    x: f64,
    y: f64,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let mut diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    let presentation = diagram
        .state_nodes
        .iter_mut()
        .find(|node| node.vertex_id == state_vertex_id)
        .ok_or("State presentation not found")?;
    presentation.x = x;
    presentation.y = y;
    Ok(())
}

#[tauri::command]
pub fn behavior_lifeline_candidates(
    diagram_id: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<Vec<LifelineCandidate>, String> {
    let diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    if diagram.kind != BehaviorDiagramKind::Sequence {
        return Err("active behavior diagram is not a Sequence Diagram".into());
    }
    let context_id = parse_element_id(&diagram.context_id)?;
    drop(diagrams);
    let project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_ref().ok_or("no project open")?;
    let mut output = Vec::new();
    collect_lifeline_candidates(
        project,
        context_id,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut output,
        0,
    );
    Ok(output)
}

#[tauri::command]
pub fn add_sequence_lifeline(
    diagram_id: String,
    represented_path: Vec<String>,
    x: f64,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let mut diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    if diagram.kind != BehaviorDiagramKind::Sequence {
        return Err("active behavior diagram is not a Sequence Diagram".into());
    }
    let interaction_id =
        parse_uuid(&diagram.semantic_id).map(systems_modeler_core::behavior::InteractionId)?;
    let context_id = parse_element_id(&diagram.context_id)?;
    let path: Vec<ElementId> = represented_path
        .iter()
        .map(|value| parse_element_id(value))
        .collect::<Result<_, _>>()?;
    let project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_ref().ok_or("no project open")?;
    project
        .resolve_structural_path(context_id, &path)
        .map_err(|error| error.to_string())?;
    let label = path
        .iter()
        .map(|id| {
            project
                .element(*id)
                .map(|element| element.name.clone())
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?
        .join(".");
    drop(project_guard);
    let id = LifelineId::new();
    let mut repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    repository
        .interactions
        .get_mut(&interaction_id)
        .ok_or("Interaction not found")?
        .lifelines
        .push(Lifeline {
            id,
            name: label,
            represented_path: path,
        });
    diagram.lifelines.push(LifelinePresentation {
        lifeline_id: id.to_string(),
        x,
        timeline_start_y: default_lifeline_timeline_start_y(),
        timeline_end_y: default_lifeline_timeline_end_y(),
    });
    Ok(id.to_string())
}

#[tauri::command]
pub fn move_sequence_lifeline(
    diagram_id: String,
    lifeline_id_value: String,
    x: f64,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let mut diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    let presentation = diagram
        .lifelines
        .iter_mut()
        .find(|item| item.lifeline_id == lifeline_id_value)
        .ok_or("Lifeline presentation not found")?;
    presentation.x = x;
    Ok(())
}

#[tauri::command]
pub fn resize_sequence_lifeline_timeline(
    diagram_id: String,
    lifeline_id_value: String,
    timeline_start_y: f64,
    timeline_end_y: f64,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    if !timeline_start_y.is_finite() || !timeline_end_y.is_finite() {
        return Err("Lifeline timeline coordinates must be finite".into());
    }
    if timeline_start_y < 90.0 {
        return Err("Lifeline timeline must start below its header".into());
    }
    if timeline_end_y - timeline_start_y < 80.0 {
        return Err("Lifeline timeline must be at least 80 diagram units long".into());
    }
    let mut diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    if diagram.kind != BehaviorDiagramKind::Sequence {
        return Err("active behavior diagram is not a Sequence Diagram".into());
    }
    let presentation = diagram
        .lifelines
        .iter_mut()
        .find(|item| item.lifeline_id == lifeline_id_value)
        .ok_or("Lifeline presentation not found")?;
    presentation.timeline_start_y = timeline_start_y;
    presentation.timeline_end_y = timeline_end_y;
    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Stable named-field Tauri IPC boundary.
pub fn add_sequence_message(
    diagram_id: String,
    source_lifeline_id: Option<String>,
    target_lifeline_id: Option<String>,
    sort: String,
    name: String,
    signature_id: Option<String>,
    arguments: Vec<String>,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    let interaction_id =
        parse_uuid(&diagram.semantic_id).map(systems_modeler_core::behavior::InteractionId)?;
    drop(diagrams);
    let mut repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    let interaction = repository
        .interactions
        .get_mut(&interaction_id)
        .ok_or("Interaction not found")?;
    let sort = message_sort(&sort)?;
    let next = next_occurrence_order(&interaction.messages);
    let send_event = source_lifeline_id
        .as_deref()
        .map(lifeline_id)
        .transpose()?
        .map(|lifeline_id| Occurrence {
            id: OccurrenceId::new(),
            lifeline_id,
            order: next,
        });
    let receive_event = target_lifeline_id
        .as_deref()
        .map(lifeline_id)
        .transpose()?
        .map(|lifeline_id| Occurrence {
            id: OccurrenceId::new(),
            lifeline_id,
            order: next + 5,
        });
    let signature = match sort {
        MessageSort::SynchCall | MessageSort::AsynchCall => signature_id
            .as_deref()
            .map(parse_element_id)
            .transpose()?
            .map(MessageSignature::Operation),
        MessageSort::AsynchSignal => signature_id
            .as_deref()
            .map(parse_element_id)
            .transpose()?
            .map(MessageSignature::Signal),
        _ => None,
    };
    let id = MessageId::new();
    interaction.messages.push(Message {
        id,
        name,
        sort,
        send_event,
        receive_event,
        signature,
        arguments,
    });
    let project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    systems_modeler_core::behavior::validate_interaction(
        project_guard.as_ref().ok_or("no project open")?,
        interaction,
    )
    .map_err(|error| {
        interaction.messages.pop();
        error.to_string()
    })?;
    Ok(id.to_string())
}

#[tauri::command]
pub fn add_execution_specification(
    diagram_id: String,
    lifeline_id_value: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    let interaction_id =
        parse_uuid(&diagram.semantic_id).map(systems_modeler_core::behavior::InteractionId)?;
    drop(diagrams);
    let mut repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    let interaction = repository
        .interactions
        .get_mut(&interaction_id)
        .ok_or("Interaction not found")?;
    let lifeline_id = lifeline_id(&lifeline_id_value)?;
    if !interaction
        .lifelines
        .iter()
        .any(|item| item.id == lifeline_id)
    {
        return Err("Execution must be attached to an existing Lifeline".into());
    }
    let start_order = next_occurrence_order(&interaction.messages);
    let id = systems_modeler_core::behavior::ExecutionId::new();
    interaction.executions.push(ExecutionSpecification {
        id,
        lifeline_id,
        start: Occurrence {
            id: OccurrenceId::new(),
            lifeline_id,
            order: start_order,
        },
        finish: Occurrence {
            id: OccurrenceId::new(),
            lifeline_id,
            order: start_order + 20,
        },
        behavior_id: None,
    });
    Ok(id.to_string())
}

#[tauri::command]
pub fn add_combined_fragment(
    diagram_id: String,
    operator: String,
    covered_lifeline_ids: Vec<String>,
    guard: Option<String>,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    let interaction_id =
        parse_uuid(&diagram.semantic_id).map(systems_modeler_core::behavior::InteractionId)?;
    drop(diagrams);
    let mut repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    let interaction = repository
        .interactions
        .get_mut(&interaction_id)
        .ok_or("Interaction not found")?;
    let covered_lifelines = covered_lifeline_ids
        .iter()
        .map(|value| lifeline_id(value))
        .collect::<Result<Vec<_>, _>>()?;
    let start = next_occurrence_order(&interaction.messages);
    let operator = interaction_operator(&operator)?;
    let mut operands = vec![InteractionOperand {
        id: systems_modeler_core::behavior::OperandId::new(),
        guard: guard.filter(|value| !value.trim().is_empty()),
        start_order: start,
        end_order: start + 30,
    }];
    if operator == InteractionOperator::Alt {
        operands.push(InteractionOperand {
            id: systems_modeler_core::behavior::OperandId::new(),
            guard: Some("else".into()),
            start_order: start + 30,
            end_order: start + 60,
        });
    }
    let id = systems_modeler_core::behavior::FragmentId::new();
    interaction.fragments.push(CombinedFragment {
        id,
        operator,
        covered_lifelines,
        operands,
    });
    Ok(id.to_string())
}

#[tauri::command]
pub fn add_state_invariant(
    diagram_id: String,
    lifeline_id_value: String,
    constraint: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    if constraint.trim().is_empty() {
        return Err("State Invariant requires a non-empty constraint".into());
    }
    let diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    let interaction_id =
        parse_uuid(&diagram.semantic_id).map(systems_modeler_core::behavior::InteractionId)?;
    drop(diagrams);
    let mut repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    let interaction = repository
        .interactions
        .get_mut(&interaction_id)
        .ok_or("Interaction not found")?;
    let id = systems_modeler_core::behavior::InvariantId::new();
    interaction.state_invariants.push(StateInvariant {
        id,
        lifeline_id: lifeline_id(&lifeline_id_value)?,
        order: next_occurrence_order(&interaction.messages),
        constraint,
    });
    Ok(id.to_string())
}

pub fn validate_behavior_workspace(
    project: &Project,
    repository: &BehaviorRepository,
    diagrams: &[BehaviorDiagram],
) -> Result<(), String> {
    repository
        .validate(project)
        .map_err(|error| error.to_string())?;
    for diagram in diagrams {
        parse_element_id(&diagram.context_id)?;
        parse_element_id(&diagram.owner_id)?;
        for route in &diagram.edge_routes {
            parse_uuid(&route.semantic_id)?;
            if route.points.len() < 2
                || route
                    .points
                    .iter()
                    .any(|point| !point.x.is_finite() || !point.y.is_finite())
            {
                return Err(format!(
                    "behavior edge has invalid presentation route: {}",
                    route.semantic_id
                ));
            }
        }
        for hidden_id in &diagram.hidden_semantic_ids {
            parse_uuid(hidden_id)?;
        }
        for copy in &diagram.presentation_copies {
            parse_uuid(&copy.id)?;
            parse_uuid(&copy.semantic_id)?;
            if copy.kind.trim().is_empty()
                || !copy.offset_x.is_finite()
                || !copy.offset_y.is_finite()
            {
                return Err(format!(
                    "behavior presentation copy is invalid: {}",
                    copy.id
                ));
            }
        }
        match diagram.kind {
            BehaviorDiagramKind::StateMachine => {
                let id = state_machine_id(&diagram.semantic_id)?;
                if !repository.state_machines.contains_key(&id) {
                    return Err(format!(
                        "State Machine semantic object missing for diagram {}",
                        diagram.name
                    ));
                }
            }
            BehaviorDiagramKind::Sequence => {
                let id = parse_uuid(&diagram.semantic_id)
                    .map(systems_modeler_core::behavior::InteractionId)?;
                if !repository.interactions.contains_key(&id) {
                    return Err(format!(
                        "Interaction semantic object missing for diagram {}",
                        diagram.name
                    ));
                }
            }
        }
    }
    Ok(())
}

fn state_machine_routes(
    diagram: &BehaviorDiagram,
    repository: &BehaviorRepository,
    bounds: Option<super::routing::RouteRect>,
) -> Result<Vec<BehaviorEdgePresentation>, String> {
    let machine_id = state_machine_id(&diagram.semantic_id)?;
    let machine = repository
        .state_machines
        .get(&machine_id)
        .ok_or("State Machine not found")?;
    let mut transitions = Vec::new();
    collect_transition_endpoints(&machine.regions, &mut transitions);
    let mut ancestors = BTreeMap::new();
    collect_vertex_ancestors(&machine.regions, &[], &mut ancestors);
    let mut reserved_routes = Vec::new();
    let mut label_obstacles = Vec::new();
    let mut routes = Vec::new();
    for (index, (id, source_id, target_id, has_visible_label)) in transitions.iter().enumerate() {
        let source = diagram
            .state_nodes
            .iter()
            .find(|node| node.vertex_id == *source_id)
            .ok_or("State transition source presentation not found")?;
        let target = diagram
            .state_nodes
            .iter()
            .find(|node| node.vertex_id == *target_id)
            .ok_or("State transition target presentation not found")?;
        let mut related = BTreeSet::from([source_id.clone(), target_id.clone()]);
        if let Some(source_ancestors) = ancestors.get(source_id) {
            related.extend(source_ancestors.iter().cloned());
        }
        if let Some(target_ancestors) = ancestors.get(target_id) {
            related.extend(target_ancestors.iter().cloned());
        }
        let mut obstacles: Vec<_> = diagram
            .state_nodes
            .iter()
            .filter(|node| !related.contains(&node.vertex_id))
            .map(|node| super::routing::RouteRect {
                x: node.x,
                y: node.y,
                width: node.width,
                height: node.height,
            })
            .collect();
        obstacles.extend(label_obstacles.iter().copied());
        let all_label_obstacles: Vec<_> = diagram
            .state_nodes
            .iter()
            .map(|node| super::routing::RouteRect {
                x: node.x,
                y: node.y,
                width: node.width,
                height: node.height,
            })
            .chain(label_obstacles.iter().copied())
            .collect();
        let same_source_count = transitions[..index]
            .iter()
            .filter(|(_, candidate_source, _, _)| candidate_source == source_id)
            .count();
        let points = super::routing::orthogonal_route(super::routing::RouteRequest {
            source: super::routing::RouteRect {
                x: source.x,
                y: source.y,
                width: source.width,
                height: source.height,
            },
            target: super::routing::RouteRect {
                x: target.x,
                y: target.y,
                width: target.width,
                height: target.height,
            },
            obstacles: &obstacles,
            lane_index: same_source_count,
            reserved_routes: &reserved_routes,
            allow_shared_departure: same_source_count > 0,
            bounds,
        })?;
        let label_anchor = if *has_visible_label {
            let anchor = super::routing::route_label_anchor_avoiding(
                &points,
                &all_label_obstacles,
                &reserved_routes,
                bounds,
            )?;
            label_obstacles.push(super::routing::label_rect(anchor));
            Some(anchor)
        } else {
            None
        };
        reserved_routes.push(points.clone());
        routes.push(BehaviorEdgePresentation {
            semantic_id: id.clone(),
            label_anchor,
            points,
        });
    }
    Ok(routes)
}

fn lifeline_x(diagram: &BehaviorDiagram, lifeline: LifelineId) -> Option<f64> {
    diagram
        .lifelines
        .iter()
        .find(|presentation| presentation.lifeline_id == lifeline.to_string())
        .map(|presentation| presentation.x)
}

fn sequence_obstacles(
    diagram: &BehaviorDiagram,
    interaction: &systems_modeler_core::behavior::Interaction,
) -> Vec<(Option<LifelineId>, super::routing::RouteRect)> {
    let mut obstacles = diagram
        .lifelines
        .iter()
        .map(|lifeline| {
            (
                None,
                super::routing::RouteRect {
                    x: lifeline.x - 65.0,
                    y: 60.0,
                    width: 130.0,
                    height: 42.0,
                },
            )
        })
        .collect::<Vec<_>>();
    obstacles.extend(interaction.executions.iter().filter_map(|execution| {
        let x = lifeline_x(diagram, execution.lifeline_id)?;
        let top = 110.0 + f64::from(execution.start.order) * 4.0;
        let bottom = 110.0 + f64::from(execution.finish.order) * 4.0;
        Some((
            Some(execution.lifeline_id),
            super::routing::RouteRect {
                x: x - 7.0,
                y: top.min(bottom),
                width: 14.0,
                height: (bottom - top).abs().max(12.0),
            },
        ))
    }));
    obstacles
}

fn sequence_routes(
    diagram: &BehaviorDiagram,
    repository: &BehaviorRepository,
    bounds: Option<super::routing::RouteRect>,
) -> Result<Vec<BehaviorEdgePresentation>, String> {
    let interaction_id =
        parse_uuid(&diagram.semantic_id).map(systems_modeler_core::behavior::InteractionId)?;
    let interaction = repository
        .interactions
        .get(&interaction_id)
        .ok_or("Interaction not found")?;
    let presentation_obstacles = sequence_obstacles(diagram, interaction);
    let mut reserved_routes = Vec::new();
    let mut label_obstacles = Vec::new();
    let mut routes = Vec::new();
    for (index, message) in interaction.messages.iter().enumerate() {
        let source_lifeline = message.send_event.as_ref().map(|event| event.lifeline_id);
        let target_lifeline = message
            .receive_event
            .as_ref()
            .map(|event| event.lifeline_id);
        let source_x = source_lifeline
            .and_then(|lifeline| lifeline_x(diagram, lifeline))
            .unwrap_or(70.0);
        let target_x = target_lifeline
            .and_then(|lifeline| lifeline_x(diagram, lifeline))
            .unwrap_or(1000.0);
        let order = message
            .send_event
            .as_ref()
            .or(message.receive_event.as_ref())
            .map_or((index as u32 + 1) * 10, |event| event.order);
        let y = 110.0 + f64::from(order) * 4.0;
        let mut obstacles: Vec<_> = presentation_obstacles
            .iter()
            .filter(|(owner, _)| {
                owner.is_none() || (*owner != source_lifeline && *owner != target_lifeline)
            })
            .map(|(_, rect)| *rect)
            .collect();
        obstacles.extend(label_obstacles.iter().copied());
        let all_label_obstacles: Vec<_> = presentation_obstacles
            .iter()
            .map(|(_, rect)| *rect)
            .chain(label_obstacles.iter().copied())
            .collect();
        let same_source_count = interaction.messages[..index]
            .iter()
            .filter(|candidate| {
                candidate.send_event.as_ref().map(|event| event.lifeline_id) == source_lifeline
            })
            .count();
        let points = if source_x == target_x {
            let lane_x = source_x + 46.0 + same_source_count as f64 * 12.0;
            let candidate = vec![
                DiagramPoint { x: source_x, y },
                DiagramPoint { x: lane_x, y },
                DiagramPoint {
                    x: lane_x,
                    y: y + 26.0,
                },
                DiagramPoint {
                    x: source_x,
                    y: y + 26.0,
                },
            ];
            let inside_bounds = bounds.is_none_or(|frame| {
                candidate.iter().all(|point| {
                    point.x >= frame.x
                        && point.x <= frame.x + frame.width
                        && point.y >= frame.y
                        && point.y <= frame.y + frame.height
                })
            });
            if !inside_bounds
                || !super::routing::route_is_clear(&candidate, &obstacles)
                || !super::routing::route_avoids_reserved(
                    &candidate,
                    &reserved_routes,
                    same_source_count > 0,
                )
            {
                return Err("no validated obstacle-clear self-message route is available inside the diagram frame; existing geometry was preserved".into());
            }
            candidate
        } else {
            super::routing::orthogonal_route(super::routing::RouteRequest {
                source: super::routing::RouteRect {
                    x: source_x,
                    y,
                    width: 0.0,
                    height: 0.0,
                },
                target: super::routing::RouteRect {
                    x: target_x,
                    y,
                    width: 0.0,
                    height: 0.0,
                },
                obstacles: &obstacles,
                lane_index: same_source_count,
                reserved_routes: &reserved_routes,
                allow_shared_departure: same_source_count > 0,
                bounds,
            })?
        };
        let label_anchor = super::routing::route_label_anchor_avoiding(
            &points,
            &all_label_obstacles,
            &reserved_routes,
            bounds,
        )?;
        label_obstacles.push(super::routing::label_rect(label_anchor));
        reserved_routes.push(points.clone());
        routes.push(BehaviorEdgePresentation {
            semantic_id: message.id.to_string(),
            points,
            label_anchor: Some(label_anchor),
        });
    }
    Ok(routes)
}

fn routed_behavior_edges(
    diagram: &BehaviorDiagram,
    repository: &BehaviorRepository,
    bounds: Option<super::routing::RouteRect>,
) -> Result<Vec<BehaviorEdgePresentation>, String> {
    match diagram.kind {
        BehaviorDiagramKind::StateMachine => state_machine_routes(diagram, repository, bounds),
        BehaviorDiagramKind::Sequence => sequence_routes(diagram, repository, bounds),
    }
}

fn behavior_presentation_changed(left: &BehaviorDiagram, right: &BehaviorDiagram) -> bool {
    left.state_nodes.len() != right.state_nodes.len()
        || left.lifelines.len() != right.lifelines.len()
        || left.edge_routes.len() != right.edge_routes.len()
        || left
            .state_nodes
            .iter()
            .zip(&right.state_nodes)
            .any(|(left, right)| {
                left.vertex_id != right.vertex_id
                    || left.x != right.x
                    || left.y != right.y
                    || left.width != right.width
                    || left.height != right.height
            })
        || left
            .lifelines
            .iter()
            .zip(&right.lifelines)
            .any(|(left, right)| {
                left.lifeline_id != right.lifeline_id
                    || left.x != right.x
                    || left.timeline_start_y != right.timeline_start_y
                    || left.timeline_end_y != right.timeline_end_y
            })
        || left
            .edge_routes
            .iter()
            .zip(&right.edge_routes)
            .any(|(left, right)| {
                left.semantic_id != right.semantic_id
                    || left.points != right.points
                    || left.label_anchor != right.label_anchor
            })
}

pub(super) fn route_behavior_with_bounds(
    diagram_id: &str,
    state: &WorkspaceState,
    bounds: Option<super::routing::RouteRect>,
) -> Result<bool, String> {
    let original = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .cloned()
        .ok_or("behavior diagram not found")?;
    let routes = {
        let repository = state
            .behavior
            .lock()
            .map_err(|_| "behavior lock poisoned")?;
        routed_behavior_edges(&original, &repository, bounds)?
    };
    let mut candidate = original.clone();
    candidate.edge_routes = routes;
    let changed = behavior_presentation_changed(&original, &candidate);
    if changed {
        let mut diagrams = state
            .behavior_diagrams
            .lock()
            .map_err(|_| "behavior diagram lock poisoned")?;
        let target = diagrams
            .iter_mut()
            .find(|diagram| diagram.id == diagram_id)
            .ok_or("behavior diagram not found")?;
        *target = candidate;
    }
    Ok(changed)
}

#[tauri::command]
pub fn route_behavior_diagram(
    diagram_id: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    route_behavior_with_bounds(&diagram_id, &state, None).map(|_| ())
}

pub(super) fn layout_behavior_with_bounds(
    diagram_id: &str,
    state: &WorkspaceState,
    bounds: Option<super::routing::RouteRect>,
) -> Result<bool, String> {
    let original = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .cloned()
        .ok_or("behavior diagram not found")?;
    let repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    let mut candidate = original.clone();
    match candidate.kind {
        BehaviorDiagramKind::StateMachine => {
            let machine_id = state_machine_id(&candidate.semantic_id)?;
            let machine = repository
                .state_machines
                .get(&machine_id)
                .ok_or("State Machine not found")?;
            let mut transitions = Vec::new();
            collect_transition_endpoints(&machine.regions, &mut transitions);
            let mut ancestors = BTreeMap::new();
            collect_vertex_ancestors(&machine.regions, &[], &mut ancestors);
            let root_id = |vertex_id: &str| {
                ancestors
                    .get(vertex_id)
                    .and_then(|items| items.iter().next())
                    .cloned()
                    .unwrap_or_else(|| vertex_id.to_string())
            };
            let roots: BTreeSet<_> = candidate
                .state_nodes
                .iter()
                .map(|node| root_id(&node.vertex_id))
                .collect();
            let edges: Vec<_> = transitions
                .iter()
                .map(|(_, source, target, _)| (root_id(source), root_id(target)))
                .filter(|(source, target)| source != target)
                .collect();
            let mut positions = super::layout::hierarchical_positions_sized(
                candidate
                    .state_nodes
                    .iter()
                    .filter(|node| roots.contains(&node.vertex_id))
                    .map(|node| super::layout::LayoutNode {
                        id: node.vertex_id.clone(),
                        width: node.width,
                        height: node.height,
                    }),
                &edges,
                systems_modeler_core::PreferredFlowDirection::TopToBottom,
            );
            if let Some(frame) = bounds
                && let (Some(min_x), Some(min_y)) = (
                    positions.values().map(|(x, _)| *x).reduce(f64::min),
                    positions.values().map(|(_, y)| *y).reduce(f64::min),
                )
            {
                // Clean Layout receives the current diagram-frame interior. Keep the
                // generated STM hierarchy inside that coordinate space instead of
                // relocating it to the global canvas origin, which would make the
                // mandatory post-layout routing validation reject the transaction.
                let offset_x = frame.x + super::routing::ROUTE_CLEARANCE - min_x;
                let offset_y = frame.y + super::routing::ROUTE_CLEARANCE - min_y;
                for (x, y) in positions.values_mut() {
                    *x += offset_x;
                    *y += offset_y;
                }
            }
            let deltas: BTreeMap<_, _> = candidate
                .state_nodes
                .iter()
                .filter_map(|node| {
                    positions
                        .get(&node.vertex_id)
                        .map(|(x, y)| (node.vertex_id.clone(), (*x - node.x, *y - node.y)))
                })
                .collect();
            for node in &mut candidate.state_nodes {
                if let Some((dx, dy)) = deltas.get(&root_id(&node.vertex_id)) {
                    node.x += dx;
                    node.y += dy;
                }
            }
        }
        BehaviorDiagramKind::Sequence => {
            let interaction_id = parse_uuid(&candidate.semantic_id)
                .map(systems_modeler_core::behavior::InteractionId)?;
            let interaction = repository
                .interactions
                .get(&interaction_id)
                .ok_or("Interaction not found")?;
            let order: BTreeMap<_, _> = interaction
                .lifelines
                .iter()
                .enumerate()
                .map(|(index, lifeline)| (lifeline.id.to_string(), index))
                .collect();
            for lifeline in &mut candidate.lifelines {
                if let Some(index) = order.get(&lifeline.lifeline_id) {
                    lifeline.x = 150.0 + *index as f64 * 210.0;
                }
            }
        }
    }
    candidate.edge_routes = routed_behavior_edges(&candidate, &repository, bounds)?;
    drop(repository);
    let changed = behavior_presentation_changed(&original, &candidate);
    if changed {
        let mut diagrams = state
            .behavior_diagrams
            .lock()
            .map_err(|_| "behavior diagram lock poisoned")?;
        let target = diagrams
            .iter_mut()
            .find(|diagram| diagram.id == diagram_id)
            .ok_or("behavior diagram not found")?;
        *target = candidate;
    }
    Ok(changed)
}

pub fn save_behavior_metadata(
    database: &mut ProjectDatabase,
    project: &Project,
    repository: &BehaviorRepository,
    diagrams: &[BehaviorDiagram],
) -> Result<(), String> {
    validate_behavior_workspace(project, repository, diagrams)?;
    database
        .save_metadata(
            project.id,
            BEHAVIOR_METADATA_KEY,
            &serde_json::to_string(repository).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
    database
        .save_metadata(
            project.id,
            BEHAVIOR_DIAGRAM_METADATA_KEY,
            &serde_json::to_string(diagrams).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn load_behavior_metadata(
    database: &ProjectDatabase,
    project: &Project,
) -> Result<(BehaviorRepository, Vec<BehaviorDiagram>), String> {
    let repository = match database
        .load_metadata(project.id, BEHAVIOR_METADATA_KEY)
        .map_err(|error| error.to_string())?
    {
        Some(payload) => serde_json::from_str(&payload)
            .map_err(|error| format!("invalid saved behavior semantics: {error}"))?,
        None => BehaviorRepository::default(),
    };
    let diagrams = match database
        .load_metadata(project.id, BEHAVIOR_DIAGRAM_METADATA_KEY)
        .map_err(|error| error.to_string())?
    {
        Some(payload) => serde_json::from_str(&payload)
            .map_err(|error| format!("invalid saved behavior presentation: {error}"))?,
        None => Vec::new(),
    };
    validate_behavior_workspace(project, &repository, &diagrams)?;
    Ok((repository, diagrams))
}

#[cfg(test)]
mod lifeline_presentation_tests {
    use super::{
        LifelinePresentation, default_lifeline_timeline_end_y, default_lifeline_timeline_start_y,
    };

    #[test]
    fn legacy_lifeline_presentation_receives_timeline_defaults() {
        let presentation: LifelinePresentation =
            serde_json::from_str(r#"{"lifeline_id":"legacy","x":240.0}"#)
                .expect("legacy Lifeline presentation should deserialize");
        assert_eq!(
            presentation.timeline_start_y,
            default_lifeline_timeline_start_y()
        );
        assert_eq!(
            presentation.timeline_end_y,
            default_lifeline_timeline_end_y()
        );
        assert!(presentation.timeline_end_y - presentation.timeline_start_y >= 80.0);
    }
}

#[cfg(test)]
mod behavior_metadata_database_tests {
    use super::*;

    #[test]
    fn behavior_metadata_database_round_trip_preserves_stm_and_seq_diagrams() {
        let mut project = Project::new("Behavior Round Trip");
        let package = project
            .create_element(ElementKind::Package, "Behavior", project.root_id)
            .expect("package");
        let block = project
            .create_element(ElementKind::Block, "Controller", package)
            .expect("block");

        let mut repository = BehaviorRepository::default();
        let state_machine_id = repository
            .create_state_machine(&project, block, "Controller States")
            .expect("state machine");
        let interaction_id = repository
            .create_interaction(&project, block, "Controller Sequence")
            .expect("interaction");
        let diagrams = vec![
            BehaviorDiagram {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Controller States".into(),
                owner_id: package.to_string(),
                context_id: block.to_string(),
                kind: BehaviorDiagramKind::StateMachine,
                semantic_id: state_machine_id.to_string(),
                state_nodes: Vec::new(),
                lifelines: Vec::new(),
                edge_routes: Vec::new(),
                hidden_semantic_ids: Vec::new(),
                presentation_copies: Vec::new(),
            },
            BehaviorDiagram {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Controller Sequence".into(),
                owner_id: package.to_string(),
                context_id: block.to_string(),
                kind: BehaviorDiagramKind::Sequence,
                semantic_id: interaction_id.to_string(),
                state_nodes: Vec::new(),
                lifelines: Vec::new(),
                edge_routes: Vec::new(),
                hidden_semantic_ids: Vec::new(),
                presentation_copies: Vec::new(),
            },
        ];

        let path = std::env::temp_dir().join(format!(
            "systems-modeler-behavior-round-trip-{}.smproj",
            uuid::Uuid::new_v4()
        ));
        {
            let mut database = ProjectDatabase::open(&path).expect("open database");
            database.save_project(&project).expect("save project");
            save_behavior_metadata(&mut database, &project, &repository, &diagrams)
                .expect("save behavior metadata");
        }
        {
            let database = ProjectDatabase::open(&path).expect("reopen database");
            let restored_project = database.load_first_project().expect("load project");
            let (restored_repository, restored_diagrams) =
                load_behavior_metadata(&database, &restored_project)
                    .expect("load behavior metadata");
            assert_eq!(restored_repository.state_machines.len(), 1);
            assert_eq!(restored_repository.interactions.len(), 1);
            assert_eq!(restored_diagrams.len(), 2);
            assert!(restored_diagrams.iter().any(|diagram| {
                diagram.kind == BehaviorDiagramKind::StateMachine
                    && diagram.semantic_id == state_machine_id.to_string()
            }));
            assert!(restored_diagrams.iter().any(|diagram| {
                diagram.kind == BehaviorDiagramKind::Sequence
                    && diagram.semantic_id == interaction_id.to_string()
            }));
        }
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod state_machine_layout_tests {
    use super::*;

    #[test]
    fn clean_layout_uses_the_current_state_machine_frame_coordinate_space() {
        let mut project = Project::new("State Layout");
        let block = project
            .create_element(ElementKind::Block, "Controller", project.root_id)
            .expect("block");
        let mut repository = BehaviorRepository::default();
        let machine_id = repository
            .create_state_machine(&project, block, "Controller States")
            .expect("state machine");
        let source_id = VertexId::new();
        let target_id = VertexId::new();
        let transition_id = TransitionId::new();
        let machine = repository
            .state_machines
            .get_mut(&machine_id)
            .expect("state machine");
        machine.regions[0].vertices.extend([
            Vertex {
                id: source_id,
                name: "Idle".into(),
                kind: VertexKind::State(State::default()),
            },
            Vertex {
                id: target_id,
                name: "Running".into(),
                kind: VertexKind::State(State::default()),
            },
        ]);
        machine.regions[0].transitions.push(Transition {
            id: transition_id,
            source_id,
            target_id,
            kind: TransitionKind::External,
            trigger: None,
            guard: None,
            effect: None,
        });

        let diagram_id = uuid::Uuid::new_v4().to_string();
        let workspace = WorkspaceState::default();
        *workspace.project.lock().expect("project lock") = Some(project);
        *workspace.behavior.lock().expect("behavior lock") = repository;
        *workspace
            .behavior_diagrams
            .lock()
            .expect("behavior diagram lock") = vec![BehaviorDiagram {
            id: diagram_id.clone(),
            name: "Controller States".into(),
            owner_id: block.to_string(),
            context_id: block.to_string(),
            kind: BehaviorDiagramKind::StateMachine,
            semantic_id: machine_id.to_string(),
            state_nodes: vec![
                StateNodePresentation {
                    vertex_id: source_id.to_string(),
                    x: 620.0,
                    y: 520.0,
                    width: 150.0,
                    height: 80.0,
                },
                StateNodePresentation {
                    vertex_id: target_id.to_string(),
                    x: 900.0,
                    y: 720.0,
                    width: 150.0,
                    height: 80.0,
                },
            ],
            lifelines: Vec::new(),
            edge_routes: Vec::new(),
            hidden_semantic_ids: Vec::new(),
            presentation_copies: Vec::new(),
        }];
        let frame = super::super::routing::RouteRect {
            x: 560.0,
            y: 480.0,
            width: 720.0,
            height: 520.0,
        };

        assert!(
            layout_behavior_with_bounds(&diagram_id, &workspace, Some(frame))
                .expect("State Machine Clean Layout")
        );
        let diagrams = workspace
            .behavior_diagrams
            .lock()
            .expect("behavior diagram lock");
        let diagram = &diagrams[0];
        assert!(diagram.state_nodes.iter().all(|node| {
            node.x >= frame.x
                && node.y >= frame.y
                && node.x + node.width <= frame.x + frame.width
                && node.y + node.height <= frame.y + frame.height
        }));
        assert_eq!(diagram.edge_routes.len(), 1);
        assert_eq!(
            diagram.edge_routes[0].semantic_id,
            transition_id.to_string()
        );
        assert!(diagram.edge_routes[0].label_anchor.is_none());
        assert!(diagram.edge_routes[0].points.iter().all(|point| {
            point.x >= frame.x
                && point.x <= frame.x + frame.width
                && point.y >= frame.y
                && point.y <= frame.y + frame.height
        }));
    }
}
