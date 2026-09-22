const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const root = path.resolve(__dirname, '..');
const read = name => fs.readFileSync(path.join(root, 'apps/desktop/frontend', name), 'utf8');

function fixture({ promptValue = null, rejectNew = false } = {}) {
  const calls = [];
  const buttons = { 'new-project': {} };
  const state = {
    snapshot: { project: { id: 'existing-project' } },
    activitySnapshot: { repository: { activities: { unsaved: {} } }, diagrams: [{ id: 'unsaved' }] },
    selectedActivityDiagramId: 'unsaved',
    selectedActivityNodeId: 'node',
    activityTool: { kind: 'Action' },
    activityPendingFlow: { source: 'node' },
    activityExecutionSnapshot: { status: 'paused' },
    activityExecutionRunning: true,
    stateMachineExecutionSnapshot: { status: 'paused' },
    parametricExecutionSnapshot: { status: 'paused' },
    pendingRelationship: { source: 'source' },
    paletteTool: { kind: 'Block' },
  };
  const context = vm.createContext({
    state,
    window: {},
    document: { addEventListener() {}, activeElement: null },
    $: id => buttons[id],
    prompt: () => promptValue,
    alert() {},
    console,
    render() {},
    renderStatus() {},
    refreshStateMachineExecution() {},
    refresh: async () => {
      if (promptValue) state.snapshot.project = { id: 'replacement-project' };
    },
    requireInvoke: () => async command => {
      calls.push(command);
      if (command === 'new_project' && rejectNew) throw new Error('native failure');
    },
  });

  const app = read('app.js');
  const start = app.indexOf('async function runCommand(');
  const end = app.indexOf('async function openProject(', start);
  vm.runInContext(app.slice(start, end), context);
  vm.runInContext("$('new-project').onclick = createProject;", context);

  const behavior = read('behavior-authoritative-renderer.js');
  const behaviorStart = behavior.indexOf('  const newProjectWithStateMachineExecution');
  const behaviorEnd = behavior.indexOf('  // PR32_STATE_MACHINE_EXECUTION_END', behaviorStart);
  vm.runInContext(`(() => { ${behavior.slice(behaviorStart, behaviorEnd)} })()`, context);
  // Preserve the production ordering: history wraps runCommand before the final
  // New Project integrity handler wraps the assembled button chain.
  vm.runInContext(read('undo-redo-ui.js'), context);
  vm.runInContext(read('project-reset-integrity.js'), context);
  return { calls, buttons, state };
}

test('cancelled New preserves every dependent workspace and execution snapshot', async () => {
  const f = fixture();
  const before = structuredClone(f.state);
  assert.equal((await f.buttons['new-project'].onclick()).outcome, 'cancelled');
  assert.deepEqual(f.calls, []);
  assert.deepEqual(f.state, before);
});

test('native New failure preserves dependent state and does not run reset commands', async () => {
  const f = fixture({ promptValue: 'Replacement', rejectNew: true });
  const before = structuredClone(f.state);
  await assert.rejects(f.buttons['new-project'].onclick(), /native failure/);
  assert.deepEqual(f.calls, ['new_project']);
  assert.deepEqual(f.state, before);
});

test('committed New resets dependent execution and Activity state exactly once', async () => {
  const f = fixture({ promptValue: 'Replacement' });
  assert.equal((await f.buttons['new-project'].onclick()).outcome, 'committed');
  assert.deepEqual(f.calls, [
    'new_project',
    'history_reset',
    'clear_state_machine_executions',
    'clear_sequence_executions',
    'clear_parametric_executions',
    'reset_activity_workspace',
    'clear_activity_executions',
  ]);
  assert.equal(Object.keys(f.state.activitySnapshot.repository.activities).length, 0);
  assert.equal(f.state.activitySnapshot.diagrams.length, 0);
  assert.equal(f.state.selectedActivityDiagramId, null);
  assert.equal(f.state.stateMachineExecutionSnapshot, null);
});
