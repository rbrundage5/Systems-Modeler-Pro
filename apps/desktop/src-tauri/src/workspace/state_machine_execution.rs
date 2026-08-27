use super::{WorkspaceState, behavior_workspace::BehaviorDiagramKind};
use std::collections::HashMap;
use std::sync::Mutex;
use systems_modeler_core::{
    BehaviorRepository, ElementId, ExecutionConfiguration, ExecutionEngine, ExecutionManager,
    ExecutionSessionId, ExecutionState, Project, RuntimeValue, StateMachineExecutionEngine,
    StateMachineExecutionSnapshot, StateMachineId,
};

#[derive(Default)]
struct StateMachineExecutionRegistry {
    manager: ExecutionManager,
    engines: HashMap<ExecutionSessionId, StateMachineExecutionEngine>,
    sessions_by_diagram: HashMap<String, ExecutionSessionId>,
    source_fingerprints: HashMap<String, String>,
}

#[derive(Default)]
pub struct StateMachineExecutionState {
    registry: Mutex<StateMachineExecutionRegistry>,
}

fn project_snapshot(workspace: &WorkspaceState) -> Result<Project, String> {
    workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or_else(|| "open a project before executing a State Machine".into())
}

fn machine_for_diagram(
    workspace: &WorkspaceState,
    diagram_id: &str,
) -> Result<(BehaviorRepository, StateMachineId), String> {
    let semantic_id = workspace
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?
        .iter()
        .find(|diagram| diagram.id == diagram_id && diagram.kind == BehaviorDiagramKind::StateMachine)
        .ok_or_else(|| format!("State Machine diagram was not found: {diagram_id}"))?
        .semantic_id
        .clone();
    let machine_id = uuid::Uuid::parse_str(&semantic_id)
        .map(StateMachineId)
        .map_err(|_| format!("invalid State Machine id: {semantic_id}"))?;
    let repository = workspace
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?
        .clone();
    if !repository.state_machines.contains_key(&machine_id) {
        return Err("State Machine diagram references missing semantics".into());
    }
    Ok((repository, machine_id))
}

fn source_fingerprint(project: &Project, repository: &BehaviorRepository) -> Result<String, String> {
    serde_json::to_string(&(project, repository)).map_err(|error| {
        format!("failed to fingerprint State Machine execution source model: {error}")
    })
}

fn session_id_for_diagram(
    registry: &StateMachineExecutionRegistry,
    diagram_id: &str,
) -> Result<ExecutionSessionId, String> {
    registry
        .sessions_by_diagram
        .get(diagram_id)
        .copied()
        .ok_or_else(|| "initialize this State Machine execution first".into())
}

fn snapshot_for(
    registry: &StateMachineExecutionRegistry,
    session_id: ExecutionSessionId,
) -> Result<StateMachineExecutionSnapshot, String> {
    let session = registry
        .manager
        .session(session_id)
        .map_err(|error| error.to_string())?;
    let engine = registry
        .engines
        .get(&session_id)
        .ok_or("State Machine execution engine is unavailable")?;
    Ok(engine.snapshot(session))
}

fn start_execution(
    project: &Project,
    repository: BehaviorRepository,
    machine_id: StateMachineId,
    diagram_id: &str,
    fingerprint: String,
    registry: &mut StateMachineExecutionRegistry,
) -> Result<ExecutionSessionId, String> {
    let machine = repository
        .state_machines
        .get(&machine_id)
        .ok_or("State Machine was not found")?;
    let configuration = ExecutionConfiguration {
        root_semantic_id: machine.context_id,
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
    let mut engine = StateMachineExecutionEngine::new(repository, machine_id);
    let initialized = engine.initialize(
        project,
        registry
            .manager
            .session_mut(session_id)
            .map_err(|error| error.to_string())?,
    );
    if let Err(error) = initialized {
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

fn ensure_current_execution(
    project: &Project,
    repository: BehaviorRepository,
    machine_id: StateMachineId,
    diagram_id: &str,
    fingerprint: String,
    registry: &mut StateMachineExecutionRegistry,
) -> Result<(ExecutionSessionId, bool), String> {
    let session_id = session_id_for_diagram(registry, diagram_id)?;
    if registry.source_fingerprints.get(diagram_id) == Some(&fingerprint) {
        return Ok((session_id, false));
    }
    let refreshed = start_execution(
        project,
        repository,
        machine_id,
        diagram_id,
        fingerprint,
        registry,
    )?;
    Ok((refreshed, true))
}

#[tauri::command]
pub fn initialize_state_machine_execution(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    execution_state: tauri::State<'_, StateMachineExecutionState>,
) -> Result<StateMachineExecutionSnapshot, String> {
    let project = project_snapshot(&workspace)?;
    let (repository, machine_id) = machine_for_diagram(&workspace, &diagram_id)?;
    let fingerprint = source_fingerprint(&project, &repository)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "State Machine execution lock poisoned")?;
    let session_id = start_execution(
        &project,
        repository,
        machine_id,
        &diagram_id,
        fingerprint,
        &mut registry,
    )?;
    snapshot_for(&registry, session_id)
}

#[tauri::command]
pub fn state_machine_execution_snapshot(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    execution_state: tauri::State<'_, StateMachineExecutionState>,
) -> Result<Option<StateMachineExecutionSnapshot>, String> {
    let project = project_snapshot(&workspace)?;
    let (repository, _) = machine_for_diagram(&workspace, &diagram_id)?;
    let fingerprint = source_fingerprint(&project, &repository)?;
    let registry = execution_state
        .registry
        .lock()
        .map_err(|_| "State Machine execution lock poisoned")?;
    let Some(session_id) = registry.sessions_by_diagram.get(&diagram_id).copied() else {
        return Ok(None);
    };
    if registry.source_fingerprints.get(&diagram_id) != Some(&fingerprint) {
        return Ok(None);
    }
    snapshot_for(&registry, session_id).map(Some)
}

#[tauri::command]
pub fn run_state_machine_execution(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    execution_state: tauri::State<'_, StateMachineExecutionState>,
) -> Result<StateMachineExecutionSnapshot, String> {
    let project = project_snapshot(&workspace)?;
    let (repository, machine_id) = machine_for_diagram(&workspace, &diagram_id)?;
    let fingerprint = source_fingerprint(&project, &repository)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "State Machine execution lock poisoned")?;
    let (session_id, _) = ensure_current_execution(
        &project,
        repository,
        machine_id,
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
pub fn step_state_machine_execution(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    execution_state: tauri::State<'_, StateMachineExecutionState>,
) -> Result<StateMachineExecutionSnapshot, String> {
    let project = project_snapshot(&workspace)?;
    let (repository, machine_id) = machine_for_diagram(&workspace, &diagram_id)?;
    let fingerprint = source_fingerprint(&project, &repository)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "State Machine execution lock poisoned")?;
    let (session_id, _) = ensure_current_execution(
        &project,
        repository,
        machine_id,
        &diagram_id,
        fingerprint,
        &mut registry,
    )?;
    let StateMachineExecutionRegistry {
        manager, engines, ..
    } = &mut *registry;
    let session = manager
        .session_mut(session_id)
        .map_err(|error| error.to_string())?;
    let engine = engines
        .get_mut(&session_id)
        .ok_or("State Machine execution engine is unavailable")?;
    if let Err(error) = engine.advance(&project, session) {
        if session.state != ExecutionState::Failed {
            session.fail(Some(project.root_id), error.to_string());
        }
        return Err(error.to_string());
    }
    Ok(engine.snapshot(session))
}

#[tauri::command]
pub fn pause_state_machine_execution(
    diagram_id: String,
    execution_state: tauri::State<'_, StateMachineExecutionState>,
) -> Result<StateMachineExecutionSnapshot, String> {
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "State Machine execution lock poisoned")?;
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
pub fn resume_state_machine_execution(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    execution_state: tauri::State<'_, StateMachineExecutionState>,
) -> Result<StateMachineExecutionSnapshot, String> {
    let project = project_snapshot(&workspace)?;
    let (repository, machine_id) = machine_for_diagram(&workspace, &diagram_id)?;
    let fingerprint = source_fingerprint(&project, &repository)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "State Machine execution lock poisoned")?;
    let (session_id, refreshed) = ensure_current_execution(
        &project,
        repository,
        machine_id,
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
pub fn reset_state_machine_execution(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    execution_state: tauri::State<'_, StateMachineExecutionState>,
) -> Result<StateMachineExecutionSnapshot, String> {
    let project = project_snapshot(&workspace)?;
    let (repository, machine_id) = machine_for_diagram(&workspace, &diagram_id)?;
    let fingerprint = source_fingerprint(&project, &repository)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "State Machine execution lock poisoned")?;
    let (session_id, refreshed) = ensure_current_execution(
        &project,
        repository,
        machine_id,
        &diagram_id,
        fingerprint,
        &mut registry,
    )?;
    if refreshed {
        return snapshot_for(&registry, session_id);
    }
    let StateMachineExecutionRegistry {
        manager, engines, ..
    } = &mut *registry;
    let session = manager
        .session_mut(session_id)
        .map_err(|error| error.to_string())?;
    let engine = engines
        .get_mut(&session_id)
        .ok_or("State Machine execution engine is unavailable")?;
    engine
        .reset(&project, session)
        .map_err(|error| error.to_string())?;
    Ok(engine.snapshot(session))
}

#[tauri::command]
pub fn terminate_state_machine_execution(
    diagram_id: String,
    execution_state: tauri::State<'_, StateMachineExecutionState>,
) -> Result<StateMachineExecutionSnapshot, String> {
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "State Machine execution lock poisoned")?;
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
pub fn queue_state_machine_signal(
    diagram_id: String,
    signal_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    execution_state: tauri::State<'_, StateMachineExecutionState>,
) -> Result<StateMachineExecutionSnapshot, String> {
    let project = project_snapshot(&workspace)?;
    let (repository, machine_id) = machine_for_diagram(&workspace, &diagram_id)?;
    let fingerprint = source_fingerprint(&project, &repository)?;
    let signal_id = uuid::Uuid::parse_str(&signal_id)
        .map(ElementId)
        .map_err(|_| "invalid Signal stable ID".to_string())?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "State Machine execution lock poisoned")?;
    let (session_id, _) = ensure_current_execution(
        &project,
        repository,
        machine_id,
        &diagram_id,
        fingerprint,
        &mut registry,
    )?;
    let StateMachineExecutionRegistry {
        manager, engines, ..
    } = &mut *registry;
    let session = manager
        .session_mut(session_id)
        .map_err(|error| error.to_string())?;
    let engine = engines
        .get(&session_id)
        .ok_or("State Machine execution engine is unavailable")?;
    let signal_name = project
        .element(signal_id)
        .map_err(|error| error.to_string())?
        .name
        .clone();
    engine
        .queue_signal(
            &project,
            session,
            signal_id,
            signal_name,
            Vec::<(String, RuntimeValue)>::new(),
        )
        .map_err(|error| error.to_string())?;
    Ok(engine.snapshot(session))
}

#[tauri::command]
pub fn clear_state_machine_executions(
    execution_state: tauri::State<'_, StateMachineExecutionState>,
) -> Result<(), String> {
    *execution_state
        .registry
        .lock()
        .map_err(|_| "State Machine execution lock poisoned")? =
        StateMachineExecutionRegistry::default();
    Ok(())
}
