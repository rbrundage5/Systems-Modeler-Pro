use super::{ActivityWorkspaceState, WorkspaceState, behavior_workspace::BehaviorDiagramKind};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use systems_modeler_core::{
    ActivityId, ActivityRepository, BehaviorRepository, ElementId, ElementKind,
    ExecutionConfiguration, ExecutionEngine, ExecutionManager, ExecutionRuntimePreview,
    ExecutionRuntimeSelection, ExecutionSessionId, ExecutionState, Project, Region,
    RuntimeInstanceId, RuntimeValue, StateMachineExecutionEngine, StateMachineExecutionSnapshot,
    StateMachineId, StructuralRuntime, VertexKind,
};

#[derive(Default)]
struct StateMachineExecutionRegistry {
    manager: ExecutionManager,
    engines: HashMap<ExecutionSessionId, StateMachineExecutionEngine>,
    sessions_by_diagram: HashMap<String, ExecutionSessionId>,
    source_fingerprints: HashMap<String, String>,
    runtime_selections: HashMap<String, ExecutionRuntimeSelection>,
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

fn activity_repository_snapshot(
    activity: &ActivityWorkspaceState,
) -> Result<ActivityRepository, String> {
    activity
        .repository
        .lock()
        .map_err(|_| "activity repository lock poisoned".to_string())
        .map(|repository| repository.clone())
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
        .find(|diagram| {
            diagram.id == diagram_id && diagram.kind == BehaviorDiagramKind::StateMachine
        })
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

fn validate_activity_reference(
    activities: &ActivityRepository,
    state_name: &str,
    role: &str,
    value: &str,
) -> Result<(), String> {
    let activity_id = uuid::Uuid::parse_str(value)
        .map(ActivityId)
        .map_err(|_| {
            format!(
                "State '{state_name}' {role} must reference a modeled Activity by stable ID; '{value}' is not an Activity ID"
            )
        })?;
    if !activities.activities.contains_key(&activity_id) {
        return Err(format!(
            "State '{state_name}' {role} references missing Activity stable ID {activity_id}"
        ));
    }
    Ok(())
}

fn validate_region_activity_references(
    regions: &[Region],
    activities: &ActivityRepository,
) -> Result<(), String> {
    for region in regions {
        for vertex in &region.vertices {
            let VertexKind::State(state) = &vertex.kind else {
                continue;
            };
            for (role, reference) in [
                ("entry", state.entry.as_deref()),
                ("doActivity", state.do_activity.as_deref()),
                ("exit", state.exit.as_deref()),
            ] {
                if let Some(reference) = reference.filter(|value| !value.trim().is_empty()) {
                    validate_activity_reference(activities, &vertex.name, role, reference)?;
                }
            }
            validate_region_activity_references(&state.regions, activities)?;
        }
    }
    Ok(())
}

fn validate_machine_activity_references(
    repository: &BehaviorRepository,
    activities: &ActivityRepository,
    machine_id: StateMachineId,
    visited: &mut HashSet<StateMachineId>,
) -> Result<(), String> {
    if !visited.insert(machine_id) {
        return Ok(());
    }
    let machine = repository
        .state_machines
        .get(&machine_id)
        .ok_or_else(|| format!("State Machine references missing semantics: {machine_id}"))?;
    validate_region_activity_references(&machine.regions, activities)?;
    fn visit_submachines(regions: &[Region], output: &mut Vec<StateMachineId>) {
        for region in regions {
            for vertex in &region.vertices {
                if let VertexKind::State(state) = &vertex.kind {
                    if let Some(submachine) = state.submachine {
                        output.push(submachine);
                    }
                    visit_submachines(&state.regions, output);
                }
            }
        }
    }
    let mut submachines = Vec::new();
    visit_submachines(&machine.regions, &mut submachines);
    submachines.sort_by_key(ToString::to_string);
    submachines.dedup();
    for submachine in submachines {
        validate_machine_activity_references(repository, activities, submachine, visited)?;
    }
    Ok(())
}

fn source_fingerprint(
    project: &Project,
    repository: &BehaviorRepository,
    activities: &ActivityRepository,
) -> Result<String, String> {
    serde_json::to_string(&(project, repository, activities)).map_err(|error| {
        format!("failed to fingerprint State Machine execution source model: {error}")
    })
}

fn execution_source(
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
    diagram_id: &str,
) -> Result<
    (
        Project,
        BehaviorRepository,
        ActivityRepository,
        StateMachineId,
        String,
    ),
    String,
> {
    let project = project_snapshot(workspace)?;
    let (repository, machine_id) = machine_for_diagram(workspace, diagram_id)?;
    let activities = activity_repository_snapshot(activity)?;
    validate_machine_activity_references(
        &repository,
        &activities,
        machine_id,
        &mut HashSet::new(),
    )?;
    let fingerprint = source_fingerprint(&project, &repository, &activities)?;
    Ok((project, repository, activities, machine_id, fingerprint))
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

fn state_machine_runtime_context(
    project: &Project,
    repository: &BehaviorRepository,
    machine_id: StateMachineId,
    selection: &ExecutionRuntimeSelection,
    require_unambiguous_selection: bool,
) -> Result<(ExecutionRuntimePreview, Option<RuntimeInstanceId>), String> {
    let machine = repository
        .state_machines
        .get(&machine_id)
        .ok_or("State Machine was not found")?;
    let root_semantic_id = selection.root_semantic_id.unwrap_or(machine.context_id);
    let root = project
        .element(root_semantic_id)
        .map_err(|error| format!("Execution runtime root is invalid: {error}"))?;
    if !matches!(
        &root.kind,
        ElementKind::Block
            | ElementKind::AssociationBlock
            | ElementKind::PartProperty
            | ElementKind::InstanceSpecification
    ) {
        return Err(format!(
            "State Machine runtime root '{}' ({:?}) is not a structural execution root.",
            root.name, root.kind
        ));
    }
    let runtime = StructuralRuntime::build(
        project,
        root_semantic_id,
        &selection.structural_configuration,
    )
    .map_err(|error| error.to_string())?;
    let compatible_runtime_instance_paths =
        runtime.compatible_instance_paths(project, machine.context_id);
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
                    "Runtime occurrence '{path}' is not compatible with State Machine '{}'. Compatible occurrence path(s): {}",
                    machine.name,
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
                "State Machine '{}' has {} compatible runtime occurrences under '{}'. Choose one runtime occurrence before initialization: {}",
                machine.name,
                compatible_runtime_instance_paths.len(),
                root.name,
                compatible_runtime_instance_paths.join(", ")
            ));
        }
        None => None,
    };
    if require_unambiguous_selection && selected_runtime_instance_path.is_none() {
        return Err(format!(
            "No runtime occurrence compatible with State Machine '{}' exists under '{}'. Select a compatible structural root or correct the model typing.",
            machine.name, root.name
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

fn invalidate_state_machine_execution(
    registry: &mut StateMachineExecutionRegistry,
    diagram_id: &str,
) {
    if let Some(previous) = registry.sessions_by_diagram.remove(diagram_id) {
        registry.engines.remove(&previous);
        registry.manager.remove_session(previous);
    }
    registry.source_fingerprints.remove(diagram_id);
}

fn start_execution(
    project: &Project,
    repository: BehaviorRepository,
    activities: ActivityRepository,
    machine_id: StateMachineId,
    diagram_id: &str,
    fingerprint: String,
    registry: &mut StateMachineExecutionRegistry,
) -> Result<ExecutionSessionId, String> {
    let selection = registry
        .runtime_selections
        .get(diagram_id)
        .cloned()
        .unwrap_or_default();
    let (preview, runtime_instance_id) =
        state_machine_runtime_context(project, &repository, machine_id, &selection, true)?;
    let configuration = ExecutionConfiguration {
        root_semantic_id: preview.root_semantic_id,
        random_seed: 0,
        max_steps: 100_000,
        max_queued_events: 10_000,
    };
    invalidate_state_machine_execution(registry, diagram_id);
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
    let mut engine = StateMachineExecutionEngine::new(repository, machine_id)
        .with_activity_repository(activities);
    if let Some(runtime_instance_id) = runtime_instance_id {
        engine = engine.with_runtime_instance(runtime_instance_id);
    }
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
    activities: ActivityRepository,
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
        activities,
        machine_id,
        diagram_id,
        fingerprint,
        registry,
    )?;
    Ok((refreshed, true))
}

#[tauri::command]
pub fn state_machine_execution_runtime_selection(
    diagram_id: String,
    execution_state: tauri::State<'_, StateMachineExecutionState>,
) -> Result<ExecutionRuntimeSelection, String> {
    let registry = execution_state
        .registry
        .lock()
        .map_err(|_| "State Machine execution lock poisoned")?;
    Ok(registry
        .runtime_selections
        .get(&diagram_id)
        .cloned()
        .unwrap_or_default())
}

#[tauri::command]
pub fn preview_state_machine_execution_runtime(
    diagram_id: String,
    selection: ExecutionRuntimeSelection,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
) -> Result<ExecutionRuntimePreview, String> {
    let (project, repository, _activities, machine_id, _fingerprint) =
        execution_source(&workspace, &activity, &diagram_id)?;
    state_machine_runtime_context(&project, &repository, machine_id, &selection, false)
        .map(|(preview, _)| preview)
}

#[tauri::command]
pub fn configure_state_machine_execution_runtime(
    diagram_id: String,
    selection: ExecutionRuntimeSelection,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    execution_state: tauri::State<'_, StateMachineExecutionState>,
) -> Result<ExecutionRuntimePreview, String> {
    let (project, repository, _activities, machine_id, _fingerprint) =
        execution_source(&workspace, &activity, &diagram_id)?;
    let (preview, _) =
        state_machine_runtime_context(&project, &repository, machine_id, &selection, true)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "State Machine execution lock poisoned")?;
    invalidate_state_machine_execution(&mut registry, &diagram_id);
    registry.runtime_selections.insert(diagram_id, selection);
    Ok(preview)
}

#[tauri::command]
pub fn initialize_state_machine_execution(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    execution_state: tauri::State<'_, StateMachineExecutionState>,
) -> Result<StateMachineExecutionSnapshot, String> {
    let (project, repository, activities, machine_id, fingerprint) =
        execution_source(&workspace, &activity, &diagram_id)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "State Machine execution lock poisoned")?;
    let session_id = start_execution(
        &project,
        repository,
        activities,
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
    activity: tauri::State<'_, ActivityWorkspaceState>,
    execution_state: tauri::State<'_, StateMachineExecutionState>,
) -> Result<Option<StateMachineExecutionSnapshot>, String> {
    let (_, _, _, _, fingerprint) = execution_source(&workspace, &activity, &diagram_id)?;
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
    activity: tauri::State<'_, ActivityWorkspaceState>,
    execution_state: tauri::State<'_, StateMachineExecutionState>,
) -> Result<StateMachineExecutionSnapshot, String> {
    let (project, repository, activities, machine_id, fingerprint) =
        execution_source(&workspace, &activity, &diagram_id)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "State Machine execution lock poisoned")?;
    let (session_id, _) = ensure_current_execution(
        &project,
        repository,
        activities,
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
    activity: tauri::State<'_, ActivityWorkspaceState>,
    execution_state: tauri::State<'_, StateMachineExecutionState>,
) -> Result<StateMachineExecutionSnapshot, String> {
    let (project, repository, activities, machine_id, fingerprint) =
        execution_source(&workspace, &activity, &diagram_id)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "State Machine execution lock poisoned")?;
    let (session_id, _) = ensure_current_execution(
        &project,
        repository,
        activities,
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
    activity: tauri::State<'_, ActivityWorkspaceState>,
    execution_state: tauri::State<'_, StateMachineExecutionState>,
) -> Result<StateMachineExecutionSnapshot, String> {
    let (project, repository, activities, machine_id, fingerprint) =
        execution_source(&workspace, &activity, &diagram_id)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "State Machine execution lock poisoned")?;
    let (session_id, refreshed) = ensure_current_execution(
        &project,
        repository,
        activities,
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
    activity: tauri::State<'_, ActivityWorkspaceState>,
    execution_state: tauri::State<'_, StateMachineExecutionState>,
) -> Result<StateMachineExecutionSnapshot, String> {
    let (project, repository, activities, machine_id, fingerprint) =
        execution_source(&workspace, &activity, &diagram_id)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "State Machine execution lock poisoned")?;
    let (session_id, refreshed) = ensure_current_execution(
        &project,
        repository,
        activities,
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
    activity: tauri::State<'_, ActivityWorkspaceState>,
    execution_state: tauri::State<'_, StateMachineExecutionState>,
) -> Result<StateMachineExecutionSnapshot, String> {
    let (project, repository, activities, machine_id, fingerprint) =
        execution_source(&workspace, &activity, &diagram_id)?;
    let signal_id = uuid::Uuid::parse_str(&signal_id)
        .map(ElementId)
        .map_err(|_| "invalid Signal stable ID".to_string())?;
    let signal = project
        .element(signal_id)
        .map_err(|error| error.to_string())?;
    if signal.kind != ElementKind::Signal {
        return Err(format!(
            "queued State Machine SignalEvent must reference a Signal; '{}' is {:?}",
            signal.name, signal.kind
        ));
    }
    let signal_name = signal.name.clone();
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "State Machine execution lock poisoned")?;
    let (session_id, _) = ensure_current_execution(
        &project,
        repository,
        activities,
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
