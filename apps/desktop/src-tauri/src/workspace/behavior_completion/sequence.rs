use super::super::behavior_workspace::BehaviorDiagramKind;
use super::super::{WorkspaceState, parse_element_id};
use systems_modeler_core::Project;
use systems_modeler_core::behavior::{
    ExecutionId, FragmentId, InteractionId, InteractionOperand, InvariantId, LifelineId, MessageId,
    MessageSignature, MessageSort, Occurrence, OccurrenceId, OperandId,
};

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

fn behavior_semantic_id(state: &WorkspaceState, diagram_id: &str) -> Result<String, String> {
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
    let semantic_id = behavior_semantic_id(&state, &diagram_id)?;
    let interaction_id = interaction_id(&semantic_id)?;
    let wanted = message_id(&message_id_value)?;
    let sort = parse_message_sort(&sort)?;
    let signature = match sort {
        MessageSort::SynchCall | MessageSort::AsynchCall => {
            Some(MessageSignature::Operation(parse_element_id(
                signature_id
                    .as_deref()
                    .ok_or("Call Message requires an Operation signature")?,
            )?))
        }
        MessageSort::AsynchSignal => Some(MessageSignature::Signal(parse_element_id(
            signature_id
                .as_deref()
                .ok_or("Signal Message requires a Signal signature")?,
        )?)),
        _ => None,
    };
    let mut repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
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
    if let Err(error) = systems_modeler_core::behavior::validate_interaction(&project, interaction)
    {
        *interaction
            .messages
            .iter_mut()
            .find(|message| message.id == wanted)
            .ok_or("Message not found")? = original;
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
    let semantic_id = behavior_semantic_id(&state, &diagram_id)?;
    let interaction_id = interaction_id(&semantic_id)?;
    let wanted = message_id(&message_id_value)?;
    let endpoint = lifeline_id_value.as_deref().map(lifeline_id).transpose()?;
    let mut repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    let interaction = repository
        .interactions
        .get_mut(&interaction_id)
        .ok_or("Interaction not found")?;
    if let Some(id) = endpoint
        && !interaction
            .lifelines
            .iter()
            .any(|lifeline| lifeline.id == id)
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
        .or_else(|| {
            message
                .receive_event
                .as_ref()
                .map(|event| event.order.saturating_sub(1))
        })
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
    if let Err(error) = systems_modeler_core::behavior::validate_interaction(&project, interaction)
    {
        *interaction
            .messages
            .iter_mut()
            .find(|message| message.id == wanted)
            .ok_or("Message not found")? = original;
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
    let semantic_id = behavior_semantic_id(&state, &diagram_id)?;
    let interaction_id = interaction_id(&semantic_id)?;
    let wanted = parse_uuid(&execution_id_value).map(ExecutionId)?;
    let mut repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
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
    if let Err(error) = systems_modeler_core::behavior::validate_interaction(&project, interaction)
    {
        *interaction
            .executions
            .iter_mut()
            .find(|item| item.id == wanted)
            .ok_or("Execution Specification not found")? = original;
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
    let semantic_id = behavior_semantic_id(&state, &diagram_id)?;
    let interaction_id = interaction_id(&semantic_id)?;
    let fragment_id = parse_uuid(&fragment_id_value).map(FragmentId)?;
    let mut repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
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
    if let Err(error) = systems_modeler_core::behavior::validate_interaction(&project, interaction)
    {
        if let Some(fragment) = interaction
            .fragments
            .iter_mut()
            .find(|item| item.id == fragment_id)
        {
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
    let semantic_id = behavior_semantic_id(&state, &diagram_id)?;
    let interaction_id = interaction_id(&semantic_id)?;
    let fragment_id = parse_uuid(&fragment_id_value).map(FragmentId)?;
    let operand_id = operand_id(&operand_id_value)?;
    let mut repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
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
    if let Err(error) = systems_modeler_core::behavior::validate_interaction(&project, interaction)
    {
        let fragment = interaction
            .fragments
            .iter_mut()
            .find(|item| item.id == fragment_id)
            .ok_or("Combined Fragment not found")?;
        *fragment
            .operands
            .iter_mut()
            .find(|item| item.id == operand_id)
            .ok_or("Interaction Operand not found")? = original;
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
    let semantic_id = behavior_semantic_id(&state, &diagram_id)?;
    let interaction_id = interaction_id(&semantic_id)?;
    let wanted = parse_uuid(&invariant_id_value).map(InvariantId)?;
    let mut repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
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
    if let Err(error) = systems_modeler_core::behavior::validate_interaction(&project, interaction)
    {
        *interaction
            .state_invariants
            .iter_mut()
            .find(|item| item.id == wanted)
            .ok_or("State Invariant not found")? = original;
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
}
