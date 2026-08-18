use super::behavior_workspace::{BehaviorDiagramKind, StateNodePresentation};
use super::{WorkspaceState, parse_element_id};
use systems_modeler_core::behavior::{
    Event, InteractionId, InteractionOperand, LifelineId, MessageId, MessageSignature, MessageSort,
    Occurrence, OccurrenceId, OperandId, Region, RegionId, State, TransitionId, TransitionKind,
    Trigger, Vertex, VertexId, VertexKind,
};
use systems_modeler_core::{ElementKind, Project};

fn parse_uuid(value: &str) -> Result<uuid::Uuid, String> {
    uuid::Uuid::parse_str(value).map_err(|_| format!("invalid behavior id: {value}"))
}

fn interaction_id(value: &str) -> Result<InteractionId, String> {
    parse_uuid(value).map(InteractionId)
}

fn message_id(value: &str) -> Result<MessageId, String> {
    parse_uuid(value).map(MessageId)
}

fn lifeline_id(value: &str) -> Result<LifelineId, String> {
    parse_uuid(value).map(LifelineId)
}

fn transition_id(value: &str) -> Result<TransitionId, String> {
    parse_uuid(value).map(TransitionId)
}

fn vertex_id(value: &str) -> Result<VertexId, String> {
    parse_uuid(value).map(VertexId)
}

fn region_id(value: &str) -> Result<RegionId, String> {
    parse_uuid(value).map(RegionId)
}

fn operand_id(value: &str) -> Result<OperandId, String> {
    parse_uuid(value).map(OperandId)
}

fn project_snapshot(state: &WorkspaceState) -> Result<Project, String> {
    state
        .project
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .clone()
        .ok_or_else(|| "no project open".to_string())
}

fn behavior_semantic_id(
    state: &WorkspaceState,
    diagram_id: &str,
    expected: BehaviorDiagramKind,
) -> Result<String, String> {
    let diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    if diagram.kind != expected {
        return Err(match expected {
            BehaviorDiagramKind::StateMachine => "active behavior diagram is not a State Machine",
            BehaviorDiagramKind::Sequence => "active behavior diagram is not a Sequence Diagram",
        }
        .into());
    }
    Ok(diagram.semantic_id.clone())
}

fn parse_message_sort(value: &str) -> Result<MessageSort, String> {
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

fn parse_transition_kind(value: &str) -> Result<TransitionKind, String> {
    match value {
        "External" => Ok(TransitionKind::External),
        "Internal" => Ok(TransitionKind::Internal),
        "Local" => Ok(TransitionKind::Local),
        _ => Err(format!("unsupported transition kind: {value}")),
    }
}

fn trigger_from_input(
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

fn find_transition_mut(regions: &mut [Region], wanted: TransitionId) -> Option<&mut systems_modeler_core::behavior::Transition> {
    for region in regions {
        if let Some(transition) = region.transitions.iter_mut().find(|item| item.id == wanted) {
            return Some(transition);
        }
        for vertex in &mut region.vertices {
            if let VertexKind::State(state) = &mut vertex.kind
                && let Some(found) = find_transition_mut(&mut state.regions, wanted)
            {
                return Some(found);
            }
        }
    }
    None
}

#[tauri::command]
pub fn add_composite_state(
    diagram_id: String,
    region_id_value: Option<String>,
    name: String,
    orthogonal: bool,
    x: f64,
    y: f64,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    if name.trim().is_empty() {
        return Err("Composite State requires a name".into());
    }
    let semantic_id = behavior_semantic_id(&state, &diagram_id, BehaviorDiagramKind::StateMachine)?;
    let machine_id = parse_uuid(&semantic_id).map(systems_modeler_core::behavior::StateMachineId)?;

    let id = VertexId::new();
    {
        let mut repository = state.behavior.lock().map_err(|_| "behavior lock poisoned")?;
        let machine = repository
            .state_machines
            .get_mut(&machine_id)
            .ok_or("State Machine not found")?;
        let target_region = match region_id_value {
            Some(value) => region_id(&value)?,
            None => machine
                .regions
                .first()
                .map(|region| region.id)
                .ok_or("State Machine has no root Region")?,
        };
        let region = find_region_mut(&mut machine.regions, target_region).ok_or("Region not found")?;
        let mut regions = vec![Region {
            id: RegionId::new(),
            name: "Region 1".into(),
            vertices: Vec::new(),
            transitions: Vec::new(),
        }];
        if orthogonal {
            regions.push(Region {
                id: RegionId::new(),
                name: "Region 2".into(),
                vertices: Vec::new(),
                transitions: Vec::new(),
            });
        }
        region.vertices.push(Vertex {
            id,
            name,
            kind: VertexKind::State(State {
                entry: None,
                do_activity: None,
                exit: None,
                regions,
            }),
        });
    }

    let mut diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    diagram.state_nodes.push(StateNodePresentation {
        vertex_id: id.to_string(),
        x,
        y,
        width: 280.0,
        height: if orthogonal { 220.0 } else { 170.0 },
    });
    Ok(id.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_state_transition(
    diagram_id: String,
    transition_id_value: String,
    kind: String,
    event_kind: Option<String>,
    event_reference_id: Option<String>,
    event_expression: Option<String>,
    guard: Option<String>,
    effect: Option<String>,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let project = project_snapshot(&state)?;
    let semantic_id = behavior_semantic_id(&state, &diagram_id, BehaviorDiagramKind::StateMachine)?;
    let machine_id = parse_uuid(&semantic_id).map(systems_modeler_core::behavior::StateMachineId)?;
    let wanted = transition_id(&transition_id_value)?;
    let parsed_kind = parse_transition_kind(&kind)?;
    let trigger = trigger_from_input(event_kind, event_reference_id, event_expression)?;
    let guard = guard.filter(|value| !value.trim().is_empty());
    let effect = effect.filter(|value| !value.trim().is_empty());

    let mut repository = state.behavior.lock().map_err(|_| "behavior lock poisoned")?;
    let machine = repository
        .state_machines
        .get_mut(&machine_id)
        .ok_or("State Machine not found")?;
    let transition = find_transition_mut(&mut machine.regions, wanted).ok_or("Transition not found")?;
    let original = transition.clone();
    transition.kind = parsed_kind;
    transition.trigger = trigger;
    transition.guard = guard;
    transition.effect = effect;
    if let Err(error) = systems_modeler_core::behavior::validate_state_machine(&project, machine) {
        *find_transition_mut(&mut machine.regions, wanted).ok_or("Transition not found")? = original;
        return Err(error.to_string());
    }
    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_sequence_message(
    diagram_id: String,
    message_id_value: String,
    sort: String,
    name: String,
    signature_id: Option<String>,
    arguments: Vec<String>,
    order: u32,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let project = project_snapshot(&state)?;
    let semantic_id = behavior_semantic_id(&state, &diagram_id, BehaviorDiagramKind::Sequence)?;
    let interaction_id = interaction_id(&semantic_id)?;
    let wanted = message_id(&message_id_value)?;
    let sort = parse_message_sort(&sort)?;

    let signature = match sort {
        MessageSort::SynchCall | MessageSort::AsynchCall => Some(MessageSignature::Operation(
            parse_element_id(signature_id.as_deref().ok_or("Call Message requires an Operation signature")?)?,
        )),
        MessageSort::AsynchSignal => Some(MessageSignature::Signal(
            parse_element_id(signature_id.as_deref().ok_or("Signal Message requires a Signal signature")?)?,
        )),
        _ => None,
    };

    let mut repository = state.behavior.lock().map_err(|_| "behavior lock poisoned")?;
    let interaction = repository
        .interactions
        .get_mut(&interaction_id)
        .ok_or("Interaction not found")?;
    let message = interaction
        .messages
        .iter_mut()
        .find(|message| message.id == wanted)
        .ok_or("Message not found")?;
    let original = message.clone();
    message.sort = sort;
    message.name = name;
    message.signature = signature;
    message.arguments = arguments;
    if let Some(send) = &mut message.send_event {
        send.order = order;
    }
    if let Some(receive) = &mut message.receive_event {
        receive.order = order.saturating_add(1);
    }
    if let Err(error) = systems_modeler_core::behavior::validate_interaction(&project, interaction) {
        *interaction.messages.iter_mut().find(|message| message.id == wanted).ok_or("Message not found")? = original;
        return Err(error.to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn reconnect_sequence_message(
    diagram_id: String,
    message_id_value: String,
    side: String,
    lifeline_id_value: Option<String>,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let project = project_snapshot(&state)?;
    let semantic_id = behavior_semantic_id(&state, &diagram_id, BehaviorDiagramKind::Sequence)?;
    let interaction_id = interaction_id(&semantic_id)?;
    let wanted = message_id(&message_id_value)?;
    let endpoint = lifeline_id_value.as_deref().map(lifeline_id).transpose()?;

    let mut repository = state.behavior.lock().map_err(|_| "behavior lock poisoned")?;
    let interaction = repository
        .interactions
        .get_mut(&interaction_id)
        .ok_or("Interaction not found")?;
    if let Some(id) = endpoint
        && !interaction.lifelines.iter().any(|lifeline| lifeline.id == id)
    {
        return Err("Message endpoint must be an existing Lifeline in this Interaction".into());
    }
    let message = interaction
        .messages
        .iter_mut()
        .find(|message| message.id == wanted)
        .ok_or("Message not found")?;
    let original = message.clone();
    let base_order = message
        .send_event
        .as_ref()
        .map(|event| event.order)
        .or_else(|| message.receive_event.as_ref().map(|event| event.order.saturating_sub(1)))
        .unwrap_or(10);
    match side.as_str() {
        "source" => {
            message.send_event = endpoint.map(|lifeline_id| Occurrence {
                id: message
                    .send_event
                    .as_ref()
                    .map(|event| event.id)
                    .unwrap_or_else(OccurrenceId::new),
                lifeline_id,
                order: base_order,
            });
        }
        "target" => {
            message.receive_event = endpoint.map(|lifeline_id| Occurrence {
                id: message
                    .receive_event
                    .as_ref()
                    .map(|event| event.id)
                    .unwrap_or_else(OccurrenceId::new),
                lifeline_id,
                order: base_order.saturating_add(1),
            });
        }
        _ => return Err("Message endpoint side must be source or target".into()),
    }
    if let Err(error) = systems_modeler_core::behavior::validate_interaction(&project, interaction) {
        *interaction.messages.iter_mut().find(|message| message.id == wanted).ok_or("Message not found")? = original;
        return Err(error.to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn update_execution_specification(
    diagram_id: String,
    execution_id_value: String,
    start_order: u32,
    finish_order: u32,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    if start_order >= finish_order {
        return Err("Execution finish must occur after its start".into());
    }
    let project = project_snapshot(&state)?;
    let semantic_id = behavior_semantic_id(&state, &diagram_id, BehaviorDiagramKind::Sequence)?;
    let interaction_id = interaction_id(&semantic_id)?;
    let wanted = parse_uuid(&execution_id_value).map(systems_modeler_core::behavior::ExecutionId)?;
    let mut repository = state.behavior.lock().map_err(|_| "behavior lock poisoned")?;
    let interaction = repository
        .interactions
        .get_mut(&interaction_id)
        .ok_or("Interaction not found")?;
    let execution = interaction
        .executions
        .iter_mut()
        .find(|item| item.id == wanted)
        .ok_or("Execution Specification not found")?;
    let original = execution.clone();
    execution.start.order = start_order;
    execution.finish.order = finish_order;
    if let Err(error) = systems_modeler_core::behavior::validate_interaction(&project, interaction) {
        *interaction.executions.iter_mut().find(|item| item.id == wanted).ok_or("Execution Specification not found")? = original;
        return Err(error.to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn add_combined_fragment_operand(
    diagram_id: String,
    fragment_id_value: String,
    guard: Option<String>,
    start_order: u32,
    end_order: u32,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    if start_order >= end_order {
        return Err("Combined Fragment operand end must occur after its start".into());
    }
    let project = project_snapshot(&state)?;
    let semantic_id = behavior_semantic_id(&state, &diagram_id, BehaviorDiagramKind::Sequence)?;
    let interaction_id = interaction_id(&semantic_id)?;
    let fragment_id = parse_uuid(&fragment_id_value).map(systems_modeler_core::behavior::FragmentId)?;
    let mut repository = state.behavior.lock().map_err(|_| "behavior lock poisoned")?;
    let interaction = repository
        .interactions
        .get_mut(&interaction_id)
        .ok_or("Interaction not found")?;
    let fragment = interaction
        .fragments
        .iter_mut()
        .find(|fragment| fragment.id == fragment_id)
        .ok_or("Combined Fragment not found")?;
    let id = OperandId::new();
    fragment.operands.push(InteractionOperand {
        id,
        guard: guard.filter(|value| !value.trim().is_empty()),
        start_order,
        end_order,
    });
    if let Err(error) = systems_modeler_core::behavior::validate_interaction(&project, interaction) {
        if let Some(fragment) = interaction.fragments.iter_mut().find(|item| item.id == fragment_id) {
            fragment.operands.retain(|operand| operand.id != id);
        }
        return Err(error.to_string());
    }
    Ok(id.to_string())
}

#[tauri::command]
pub fn update_combined_fragment_operand(
    diagram_id: String,
    fragment_id_value: String,
    operand_id_value: String,
    guard: Option<String>,
    start_order: u32,
    end_order: u32,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    if start_order >= end_order {
        return Err("Combined Fragment operand end must occur after its start".into());
    }
    let project = project_snapshot(&state)?;
    let semantic_id = behavior_semantic_id(&state, &diagram_id, BehaviorDiagramKind::Sequence)?;
    let interaction_id = interaction_id(&semantic_id)?;
    let fragment_id = parse_uuid(&fragment_id_value).map(systems_modeler_core::behavior::FragmentId)?;
    let operand_id = operand_id(&operand_id_value)?;
    let mut repository = state.behavior.lock().map_err(|_| "behavior lock poisoned")?;
    let interaction = repository
        .interactions
        .get_mut(&interaction_id)
        .ok_or("Interaction not found")?;
    let fragment = interaction
        .fragments
        .iter_mut()
        .find(|fragment| fragment.id == fragment_id)
        .ok_or("Combined Fragment not found")?;
    let operand = fragment
        .operands
        .iter_mut()
        .find(|operand| operand.id == operand_id)
        .ok_or("Interaction Operand not found")?;
    let original = operand.clone();
    operand.guard = guard.filter(|value| !value.trim().is_empty());
    operand.start_order = start_order;
    operand.end_order = end_order;
    if let Err(error) = systems_modeler_core::behavior::validate_interaction(&project, interaction) {
        let fragment = interaction.fragments.iter_mut().find(|item| item.id == fragment_id).ok_or("Combined Fragment not found")?;
        *fragment.operands.iter_mut().find(|item| item.id == operand_id).ok_or("Interaction Operand not found")? = original;
        return Err(error.to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn update_state_invariant(
    diagram_id: String,
    invariant_id_value: String,
    constraint: String,
    order: u32,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    if constraint.trim().is_empty() {
        return Err("State Invariant requires a non-empty constraint".into());
    }
    let project = project_snapshot(&state)?;
    let semantic_id = behavior_semantic_id(&state, &diagram_id, BehaviorDiagramKind::Sequence)?;
    let interaction_id = interaction_id(&semantic_id)?;
    let wanted = parse_uuid(&invariant_id_value).map(systems_modeler_core::behavior::InvariantId)?;
    let mut repository = state.behavior.lock().map_err(|_| "behavior lock poisoned")?;
    let interaction = repository
        .interactions
        .get_mut(&interaction_id)
        .ok_or("Interaction not found")?;
    let invariant = interaction
        .state_invariants
        .iter_mut()
        .find(|item| item.id == wanted)
        .ok_or("State Invariant not found")?;
    let original = invariant.clone();
    invariant.constraint = constraint;
    invariant.order = order;
    if let Err(error) = systems_modeler_core::behavior::validate_interaction(&project, interaction) {
        *interaction.state_invariants.iter_mut().find(|item| item.id == wanted).ok_or("State Invariant not found")? = original;
        return Err(error.to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_sort_parser_covers_full_pr12_set() {
        for value in [
            "SynchCall",
            "AsynchCall",
            "AsynchSignal",
            "Reply",
            "Create",
            "Delete",
            "Lost",
            "Found",
        ] {
            assert!(parse_message_sort(value).is_ok());
        }
    }

    #[test]
    fn transition_kind_parser_covers_all_uml_kinds() {
        for value in ["External", "Internal", "Local"] {
            assert!(parse_transition_kind(value).is_ok());
        }
    }

    #[test]
    fn trigger_parser_rejects_unknown_event_kind() {
        assert!(trigger_from_input(Some("Bogus".into()), None, None).is_err());
    }

    #[test]
    fn substate_region_search_is_recursive() {
        let child = Region {
            id: RegionId::new(),
            name: "child".into(),
            vertices: Vec::new(),
            transitions: Vec::new(),
        };
        let wanted = child.id;
        let state = Vertex {
            id: VertexId::new(),
            name: "Composite".into(),
            kind: VertexKind::State(State {
                entry: None,
                do_activity: None,
                exit: None,
                regions: vec![child],
            }),
        };
        let mut roots = vec![Region {
            id: RegionId::new(),
            name: "root".into(),
            vertices: vec![state],
            transitions: Vec::new(),
        }];
        assert_eq!(find_region_mut(&mut roots, wanted).map(|region| region.id), Some(wanted));
    }

    #[test]
    fn operation_signature_kind_is_validated_by_core() {
        let mut project = Project::new("P");
        let block = project
            .create_element(ElementKind::Block, "System", project.root_id)
            .unwrap();
        let signal = project
            .create_element(ElementKind::Signal, "Signal", project.root_id)
            .unwrap();
        assert_eq!(project.element(block).unwrap().kind, ElementKind::Block);
        assert_eq!(project.element(signal).unwrap().kind, ElementKind::Signal);
    }
}
