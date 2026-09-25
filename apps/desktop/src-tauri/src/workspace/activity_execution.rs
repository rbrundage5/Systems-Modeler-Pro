use super::{ActivityWorkspaceState, WorkspaceState, activity_workspace::parse_activity_id};
use std::collections::HashMap;
use std::sync::Mutex;
use systems_modeler_core::{
    ActivityExecutionEngine, ActivityExecutionSnapshot, ActivityId, ActivityRepository,
    ElementKind, ExecutionConfiguration, ExecutionEngine, ExecutionManager,
    ExecutionRuntimePreview, ExecutionRuntimeSelection, ExecutionSessionId, ExecutionState,
    Project, RuntimeInstanceId, StructuralRuntime,
};

#[derive(Default)]
struct ActivityExecutionRegistry {
    manager: ExecutionManager,
    engines: HashMap<ExecutionSessionId, ActivityExecutionEngine>,
    sessions_by_diagram: HashMap<String, ExecutionSessionId>,
    source_fingerprints: HashMap<String, String>,
    runtime_selections: HashMap<String, ExecutionRuntimeSelection>,
}

#[derive(Default)]
pub struct ActivityExecutionState {
    registry: Mutex<ActivityExecutionRegistry>,
}

fn project_snapshot(workspace: &WorkspaceState) -> Result<Project, String> {
    workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or_else(|| "open a project before executing an Activity".into())
}

fn activity_for_diagram(
    activity_state: &ActivityWorkspaceState,
    diagram_id: &str,
) -> Result<(ActivityRepository, ActivityId), String> {
    let activity_id = activity_state
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or_else(|| format!("Activity diagram was not found: {diagram_id}"))
        .and_then(|diagram| parse_activity_id(&diagram.activity_id))?;
    let repository = activity_state
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?
        .clone();
    if !repository.activities.contains_key(&activity_id) {
        return Err("Activity diagram references a missing Activity".into());
    }
    Ok((repository, activity_id))
}

fn source_fingerprint(
    project: &Project,
    repository: &ActivityRepository,
) -> Result<String, String> {
    serde_json::to_string(&(project, repository))
        .map_err(|error| format!("failed to fingerprint Activity execution source model: {error}"))
}

fn session_id_for_diagram(
    registry: &ActivityExecutionRegistry,
    diagram_id: &str,
) -> Result<ExecutionSessionId, String> {
    registry
        .sessions_by_diagram
        .get(diagram_id)
        .copied()
        .ok_or_else(|| "initialize this Activity execution first".into())
}

fn snapshot_for(
    registry: &ActivityExecutionRegistry,
    session_id: ExecutionSessionId,
) -> Result<ActivityExecutionSnapshot, String> {
    let session = registry
        .manager
        .session(session_id)
        .map_err(|error| error.to_string())?;
    let engine = registry
        .engines
        .get(&session_id)
        .ok_or_else(|| "Activity execution engine is unavailable".to_string())?;
    Ok(engine.snapshot(session))
}

fn is_structural_root(kind: &ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Block
            | ElementKind::AssociationBlock
            | ElementKind::PartProperty
            | ElementKind::InstanceSpecification
    )
}

fn activity_runtime_context(
    project: &Project,
    repository: &ActivityRepository,
    activity_id: ActivityId,
    selection: &ExecutionRuntimeSelection,
    require_unambiguous_selection: bool,
) -> Result<(ExecutionRuntimePreview, Option<RuntimeInstanceId>), String> {
    let activity = repository
        .activities
        .get(&activity_id)
        .ok_or("Activity was not found")?;
    let default_root = activity.context_id.unwrap_or(activity.owner_id);
    let root_semantic_id = selection.root_semantic_id.unwrap_or(default_root);
    let root = project
        .element(root_semantic_id)
        .map_err(|error| format!("Execution runtime root is invalid: {error}"))?;
    if !is_structural_root(&root.kind) {
        if selection.root_semantic_id.is_some() {
            return Err(format!(
                "Execution runtime root '{}' ({:?}) is not a Block, PartProperty, AssociationBlock, or typed InstanceSpecification.",
                root.name, root.kind
            ));
        }
        return Ok((
            ExecutionRuntimePreview {
                root_semantic_id,
                structural_runtime: None,
                compatible_runtime_instance_paths: Vec::new(),
                selected_runtime_instance_path: None,
            },
            None,
        ));
    }

    let runtime = StructuralRuntime::build(
        project,
        root_semantic_id,
        &selection.structural_configuration,
    )
    .map_err(|error| error.to_string())?;
    let expected_classifier = activity.context_id.filter(|context_id| {
        project.element(*context_id).is_ok_and(|element| {
            matches!(
                &element.kind,
                ElementKind::Block | ElementKind::AssociationBlock
            )
        })
    });
    let compatible_runtime_instance_paths = expected_classifier
        .map(|classifier| runtime.compatible_instance_paths(project, classifier))
        .unwrap_or_default();
    let selected_runtime_instance_path = match selection
        .runtime_instance_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    {
        Some(path) => {
            if !compatible_runtime_instance_paths
                .iter()
                .any(|candidate| candidate == path)
            {
                return Err(format!(
                    "Runtime occurrence '{path}' is not compatible with Activity '{}'. Compatible occurrence path(s): {}",
                    activity.name,
                    if compatible_runtime_instance_paths.is_empty() {
                        "none".to_string()
                    } else {
                        compatible_runtime_instance_paths.join(", ")
                    }
                ));
            }
            Some(path.to_string())
        }
        None if compatible_runtime_instance_paths.len() == 1 => {
            compatible_runtime_instance_paths.first().cloned()
        }
        None if require_unambiguous_selection && compatible_runtime_instance_paths.len() > 1 => {
            return Err(format!(
                "Activity '{}' has {} compatible runtime occurrences under '{}'. Choose one runtime occurrence before initialization: {}",
                activity.name,
                compatible_runtime_instance_paths.len(),
                root.name,
                compatible_runtime_instance_paths.join(", ")
            ));
        }
        None => None,
    };
    if require_unambiguous_selection
        && expected_classifier.is_some()
        && selected_runtime_instance_path.is_none()
    {
        return Err(format!(
            "No runtime occurrence compatible with Activity '{}' exists under '{}'. Select a compatible structural root or correct the model typing.",
            activity.name, root.name
        ));
    }
    let selected_runtime_instance_id =
        selected_runtime_instance_path
            .as_deref()
            .and_then(|runtime_path| {
                runtime
                    .instance_by_path(runtime_path)
                    .map(|instance| instance.id)
            });
    Ok((
        ExecutionRuntimePreview {
            root_semantic_id,
            structural_runtime: Some(runtime.snapshot()),
            compatible_runtime_instance_paths,
            selected_runtime_instance_path,
        },
        selected_runtime_instance_id,
    ))
}

fn invalidate_activity_execution(registry: &mut ActivityExecutionRegistry, diagram_id: &str) {
    if let Some(previous) = registry.sessions_by_diagram.remove(diagram_id) {
        registry.engines.remove(&previous);
        registry.manager.remove_session(previous);
    }
    registry.source_fingerprints.remove(diagram_id);
}

fn start_execution(
    project: &Project,
    repository: ActivityRepository,
    activity_id: ActivityId,
    diagram_id: &str,
    fingerprint: String,
    registry: &mut ActivityExecutionRegistry,
) -> Result<ExecutionSessionId, String> {
    let selection = registry
        .runtime_selections
        .get(diagram_id)
        .cloned()
        .unwrap_or_default();
    let (preview, runtime_instance_id) =
        activity_runtime_context(project, &repository, activity_id, &selection, true)?;
    let configuration = ExecutionConfiguration {
        root_semantic_id: preview.root_semantic_id,
        random_seed: 0,
        max_steps: 100_000,
        max_queued_events: 10_000,
    };
    invalidate_activity_execution(registry, diagram_id);
    let session_id = registry
        .manager
        .create_session(project, configuration)
        .map_err(|error| error.to_string())?;
    registry
        .manager
        .session_mut(session_id)
        .map_err(|error| error.to_string())?
        .set_structural_configuration(selection.structural_configuration.clone())
        .map_err(|error| error.to_string())?;
    let mut engine = ActivityExecutionEngine::new(repository, activity_id);
    if let Some(runtime_instance_id) = runtime_instance_id {
        engine = engine.with_runtime_instance(runtime_instance_id);
    }
    engine
        .initialize(
            project,
            registry
                .manager
                .session_mut(session_id)
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
    registry.engines.insert(session_id, engine);
    registry
        .sessions_by_diagram
        .insert(diagram_id.to_string(), session_id);
    registry
        .source_fingerprints
        .insert(diagram_id.to_string(), fingerprint);
    Ok(session_id)
}

fn ensure_current_execution(
    project: &Project,
    repository: ActivityRepository,
    activity_id: ActivityId,
    diagram_id: &str,
    fingerprint: String,
    registry: &mut ActivityExecutionRegistry,
) -> Result<(ExecutionSessionId, bool), String> {
    let session_id = session_id_for_diagram(registry, diagram_id)?;
    if registry.source_fingerprints.get(diagram_id) == Some(&fingerprint) {
        return Ok((session_id, false));
    }
    let refreshed = start_execution(
        project,
        repository,
        activity_id,
        diagram_id,
        fingerprint,
        registry,
    )?;
    Ok((refreshed, true))
}

#[tauri::command]
pub fn activity_execution_runtime_selection(
    diagram_id: String,
    execution_state: tauri::State<'_, ActivityExecutionState>,
) -> Result<ExecutionRuntimeSelection, String> {
    let registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Activity execution lock poisoned")?;
    Ok(registry
        .runtime_selections
        .get(&diagram_id)
        .cloned()
        .unwrap_or_default())
}

#[tauri::command]
pub fn preview_activity_execution_runtime(
    diagram_id: String,
    selection: ExecutionRuntimeSelection,
    workspace: tauri::State<'_, WorkspaceState>,
    activity_state: tauri::State<'_, ActivityWorkspaceState>,
) -> Result<ExecutionRuntimePreview, String> {
    let project = project_snapshot(&workspace)?;
    let (repository, activity_id) = activity_for_diagram(&activity_state, &diagram_id)?;
    activity_runtime_context(&project, &repository, activity_id, &selection, false)
        .map(|(preview, _)| preview)
}

#[tauri::command]
pub fn configure_activity_execution_runtime(
    diagram_id: String,
    selection: ExecutionRuntimeSelection,
    workspace: tauri::State<'_, WorkspaceState>,
    activity_state: tauri::State<'_, ActivityWorkspaceState>,
    execution_state: tauri::State<'_, ActivityExecutionState>,
) -> Result<ExecutionRuntimePreview, String> {
    let project = project_snapshot(&workspace)?;
    let (repository, activity_id) = activity_for_diagram(&activity_state, &diagram_id)?;
    let (preview, _) =
        activity_runtime_context(&project, &repository, activity_id, &selection, true)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Activity execution lock poisoned")?;
    invalidate_activity_execution(&mut registry, &diagram_id);
    registry.runtime_selections.insert(diagram_id, selection);
    Ok(preview)
}

#[tauri::command]
pub fn initialize_activity_execution(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity_state: tauri::State<'_, ActivityWorkspaceState>,
    execution_state: tauri::State<'_, ActivityExecutionState>,
) -> Result<ActivityExecutionSnapshot, String> {
    let project = project_snapshot(&workspace)?;
    let (repository, activity_id) = activity_for_diagram(&activity_state, &diagram_id)?;
    let fingerprint = source_fingerprint(&project, &repository)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Activity execution lock poisoned")?;
    let session_id = start_execution(
        &project,
        repository,
        activity_id,
        &diagram_id,
        fingerprint,
        &mut registry,
    )?;
    snapshot_for(&registry, session_id)
}

#[tauri::command]
pub fn activity_execution_snapshot(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity_state: tauri::State<'_, ActivityWorkspaceState>,
    execution_state: tauri::State<'_, ActivityExecutionState>,
) -> Result<Option<ActivityExecutionSnapshot>, String> {
    let project = project_snapshot(&workspace)?;
    let (repository, _) = activity_for_diagram(&activity_state, &diagram_id)?;
    let fingerprint = source_fingerprint(&project, &repository)?;
    let registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Activity execution lock poisoned")?;
    let Some(session_id) = registry.sessions_by_diagram.get(&diagram_id).copied() else {
        return Ok(None);
    };
    if registry.source_fingerprints.get(&diagram_id) != Some(&fingerprint) {
        return Ok(None);
    }
    snapshot_for(&registry, session_id).map(Some)
}

#[tauri::command]
pub fn run_activity_execution(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity_state: tauri::State<'_, ActivityWorkspaceState>,
    execution_state: tauri::State<'_, ActivityExecutionState>,
) -> Result<ActivityExecutionSnapshot, String> {
    let project = project_snapshot(&workspace)?;
    let (repository, activity_id) = activity_for_diagram(&activity_state, &diagram_id)?;
    let fingerprint = source_fingerprint(&project, &repository)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Activity execution lock poisoned")?;
    let (session_id, _) = ensure_current_execution(
        &project,
        repository,
        activity_id,
        &diagram_id,
        fingerprint,
        &mut registry,
    )?;
    registry
        .manager
        .session_mut(session_id)
        .map_err(|error| error.to_string())?
        .run()
        .map_err(|error| error.to_string())?;
    snapshot_for(&registry, session_id)
}

#[tauri::command]
pub fn step_activity_execution(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity_state: tauri::State<'_, ActivityWorkspaceState>,
    execution_state: tauri::State<'_, ActivityExecutionState>,
) -> Result<ActivityExecutionSnapshot, String> {
    let project = project_snapshot(&workspace)?;
    let (repository, activity_id) = activity_for_diagram(&activity_state, &diagram_id)?;
    let fingerprint = source_fingerprint(&project, &repository)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Activity execution lock poisoned")?;
    let (session_id, _) = ensure_current_execution(
        &project,
        repository,
        activity_id,
        &diagram_id,
        fingerprint,
        &mut registry,
    )?;
    let ActivityExecutionRegistry {
        manager, engines, ..
    } = &mut *registry;
    let session = manager
        .session_mut(session_id)
        .map_err(|error| error.to_string())?;
    let engine = engines
        .get_mut(&session_id)
        .ok_or("Activity execution engine is unavailable")?;
    engine
        .advance(&project, session)
        .map_err(|error| error.to_string())?;
    Ok(engine.snapshot(session))
}

#[tauri::command]
pub fn pause_activity_execution(
    diagram_id: String,
    execution_state: tauri::State<'_, ActivityExecutionState>,
) -> Result<ActivityExecutionSnapshot, String> {
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Activity execution lock poisoned")?;
    let session_id = session_id_for_diagram(&registry, &diagram_id)?;
    registry
        .manager
        .session_mut(session_id)
        .map_err(|error| error.to_string())?
        .pause()
        .map_err(|error| error.to_string())?;
    snapshot_for(&registry, session_id)
}

#[tauri::command]
pub fn resume_activity_execution(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity_state: tauri::State<'_, ActivityWorkspaceState>,
    execution_state: tauri::State<'_, ActivityExecutionState>,
) -> Result<ActivityExecutionSnapshot, String> {
    let project = project_snapshot(&workspace)?;
    let (repository, activity_id) = activity_for_diagram(&activity_state, &diagram_id)?;
    let fingerprint = source_fingerprint(&project, &repository)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Activity execution lock poisoned")?;
    let (session_id, refreshed) = ensure_current_execution(
        &project,
        repository,
        activity_id,
        &diagram_id,
        fingerprint,
        &mut registry,
    )?;
    let session = registry
        .manager
        .session_mut(session_id)
        .map_err(|error| error.to_string())?;
    if refreshed {
        session.run().map_err(|error| error.to_string())?;
    } else {
        session.resume().map_err(|error| error.to_string())?;
    }
    snapshot_for(&registry, session_id)
}

#[tauri::command]
pub fn reset_activity_execution(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity_state: tauri::State<'_, ActivityWorkspaceState>,
    execution_state: tauri::State<'_, ActivityExecutionState>,
) -> Result<ActivityExecutionSnapshot, String> {
    let project = project_snapshot(&workspace)?;
    let (repository, activity_id) = activity_for_diagram(&activity_state, &diagram_id)?;
    let fingerprint = source_fingerprint(&project, &repository)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Activity execution lock poisoned")?;
    let (session_id, refreshed) = ensure_current_execution(
        &project,
        repository,
        activity_id,
        &diagram_id,
        fingerprint,
        &mut registry,
    )?;
    if refreshed {
        return snapshot_for(&registry, session_id);
    }
    let ActivityExecutionRegistry {
        manager, engines, ..
    } = &mut *registry;
    let session = manager
        .session_mut(session_id)
        .map_err(|error| error.to_string())?;
    let engine = engines
        .get_mut(&session_id)
        .ok_or("Activity execution engine is unavailable")?;
    engine
        .reset(&project, session)
        .map_err(|error| error.to_string())?;
    Ok(engine.snapshot(session))
}

#[tauri::command]
pub fn terminate_activity_execution(
    diagram_id: String,
    execution_state: tauri::State<'_, ActivityExecutionState>,
) -> Result<ActivityExecutionSnapshot, String> {
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Activity execution lock poisoned")?;
    let session_id = session_id_for_diagram(&registry, &diagram_id)?;
    let session = registry
        .manager
        .session_mut(session_id)
        .map_err(|error| error.to_string())?;
    if session.state != ExecutionState::Terminated {
        session.terminate().map_err(|error| error.to_string())?;
    }
    snapshot_for(&registry, session_id)
}

#[tauri::command]
pub fn clear_activity_executions(
    execution_state: tauri::State<'_, ActivityExecutionState>,
) -> Result<(), String> {
    *execution_state
        .registry
        .lock()
        .map_err(|_| "Activity execution lock poisoned")? = ActivityExecutionRegistry::default();
    Ok(())
}
