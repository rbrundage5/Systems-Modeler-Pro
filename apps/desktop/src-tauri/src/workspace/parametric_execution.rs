use super::{WorkspaceState, parametrics::evaluation_scope};
use std::collections::HashMap;
use std::sync::Mutex;
use systems_modeler_core::{
    ElementKind, ExecutionConfiguration, ExecutionEngine, ExecutionManager, ExecutionRuntimePreview,
    ExecutionRuntimeSelection, ExecutionSessionId, ParametricEvaluationScope,
    ParametricExecutionEngine, ParametricExecutionSnapshot, Project, StructuralRuntime,
};

#[derive(Default)]
struct ParametricExecutionRegistry {
    manager: ExecutionManager,
    engines: HashMap<ExecutionSessionId, ParametricExecutionEngine>,
    sessions_by_diagram: HashMap<String, ExecutionSessionId>,
    source_fingerprints: HashMap<String, String>,
    runtime_selections: HashMap<String, ExecutionRuntimeSelection>,
}

#[derive(Default)]
pub struct ParametricExecutionState {
    registry: Mutex<ParametricExecutionRegistry>,
}

fn execution_source(
    workspace: &WorkspaceState,
    diagram_id: &str,
) -> Result<(Project, ParametricEvaluationScope, String), String> {
    let project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or_else(|| "open a project before evaluating Parametrics".to_string())?;
    let diagram = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .iter()
        .find(|diagram| diagram.id == diagram_id && diagram.family == "parametric")
        .cloned()
        .ok_or_else(|| format!("Parametric Diagram was not found: {diagram_id}"))?;
    let scope = evaluation_scope(&diagram, &project)?;
    let fingerprint = serde_json::to_string(&(&project, &scope))
        .map_err(|error| format!("failed to fingerprint Parametric execution source: {error}"))?;
    Ok((project, scope, fingerprint))
}

fn runtime_preview(
    project: &Project,
    scope: &ParametricEvaluationScope,
    selection: &ExecutionRuntimeSelection,
) -> Result<ExecutionRuntimePreview, String> {
    let context = project
        .element(scope.context_id)
        .map_err(|error| format!("Parametric context is invalid: {error}"))?;
    if context.kind == ElementKind::ConstraintBlock {
        if selection.runtime_instance_path.is_some() {
            return Err(format!(
                "ConstraintBlock '{}' is a reusable definition and cannot select a runtime occurrence.",
                context.name
            ));
        }
        return Ok(ExecutionRuntimePreview {
            root_semantic_id: project.root_id,
            structural_runtime: None,
            compatible_runtime_instance_paths: Vec::new(),
            selected_runtime_instance_path: None,
        });
    }
    if !matches!(context.kind, ElementKind::Block | ElementKind::AssociationBlock) {
        return Err(format!(
            "Parametric context '{}' ({:?}) is not an executable structural classifier.",
            context.name, context.kind
        ));
    }
    let root_semantic_id = selection.root_semantic_id.unwrap_or(scope.context_id);
    let root = project
        .element(root_semantic_id)
        .map_err(|error| format!("Parametric runtime root is invalid: {error}"))?;
    if !matches!(
        root.kind,
        ElementKind::Block
            | ElementKind::AssociationBlock
            | ElementKind::PartProperty
            | ElementKind::InstanceSpecification
    ) {
        return Err(format!(
            "Parametric runtime root '{}' ({:?}) is not a structural execution root.",
            root.name, root.kind
        ));
    }
    let runtime = StructuralRuntime::build(
        project,
        root_semantic_id,
        &selection.structural_configuration,
    )
    .map_err(|error| error.to_string())?;
    let compatible = runtime.compatible_instance_paths(project, scope.context_id);
    let selected = match selection.runtime_instance_path.as_deref() {
        Some(path) if compatible.iter().any(|candidate| candidate == path) => {
            Some(path.to_string())
        }
        Some(path) => {
            return Err(format!(
                "Parametric runtime occurrence '{}' is not compatible with context '{}'. Compatible occurrence(s): {}.",
                path,
                context.name,
                if compatible.is_empty() {
                    "none".into()
                } else {
                    compatible.join(", ")
                }
            ));
        }
        None if compatible.len() == 1 => compatible.first().cloned(),
        None => None,
    };
    Ok(ExecutionRuntimePreview {
        root_semantic_id,
        structural_runtime: Some(runtime.snapshot()),
        compatible_runtime_instance_paths: compatible,
        selected_runtime_instance_path: selected,
    })
}

fn invalidate(registry: &mut ParametricExecutionRegistry, diagram_id: &str) {
    if let Some(session_id) = registry.sessions_by_diagram.remove(diagram_id) {
        registry.engines.remove(&session_id);
        registry.manager.remove_session(session_id);
    }
    registry.source_fingerprints.remove(diagram_id);
}

fn start_execution(
    project: &Project,
    scope: ParametricEvaluationScope,
    diagram_id: &str,
    fingerprint: String,
    registry: &mut ParametricExecutionRegistry,
) -> Result<ExecutionSessionId, String> {
    let selection = registry
        .runtime_selections
        .get(diagram_id)
        .cloned()
        .unwrap_or_default();
    let preview = runtime_preview(project, &scope, &selection)?;
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
    let mut engine = ParametricExecutionEngine::new(scope)
        .with_runtime_instance_path(preview.selected_runtime_instance_path);
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
    registry: &ParametricExecutionRegistry,
    session_id: ExecutionSessionId,
) -> Result<ParametricExecutionSnapshot, String> {
    let session = registry
        .manager
        .session(session_id)
        .map_err(|error| error.to_string())?;
    let engine = registry
        .engines
        .get(&session_id)
        .ok_or("Parametric execution engine is unavailable")?;
    Ok(engine.snapshot(session))
}

fn current_session(
    registry: &ParametricExecutionRegistry,
    diagram_id: &str,
) -> Result<ExecutionSessionId, String> {
    registry
        .sessions_by_diagram
        .get(diagram_id)
        .copied()
        .ok_or_else(|| "initialize this Parametric execution first".into())
}

#[tauri::command]
pub fn parametric_execution_runtime_selection(
    diagram_id: String,
    execution_state: tauri::State<'_, ParametricExecutionState>,
) -> Result<ExecutionRuntimeSelection, String> {
    let registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Parametric execution lock poisoned")?;
    Ok(registry
        .runtime_selections
        .get(&diagram_id)
        .cloned()
        .unwrap_or_default())
}

#[tauri::command]
pub fn preview_parametric_execution_runtime(
    diagram_id: String,
    selection: ExecutionRuntimeSelection,
    workspace: tauri::State<'_, WorkspaceState>,
) -> Result<ExecutionRuntimePreview, String> {
    let (project, scope, _) = execution_source(&workspace, &diagram_id)?;
    runtime_preview(&project, &scope, &selection)
}

#[tauri::command]
pub fn configure_parametric_execution_runtime(
    diagram_id: String,
    selection: ExecutionRuntimeSelection,
    workspace: tauri::State<'_, WorkspaceState>,
    execution_state: tauri::State<'_, ParametricExecutionState>,
) -> Result<ExecutionRuntimePreview, String> {
    let (project, scope, _) = execution_source(&workspace, &diagram_id)?;
    let preview = runtime_preview(&project, &scope, &selection)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Parametric execution lock poisoned")?;
    invalidate(&mut registry, &diagram_id);
    registry.runtime_selections.insert(diagram_id, selection);
    Ok(preview)
}

#[tauri::command]
pub fn initialize_parametric_execution(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    execution_state: tauri::State<'_, ParametricExecutionState>,
) -> Result<ParametricExecutionSnapshot, String> {
    let (project, scope, fingerprint) = execution_source(&workspace, &diagram_id)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Parametric execution lock poisoned")?;
    let session_id = start_execution(
        &project,
        scope,
        &diagram_id,
        fingerprint,
        &mut registry,
    )?;
    snapshot_for(&registry, session_id)
}

#[tauri::command]
pub fn parametric_execution_snapshot(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    execution_state: tauri::State<'_, ParametricExecutionState>,
) -> Result<Option<ParametricExecutionSnapshot>, String> {
    let (_, _, fingerprint) = execution_source(&workspace, &diagram_id)?;
    let registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Parametric execution lock poisoned")?;
    let Some(session_id) = registry.sessions_by_diagram.get(&diagram_id).copied() else {
        return Ok(None);
    };
    if registry.source_fingerprints.get(&diagram_id) != Some(&fingerprint) {
        return Ok(None);
    }
    snapshot_for(&registry, session_id).map(Some)
}

#[tauri::command]
pub fn run_parametric_execution(
    diagram_id: String,
    execution_state: tauri::State<'_, ParametricExecutionState>,
) -> Result<ParametricExecutionSnapshot, String> {
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Parametric execution lock poisoned")?;
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
pub fn step_parametric_execution(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    execution_state: tauri::State<'_, ParametricExecutionState>,
) -> Result<ParametricExecutionSnapshot, String> {
    let (project, _, fingerprint) = execution_source(&workspace, &diagram_id)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Parametric execution lock poisoned")?;
    let session_id = current_session(&registry, &diagram_id)?;
    if registry.source_fingerprints.get(&diagram_id) != Some(&fingerprint) {
        return Err("Parametric model changed; initialize execution again".into());
    }
    let ParametricExecutionRegistry {
        manager, engines, ..
    } = &mut *registry;
    let session = manager
        .session_mut(session_id)
        .map_err(|error| error.to_string())?;
    let engine = engines
        .get_mut(&session_id)
        .ok_or("Parametric execution engine is unavailable")?;
    engine
        .step(&project, session)
        .map_err(|error| error.to_string())?;
    Ok(engine.snapshot(session))
}

#[tauri::command]
pub fn evaluate_parametric_execution(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    execution_state: tauri::State<'_, ParametricExecutionState>,
) -> Result<ParametricExecutionSnapshot, String> {
    let (project, scope, fingerprint) = execution_source(&workspace, &diagram_id)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Parametric execution lock poisoned")?;
    let session_id = start_execution(
        &project,
        scope,
        &diagram_id,
        fingerprint,
        &mut registry,
    )?;
    let ParametricExecutionRegistry {
        manager, engines, ..
    } = &mut *registry;
    let session = manager
        .session_mut(session_id)
        .map_err(|error| error.to_string())?;
    session.run().map_err(|error| error.to_string())?;
    let engine = engines
        .get_mut(&session_id)
        .ok_or("Parametric execution engine is unavailable")?;
    engine
        .step(&project, session)
        .map_err(|error| error.to_string())?;
    Ok(engine.snapshot(session))
}

#[tauri::command]
pub fn reset_parametric_execution(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    execution_state: tauri::State<'_, ParametricExecutionState>,
) -> Result<ParametricExecutionSnapshot, String> {
    let (project, _, fingerprint) = execution_source(&workspace, &diagram_id)?;
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Parametric execution lock poisoned")?;
    let session_id = current_session(&registry, &diagram_id)?;
    if registry.source_fingerprints.get(&diagram_id) != Some(&fingerprint) {
        return Err("Parametric model changed; initialize execution again".into());
    }
    let ParametricExecutionRegistry {
        manager, engines, ..
    } = &mut *registry;
    let session = manager
        .session_mut(session_id)
        .map_err(|error| error.to_string())?;
    let engine = engines
        .get_mut(&session_id)
        .ok_or("Parametric execution engine is unavailable")?;
    engine
        .reset(&project, session)
        .map_err(|error| error.to_string())?;
    Ok(engine.snapshot(session))
}

#[tauri::command]
pub fn terminate_parametric_execution(
    diagram_id: String,
    execution_state: tauri::State<'_, ParametricExecutionState>,
) -> Result<ParametricExecutionSnapshot, String> {
    let mut registry = execution_state
        .registry
        .lock()
        .map_err(|_| "Parametric execution lock poisoned")?;
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
pub fn clear_parametric_executions(
    execution_state: tauri::State<'_, ParametricExecutionState>,
) -> Result<(), String> {
    *execution_state
        .registry
        .lock()
        .map_err(|_| "Parametric execution lock poisoned")? =
        ParametricExecutionRegistry::default();
    Ok(())
}
