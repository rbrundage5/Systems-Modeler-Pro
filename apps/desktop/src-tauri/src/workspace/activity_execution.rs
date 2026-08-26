use super::{ActivityWorkspaceState, WorkspaceState, activity_workspace::parse_activity_id};
use std::collections::HashMap;
use std::sync::Mutex;
use systems_modeler_core::{
    ActivityExecutionEngine, ActivityExecutionSnapshot, ActivityId, ActivityRepository,
    ExecutionConfiguration, ExecutionEngine, ExecutionManager, ExecutionSessionId, ExecutionState,
    Project,
};

#[derive(Default)]
struct ActivityExecutionRegistry {
    manager: ExecutionManager,
    engines: HashMap<ExecutionSessionId, ActivityExecutionEngine>,
    sessions_by_diagram: HashMap<String, ExecutionSessionId>,
    source_fingerprints: HashMap<String, String>,
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

fn start_execution(
    project: &Project,
    repository: ActivityRepository,
    activity_id: ActivityId,
    diagram_id: &str,
    fingerprint: String,
    registry: &mut ActivityExecutionRegistry,
) -> Result<ExecutionSessionId, String> {
    let activity = repository
        .activities
        .get(&activity_id)
        .ok_or("Activity was not found")?;
    let configuration = ExecutionConfiguration {
        root_semantic_id: activity.context_id.unwrap_or(activity.owner_id),
        random_seed: 0,
        max_steps: 100_000,
        max_queued_events: 10_000,
    };
    if let Some(previous) = registry.sessions_by_diagram.remove(diagram_id) {
        registry.engines.remove(&previous);
        registry.manager.remove_session(previous);
    }
    registry.source_fingerprints.remove(diagram_id);
    let session_id = registry
        .manager
        .create_session(project, configuration)
        .map_err(|error| error.to_string())?;
    let mut engine = ActivityExecutionEngine::new(repository, activity_id);
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
