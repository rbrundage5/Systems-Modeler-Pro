const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const root = path.resolve(__dirname, '..');
const read = name => fs.readFileSync(path.join(root, 'apps/desktop/frontend', name), 'utf8');

function fixture({ promptValue = null, rejectOpen = false, failSnapshot = false } = {}) {
  const calls = [];
  const history = { undo: ['old'], redo: ['old-redo'] };
  const buttons = { 'open-project': {} };
  const state = {
    snapshot: {
      project: { id: 'existing-project', name: 'Existing project' },
      current_file: 'existing.smproj',
      diagrams: [],
    },
    activitySnapshot: { repository: { activities: { unsaved: {} } }, diagrams: [{ id: 'activity' }] },
    selectedDiagramId: 'diagram',
    selectedActivityDiagramId: 'activity',
    selectedActivityNodeId: 'node',
    selectedBehaviorDiagramId: 'behavior',
    pendingRelationship: { source: 'source' },
    activityPendingFlow: { source: 'node' },
    activityTool: { kind: 'Action' },
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
    selectDiagram: async () => {},
    refresh: async () => {},
    requireInvoke: () => async (command, args) => {
      calls.push({ command, args });
      if (command === 'open_project_file_complete') {
        if (rejectOpen) throw new Error('malformed project');
        history.undo = [];
        history.redo = [];
        state.snapshot = {
          project: { id: 'opened-project', name: 'Opened project' },
          current_file: args.path,
          diagrams: [],
        };
        return args.path;
      }
      if (command === 'activity_snapshot') {
        if (failSnapshot) throw new Error('snapshot unavailable');
        return { repository: { activities: {} }, diagrams: [] };
      }
      return null;
    },
  });

  vm.runInContext(read('project-open-compat.js'), context);
  const undo = read('undo-redo-ui.js');
  vm.runInContext(`
    async function runCommand(label, operation) {
      try { return await operation(); } catch (error) { throw error; }
    }
    ${undo}
  `, context);
  return { buttons, calls, state, history };
}

test('cancelled Open preserves the session and both history stacks', async () => {
  const f = fixture();
  const before = structuredClone(f.state);
  assert.equal((await f.buttons['open-project'].onclick()).outcome, 'cancelled');
  assert.deepEqual(f.calls, []);
  assert.deepEqual(f.state, before);
});

test('failed Open does not reset history or clear frontend session state', async () => {
  const f = fixture({ promptValue: 'broken.smproj', rejectOpen: true });
  const before = structuredClone(f.state);
  await assert.rejects(f.buttons['open-project'].onclick(), /malformed project/);
  assert.deepEqual(f.calls.map(call => call.command), ['open_project_file_complete']);
  assert.deepEqual(f.state, before);
});

test('committed Open relies on native history retirement without a second reset', async () => {
  const f = fixture({ promptValue: '/models/opened.smproj' });
  assert.equal((await f.buttons['open-project'].onclick()).outcome, 'committed');
  assert.deepEqual(f.calls.map(call => call.command), [
    'open_project_file_complete',
    'activity_snapshot',
    'activity_snapshot',
  ]);
  assert.equal(f.state.snapshot.project.id, 'opened-project');
  assert.deepEqual(f.history, { undo: [], redo: [] });
});

test('post-publication snapshot failure cannot retain the previous project history', async () => {
  const f = fixture({ promptValue: '/models/opened.smproj', failSnapshot: true });
  await assert.rejects(f.buttons['open-project'].onclick(), /snapshot unavailable/);
  assert.equal(f.state.snapshot.project.id, 'opened-project');
  assert.deepEqual(f.history, { undo: [], redo: [] });
  assert.deepEqual(f.calls.map(call => call.command), ['open_project_file_complete', 'activity_snapshot']);
});
