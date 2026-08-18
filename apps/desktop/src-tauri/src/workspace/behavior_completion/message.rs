use super::super::behavior_workspace::BehaviorDiagramKind;
use super::super::{WorkspaceState, parse_element_id};
use systems_modeler_core::Project;
use systems_modeler_core::behavior::{
    InteractionId, LifelineId, MessageId, MessageSignature, MessageSort, Occurrence, OccurrenceId,
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

fn project_snapshot(state: &WorkspaceState) -> Result<Project, String> {
    state
        .project
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .clone()
        .ok_or_else(|| "no project open".to_string())
}

fn sequence_semantic_id(state: &WorkspaceState, diagram_id: &str) -> Result<String, String> {
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

fn occurrence(previous: Option<&Occurrence>, lifeline_id: LifelineId, order: u32) -> Occurrence {
    Occurrence {
        id: previous
            .map(|item| item.id)
            .unwrap_or_else(OccurrenceId::new),
        lifeline_id,
        order,
    }
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_sequence_message_complete(
    diagram_id: String,
    message_id_value: String,
    sort: String,
    name: String,
    signature_id: Option<String>,
    arguments: Vec<String>,
    order: u32,
    source_lifeline_id: Option<String>,
    target_lifeline_id: Option<String>,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let project = project_snapshot(&state)?;
    let semantic_id = sequence_semantic_id(&state, &diagram_id)?;
    let interaction_id = interaction_id(&semantic_id)?;
    let wanted = message_id(&message_id_value)?;
    let sort = parse_message_sort(&sort)?;
    let source = source_lifeline_id.as_deref().map(lifeline_id).transpose()?;
    let target = target_lifeline_id.as_deref().map(lifeline_id).transpose()?;
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
    for endpoint in [source, target].into_iter().flatten() {
        if !interaction
            .lifelines
            .iter()
            .any(|lifeline| lifeline.id == endpoint)
        {
            return Err("Message endpoint must be a Lifeline in this Interaction".into());
        }
    }

    let index = interaction
        .messages
        .iter()
        .position(|message| message.id == wanted)
        .ok_or("Message not found")?;
    let original = interaction.messages[index].clone();
    let message = &mut interaction.messages[index];
    message.sort = sort;
    message.name = name;
    message.signature = signature;
    message.arguments = arguments;
    message.send_event = source.map(|id| occurrence(original.send_event.as_ref(), id, order));
    message.receive_event =
        target.map(|id| occurrence(original.receive_event.as_ref(), id, order.saturating_add(1)));

    if let Err(error) = systems_modeler_core::behavior::validate_interaction(&project, interaction)
    {
        interaction.messages[index] = original;
        return Err(error.to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_message_sorts_parse() {
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
