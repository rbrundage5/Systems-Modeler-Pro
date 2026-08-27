from pathlib import Path


def replace_once(text, old, new, label):
    count = text.count(old)
    if count != 1:
        raise SystemExit(f'{label}: expected one match, found {count}')
    return text.replace(old, new, 1)

# ---------------------------------------------------------------------------
# Shared model-core runtime selection / preview contract.
# ---------------------------------------------------------------------------
path = Path('crates/model-core/src/structural_runtime.rs')
text = path.read_text()
anchor = '''#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuntimePortKey {
'''
insert = '''#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionRuntimeSelection {
    /// Optional structural execution root. `None` preserves the behavior's
    /// existing context root.
    pub root_semantic_id: Option<ElementId>,
    #[serde(default)]
    pub structural_configuration: StructuralRuntimeConfiguration,
    /// Qualified runtime occurrence path. Required when the behavior classifier
    /// appears more than once below the selected structural root.
    pub runtime_instance_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionRuntimePreview {
    pub root_semantic_id: ElementId,
    pub structural_runtime: Option<StructuralRuntimeSnapshot>,
    pub compatible_runtime_instance_paths: Vec<String>,
    pub selected_runtime_instance_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuntimePortKey {
'''
text = replace_once(text, anchor, insert, 'runtime selection types')
anchor = '''    pub fn root_instance_id(&self) -> Option<RuntimeInstanceId> {
        self.root_instance_ids.first().copied()
    }

    pub fn snapshot(&self) -> StructuralRuntimeSnapshot {
'''
insert = '''    pub fn root_instance_id(&self) -> Option<RuntimeInstanceId> {
        self.root_instance_ids.first().copied()
    }

    pub fn instance_conforms_to(
        &self,
        project: &Project,
        instance_id: RuntimeInstanceId,
        expected_classifier_id: ElementId,
    ) -> bool {
        self.instances
            .get(&instance_id)
            .and_then(|instance| instance.classifier_id)
            .is_some_and(|actual| classifier_conforms(project, actual, expected_classifier_id))
    }

    pub fn compatible_instance_paths(
        &self,
        project: &Project,
        expected_classifier_id: ElementId,
    ) -> Vec<String> {
        let mut paths: Vec<_> = self
            .instances
            .values()
            .filter(|instance| self.instance_conforms_to(project, instance.id, expected_classifier_id))
            .map(|instance| instance.qualified_path.clone())
            .collect();
        paths.sort();
        paths.dedup();
        paths
    }

    pub fn snapshot(&self) -> StructuralRuntimeSnapshot {
'''
text = replace_once(text, anchor, insert, 'runtime compatibility methods')
path.write_text(text)

# ---------------------------------------------------------------------------
# Activity desktop execution: transient structural configuration and explicit
# runtime occurrence selection.
# ---------------------------------------------------------------------------
path = Path('apps/desktop/src-tauri/src/workspace/activity_execution.rs')
text = path.read_text()
old = '''use systems_modeler_core::{
    ActivityExecutionEngine, ActivityExecutionSnapshot, ActivityId, ActivityRepository,
    ExecutionConfiguration, ExecutionEngine, ExecutionManager, ExecutionSessionId, ExecutionState,
    Project,
};
'''
new = '''use systems_modeler_core::{
    ActivityExecutionEngine, ActivityExecutionSnapshot, ActivityId, ActivityRepository, ElementId,
    ElementKind, ExecutionConfiguration, ExecutionEngine, ExecutionManager, ExecutionRuntimePreview,
    ExecutionRuntimeSelection, ExecutionSessionId, ExecutionState, Project, RuntimeInstanceId,
    StructuralRuntime,
};
'''
text = replace_once(text, old, new, 'activity imports')
old = '''    sessions_by_diagram: HashMap<String, ExecutionSessionId>,
    source_fingerprints: HashMap<String, String>,
}
'''
new = '''    sessions_by_diagram: HashMap<String, ExecutionSessionId>,
    source_fingerprints: HashMap<String, String>,
    runtime_selections: HashMap<String, ExecutionRuntimeSelection>,
}
'''
text = replace_once(text, old, new, 'activity registry selection')
anchor = '''fn start_execution(
    project: &Project,
'''
helper = r'''fn is_structural_root(kind: ElementKind) -> bool {
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
    if !is_structural_root(root.kind) {
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
            matches!(element.kind, ElementKind::Block | ElementKind::AssociationBlock)
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
            if !compatible_runtime_instance_paths.iter().any(|candidate| candidate == path) {
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
    let selected_runtime_instance_id = selected_runtime_instance_path
        .as_deref()
        .and_then(|runtime_path| runtime.instance_by_path(runtime_path).map(|instance| instance.id));
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
'''
text = replace_once(text, anchor, helper, 'activity runtime helper')
old = '''    let activity = repository
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
'''
new = '''    let selection = registry
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
'''
text = replace_once(text, old, new, 'activity start execution')
anchor = '''#[tauri::command]
pub fn initialize_activity_execution(
'''
commands = r'''#[tauri::command]
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
    let (preview, _) = activity_runtime_context(&project, &repository, activity_id, &selection, true)?;
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
'''
text = replace_once(text, anchor, commands, 'activity config commands')
path.write_text(text)

# ---------------------------------------------------------------------------
# State Machine desktop execution: same selection contract.
# ---------------------------------------------------------------------------
path = Path('apps/desktop/src-tauri/src/workspace/state_machine_execution.rs')
text = path.read_text()
old = '''    ActivityId, ActivityRepository, BehaviorRepository, ElementId, ElementKind,
    ExecutionConfiguration, ExecutionEngine, ExecutionManager, ExecutionSessionId, ExecutionState,
    Project, Region, RuntimeValue, StateMachineExecutionEngine, StateMachineExecutionSnapshot,
    StateMachineId, VertexKind,
'''
new = '''    ActivityId, ActivityRepository, BehaviorRepository, ElementId, ElementKind,
    ExecutionConfiguration, ExecutionEngine, ExecutionManager, ExecutionRuntimePreview,
    ExecutionRuntimeSelection, ExecutionSessionId, ExecutionState, Project, Region,
    RuntimeInstanceId, RuntimeValue, StateMachineExecutionEngine, StateMachineExecutionSnapshot,
    StateMachineId, StructuralRuntime, VertexKind,
'''
text = replace_once(text, old, new, 'state imports')
old = '''    sessions_by_diagram: HashMap<String, ExecutionSessionId>,
    source_fingerprints: HashMap<String, String>,
}
'''
new = '''    sessions_by_diagram: HashMap<String, ExecutionSessionId>,
    source_fingerprints: HashMap<String, String>,
    runtime_selections: HashMap<String, ExecutionRuntimeSelection>,
}
'''
text = replace_once(text, old, new, 'state registry selection')
anchor = '''fn start_execution(
    project: &Project,
'''
helper = r'''fn state_machine_runtime_context(
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
        root.kind,
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
            if !compatible_runtime_instance_paths.iter().any(|candidate| candidate == path) {
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
    let selected_runtime_instance_id = selected_runtime_instance_path
        .as_deref()
        .and_then(|runtime_path| runtime.instance_by_path(runtime_path).map(|instance| instance.id));
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
'''
text = replace_once(text, anchor, helper, 'state runtime helper')
old = '''    let machine = repository
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
    let mut engine = StateMachineExecutionEngine::new(repository, machine_id)
        .with_activity_repository(activities);
'''
new = '''    let selection = registry
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
'''
text = replace_once(text, old, new, 'state start execution')
anchor = '''#[tauri::command]
pub fn initialize_state_machine_execution(
'''
commands = r'''#[tauri::command]
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
'''
text = replace_once(text, anchor, commands, 'state config commands')
path.write_text(text)

# ---------------------------------------------------------------------------
# Main command registration.
# ---------------------------------------------------------------------------
path = Path('apps/desktop/src-tauri/src/main.rs')
text = path.read_text()
old = '''            activity_execution_snapshot,
            initialize_activity_execution,
'''
new = '''            activity_execution_snapshot,
            activity_execution_runtime_selection,
            preview_activity_execution_runtime,
            configure_activity_execution_runtime,
            initialize_activity_execution,
'''
text = replace_once(text, old, new, 'main activity commands')
old = '''            state_machine_execution_snapshot,
            initialize_state_machine_execution,
'''
new = '''            state_machine_execution_snapshot,
            state_machine_execution_runtime_selection,
            preview_state_machine_execution_runtime,
            configure_state_machine_execution_runtime,
            initialize_state_machine_execution,
'''
text = replace_once(text, old, new, 'main state commands')
path.write_text(text)

# ---------------------------------------------------------------------------
# Activity ribbon: expose Runtime configuration without adding a controller.
# ---------------------------------------------------------------------------
path = Path('apps/desktop/frontend/activity-rich-ui.js')
text = path.read_text()
old = '''        <button class="ribbon-command" data-activity-execution="initialize"><span class="command-icon">◇</span><span>Initialize</span></button>
        <button class="ribbon-command" data-activity-execution="run"><span class="command-icon">▶</span><span>Run</span></button>
'''
new = '''        <button class="ribbon-command" data-activity-execution="runtime"><span class="command-icon">◎</span><span>Runtime</span></button>
        <button class="ribbon-command" data-activity-execution="initialize"><span class="command-icon">◇</span><span>Initialize</span></button>
        <button class="ribbon-command" data-activity-execution="run"><span class="command-icon">▶</span><span>Run</span></button>
'''
text = replace_once(text, old, new, 'activity runtime button')
old = '''          if (command === 'initialize') await initializeExecution();
          else if (command === 'run') await runExecution(false);
'''
new = '''          if (command === 'runtime') {
            await window.smpOpenStructuralRuntimeConfiguration?.('activity', activeDiagram().id);
            Object.assign(state, { activityExecutionSnapshot: null });
            refreshExecutionUi();
          } else if (command === 'initialize') await initializeExecution();
          else if (command === 'run') await runExecution(false);
'''
text = replace_once(text, old, new, 'activity runtime handler')
old = '''  function refreshExecutionUi() {
'''
new = '''  window.smpRefreshActivityExecution = () => refreshExecutionUi();

  function refreshExecutionUi() {
'''
# Function declarations are hoisted, so exposing before declaration is safe.
text = replace_once(text, old, new, 'activity refresh export')
path.write_text(text)

# ---------------------------------------------------------------------------
# State Machine ribbon and shared thin configuration dialog.
# ---------------------------------------------------------------------------
path = Path('apps/desktop/frontend/behavior-authoritative-renderer.js')
text = path.read_text()
old = "const controls = [['initialize','◇','Initialize'],['run','▶','Run'],['step','▸','Step'],['pause','Ⅱ','Pause'],['resume','▷','Resume'],['reset','↺','Reset'],['terminate','■','Terminate'],['signal','⇢','Signal']];"
new = "const controls = [['runtime','◎','Runtime'],['initialize','◇','Initialize'],['run','▶','Run'],['step','▸','Step'],['pause','Ⅱ','Pause'],['resume','▷','Resume'],['reset','↺','Reset'],['terminate','■','Terminate'],['signal','⇢','Signal']];"
text = replace_once(text, old, new, 'state runtime button')
old = '''      if (command === 'initialize') await initializeStateMachineExecution();
      else if (command === 'run') await runStateMachineExecution(false);
'''
new = '''      if (command === 'runtime') {
        await window.smpOpenStructuralRuntimeConfiguration?.('stateMachine', executionDiagram().id);
        Object.assign(state, { stateMachineExecutionSnapshot: null });
        refreshStateMachineExecution();
      } else if (command === 'initialize') await initializeStateMachineExecution();
      else if (command === 'run') await runStateMachineExecution(false);
'''
text = replace_once(text, old, new, 'state runtime handler')
text += r'''

// PR33_STRUCTURAL_RUNTIME_CONFIGURATION_BEGIN
(() => {
  'use strict';

  const invoke = () => {
    const command = window.__TAURI__?.core?.invoke;
    if (!command) throw new Error('Tauri command bridge is unavailable.');
    return command;
  };
  const esc = (value) => String(value ?? '')
    .replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;').replaceAll("'", '&#039;');

  function parseArray(text, label) {
    const source = String(text || '').trim();
    if (!source) return [];
    let value;
    try { value = JSON.parse(source); } catch (error) {
      throw new Error(`${label} must be valid JSON: ${error.message}`);
    }
    if (!Array.isArray(value)) throw new Error(`${label} must be a JSON array.`);
    return value;
  }

  function selectionFromDialog(dialog) {
    return {
      root_semantic_id: dialog.querySelector('[data-runtime-root]').value || null,
      structural_configuration: {
        root_instance_name: dialog.querySelector('[data-runtime-root-name]').value.trim() || null,
        populations: parseArray(dialog.querySelector('[data-runtime-populations]').value, 'Population decisions'),
        reference_bindings: parseArray(dialog.querySelector('[data-runtime-references]').value, 'Reference bindings'),
        configured_instance_specification_ids: parseArray(dialog.querySelector('[data-runtime-instances]').value, 'Configured InstanceSpecification IDs'),
      },
      runtime_instance_path: dialog.querySelector('[data-runtime-path]').value.trim() || null,
    };
  }

  function commands(kind) {
    if (kind === 'activity') return {
      get: 'activity_execution_runtime_selection',
      preview: 'preview_activity_execution_runtime',
      configure: 'configure_activity_execution_runtime',
      label: 'Activity',
    };
    return {
      get: 'state_machine_execution_runtime_selection',
      preview: 'preview_state_machine_execution_runtime',
      configure: 'configure_state_machine_execution_runtime',
      label: 'State Machine',
    };
  }

  function runtimeRootOptions(current) {
    const elements = window.smpState?.snapshot?.project?.elements || [];
    const supported = new Set(['Block', 'AssociationBlock', 'PartProperty', 'InstanceSpecification']);
    return elements
      .filter((element) => supported.has(element.kind))
      .sort((left, right) => `${left.name}:${left.kind}`.localeCompare(`${right.name}:${right.kind}`))
      .map((element) => `<option value="${esc(element.id)}" ${String(current || '') === String(element.id) ? 'selected' : ''}>${esc(element.name)} · ${esc(element.kind)}</option>`)
      .join('');
  }

  window.smpOpenStructuralRuntimeConfiguration = async function openStructuralRuntimeConfiguration(kind, diagramId) {
    const api = commands(kind);
    const existing = await invoke()(api.get, { diagramId });
    document.querySelector('.structural-runtime-config-backdrop')?.remove();
    const backdrop = document.createElement('div');
    backdrop.className = 'structural-runtime-config-backdrop';
    const structural = existing?.structural_configuration || {};
    backdrop.innerHTML = `<section class="structural-runtime-config-dialog" role="dialog" aria-modal="true" aria-label="${esc(api.label)} runtime configuration">
      <header><div><strong>${esc(api.label)} Runtime Context</strong><p>Choose the structural system occurrence this behavior executes on. Rust validates and owns the resulting runtime graph.</p></div><button type="button" data-runtime-close aria-label="Close">×</button></header>
      <label>Structural execution root<select data-runtime-root><option value="">Behavior context (default)</option>${runtimeRootOptions(existing?.root_semantic_id)}</select></label>
      <label>Root occurrence name<input data-runtime-root-name value="${esc(structural.root_instance_name || '')}" placeholder="Optional engineer-facing runtime root name" /></label>
      <label>Behavior runtime occurrence<input data-runtime-path list="structural-runtime-compatible-paths" value="${esc(existing?.runtime_instance_path || '')}" placeholder="Auto-select when exactly one compatible occurrence exists" /><datalist id="structural-runtime-compatible-paths"></datalist></label>
      <details><summary>Advanced structural configuration</summary>
        <p class="muted">These are transient runtime decisions. They do not modify PartProperty multiplicity, ReferenceProperty ownership, or authored InstanceSpecifications.</p>
        <label>Population decisions (JSON array)<textarea data-runtime-populations rows="4">${esc(JSON.stringify(structural.populations || [], null, 2))}</textarea></label>
        <label>Reference bindings (JSON array)<textarea data-runtime-references rows="5">${esc(JSON.stringify(structural.reference_bindings || [], null, 2))}</textarea></label>
        <label>Additional configured InstanceSpecification IDs (JSON array)<textarea data-runtime-instances rows="3">${esc(JSON.stringify(structural.configured_instance_specification_ids || [], null, 2))}</textarea></label>
      </details>
      <div class="structural-runtime-config-preview" data-runtime-preview>Preview has not been run.</div>
      <footer><button type="button" data-runtime-preview-button>Preview</button><button type="button" data-runtime-apply>Apply Runtime Context</button><button type="button" data-runtime-close>Cancel</button></footer>
    </section>`;
    document.body.appendChild(backdrop);
    const dialog = backdrop.querySelector('.structural-runtime-config-dialog');
    const previewHost = dialog.querySelector('[data-runtime-preview]');
    const list = dialog.querySelector('#structural-runtime-compatible-paths');
    const close = () => backdrop.remove();
    dialog.querySelectorAll('[data-runtime-close]').forEach((button) => button.onclick = close);
    backdrop.addEventListener('click', (event) => { if (event.target === backdrop) close(); });

    async function preview() {
      const selection = selectionFromDialog(dialog);
      const result = await invoke()(api.preview, { diagramId, selection });
      list.innerHTML = (result.compatible_runtime_instance_paths || [])
        .map((path) => `<option value="${esc(path)}"></option>`).join('');
      const runtime = result.structural_runtime;
      previewHost.innerHTML = runtime
        ? `<b>${runtime.instances?.length || 0} runtime instance(s)</b> · ${runtime.ports?.length || 0} Port(s) · ${runtime.connector_links?.length || 0} connector link(s)<br/>Compatible behavior occurrence(s): ${esc((result.compatible_runtime_instance_paths || []).join(', ') || 'none')}`
        : 'This behavior has no structural runtime context. Existing non-structural execution will be preserved.';
      if (!dialog.querySelector('[data-runtime-path]').value && result.selected_runtime_instance_path) {
        dialog.querySelector('[data-runtime-path]').value = result.selected_runtime_instance_path;
      }
      return result;
    }

    dialog.querySelector('[data-runtime-preview-button]').onclick = async () => {
      try { await preview(); }
      catch (error) { previewHost.textContent = error?.message || String(error); previewHost.classList.add('runtime-error'); }
    };
    dialog.querySelector('[data-runtime-apply]').onclick = async () => {
      try {
        const selection = selectionFromDialog(dialog);
        await invoke()(api.configure, { diagramId, selection });
        if (kind === 'activity') {
          window.smpState.activityExecutionSnapshot = null;
          window.smpRefreshActivityExecution?.();
        } else {
          window.smpState.stateMachineExecutionSnapshot = null;
          window.smpRefreshStateMachineExecution?.();
        }
        window.smpDialogs?.notify?.('Runtime context configured. Initialize execution to build the validated structural runtime.', 'info');
        close();
      } catch (error) {
        previewHost.textContent = error?.message || String(error);
        previewHost.classList.add('runtime-error');
      }
    };
    try { await preview(); } catch (error) { previewHost.textContent = error?.message || String(error); }
  };
})();
// PR33_STRUCTURAL_RUNTIME_CONFIGURATION_END
'''
path.write_text(text)

# ---------------------------------------------------------------------------
# Configuration UI styles.
# ---------------------------------------------------------------------------
path = Path('apps/desktop/frontend/structural-runtime.css')
text = path.read_text()
text += r'''

.structural-runtime-config-backdrop {
  position: fixed;
  inset: 0;
  z-index: 1200;
  display: grid;
  place-items: center;
  background: rgba(20, 28, 38, 0.35);
}
.structural-runtime-config-dialog {
  width: min(760px, calc(100vw - 48px));
  max-height: calc(100vh - 64px);
  overflow: auto;
  background: var(--panel-bg, #fff);
  border: 1px solid var(--border, #b9c1cc);
  box-shadow: 0 18px 50px rgba(0,0,0,.28);
  padding: 16px;
  display: grid;
  gap: 12px;
}
.structural-runtime-config-dialog header,
.structural-runtime-config-dialog footer {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
}
.structural-runtime-config-dialog header p { margin: 4px 0 0; max-width: 620px; }
.structural-runtime-config-dialog label { display: grid; gap: 4px; font-size: 12px; font-weight: 600; }
.structural-runtime-config-dialog input,
.structural-runtime-config-dialog select,
.structural-runtime-config-dialog textarea {
  width: 100%;
  box-sizing: border-box;
  font: inherit;
  font-weight: 400;
  padding: 7px 8px;
  border: 1px solid var(--border, #b9c1cc);
  background: var(--input-bg, #fff);
}
.structural-runtime-config-dialog textarea { font-family: ui-monospace, SFMono-Regular, Consolas, monospace; }
.structural-runtime-config-dialog details { border: 1px solid var(--border, #ccd2da); padding: 8px; }
.structural-runtime-config-dialog details label { margin-top: 8px; }
.structural-runtime-config-preview { padding: 9px; border: 1px solid var(--border, #ccd2da); background: rgba(127,127,127,.06); font-size: 12px; }
.structural-runtime-config-preview.runtime-error { border-color: #a33; }
.structural-runtime-config-dialog footer { justify-content: flex-end; }
'''
path.write_text(text)

# ---------------------------------------------------------------------------
# Permanent PR33 desktop integration contract + CI registration.
# ---------------------------------------------------------------------------
validator = Path('scripts/validate_structural_runtime_integration.py')
validator.write_text(r'''from pathlib import Path

root = Path(__file__).resolve().parents[1]

def content(path):
    return (root / path).read_text(encoding='utf-8')

def require(path, *tokens):
    text = content(path)
    missing = [token for token in tokens if token not in text]
    if missing:
        raise SystemExit(f"{path}: missing PR33 structural runtime integration contract token(s): {missing}")

require('crates/model-core/src/structural_runtime.rs',
        'pub struct ExecutionRuntimeSelection',
        'pub struct ExecutionRuntimePreview',
        'pub fn compatible_instance_paths',
        'DuplicateRuntimePath',
        'DuplicatePopulationDecision',
        'DuplicateReferenceBindingDecision',
        'validate_runtime_assignment(self.project, property, &value)',
        'classifier_conforms(project, conveyed_id, contract.type_id)',
        'classifier_conforms(project, signal_id, accepted)')
require('crates/model-core/src/execution.rs',
        'pub structural_runtime: Option<StructuralRuntime>',
        'pub fn queue_structural_signal',
        'RuntimeEventAddress',
        'source_port_id',
        'target_port_id',
        'Runtime-occurrence state is authoritative')
require('apps/desktop/src-tauri/src/workspace/activity_execution.rs',
        'runtime_selections: HashMap<String, ExecutionRuntimeSelection>',
        'preview_activity_execution_runtime',
        'configure_activity_execution_runtime',
        '.set_structural_configuration(selection.structural_configuration.clone())',
        '.with_runtime_instance(runtime_instance_id)')
require('apps/desktop/src-tauri/src/workspace/state_machine_execution.rs',
        'runtime_selections: HashMap<String, ExecutionRuntimeSelection>',
        'preview_state_machine_execution_runtime',
        'configure_state_machine_execution_runtime',
        '.set_structural_configuration(selection.structural_configuration.clone())',
        '.with_runtime_instance(runtime_instance_id)')
require('apps/desktop/frontend/behavior-authoritative-renderer.js',
        'window.renderStructuralRuntimeInspector',
        'window.smpOpenStructuralRuntimeConfiguration',
        "preview_state_machine_execution_runtime",
        "configure_state_machine_execution_runtime")
require('apps/desktop/frontend/activity-rich-ui.js',
        "data-activity-execution=\"runtime\"",
        "smpOpenStructuralRuntimeConfiguration?.('activity'",
        'window.smpRefreshActivityExecution')

frontend = content('apps/desktop/frontend/behavior-authoritative-renderer.js') + content('apps/desktop/frontend/activity-rich-ui.js')
for forbidden in ('new StructuralRuntime(', 'buildStructuralRuntime(', 'routeStructuralSignal('):
    if forbidden in frontend:
        raise SystemExit(f'Frontend may not own structural runtime semantics: {forbidden}')

print('PR33 structural runtime desktop integration contract passed.')
''')

path = Path('.github/workflows/ci.yml')
text = path.read_text()
needle = '''      - name: State Machine execution integration contract
        run: python scripts/validate_state_machine_execution.py
'''
replacement = needle + '''      - name: Structural runtime integration contract
        run: python scripts/validate_structural_runtime_integration.py
'''
if text.count(needle) != 2:
    raise SystemExit(f'CI State Machine contract anchor count {text.count(needle)}')
text = text.replace(needle, replacement)
path.write_text(text)

# Add focused core qualification for preview candidates / selection serialization.
path = Path('crates/model-core/tests/pr33_structural_runtime.rs')
text = path.read_text()
if 'fn runtime_selection_preview_supports_repeated_classifier_occurrences()' not in text:
    text += r'''

#[test]
fn runtime_selection_preview_supports_repeated_classifier_occurrences() {
    let fixture = vehicle_fixture();
    let runtime = build_vehicle(&fixture);
    let paths = runtime.compatible_instance_paths(&fixture.project, fixture.sensor);
    assert_eq!(paths, vec!["vehicle.leftSensor", "vehicle.rightSensor"]);
    let selection = ExecutionRuntimeSelection {
        root_semantic_id: Some(fixture.vehicle),
        structural_configuration: StructuralRuntimeConfiguration {
            root_instance_name: Some("vehicle".into()),
            ..StructuralRuntimeConfiguration::default()
        },
        runtime_instance_path: Some("vehicle.rightSensor".into()),
    };
    let encoded = serde_json::to_string(&selection).unwrap();
    let decoded: ExecutionRuntimeSelection = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, selection);
}
'''
path.write_text(text)
