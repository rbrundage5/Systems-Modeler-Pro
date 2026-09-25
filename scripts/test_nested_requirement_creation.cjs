const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');

function fixture({ rejectFirst = false, cancel = false, refreshFails = false } = {}) {
  const calls = [], dialogs = [], alerts = [];
  let refreshes = 0;
  const values = { name: 'Child', requirementId: 'REQ-1.1', text: 'Child text' };
  const source = fs.readFileSync(path.join(__dirname, '../apps/desktop/frontend/app.js'), 'utf8');
  const handler = source.slice(source.indexOf('async function createNestedRequirement('), source.indexOf('async function createRequirementDiagram('));
  const context = { state: {}, document: { addEventListener() {} }, renderStatus: message => alerts.push(message),
    window: { smpDialogs: { edit: async request => { dialogs.push(request); return cancel ? null : { values }; } } },
    runCommand: async (_, action) => action(),
    requireInvoke: () => async (command, args) => {
      calls.push({ command, args });
      if (rejectFirst && calls.length === 1) throw new Error('Duplicate Requirement ID');
      return 'child-id';
    },
    refresh: async () => { refreshes++; if (refreshFails) throw new Error('Snapshot unavailable'); },
  };
  vm.createContext(context); vm.runInContext(handler, context);
  vm.runInContext(fs.readFileSync(path.join(__dirname, '../apps/desktop/frontend/undo-redo-ui.js'), 'utf8'), context);
  return { context, calls, dialogs, alerts, get refreshes() { return refreshes; } };
}

test('nested creation uses parent identity and native history exactly once', async () => {
  const ui = fixture();
  await ui.context.createNestedRequirement({ id: 'parent-id', name: 'System', requirement_id: 'REQ-1' });
  assert.equal(ui.calls.length, 1);
  assert.equal(ui.calls[0].command, 'create_requirement');
  assert.equal(ui.calls[0].args.ownerId, 'parent-id');
  assert.equal(ui.context.state.selectedElementId, 'child-id');
  assert.equal(ui.refreshes, 1);
});

test('rejected nested creation retains entered values for correction', async () => {
  const ui = fixture({ rejectFirst: true });
  await ui.context.createNestedRequirement({ id: 'parent-id', name: 'System' });
  assert.equal(ui.calls.length, 2);
  assert.equal(ui.dialogs[1].fields[0].value, 'Child');
  assert.equal(ui.dialogs[1].fields[1].value, 'REQ-1.1');
  assert.match(ui.dialogs[1].description, /Duplicate Requirement ID/);
  assert.equal(ui.refreshes, 1);
});

test('cancelled nested creation changes nothing', async () => {
  const ui = fixture({ cancel: true });
  await ui.context.createNestedRequirement({ id: 'parent-id', name: 'System' });
  assert.equal(ui.calls.length, 0);
  assert.equal(ui.refreshes, 0);
});

test('a refresh failure after creation never retries the committed mutation', async () => {
  const ui = fixture({ refreshFails: true });
  await ui.context.createNestedRequirement({ id: 'parent-id', name: 'System' });
  assert.equal(ui.calls.length, 1);
  assert.equal(ui.dialogs.length, 1);
  assert.equal(ui.context.state.selectedElementId, 'child-id');
  assert.match(ui.alerts[0], /Requirement created/);
});
