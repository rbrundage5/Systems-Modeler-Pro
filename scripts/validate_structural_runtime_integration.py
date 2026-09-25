from pathlib import Path

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
