// Read-only audit probe against the actual frontend source at the review baseline.
// A successful run means the recorded defects were reproduced, NOT fixed.
// Run from the repository root: node docs/reviews/2026-09-22/frontend-evidence.cjs
const fs = require('node:fs');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const path = require('node:path');
const root = path.resolve(__dirname, '../../..');
const read = name => fs.readFileSync(path.join(root, 'apps/desktop/frontend', name), 'utf8');

function fixture(rejectCommand = null) {
  const calls = [];
  const buttons = Object.fromEntries(['new-project', 'open-project', 'save-project', 'save-project-as', 'update-requirement', 'property-name', 'requirement-id', 'requirement-text', 'requirement-documentation'].map(id => [id, { value: 'audit' }]));
  const state = {
    snapshot: { project: { id: 'existing-project', name: 'Existing' }, current_file: 'existing.smproj' },
    activitySnapshot: { repository: { activities: { unsaved: {} } }, diagrams: [{}] },
  };
  const context = vm.createContext({
    state, window: {}, document: { addEventListener() {}, activeElement: null },
    $: id => buttons[id], prompt: () => null, alert() {}, console,
    requireInvoke: () => async (command) => {
      calls.push(command);
      if (command === rejectCommand) throw new Error('semantic rejection');
    },
    refresh: async () => {}, render() {}, renderStatus() {}, loadActivitySnapshot: async () => {},
    refreshStateMachineExecution() {},
  });
  const app = read('app.js');
  vm.runInContext(app.slice(app.indexOf('async function runCommand('), app.indexOf('async function createPackage(')), context);
  vm.runInContext("$('new-project').onclick = createProject; $('open-project').onclick = openProject; $('save-project').onclick = saveProject; $('save-project-as').onclick = saveProjectAs;", context);
  const completion = read('bdd-completion-ui.js');
  const completeStart = completion.indexOf('async function saveProjectAsComplete(');
  const completeEnd = completion.indexOf('\nrefresh().catch(', completeStart);
  assert(completeStart >= 0 && completeEnd > completeStart, 'BDD lifecycle source changed');
  vm.runInContext(completion.slice(completeStart, completeEnd), context);
  vm.runInContext(read('behavior-command-authority.js'), context);
  const behavior = read('behavior-authoritative-renderer.js');
  const behaviorStart = behavior.indexOf('  const newProjectWithStateMachineExecution');
  const behaviorEnd = behavior.indexOf('  // PR32_STATE_MACHINE_EXECUTION_END', behaviorStart);
  assert(behaviorStart >= 0 && behaviorEnd > behaviorStart, 'Behavior lifecycle source changed');
  vm.runInContext(`(() => { ${behavior.slice(behaviorStart, behaviorEnd)} })()`, context);
  const activity = read('activity-ui.js');
  const start = activity.indexOf('  const originalNewProject');
  const end = activity.indexOf('  loadActivitySnapshot().then(render)', start);
  assert(start >= 0 && end > start, 'Activity handler source changed; update probe extraction');
  vm.runInContext(`(() => { ${activity.slice(start, end)} })()`, context);
  // Preserve the actual later overrides and their production load order.
  for (const file of ['project-open-compat.js', 'undo-redo-ui.js', 'project-reset-integrity.js']) {
    vm.runInContext(read(file), context, { filename: file });
  }
  const applyStart = app.indexOf("    $('update-requirement').onclick = async () => {");
  const applyEnd = app.indexOf('\n    };', applyStart);
  assert(applyStart >= 0 && applyEnd > applyStart, 'Requirement Apply handler changed');
  vm.runInContext(`(() => { const element = { id: 'requirement' }; ${app.slice(applyStart, applyEnd + 7)} })()`, context);
  return { calls, buttons, state, context };
}

(async () => {
  const results = [];
  {
    const f = fixture();
    await f.buttons['new-project'].onclick();
    assert(!f.calls.includes('new_project'));
    assert(f.calls.includes('reset_activity_workspace'));
    assert.equal(Object.keys(f.state.activitySnapshot.repository.activities).length, 0);
    results.push({ case: 'Cancel New Project with an existing model', observed: f.calls, defect: 'Activity reset still executes, including beneath project-reset-integrity guard' });
  }
  {
    const f = fixture();
    await f.buttons['open-project'].onclick();
    assert(!f.calls.includes('open_project_file'));
    assert(f.calls.includes('history_reset'));
    results.push({ case: 'Cancel Open Project', observed: f.calls, defect: 'Current project history is reset despite cancellation' });
  }
  {
    const f = fixture();
    await f.buttons['save-project-as'].onclick();
    assert(!f.calls.some(command => command.startsWith('save_project_file')));
    assert(f.calls.includes('save_activity_workspace'));
    results.push({ case: 'Cancel Save As', observed: f.calls, defect: 'Activity save is still requested against the previous path' });
  }
  {
    const f = fixture('update_requirement');
    await assert.rejects(f.buttons['update-requirement'].onclick(), /semantic rejection/);
    assert.deepEqual(f.calls, ['history_checkpoint', 'update_requirement']);
    results.push({ case: 'Rejected requirement Apply through actual button and history wrapper', observed: f.calls, defect: 'Frontend checkpoints before Rust transaction can reject' });
  }
  console.log(JSON.stringify({ baseline: '848b9e9650627ff819cea3d2e2cff382fa77d3af', reproduced: results }, null, 2));
})().catch(error => { console.error(error); process.exitCode = 1; });
