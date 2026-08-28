from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def write(path: str, text: str) -> None:
    (ROOT / path).write_text(text, encoding="utf-8")


def ensure_replace(path: str, old: str, new: str, *, all_occurrences: bool = False) -> None:
    text = read(path)
    if new in text and not all_occurrences:
        return
    if old not in text:
        if all_occurrences and new in text:
            return
        raise SystemExit(f"missing anchor in {path}: {old[:120]!r}")
    text = text.replace(old, new) if all_occurrences else text.replace(old, new, 1)
    write(path, text)


def ensure_insert_before(path: str, marker: str, insertion: str) -> None:
    text = read(path)
    if insertion.strip() in text:
        return
    if marker not in text:
        raise SystemExit(f"missing insertion marker in {path}: {marker[:120]!r}")
    write(path, text.replace(marker, insertion + marker, 1))


def ensure_insert_after(path: str, marker: str, insertion: str) -> None:
    text = read(path)
    if insertion.strip() in text:
        return
    if marker not in text:
        raise SystemExit(f"missing insertion marker in {path}: {marker[:120]!r}")
    write(path, text.replace(marker, marker + insertion, 1))


# Core: export the new runtime adapter and make the existing evaluation scope serializable.
ensure_replace(
    "crates/model-core/src/lib.rs",
    "pub mod operation_signal_sequence_execution;\npub use operation_signal_sequence_execution::*;\n",
    "pub mod operation_signal_sequence_execution;\npub use operation_signal_sequence_execution::*;\n\npub mod parametric_execution;\npub use parametric_execution::*;\n",
)
ensure_replace(
    "crates/model-core/src/parametrics.rs",
    "#[derive(Debug, Clone, PartialEq, Eq)]\npub struct ParametricEvaluationScope {",
    "#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\npub struct ParametricEvaluationScope {",
)

# Desktop Parametric authoring: expose the diagram scope helper to the runtime module.
p = "apps/desktop/src-tauri/src/workspace/parametrics.rs"
ensure_replace(
    p,
    "fn diagram_context(diagram: &BddDiagram) -> Result<ElementId, String> {",
    "pub(super) fn diagram_context(diagram: &BddDiagram) -> Result<ElementId, String> {",
)
helper = '''pub(super) fn evaluation_scope(\n    diagram: &BddDiagram,\n    project: &Project,\n) -> Result<ParametricEvaluationScope, String> {\n    Ok(ParametricEvaluationScope {\n        context_id: diagram_context(diagram)?,\n        constraint_property_ids: diagram\n            .nodes\n            .iter()\n            .filter_map(|node| {\n                let id = parse_element_id(&node.element_id).ok()?;\n                (project.element(id).ok()?.kind == ElementKind::ConstraintProperty).then_some(id)\n            })\n            .collect(),\n        value_property_ids: diagram\n            .nodes\n            .iter()\n            .filter_map(|node| {\n                let id = parse_element_id(&node.element_id).ok()?;\n                (project.element(id).ok()?.kind == ElementKind::ValueProperty).then_some(id)\n            })\n            .collect(),\n        binding_relationship_ids: diagram\n            .edges\n            .iter()\n            .map(|edge| parse_relationship_id(&edge.relationship_id))\n            .collect::<Result<Vec<_>, _>>()?,\n    })\n}\n\n'''
ensure_insert_before(p, "fn parameter_layout(\n", helper)
old_direct = '''    let report = evaluate_parametrics(&mut project, &scope).map_err(|error| error.to_string())?;\n    project.validate().map_err(|error| error.to_string())?;\n    if !report.updates.is_empty() {\n        checkpoint(&workspace, &activity, &history)?;\n        *workspace\n            .project\n            .lock()\n            .map_err(|_| "project lock poisoned")? = Some(project);\n    }\n    Ok(ParametricEvaluationSnapshot {'''
new_direct = '''    let report = evaluate_parametrics(&mut project, &scope).map_err(|error| error.to_string())?;\n    // PR35 keeps this legacy command preview-only. Runtime values are owned by\n    // the shared ExecutionSession path and never overwrite authored defaults.\n    let _ = (&workspace, &activity, &history);\n    Ok(ParametricEvaluationSnapshot {'''
ensure_replace(p, old_direct, new_direct)

# Desktop shell: register the Parametric runtime module, state, and Tauri commands without
# perturbing the large shared import list.
m = "apps/desktop/src-tauri/src/main.rs"
ensure_replace(m, "    mod parametrics;\n    mod presentation_interaction;", "    mod parametrics;\n    mod parametric_execution;\n    mod presentation_interaction;")
parametric_exports = '''    pub use parametric_execution::{\n        ParametricExecutionState, clear_parametric_executions,\n        configure_parametric_execution_runtime, evaluate_parametric_execution,\n        initialize_parametric_execution, parametric_execution_runtime_selection,\n        parametric_execution_snapshot, preview_parametric_execution_runtime,\n        reset_parametric_execution, run_parametric_execution, step_parametric_execution,\n        terminate_parametric_execution,\n    };\n'''
ensure_insert_before(m, "    pub use presentation_interaction::{\n", parametric_exports)
extra_use = '''use workspace::{\n    ParametricExecutionState, clear_parametric_executions, configure_parametric_execution_runtime,\n    evaluate_parametric_execution, initialize_parametric_execution,\n    parametric_execution_runtime_selection, parametric_execution_snapshot,\n    preview_parametric_execution_runtime, reset_parametric_execution, run_parametric_execution,\n    step_parametric_execution, terminate_parametric_execution,\n};\n'''
ensure_insert_after(m, "use serde::Serialize;\n", extra_use)
ensure_replace(
    m,
    "        .manage(SequenceExecutionState::default())\n        .manage(HistoryState::default())",
    "        .manage(SequenceExecutionState::default())\n        .manage(ParametricExecutionState::default())\n        .manage(HistoryState::default())",
)
parametric_handler = '''            parametric_execution_snapshot,\n            parametric_execution_runtime_selection,\n            preview_parametric_execution_runtime,\n            configure_parametric_execution_runtime,\n            initialize_parametric_execution,\n            evaluate_parametric_execution,\n            run_parametric_execution,\n            step_parametric_execution,\n            reset_parametric_execution,\n            terminate_parametric_execution,\n            clear_parametric_executions,\n'''
ensure_insert_after(m, "            evaluate_parametric_diagram,\n", parametric_handler)

# Shared command manifest: explicit evaluation now targets the transient runtime command.
theme = "apps/desktop/src-tauri/src/workspace/presentation_theme.rs"
ensure_replace(theme, 'rust_adapter: Some("evaluate_parametric_diagram")', 'rust_adapter: Some("evaluate_parametric_execution")')
ensure_replace(theme, 'label: "Evaluate Parametrics",', 'label: "Evaluate Runtime",')

# Reuse the PR33 structural-runtime configuration dialog for Parametric execution.
r = "apps/desktop/frontend/behavior-authoritative-renderer.js"
parametric_command_branch = '''    if (kind === 'parametric') return {\n      get: 'parametric_execution_runtime_selection',\n      preview: 'preview_parametric_execution_runtime',\n      configure: 'configure_parametric_execution_runtime',\n      label: 'Parametric',\n    };\n'''
ensure_insert_before(r, "    return {\n      get: 'state_machine_execution_runtime_selection',", parametric_command_branch)
ensure_replace(
    r,
    "Choose the structural system occurrence this behavior executes on. Rust validates and owns the resulting runtime graph.",
    "Choose the structural system occurrence this execution runs on. Rust validates and owns the resulting runtime graph.",
)
old_apply = '''        if (kind === 'activity') {\n          window.smpState.activityExecutionSnapshot = null;\n          window.smpRefreshActivityExecution?.();\n        } else {\n          window.smpState.stateMachineExecutionSnapshot = null;\n          window.smpRefreshStateMachineExecution?.();\n        }'''
new_apply = '''        if (kind === 'activity') {\n          window.smpState.activityExecutionSnapshot = null;\n          window.smpRefreshActivityExecution?.();\n        } else if (kind === 'sequence') {\n          window.smpState.sequenceExecutionSnapshot = null;\n          window.smpRefreshSequenceExecution?.();\n        } else if (kind === 'parametric') {\n          window.smpState.parametricExecutionSnapshot = null;\n          window.smpRefreshParametricExecution?.();\n        } else {\n          window.smpState.stateMachineExecutionSnapshot = null;\n          window.smpRefreshStateMachineExecution?.();\n        }'''
ensure_replace(r, old_apply, new_apply)
ensure_replace(
    r,
    "    await requireInvoke()('clear_sequence_executions');\n    Object.assign(state, { stateMachineExecutionSnapshot: null });",
    "    await requireInvoke()('clear_sequence_executions');\n    await requireInvoke()('clear_parametric_executions');\n    Object.assign(state, { stateMachineExecutionSnapshot: null, parametricExecutionSnapshot: null });",
    all_occurrences=True,
)

# Parametric frontend: thin, contextual runtime controls and presentation-only highlighting.
f = "apps/desktop/frontend/parametric-ui.js"
runtime_ui = r'''

  Object.assign(state, { parametricExecutionSnapshot: null });

  function currentParametricExecution() {
    const diagram = selectedParametricDiagram();
    const snapshot = state.parametricExecutionSnapshot;
    return snapshot && diagram && String(snapshot.context_id) === String(diagram.semantic_context_id)
      ? snapshot
      : null;
  }

  function visualizeParametricExecution() {
    const snapshot = currentParametricExecution();
    const calculated = new Set((snapshot?.updates || []).map((update) => String(update.element_id)));
    const evaluated = new Set((snapshot?.evaluated_constraint_property_ids || []).map(String));
    document.querySelectorAll('.parametric-presentation[data-element-id]').forEach((node) => {
      const id = String(node.dataset.elementId || '');
      node.classList.toggle('runtime-calculated-value', calculated.has(id));
      node.classList.toggle('runtime-evaluated-constraint', evaluated.has(id));
    });
  }

  function renderParametricExecutionPanel() {
    const host = document.querySelector('.diagram-workspace');
    const snapshot = currentParametricExecution();
    let panel = document.querySelector('.parametric-execution-panel');
    if (!host || !snapshot) {
      panel?.remove();
      return;
    }
    if (!panel) {
      panel = document.createElement('aside');
      panel.className = 'parametric-execution-panel';
      panel.dataset.workspaceOverlay = 'true';
      host.appendChild(panel);
    }
    const execution = snapshot.execution;
    const diagnostics = (execution.diagnostics || []).slice(-4);
    const trace = (execution.trace || []).slice(-6);
    const updates = snapshot.updates || [];
    panel.innerHTML = `<div class="parametric-execution-heading"><strong>${escapeHtml(execution.state)}</strong><span>${escapeHtml(snapshot.runtime_instance_path || 'model scope')}</span></div>
      <div class="parametric-execution-metrics"><span>Step ${execution.steps_executed}</span><span>${snapshot.evaluated_constraints || 0} constraint(s)</span><span>${updates.length} calculated value(s)</span></div>
      ${updates.length ? `<div class="parametric-execution-values">${updates.map((update) => `<div><b>${escapeHtml(update.element_name)}</b><span>${escapeHtml(update.display_value)}</span></div>`).join('')}</div>` : ''}
      ${diagnostics.length ? `<div class="parametric-execution-diagnostics">${diagnostics.map((item) => `<div>${escapeHtml(item.message)}</div>`).join('')}</div>` : ''}
      <div class="parametric-execution-trace">${trace.map((item) => `<div><span>${item.simulation_time}</span>${escapeHtml(item.message)}</div>`).join('')}</div>
      ${window.renderStructuralRuntimeInspector?.(snapshot) || ''}`;
  }

  function refreshParametricExecution() {
    const visible = Boolean(selectedParametricDiagram());
    let group = document.querySelector('.parametric-execution-ribbon-group');
    if (!group) {
      group = document.createElement('section');
      group.className = 'ribbon-group parametric-execution-ribbon-group';
      const controls = [
        ['runtime', '◎', 'Runtime'], ['initialize', '◇', 'Initialize'],
        ['evaluate', '▶', 'Evaluate'], ['step', '▸', 'Step'],
        ['reset', '↺', 'Reset'], ['terminate', '■', 'Terminate'],
      ];
      group.innerHTML = `<div class="ribbon-actions parametric-execution-actions">${controls.map(([command, icon, label]) => `<button class="ribbon-command" data-parametric-execution="${command}"><span class="command-icon">${icon}</span><span>${label}</span></button>`).join('')}</div><div class="ribbon-label">Parametric Execution</div>`;
      group.addEventListener('click', handleParametricExecutionCommand);
      document.querySelector('.ribbon')?.insertBefore(group, document.querySelector('.ribbon-context'));
    }
    group.hidden = !visible;
    const snapshot = currentParametricExecution();
    const current = snapshot?.execution?.state || 'Not initialized';
    group.querySelectorAll('[data-parametric-execution]').forEach((button) => {
      const command = button.dataset.parametricExecution;
      button.disabled = !visible
        || (command === 'step' && (!snapshot || ['Completed', 'Failed', 'Terminated'].includes(current)))
        || (command === 'reset' && !snapshot)
        || (command === 'terminate' && (!snapshot || ['Completed', 'Failed', 'Terminated'].includes(current)));
    });
    visualizeParametricExecution();
    renderParametricExecutionPanel();
  }

  async function invokeParametricExecution(command) {
    const diagram = selectedParametricDiagram();
    if (!diagram) throw new Error('Open a Parametric Diagram first.');
    const snapshot = await requireInvoke()(command, { diagramId: diagram.id });
    Object.assign(state, { parametricExecutionSnapshot: snapshot });
    refreshParametricExecution();
    return snapshot;
  }

  async function evaluateParametricRuntime() {
    return invokeParametricExecution('evaluate_parametric_execution');
  }

  async function handleParametricExecutionCommand(event) {
    const command = event.target.closest?.('[data-parametric-execution]')?.dataset.parametricExecution;
    if (!command) return;
    try {
      if (command === 'runtime') {
        const diagram = selectedParametricDiagram();
        await window.smpOpenStructuralRuntimeConfiguration?.('parametric', diagram.id);
        Object.assign(state, { parametricExecutionSnapshot: null });
        refreshParametricExecution();
      } else if (command === 'initialize') {
        await invokeParametricExecution('initialize_parametric_execution');
      } else if (command === 'evaluate') {
        await evaluateParametricRuntime();
      } else if (command === 'step') {
        if (!currentParametricExecution()) await invokeParametricExecution('initialize_parametric_execution');
        await invokeParametricExecution('step_parametric_execution');
      } else if (command === 'reset') {
        await invokeParametricExecution('reset_parametric_execution');
      } else if (command === 'terminate') {
        await invokeParametricExecution('terminate_parametric_execution');
      }
    } catch (error) {
      window.smpDialogs?.notify?.(error?.message || String(error), 'error');
      refreshParametricExecution();
    }
  }

  async function loadParametricExecutionSnapshot() {
    const diagram = selectedParametricDiagram();
    if (!diagram) {
      Object.assign(state, { parametricExecutionSnapshot: null });
      refreshParametricExecution();
      return;
    }
    try {
      const snapshot = await requireInvoke()('parametric_execution_snapshot', { diagramId: diagram.id });
      Object.assign(state, { parametricExecutionSnapshot: snapshot });
    } catch (_error) {
      Object.assign(state, { parametricExecutionSnapshot: null });
    }
    refreshParametricExecution();
  }

  window.smpRefreshParametricExecution = refreshParametricExecution;
  window.smpEvaluateParametricRuntime = evaluateParametricRuntime;
'''
ensure_insert_after(f, "  function selectedParametricDiagram() {\n    return (state.snapshot?.diagrams || []).find(\n      (diagram) => diagram.id === state.selectedDiagramId && diagram.family === 'parametric',\n    );\n  }\n", runtime_ui)
ensure_replace(
    f,
    "      presentation.dataset.presentationId = node.id;",
    "      presentation.dataset.presentationId = node.id;\n      presentation.dataset.elementId = property.id;",
)
old_properties = '''    panel.innerHTML = `<div class="property-heading">Parametric Diagram</div>\n      <label>Name<input value="${escapeAttr(diagram.name)}" disabled></label>\n      <label>Semantic context<input value="${escapeAttr(`${context?.name || 'Unresolved'} (${context?.kind || '?'})`)}" disabled></label>\n      <div class="parametric-evaluation-summary">Evaluation is explicit. Opening the diagram never changes engineering values.</div>\n      <button id="evaluate-parametrics" class="primary">Evaluate Parametrics</button>`;\n    $('evaluate-parametrics').onclick = async () => {\n      const result = await runCommand('Evaluating Parametrics…', () => requireInvoke()('evaluate_parametric_diagram', {\n        diagramId: diagram.id,\n      }));\n      await refresh();\n      renderStatus(`Evaluated ${result.evaluated_constraints} constraint(s); ${result.changed_values} value(s) changed.`);\n    };'''
new_properties = '''    panel.innerHTML = `<div class="property-heading">Parametric Diagram</div>\n      <label>Name<input value="${escapeAttr(diagram.name)}" disabled></label>\n      <label>Semantic context<input value="${escapeAttr(`${context?.name || 'Unresolved'} (${context?.kind || '?'})`)}" disabled></label>\n      <div class="parametric-evaluation-summary">Runtime evaluation is transient and occurrence-scoped. Opening or evaluating the diagram never rewrites authored ValueProperty defaults.</div>\n      <button id="evaluate-parametrics" class="primary">Evaluate Runtime</button>`;\n    $('evaluate-parametrics').onclick = async () => {\n      const result = await runCommand('Evaluating Parametric runtime…', () => evaluateParametricRuntime());\n      renderStatus(`Evaluated ${result.evaluated_constraints} constraint(s); ${result.updates?.length || 0} runtime value(s) calculated.`);\n    };'''
ensure_replace(f, old_properties, new_properties)
render_wrapper = '''\n  const baseParametricRenderWithRuntime = renderCanvas;\n  renderCanvas = function renderParametricCanvasWithRuntime() {\n    const result = baseParametricRenderWithRuntime();\n    queueMicrotask(loadParametricExecutionSnapshot);\n    return result;\n  };\n\n'''
ensure_insert_before(f, "  window.smpCreateParametricDiagram = createParametricDiagram;\n", render_wrapper)

# Contextual presentation only. No new JS module is introduced.
css = "apps/desktop/frontend/structural-runtime.css"
css_addition = '''\n/* PR35 Parametric runtime controls share the authoritative active-family gate. */\nbody:not([data-execution-family="parametric"]) .parametric-execution-ribbon-group,\nbody:not([data-execution-family="parametric"]) .parametric-execution-panel {\n  display: none !important;\n}\n.parametric-execution-ribbon-group {\n  display: flex !important;\n  flex-direction: column;\n  align-items: stretch;\n}\n.parametric-execution-ribbon-group .ribbon-actions {\n  display: flex !important;\n  grid-template-columns: none !important;\n  flex-wrap: nowrap;\n  gap: 4px;\n}\n.parametric-execution-ribbon-group .ribbon-label {\n  white-space: nowrap;\n}\n.parametric-execution-panel {\n  position: absolute;\n  right: 16px;\n  bottom: 16px;\n  z-index: 12;\n  width: min(440px, 40vw);\n  max-height: 46vh;\n  overflow: auto;\n  background: rgba(255, 255, 255, .96);\n  border: 1px solid #9aa6b2;\n  border-radius: 4px;\n  box-shadow: 0 5px 18px rgba(30, 42, 55, .18);\n  padding: 10px;\n  font-size: 12px;\n}\n.parametric-execution-heading,\n.parametric-execution-metrics,\n.parametric-execution-values > div {\n  display: flex;\n  justify-content: space-between;\n  gap: 8px;\n}\n.parametric-execution-values,\n.parametric-execution-diagnostics,\n.parametric-execution-trace {\n  margin-top: 6px;\n}\n.parametric-execution-trace > div {\n  display: grid;\n  grid-template-columns: 70px 1fr;\n  gap: 6px;\n  padding: 2px 0;\n}\n.parametric-presentation.runtime-calculated-value {\n  outline: 2px solid currentColor;\n  outline-offset: 2px;\n}\n.parametric-presentation.runtime-evaluated-constraint {\n  box-shadow: 0 0 0 2px rgba(0, 0, 0, .16);\n}\n'''
text = read(css)
if "PR35 Parametric runtime controls" not in text:
    write(css, text.rstrip() + "\n" + css_addition)

# Update the legacy Parametric contract for transient runtime evaluation.
v = "scripts/validate_parametric_integration.py"
ensure_replace(v, 'assert \'rust_adapter: Some("evaluate_parametric_diagram")\' in manifest', 'assert \'rust_adapter: Some("evaluate_parametric_execution")\' in manifest')
ensure_replace(v, '    "Evaluate Parametrics",', '    "Evaluate Runtime",')

# Run the new integration validator in both Linux and Windows CI jobs.
ci = ".github/workflows/ci.yml"
ci_anchor = "      - name: Parametric desktop integration contract\n        run: python scripts/validate_parametric_integration.py\n"
ci_extended = ci_anchor + "      - name: Parametric runtime integration contract\n        run: python scripts/validate_parametric_runtime_integration.py\n"
text = read(ci)
if text.count("validate_parametric_runtime_integration.py") < 2:
    if text.count(ci_anchor) != 2:
        raise SystemExit("expected Parametric CI anchor in both jobs")
    write(ci, text.replace(ci_anchor, ci_extended))

# Staging helpers must not survive the implementation commit.
for temporary in [
    ROOT / ".github/workflows/pr35-implement-runtime.yml",
    ROOT / "scripts/pr35_finish.py",
]:
    if temporary.exists():
        temporary.unlink()

print("PR35 integration wiring staged successfully")
