use super::{parse_element_id, WorkspaceState};
use serde::{Deserialize, Serialize};
use systems_modeler_core::behavior::{
    BehaviorRepository, CombinedFragment, Event, ExecutionSpecification, InteractionOperator,
    InteractionOperand, Lifeline, LifelineId, Message, MessageId, MessageSignature, MessageSort,
    Occurrence, OccurrenceId, PseudostateKind, Region, RegionId, State, StateInvariant,
    StateMachineId, Transition, TransitionId, TransitionKind, Trigger, Vertex, VertexId, VertexKind,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifelinePresentation {
    pub lifeline_id: String,
    pub x: f64,
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
    let context = project.element(context_id).map_err(|error| error.to_string())?;
    if !matches!(context.kind, ElementKind::Block | ElementKind::AssociationBlock | ElementKind::InterfaceBlock) {
        return Err("State Machine and Sequence diagrams require a Block-like classifier context".into());
    }
    Ok(context.owner_id.unwrap_or(project.root_id))
}

fn root_region_id(repository: &BehaviorRepository, machine_id: StateMachineId) -> Result<RegionId, String> {
    repository
        .state_machines
        .get(&machine_id)
        .and_then(|machine| machine.regions.first())
        .map(|region| region.id)
        .ok_or_else(|| "state machine has no root Region".into())
}

fn find_region_mut<'a>(regions: &'a mut [Region], wanted: RegionId) -> Option<&'a mut Region> {
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

fn find_vertex_mut<'a>(regions: &'a mut [Region], wanted: VertexId) -> Option<&'a mut Vertex> {
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

fn find_vertex<'a>(regions: &'a [Region], wanted: VertexId) -> Option<&'a Vertex> {
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
            signal_id: parse_element_id(event_reference_id.as_deref().ok_or("Signal trigger requires a Signal")?)?,
        },
        "Call" => Event::Call {
            operation_id: parse_element_id(event_reference_id.as_deref().ok_or("Call trigger requires an Operation")?)?,
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
        .filter(|element| matches!(element.kind, ElementKind::PartProperty | ElementKind::ReferenceProperty))
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
pub fn behavior_snapshot(state: tauri::State<'_, WorkspaceState>) -> Result<BehaviorWorkspaceSnapshot, String> {
    Ok(BehaviorWorkspaceSnapshot {
        repository: state.behavior.lock().map_err(|_| "behavior lock poisoned")?.clone(),
        diagrams: state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")?.clone(),
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
    let mut repository = state.behavior.lock().map_err(|_| "behavior lock poisoned")?;
    let semantic_id = repository
        .create_state_machine(project, context_id, name.clone())
        .map_err(|error| error.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")?.push(BehaviorDiagram {
        id: id.clone(), name, owner_id: owner_id.to_string(), context_id: context_id.to_string(),
        kind: BehaviorDiagramKind::StateMachine, semantic_id: semantic_id.to_string(), state_nodes: Vec::new(), lifelines: Vec::new(),
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
    let mut repository = state.behavior.lock().map_err(|_| "behavior lock poisoned")?;
    let semantic_id = repository
        .create_interaction(project, context_id, name.clone())
        .map_err(|error| error.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")?.push(BehaviorDiagram {
        id: id.clone(), name, owner_id: owner_id.to_string(), context_id: context_id.to_string(),
        kind: BehaviorDiagramKind::Sequence, semantic_id: semantic_id.to_string(), state_nodes: Vec::new(), lifelines: Vec::new(),
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
    let mut diagrams = state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams.iter_mut().find(|diagram| diagram.id == diagram_id).ok_or("behavior diagram not found")?;
    if diagram.kind != BehaviorDiagramKind::StateMachine { return Err("active behavior diagram is not a State Machine".into()); }
    let machine_id = state_machine_id(&diagram.semantic_id)?;
    let mut repository = state.behavior.lock().map_err(|_| "behavior lock poisoned")?;
    let target_region = match region_id_value {
        Some(value) => region_id(&value)?,
        None => root_region_id(&repository, machine_id)?,
    };
    let machine = repository.state_machines.get_mut(&machine_id).ok_or("State Machine not found")?;
    let region = find_region_mut(&mut machine.regions, target_region).ok_or("Region not found")?;
    let parsed_kind = vertex_kind(&kind)?;
    if matches!(parsed_kind, VertexKind::Pseudostate(PseudostateKind::Initial))
        && region.vertices.iter().any(|vertex| matches!(vertex.kind, VertexKind::Pseudostate(PseudostateKind::Initial)))
    { return Err("A Region can have only one Initial pseudostate".into()); }
    let id = VertexId::new();
    region.vertices.push(Vertex { id, name, kind: parsed_kind });
    let (width, height) = if kind == "State" { (150.0, 80.0) } else { (24.0, 24.0) };
    diagram.state_nodes.push(StateNodePresentation { vertex_id: id.to_string(), x, y, width, height });
    Ok(id.to_string())
}

#[tauri::command]
pub fn add_state_region(
    diagram_id: String,
    state_vertex_id: String,
    name: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let diagrams = state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams.iter().find(|diagram| diagram.id == diagram_id).ok_or("behavior diagram not found")?;
    let machine_id = state_machine_id(&diagram.semantic_id)?;
    drop(diagrams);
    let mut repository = state.behavior.lock().map_err(|_| "behavior lock poisoned")?;
    let machine = repository.state_machines.get_mut(&machine_id).ok_or("State Machine not found")?;
    let vertex = find_vertex_mut(&mut machine.regions, vertex_id(&state_vertex_id)?).ok_or("State not found")?;
    let VertexKind::State(state_semantic) = &mut vertex.kind else { return Err("Regions can only be owned by a State".into()); };
    let id = RegionId::new();
    state_semantic.regions.push(Region { id, name, vertices: Vec::new(), transitions: Vec::new() });
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
    let diagrams = state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams.iter().find(|diagram| diagram.id == diagram_id).ok_or("behavior diagram not found")?;
    let machine_id = state_machine_id(&diagram.semantic_id)?;
    drop(diagrams);
    let mut repository = state.behavior.lock().map_err(|_| "behavior lock poisoned")?;
    let machine = repository.state_machines.get_mut(&machine_id).ok_or("State Machine not found")?;
    let vertex = find_vertex_mut(&mut machine.regions, vertex_id(&state_vertex_id)?).ok_or("State not found")?;
    let VertexKind::State(state_semantic) = &mut vertex.kind else { return Err("selected vertex is not a State".into()); };
    state_semantic.entry = entry.filter(|value| !value.trim().is_empty());
    state_semantic.do_activity = do_activity.filter(|value| !value.trim().is_empty());
    state_semantic.exit = exit.filter(|value| !value.trim().is_empty());
    Ok(())
}

#[tauri::command]
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
    let diagrams = state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams.iter().find(|diagram| diagram.id == diagram_id).ok_or("behavior diagram not found")?;
    let machine_id = state_machine_id(&diagram.semantic_id)?;
    drop(diagrams);
    let mut repository = state.behavior.lock().map_err(|_| "behavior lock poisoned")?;
    let root_region = match region_id_value { Some(value) => region_id(&value)?, None => root_region_id(&repository, machine_id)? };
    let machine = repository.state_machines.get_mut(&machine_id).ok_or("State Machine not found")?;
    let source_id = vertex_id(&source_vertex_id)?;
    let target_id = vertex_id(&target_vertex_id)?;
    let source = find_vertex(&machine.regions, source_id).ok_or("transition source vertex not found")?;
    find_vertex(&machine.regions, target_id).ok_or("transition target vertex not found")?;
    let trigger = event_from_input(event_kind, event_reference_id, event_expression)?;
    let guard = guard.filter(|value| !value.trim().is_empty());
    if matches!(source.kind, VertexKind::Pseudostate(PseudostateKind::Initial)) && (trigger.is_some() || guard.is_some()) {
        return Err("Initial transition must be triggerless and guardless. Connect Initial directly to its first State, then place triggers/guards on later Transitions.".into());
    }
    if matches!(source.kind, VertexKind::FinalState) { return Err("Final State cannot have outgoing Transitions".into()); }
    let id = TransitionId::new();
    let transition = Transition { id, source_id, target_id, kind: transition_kind(&kind)?, trigger, guard, effect: effect.filter(|value| !value.trim().is_empty()) };
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
    let mut diagrams = state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams.iter_mut().find(|diagram| diagram.id == diagram_id).ok_or("behavior diagram not found")?;
    let presentation = diagram.state_nodes.iter_mut().find(|node| node.vertex_id == state_vertex_id).ok_or("State presentation not found")?;
    presentation.x = x; presentation.y = y; Ok(())
}

#[tauri::command]
pub fn behavior_lifeline_candidates(
    diagram_id: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<Vec<LifelineCandidate>, String> {
    let diagrams = state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams.iter().find(|diagram| diagram.id == diagram_id).ok_or("behavior diagram not found")?;
    if diagram.kind != BehaviorDiagramKind::Sequence { return Err("active behavior diagram is not a Sequence Diagram".into()); }
    let context_id = parse_element_id(&diagram.context_id)?;
    drop(diagrams);
    let project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_ref().ok_or("no project open")?;
    let mut output = Vec::new();
    collect_lifeline_candidates(project, context_id, &mut Vec::new(), &mut Vec::new(), &mut output, 0);
    Ok(output)
}

#[tauri::command]
pub fn add_sequence_lifeline(
    diagram_id: String,
    represented_path: Vec<String>,
    x: f64,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let mut diagrams = state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams.iter_mut().find(|diagram| diagram.id == diagram_id).ok_or("behavior diagram not found")?;
    if diagram.kind != BehaviorDiagramKind::Sequence { return Err("active behavior diagram is not a Sequence Diagram".into()); }
    let interaction_id = parse_uuid(&diagram.semantic_id).map(systems_modeler_core::behavior::InteractionId)?;
    let context_id = parse_element_id(&diagram.context_id)?;
    let path: Vec<ElementId> = represented_path.iter().map(|value| parse_element_id(value)).collect::<Result<_, _>>()?;
    let project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_ref().ok_or("no project open")?;
    project.resolve_structural_path(context_id, &path).map_err(|error| error.to_string())?;
    let label = path.iter().map(|id| project.element(*id).map(|element| element.name.clone()).map_err(|error| error.to_string())).collect::<Result<Vec<_>, _>>()?.join(".");
    drop(project_guard);
    let id = LifelineId::new();
    let mut repository = state.behavior.lock().map_err(|_| "behavior lock poisoned")?;
    repository.interactions.get_mut(&interaction_id).ok_or("Interaction not found")?.lifelines.push(Lifeline { id, name: label, represented_path: path });
    diagram.lifelines.push(LifelinePresentation { lifeline_id: id.to_string(), x });
    Ok(id.to_string())
}

#[tauri::command]
pub fn move_sequence_lifeline(
    diagram_id: String,
    lifeline_id_value: String,
    x: f64,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let mut diagrams = state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams.iter_mut().find(|diagram| diagram.id == diagram_id).ok_or("behavior diagram not found")?;
    let presentation = diagram.lifelines.iter_mut().find(|item| item.lifeline_id == lifeline_id_value).ok_or("Lifeline presentation not found")?;
    presentation.x = x; Ok(())
}

#[tauri::command]
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
    let diagrams = state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams.iter().find(|diagram| diagram.id == diagram_id).ok_or("behavior diagram not found")?;
    let interaction_id = parse_uuid(&diagram.semantic_id).map(systems_modeler_core::behavior::InteractionId)?;
    drop(diagrams);
    let mut repository = state.behavior.lock().map_err(|_| "behavior lock poisoned")?;
    let interaction = repository.interactions.get_mut(&interaction_id).ok_or("Interaction not found")?;
    let sort = message_sort(&sort)?;
    let next = next_occurrence_order(&interaction.messages);
    let send_event = source_lifeline_id.as_deref().map(lifeline_id).transpose()?.map(|lifeline_id| Occurrence { id: OccurrenceId::new(), lifeline_id, order: next });
    let receive_event = target_lifeline_id.as_deref().map(lifeline_id).transpose()?.map(|lifeline_id| Occurrence { id: OccurrenceId::new(), lifeline_id, order: next + 5 });
    let signature = match sort {
        MessageSort::SynchCall | MessageSort::AsynchCall => signature_id.as_deref().map(parse_element_id).transpose()?.map(MessageSignature::Operation),
        MessageSort::AsynchSignal => signature_id.as_deref().map(parse_element_id).transpose()?.map(MessageSignature::Signal),
        _ => None,
    };
    let id = MessageId::new();
    interaction.messages.push(Message { id, name, sort, send_event, receive_event, signature, arguments });
    let project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    systems_modeler_core::behavior::validate_interaction(project_guard.as_ref().ok_or("no project open")?, interaction).map_err(|error| { interaction.messages.pop(); error.to_string() })?;
    Ok(id.to_string())
}

#[tauri::command]
pub fn add_execution_specification(
    diagram_id: String,
    lifeline_id_value: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let diagrams = state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams.iter().find(|diagram| diagram.id == diagram_id).ok_or("behavior diagram not found")?;
    let interaction_id = parse_uuid(&diagram.semantic_id).map(systems_modeler_core::behavior::InteractionId)?;
    drop(diagrams);
    let mut repository = state.behavior.lock().map_err(|_| "behavior lock poisoned")?;
    let interaction = repository.interactions.get_mut(&interaction_id).ok_or("Interaction not found")?;
    let lifeline_id = lifeline_id(&lifeline_id_value)?;
    if !interaction.lifelines.iter().any(|item| item.id == lifeline_id) { return Err("Execution must be attached to an existing Lifeline".into()); }
    let start_order = next_occurrence_order(&interaction.messages);
    let id = systems_modeler_core::behavior::ExecutionId::new();
    interaction.executions.push(ExecutionSpecification {
        id, lifeline_id,
        start: Occurrence { id: OccurrenceId::new(), lifeline_id, order: start_order },
        finish: Occurrence { id: OccurrenceId::new(), lifeline_id, order: start_order + 20 },
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
    let diagrams = state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams.iter().find(|diagram| diagram.id == diagram_id).ok_or("behavior diagram not found")?;
    let interaction_id = parse_uuid(&diagram.semantic_id).map(systems_modeler_core::behavior::InteractionId)?;
    drop(diagrams);
    let mut repository = state.behavior.lock().map_err(|_| "behavior lock poisoned")?;
    let interaction = repository.interactions.get_mut(&interaction_id).ok_or("Interaction not found")?;
    let covered_lifelines = covered_lifeline_ids.iter().map(|value| lifeline_id(value)).collect::<Result<Vec<_>, _>>()?;
    let start = next_occurrence_order(&interaction.messages);
    let operator = interaction_operator(&operator)?;
    let mut operands = vec![InteractionOperand { id: systems_modeler_core::behavior::OperandId::new(), guard: guard.filter(|value| !value.trim().is_empty()), start_order: start, end_order: start + 30 }];
    if operator == InteractionOperator::Alt { operands.push(InteractionOperand { id: systems_modeler_core::behavior::OperandId::new(), guard: Some("else".into()), start_order: start + 30, end_order: start + 60 }); }
    let id = systems_modeler_core::behavior::FragmentId::new();
    interaction.fragments.push(CombinedFragment { id, operator, covered_lifelines, operands });
    Ok(id.to_string())
}

#[tauri::command]
pub fn add_state_invariant(
    diagram_id: String,
    lifeline_id_value: String,
    constraint: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    if constraint.trim().is_empty() { return Err("State Invariant requires a non-empty constraint".into()); }
    let diagrams = state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams.iter().find(|diagram| diagram.id == diagram_id).ok_or("behavior diagram not found")?;
    let interaction_id = parse_uuid(&diagram.semantic_id).map(systems_modeler_core::behavior::InteractionId)?;
    drop(diagrams);
    let mut repository = state.behavior.lock().map_err(|_| "behavior lock poisoned")?;
    let interaction = repository.interactions.get_mut(&interaction_id).ok_or("Interaction not found")?;
    let id = systems_modeler_core::behavior::InvariantId::new();
    interaction.state_invariants.push(StateInvariant { id, lifeline_id: lifeline_id(&lifeline_id_value)?, order: next_occurrence_order(&interaction.messages), constraint });
    Ok(id.to_string())
}

pub fn validate_behavior_workspace(project: &Project, repository: &BehaviorRepository, diagrams: &[BehaviorDiagram]) -> Result<(), String> {
    repository.validate(project).map_err(|error| error.to_string())?;
    for diagram in diagrams {
        parse_element_id(&diagram.context_id)?;
        parse_element_id(&diagram.owner_id)?;
        match diagram.kind {
            BehaviorDiagramKind::StateMachine => {
                let id = state_machine_id(&diagram.semantic_id)?;
                if !repository.state_machines.contains_key(&id) { return Err(format!("State Machine semantic object missing for diagram {}", diagram.name)); }
            }
            BehaviorDiagramKind::Sequence => {
                let id = parse_uuid(&diagram.semantic_id).map(systems_modeler_core::behavior::InteractionId)?;
                if !repository.interactions.contains_key(&id) { return Err(format!("Interaction semantic object missing for diagram {}", diagram.name)); }
            }
        }
    }
    Ok(())
}

pub fn save_behavior_metadata(database: &mut ProjectDatabase, project: &Project, repository: &BehaviorRepository, diagrams: &[BehaviorDiagram]) -> Result<(), String> {
    validate_behavior_workspace(project, repository, diagrams)?;
    database.save_metadata(project.id, BEHAVIOR_METADATA_KEY, &serde_json::to_string(repository).map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
    database.save_metadata(project.id, BEHAVIOR_DIAGRAM_METADATA_KEY, &serde_json::to_string(diagrams).map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
    Ok(())
}

pub fn load_behavior_metadata(database: &ProjectDatabase, project: &Project) -> Result<(BehaviorRepository, Vec<BehaviorDiagram>), String> {
    let repository = match database.load_metadata(project.id, BEHAVIOR_METADATA_KEY).map_err(|error| error.to_string())? {
        Some(payload) => serde_json::from_str(&payload).map_err(|error| format!("invalid saved behavior semantics: {error}"))?,
        None => BehaviorRepository::default(),
    };
    let diagrams = match database.load_metadata(project.id, BEHAVIOR_DIAGRAM_METADATA_KEY).map_err(|error| error.to_string())? {
        Some(payload) => serde_json::from_str(&payload).map_err(|error| format!("invalid saved behavior presentation: {error}"))?,
        None => Vec::new(),
    };
    validate_behavior_workspace(project, &repository, &diagrams)?;
    Ok((repository, diagrams))
}
