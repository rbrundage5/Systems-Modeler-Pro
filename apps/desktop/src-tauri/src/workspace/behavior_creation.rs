use super::behavior_workspace::{BehaviorDiagram, BehaviorDiagramKind};
use super::{parse_element_id, WorkspaceState};
use systems_modeler_core::{ElementId, ElementKind, Project};

fn behavior_context_snapshot(
    state: &WorkspaceState,
    context_id: ElementId,
) -> Result<(Project, ElementId), String> {
    let project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_ref().ok_or("no project open")?;
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
    let owner_id = context.owner_id.unwrap_or(project.root_id);
    Ok((project.clone(), owner_id))
}

fn append_behavior_diagram(
    state: &WorkspaceState,
    name: String,
    owner_id: ElementId,
    context_id: ElementId,
    kind: BehaviorDiagramKind,
    semantic_id: String,
) -> Result<String, String> {
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
            kind,
            semantic_id,
            state_nodes: Vec::new(),
            lifelines: Vec::new(),
        });
    Ok(id)
}

/// Creates a State Machine without holding more than one workspace mutex at a time.
/// The project snapshot is immutable input to the Rust behavior service; semantic creation
/// completes before presentation creation begins.
#[tauri::command]
pub fn create_state_machine_diagram_staged(
    context_id: String,
    name: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let context_id = parse_element_id(&context_id)?;
    let (project, owner_id) = behavior_context_snapshot(&state, context_id)?;

    let semantic_id = {
        let mut repository = state
            .behavior
            .lock()
            .map_err(|_| "behavior lock poisoned")?;
        repository
            .create_state_machine(&project, context_id, name.clone())
            .map_err(|error| error.to_string())?
    };

    append_behavior_diagram(
        &state,
        name,
        owner_id,
        context_id,
        BehaviorDiagramKind::StateMachine,
        semantic_id.to_string(),
    )
}

/// Creates an Interaction/Sequence diagram using the same single-lock staging discipline.
#[tauri::command]
pub fn create_sequence_diagram_staged(
    context_id: String,
    name: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let context_id = parse_element_id(&context_id)?;
    let (project, owner_id) = behavior_context_snapshot(&state, context_id)?;

    let semantic_id = {
        let mut repository = state
            .behavior
            .lock()
            .map_err(|_| "behavior lock poisoned")?;
        repository
            .create_interaction(&project, context_id, name.clone())
            .map_err(|error| error.to_string())?
    };

    append_behavior_diagram(
        &state,
        name,
        owner_id,
        context_id,
        BehaviorDiagramKind::Sequence,
        semantic_id.to_string(),
    )
}
