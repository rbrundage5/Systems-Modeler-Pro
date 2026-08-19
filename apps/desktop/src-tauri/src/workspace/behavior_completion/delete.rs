use super::super::WorkspaceState;
use super::super::behavior_workspace::BehaviorDiagramKind;
use super::validation::validate_state_machine_editing;
use std::collections::HashSet;
use systems_modeler_core::Project;
use systems_modeler_core::behavior::{
    FragmentId, InteractionId, InvariantId, LifelineId, MessageId, Region, StateMachineId,
    TransitionId, VertexId, VertexKind,
};

fn parse_uuid(value: &str) -> Result<uuid::Uuid, String> {
    uuid::Uuid::parse_str(value).map_err(|_| format!("invalid behavior id: {value}"))
}

fn state_machine_id(value: &str) -> Result<StateMachineId, String> {
    parse_uuid(value).map(StateMachineId)
}

fn interaction_id(value: &str) -> Result<InteractionId, String> {
    parse_uuid(value).map(InteractionId)
}

fn project_snapshot(state: &WorkspaceState) -> Result<Project, String> {
    state
        .project
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .clone()
        .ok_or_else(|| "no project open".to_string())
}

fn diagram_semantics(
    state: &WorkspaceState,
    diagram_id: &str,
) -> Result<(BehaviorDiagramKind, String), String> {
    let diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    Ok((diagram.kind.clone(), diagram.semantic_id.clone()))
}

fn collect_vertex_subtree(
    vertex: &systems_modeler_core::behavior::Vertex,
    ids: &mut HashSet<VertexId>,
) {
    ids.insert(vertex.id);
    if let VertexKind::State(state) = &vertex.kind {
        for region in &state.regions {
            for child in &region.vertices {
                collect_vertex_subtree(child, ids);
            }
        }
    }
}

fn remove_vertex_from_regions(
    regions: &mut [Region],
    wanted: VertexId,
) -> Option<HashSet<VertexId>> {
    for region in regions {
        if let Some(index) = region
            .vertices
            .iter()
            .position(|vertex| vertex.id == wanted)
        {
            let vertex = region.vertices.remove(index);
            let mut removed = HashSet::new();
            collect_vertex_subtree(&vertex, &mut removed);
            region.transitions.retain(|transition| {
                !removed.contains(&transition.source_id) && !removed.contains(&transition.target_id)
            });
            return Some(removed);
        }
        for vertex in &mut region.vertices {
            if let VertexKind::State(state) = &mut vertex.kind
                && let Some(removed) = remove_vertex_from_regions(&mut state.regions, wanted)
            {
                for child_region in &mut state.regions {
                    child_region.transitions.retain(|transition| {
                        !removed.contains(&transition.source_id)
                            && !removed.contains(&transition.target_id)
                    });
                }
                return Some(removed);
            }
        }
    }
    None
}

fn remove_transition(regions: &mut [Region], wanted: TransitionId) -> bool {
    for region in regions {
        let before = region.transitions.len();
        region
            .transitions
            .retain(|transition| transition.id != wanted);
        if region.transitions.len() != before {
            return true;
        }
        for vertex in &mut region.vertices {
            if let VertexKind::State(state) = &mut vertex.kind
                && remove_transition(&mut state.regions, wanted)
            {
                return true;
            }
        }
    }
    false
}

fn delete_state_item(
    state: &WorkspaceState,
    diagram_id: &str,
    semantic_id: &str,
    item_type: &str,
    item_id: &str,
) -> Result<(), String> {
    let project = project_snapshot(state)?;
    let machine_id = state_machine_id(semantic_id)?;
    match item_type {
        "Vertex" => {
            let wanted = parse_uuid(item_id).map(VertexId)?;
            let removed = {
                let mut repository = state
                    .behavior
                    .lock()
                    .map_err(|_| "behavior lock poisoned")?;
                let machine = repository
                    .state_machines
                    .get_mut(&machine_id)
                    .ok_or("State Machine not found")?;
                let original = machine.clone();
                let removed = remove_vertex_from_regions(&mut machine.regions, wanted)
                    .ok_or("State not found")?;
                if let Err(error) = validate_state_machine_editing(&project, machine) {
                    *machine = original;
                    return Err(format!(
                        "Deletion rejected because it would leave the State Machine invalid: {error}"
                    ));
                }
                removed
            };
            let mut diagrams = state
                .behavior_diagrams
                .lock()
                .map_err(|_| "behavior diagram lock poisoned")?;
            let diagram = diagrams
                .iter_mut()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("behavior diagram not found")?;
            diagram.state_nodes.retain(|node| {
                parse_uuid(&node.vertex_id)
                    .map(VertexId)
                    .map(|id| !removed.contains(&id))
                    .unwrap_or(true)
            });
            Ok(())
        }
        "Transition" => {
            let wanted = parse_uuid(item_id).map(TransitionId)?;
            let mut repository = state
                .behavior
                .lock()
                .map_err(|_| "behavior lock poisoned")?;
            let machine = repository
                .state_machines
                .get_mut(&machine_id)
                .ok_or("State Machine not found")?;
            let original = machine.clone();
            if !remove_transition(&mut machine.regions, wanted) {
                return Err("Transition not found".into());
            }
            if let Err(error) = validate_state_machine_editing(&project, machine) {
                *machine = original;
                return Err(format!(
                    "Deletion rejected because it would leave the State Machine invalid: {error}"
                ));
            }
            Ok(())
        }
        _ => Err(format!(
            "unsupported State Machine deletion type: {item_type}"
        )),
    }
}

fn delete_sequence_item(
    state: &WorkspaceState,
    diagram_id: &str,
    semantic_id: &str,
    item_type: &str,
    item_id: &str,
) -> Result<(), String> {
    let interaction_id = interaction_id(semantic_id)?;
    let mut repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    let interaction = repository
        .interactions
        .get_mut(&interaction_id)
        .ok_or("Interaction not found")?;
    match item_type {
        "Message" => {
            let wanted = parse_uuid(item_id).map(MessageId)?;
            let before = interaction.messages.len();
            interaction.messages.retain(|message| message.id != wanted);
            if interaction.messages.len() == before {
                return Err("Message not found".into());
            }
        }
        "Execution" => {
            let wanted = parse_uuid(item_id).map(systems_modeler_core::behavior::ExecutionId)?;
            let before = interaction.executions.len();
            interaction
                .executions
                .retain(|execution| execution.id != wanted);
            if interaction.executions.len() == before {
                return Err("Execution Specification not found".into());
            }
        }
        "Fragment" => {
            let wanted = parse_uuid(item_id).map(FragmentId)?;
            let before = interaction.fragments.len();
            interaction
                .fragments
                .retain(|fragment| fragment.id != wanted);
            if interaction.fragments.len() == before {
                return Err("Combined Fragment not found".into());
            }
        }
        "Invariant" => {
            let wanted = parse_uuid(item_id).map(InvariantId)?;
            let before = interaction.state_invariants.len();
            interaction
                .state_invariants
                .retain(|invariant| invariant.id != wanted);
            if interaction.state_invariants.len() == before {
                return Err("State Invariant not found".into());
            }
        }
        "Lifeline" => {
            let wanted = parse_uuid(item_id).map(LifelineId)?;
            let has_message = interaction.messages.iter().any(|message| {
                message
                    .send_event
                    .as_ref()
                    .is_some_and(|event| event.lifeline_id == wanted)
                    || message
                        .receive_event
                        .as_ref()
                        .is_some_and(|event| event.lifeline_id == wanted)
            });
            let has_execution = interaction
                .executions
                .iter()
                .any(|execution| execution.lifeline_id == wanted);
            let has_fragment = interaction
                .fragments
                .iter()
                .any(|fragment| fragment.covered_lifelines.contains(&wanted));
            let has_invariant = interaction
                .state_invariants
                .iter()
                .any(|invariant| invariant.lifeline_id == wanted);
            if has_message || has_execution || has_fragment || has_invariant {
                return Err("Lifeline is still referenced by Messages, Executions, Combined Fragments, or State Invariants. Delete those dependent interaction elements first.".into());
            }
            let before = interaction.lifelines.len();
            interaction
                .lifelines
                .retain(|lifeline| lifeline.id != wanted);
            if interaction.lifelines.len() == before {
                return Err("Lifeline not found".into());
            }
            drop(repository);
            let mut diagrams = state
                .behavior_diagrams
                .lock()
                .map_err(|_| "behavior diagram lock poisoned")?;
            let diagram = diagrams
                .iter_mut()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("behavior diagram not found")?;
            diagram
                .lifelines
                .retain(|presentation| presentation.lifeline_id != item_id);
            return Ok(());
        }
        _ => return Err(format!("unsupported Sequence deletion type: {item_type}")),
    }
    Ok(())
}

#[tauri::command]
pub fn delete_behavior_item(
    diagram_id: String,
    item_type: String,
    item_id: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let (kind, semantic_id) = diagram_semantics(&state, &diagram_id)?;
    match kind {
        BehaviorDiagramKind::StateMachine => {
            delete_state_item(&state, &diagram_id, &semantic_id, &item_type, &item_id)
        }
        BehaviorDiagramKind::Sequence => {
            delete_sequence_item(&state, &diagram_id, &semantic_id, &item_type, &item_id)
        }
    }
}
