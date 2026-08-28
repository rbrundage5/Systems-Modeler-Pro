use super::{WorkspaceState, behavior_workspace::BehaviorDiagramKind};
use std::collections::HashMap;
use std::sync::Mutex;
use systems_modeler_core::{
    BehaviorRepository, ElementKind, ExecutionConfiguration, ExecutionEngine, ExecutionManager,
    ExecutionRuntimePreview, ExecutionRuntimeSelection, ExecutionSessionId, InteractionId,
    Project, SequenceExecutionEngine, SequenceExecutionSnapshot, StructuralRuntime,
};

#[derive(Default)]
struct SequenceExecutionRegistry {
    manager: ExecutionManager,
    engines: HashMap<ExecutionSessionId, SequenceExecutionEngine>,
    sessions_by_diagram: HashMap<String, ExecutionSessionId>,
    source_fingerprints: HashMap<String, String>,
    runtime_selections: HashMap<String, ExecutionRuntimeSelection>,
}

#[derive(Default)]
pub struct SequenceExecutionState {
    registry: Mutex<SequenceExecutionRegistry>,
}

fn execution_source(
    workspace: &WorkspaceState,
    diagram_id: &str,
) -> Result<(Project, BehaviorRepository, InteractionId, String), String> {
    let project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or_else(|| "open a project before executing a Sequence".to_string())?;
    let semantic_id = workspace
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?
        .iter()
        .find(|diagram| diagram.id == diagram_id && diagram.kind == BehaviorDiagramKind::Sequence)
        .ok_or_else(|| format!("Sequence diagram was not found: {diagram_id}"))?
        .semantic_id
        .clone();
    let interaction_id = uuid::Uuid::parse_str(&semantic_id)
        .map(InteractionId)
        .map_err(|_| format!("invalid Interaction id: {semantic_id}"))?;
    let repository = workspace
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?
        .clone();
    if !repository.interactions.contains_key(&interaction_id) {
        return Err("Sequence diagram references missing Interaction semantics".into());
    }
    repository.validate(&project).map_err(|error| error.to_string())?;
    let fingerprint = serde_json::to_string(&(&project, &repository))
        .map_err(|error| format!("failed to fingerprint Sequence execution source: {error}"))?;
    Ok((project, repository, interaction_id, fingerprint))
}

fn runtime_preview(
    project: &Project,
    repository: &BehaviorRepository,
    interaction_id: InteractionId,
    selection: &ExecutionRuntimeSelection,
) -> Result<ExecutionRuntimePreview, String> {
    let interaction = repository
        .interactions
        .get(&interaction_id)
        .ok_or("Sequence Interaction was not found")?;
    let root_semantic_id = selection.root_semantic_id.unwrap_or(interaction.context_id);
    let root = project
        .element(root_semantic_id)
        .map_err(|error| format!("Sequence runtime root is invalid: {error}"))?;
    if !matches!(
        root.kind,
        ElementKind::Block
            | ElementKind::AssociationBlock
            | ElementKind::PartProperty
            | ElementKind::InstanceSpecification
    ) {
        return Err(format!(
            "Sequence runtime root '{}' ({:?}) is not a structural execution root.",
            root.name, root.kind
        ));
    }
    let runtime = StructuralRuntime::build(
        project,
        root_semantic_id,
        &selection.structural_configuration,
    )
    .map_err(|error| error.to_string())?;
    Ok(ExecutionRuntimePreview {
        root_semantic_id,
        structural_runtime: Some(runtime.snapshot()),
        compatible_runtime_instance_paths: Vec::new(),
        selected_runtime_instance_path: None,
    })
}

fn invalidate(registry: &mut SequenceExecutionRegistry, diagram_id: &str) {
    if let Some(session_id) = registry.sessions_by_diagram.remove(diagram_id) {
        registry.engines.remove(&session_id);
        registry.manager.remove_session(session_id);
    }
    registry.source_fingerprints.remove(diagram_id);
}

fn start_execution(
    project: &Project,
    repository: BehaviorRepository,
    interaction_id: InteractionId,
    diagram_id: &str,
    fingerprint: String,
    registry: &mut SequenceExecutionRegistry,
) -> Result<ExecutionSessionId, String> {
    let selection = registry
        .runtime_selections
        .get(diagram_id)
        .cloned()
        .unwrap_or_default();
    let preview = runtime_preview(project, &repository, interaction_id, &selection)?;
    invalidate(registry, diagram_id);
    let session_id = registry
        .manager
        .create_session(
            project,
            ExecutionConfiguration {
                root_semantic_id: preview.root_semantic_id,
                random_seed: 0,
                max_steps: 100_000,
                max_queued_events: 10_000,
            },
        )
        .map_err(|error| error.to_string())?;
    registry
        .manager
        .session_mut(session_id)
        .map_err(|error| error.to_string())?
        .set_structural_configuration(selection.structural_configuration)
        .map_err(|error| error.to_string())?;
    let mut engine = SequenceExecutionEngine::new(repository, interaction_id);
    if let Err(error) = engine.initialize(
        project,
        registry
            .manager
            .session_mut(session_id)
            .map_err(|error| error.to_string())?,
    ) {
        registry.manager.remove_session(session_id);
        return Err(error.to_string());
    }
    registry.engines.insert(session_id, engine);
    registry
        .sessions_by_diagram
        .insert(diagram_id.to_string(), session_id);
    registry
        .source_fingerprints
        .insert(diagram_id.to_string(), fingerprint);
    Ok(session_id)
}

fn snapshot_for(
    registry: &SequenceExecutionRegistry,
    session_id: ExecutionSessionId,
) -> Result<SequenceExecutionSnapshot, String> {
    let session = registry
        .manager
        .session(session_id)
        .map_err(|error| error.to_string())?;
    let engine = registry
        .engines
        .get(&session_id)
        .ok_or("Sequence execution engine is unavailable")?;
    Ok(engine.snapshot(session))
}

#[tauri::command]
pub fn sequence_execution_runtime_selection(
    diagram_id: String,
    execution_state: tauri::State<'_, SequenceExecutionState>,
) -> Result<ExecutionRuntimeSelection, String> {
    let registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Sequence execution lock poisoned")?;
    Ok(registry
        .runtime_selections
        .get(&diagram_id)
        .cloned()
        .unwrap_or_default())
}

#[tauri::command]
pub fn preview_sequence_execution_runtime(
    diagram_id: String,
    selection: ExecutionRuntimeSelection,
    workspace: tauri::State<'_, WorkspaceState>,
) -> Result<ExecutionRuntimePreview, String> {
    let (project, repository, interaction_id, _) = execution_source(&workspace, &diagram_id)?;
    runtime_preview(&project, &repository, interaction_id, &selection)
}

#[tauri::command]
pub fn configure_sequence_execution_runtime(
    diagram_id: String,
    selection: ExecutionRuntimeSelection,
    workspace: tauri::State<'_, WorkspaceState>,
    execution_state: tauri::State<'_, SequenceExecutionState>,
) -> Result<ExecutionRuntimePreview, String> {
    let (project, repository, interaction_id, _) = execution_source(&workspace, &diagram_id)?;
    let preview = runtime_preview(&project, &repository, interaction_id, &selection)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Sequence execution lock poisoned")?;
    invalidate(&mut registry, &diagram_id);
    registry.runtime_selections.insert(diagram_id, selection);
    Ok(preview)
}

#[tauri::command]
pub fn initialize_sequence_execution(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    execution_state: tauri::State<'_, SequenceExecutionState>,
) -> Result<SequenceExecutionSnapshot, String> {
    let (project, repository, interaction_id, fingerprint) =
        execution_source(&workspace, &diagram_id)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Sequence execution lock poisoned")?;
    let session_id = start_execution(
        &project,
        repository,
        interaction_id,
        &diagram_id,
        fingerprint,
        &mut registry,
    )?;
    snapshot_for(&registry, session_id)
}

#[tauri::command]
pub fn sequence_execution_snapshot(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    execution_state: tauri::State<'_, SequenceExecutionState>,
) -> Result<Option<SequenceExecutionSnapshot>, String> {
    let (_, _, _, fingerprint) = execution_source(&workspace, &diagram_id)?;
    let registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Sequence execution lock poisoned")?;
    let Some(session_id) = registry.sessions_by_diagram.get(&diagram_id).copied() else {
        return Ok(None);
    };
    if registry.source_fingerprints.get(&diagram_id) != Some(&fingerprint) {
        return Ok(None);
    }
    snapshot_for(&registry, session_id).map(Some)
}

fn current_session(
    registry: &SequenceExecutionRegistry,
    diagram_id: &str,
) -> Result<ExecutionSessionId, String> {
    registry
        .sessions_by_diagram
        .get(diagram_id)
        .copied()
        .ok_or_else(|| "initialize this Sequence execution first".into())
}

#[tauri::command]
pub fn run_sequence_execution(
    diagram_id: String,
    execution_state: tauri::State<'_, SequenceExecutionState>,
) -> Result<SequenceExecutionSnapshot, String> {
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Sequence execution lock poisoned")?;
    let session_id = current_session(&registry, &diagram_id)?;
    registry
        .manager
        .session_mut(session_id)
        .map_err(|error| error.to_string())?
        .run()
        .map_err(|error| error.to_string())?;
    snapshot_for(&registry, session_id)
}

#[tauri::command]
pub fn step_sequence_execution(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    execution_state: tauri::State<'_, SequenceExecutionState>,
) -> Result<SequenceExecutionSnapshot, String> {
    let (project, _, _, fingerprint) = execution_source(&workspace, &diagram_id)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Sequence execution lock poisoned")?;
    let session_id = current_session(&registry, &diagram_id)?;
    if registry.source_fingerprints.get(&diagram_id) != Some(&fingerprint) {
        return Err("Sequence model changed; initialize execution again".into());
    }
    let SequenceExecutionRegistry {
        manager, engines, ..
    } = &mut *registry;
    let session = manager
        .session_mut(session_id)
        .map_err(|error| error.to_string())?;
    let engine = engines
        .get_mut(&session_id)
        .ok_or("Sequence execution engine is unavailable")?;
    engine
        .step(&project, session)
        .map_err(|error| error.to_string())?;
    Ok(engine.snapshot(session))
}

#[tauri::command]
pub fn pause_sequence_execution(
    diagram_id: String,
    execution_state: tauri::State<'_, SequenceExecutionState>,
) -> Result<SequenceExecutionSnapshot, String> {
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Sequence execution lock poisoned")?;
    let session_id = current_session(&registry, &diagram_id)?;
    registry
        .manager
        .session_mut(session_id)
        .map_err(|error| error.to_string())?
        .pause()
        .map_err(|error| error.to_string())?;
    snapshot_for(&registry, session_id)
}

#[tauri::command]
pub fn resume_sequence_execution(
    diagram_id: String,
    execution_state: tauri::State<'_, SequenceExecutionState>,
) -> Result<SequenceExecutionSnapshot, String> {
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Sequence execution lock poisoned")?;
    let session_id = current_session(&registry, &diagram_id)?;
    registry
        .manager
        .session_mut(session_id)
        .map_err(|error| error.to_string())?
        .resume()
        .map_err(|error| error.to_string())?;
    snapshot_for(&registry, session_id)
}

#[tauri::command]
pub fn reset_sequence_execution(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    execution_state: tauri::State<'_, SequenceExecutionState>,
) -> Result<SequenceExecutionSnapshot, String> {
    let (project, _, _, fingerprint) = execution_source(&workspace, &diagram_id)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Sequence execution lock poisoned")?;
    let session_id = current_session(&registry, &diagram_id)?;
    if registry.source_fingerprints.get(&diagram_id) != Some(&fingerprint) {
        return Err("Sequence model changed; initialize execution again".into());
    }
    let SequenceExecutionRegistry {
        manager, engines, ..
    } = &mut *registry;
    let session = manager
        .session_mut(session_id)
        .map_err(|error| error.to_string())?;
    let engine = engines
        .get_mut(&session_id)
        .ok_or("Sequence execution engine is unavailable")?;
    engine
        .reset(&project, session)
        .map_err(|error| error.to_string())?;
    Ok(engine.snapshot(session))
}

#[tauri::command]
pub fn terminate_sequence_execution(
    diagram_id: String,
    execution_state: tauri::State<'_, SequenceExecutionState>,
) -> Result<SequenceExecutionSnapshot, String> {
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Sequence execution lock poisoned")?;
    let session_id = current_session(&registry, &diagram_id)?;
    registry
        .manager
        .session_mut(session_id)
        .map_err(|error| error.to_string())?
        .terminate()
        .map_err(|error| error.to_string())?;
    snapshot_for(&registry, session_id)
}

#[tauri::command]
pub fn clear_sequence_executions(
    execution_state: tauri::State<'_, SequenceExecutionState>,
) -> Result<(), String> {
    *execution_state
        .registry
        .lock()
        .map_err(|_| "Sequence execution lock poisoned")? = SequenceExecutionRegistry::default();
    Ok(())
}
