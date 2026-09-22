const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const root = path.resolve(__dirname, '..');
const read = name => fs.readFileSync(path.join(root, 'apps/desktop/frontend', name), 'utf8');

function fixture({ promptValue = null, rejectSave = false } = {}) {
  const calls = [];
  const buttons = Object.fromEntries(['open-project', 'save-project', 'save-project-as'].map(id => [id, {}]));
  const state = {
    snapshot: { project: { id: 'project', name: 'Existing' }, current_file: 'existing.smproj' },
    activitySnapshot: { repository: { activities: { unsaved: {} } }, diagrams: [{ id: 'activity' }] },
  };
  const context = vm.createContext({
    state,
    window: {},
    document: { addEventListener() {} },
    $: id => buttons[id],
    prompt: () => promptValue,
    alert() {},
    console,
    render() {},
    renderStatus() {},
    refresh: async () => {},
    loadActivitySnapshot: async () => {},
    runCommand: async (_label, operation) => operation(),
    requireInvoke: () => async (command, args) => {
      calls.push({ command, args });
      if (command === 'save_project_file_complete' && rejectSave) throw new Error('save failed');
      if (command === 'save_project_file_complete') state.snapshot.current_file = args.path;
    },
  });

  const completion = read('bdd-completion-ui.js');
  const start = completion.indexOf('async function saveProjectAsComplete(');
  const end = completion.indexOf('\nasync function openProjectComplete(', start);
  vm.runInContext(completion.slice(start, end), context);
  vm.runInContext("$('save-project-as').onclick = saveProjectAsComplete;", context);

  const activity = read('activity-ui.js');
  const activityStart = activity.indexOf("  const originalOpenProject = $('open-project')?.onclick;");
  const activityEnd = activity.indexOf('\n  loadActivitySnapshot().then(render)', activityStart);
  vm.runInContext(`(() => { ${activity.slice(activityStart, activityEnd)} })()`, context);
  return { buttons, calls, state };
}

test('cancelled Save As issues no core or Activity persistence command', async () => {
  const f = fixture();
  const before = structuredClone(f.state);
  assert.equal((await f.buttons['save-project-as'].onclick()).outcome, 'cancelled');
  assert.deepEqual(f.calls, []);
  assert.deepEqual(f.state, before);
});

test('failed Save As does not write Activity state to the previous file', async () => {
  const f = fixture({ promptValue: 'replacement.smproj', rejectSave: true });
  const before = structuredClone(f.state);
  await assert.rejects(f.buttons['save-project-as'].onclick(), /save failed/);
  assert.deepEqual(f.calls.map(call => call.command), ['save_project_file_complete']);
  assert.deepEqual(f.state, before);
});

test('committed Save As writes Activity state to the selected file', async () => {
  const f = fixture({ promptValue: 'replacement.smproj' });
  assert.equal((await f.buttons['save-project-as'].onclick()).outcome, 'committed');
  assert.deepEqual(f.calls.map(call => call.command), [
    'save_project_file_complete',
    'save_activity_workspace',
  ]);
  assert.equal(f.calls[1].args.path, 'replacement.smproj');
});
