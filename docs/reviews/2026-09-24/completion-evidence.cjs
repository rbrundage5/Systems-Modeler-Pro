// Audit witness, not a passing-product regression suite. These assertions
// reproduce known gaps at f3e45cc0d1bf5ff40001afe75ac03fc1e4bfa8c7.
// Production frontend code runs in a VM; native IPC and rendering are mocked.
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const assert = require('node:assert/strict');

const root = path.resolve(__dirname, '../../..');
const read = name => fs.readFileSync(path.join(root, 'apps/desktop/frontend', name), 'utf8');

function fixture({ failSnapshot = false } = {}) {
  const calls = [];
  const buttons = { 'open-project': {} };
  const history = { undo: ['prior-project'], redo: ['redo-entry'] };
  const state = { snapshot: { project: { id: 'prior-project' }, diagrams: [] } };
  const context = vm.createContext({
    state, window: {}, console,
    document: { addEventListener() {}, activeElement: null },
    $: id => buttons[id], prompt: () => '/models/replacement.smproj',
    alert() {}, renderStatus() {}, render() {},
    refresh: async () => {}, selectDiagram: async () => {},
    requireInvoke: () => async (command, args) => {
      calls.push(command);
      if (command === 'history_checkpoint') {
        // Mirrors history.rs::commit_snapshot. This is a native mock.
        history.undo.push(state.snapshot.project.id);
        history.redo = [];
      }
      if (command === 'history_reset') history.undo = history.redo = [];
      if (command === 'open_project_file_complete') {
        state.snapshot = { project: { id: 'replacement-project' }, diagrams: [], current_file: args.path };
        return args.path;
      }
      if (command === 'activity_snapshot') {
        if (failSnapshot) throw new Error('injected post-publication snapshot failure');
        return { repository: { activities: {} }, diagrams: [] };
      }
    },
  });
  const app = read('app.js');
  const start = app.indexOf('async function runCommand(');
  const end = app.indexOf('async function createProject(', start);
  assert(start >= 0 && end > start, 'production runCommand boundaries changed');
  vm.runInContext(app.slice(start, end), context);
  vm.runInContext(read('project-open-compat.js'), context);
  vm.runInContext(read('undo-redo-ui.js'), context);
  return { context, calls, history, state, buttons };
}

(async () => {
  const rejected = fixture();
  await assert.rejects(vm.runInContext(
    "runCommand('Reconnecting source…', async () => { throw new Error('rejected semantic edit'); })",
    rejected.context,
  ), /rejected semantic edit/);
  assert.deepEqual(rejected.calls, ['history_checkpoint']);
  assert.deepEqual(rejected.history.redo, []);
  assert.equal(rejected.history.undo.length, 2);
  console.log('REPRODUCED TC-03: rejected reconnect requests a checkpoint before native mutation; mocked redo is cleared.');

  const opened = fixture({ failSnapshot: true });
  await assert.rejects(opened.buttons['open-project'].onclick(), /post-publication snapshot failure/);
  assert.equal(opened.state.snapshot.project.id, 'replacement-project');
  assert.deepEqual(opened.calls, ['open_project_file_complete', 'activity_snapshot']);
  assert.deepEqual(opened.history.undo, ['prior-project']);
  assert.deepEqual(opened.history.redo, ['redo-entry']);
  console.log('REPRODUCED TC-02: post-publication Open refresh failure bypasses history_reset; prior history remains in the mock.');
  console.log('2 audit gaps reproduced; native fault injection and installed UI acceptance are still required.');
})().catch(error => { console.error(error); process.exitCode = 1; });
