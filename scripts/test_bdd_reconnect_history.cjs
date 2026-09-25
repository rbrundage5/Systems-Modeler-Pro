const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');
const frontend = path.join(__dirname, '../apps/desktop/frontend');
const app = fs.readFileSync(path.join(frontend, 'app.js'), 'utf8');
const start = app.indexOf('  const reconnect = async (side) => {');
const end = app.indexOf('  (relationship.association_ends || [])', start);
assert.ok(start > 0 && end > start);

function fixture(reject = false, kind = 'Association') {
  const calls = [];
  const fields = new Map(['apply-source', 'apply-target', 'relationship-source', 'relationship-target'].map(id => [id, { value: 'replacement' }]));
  let refreshes = 0;
  const invoke = async (command, args) => {
    calls.push({ command, args });
    if (reject && command.startsWith('reconnect_')) throw new Error('invalid endpoint');
  };
  const context = {
    console, window: {}, document: { addEventListener() {} },
    state: { selectedDiagramId: 'view' }, relationship: { id: 'association', kind },
    TRACEABILITY_KINDS: new Set(['Copy']), $: id => fields.get(id),
    requireInvoke: () => invoke, runCommand: async (_, operation) => operation(),
    refresh: async () => { refreshes++; }, renderStatus() {},
  };
  vm.createContext(context);
  vm.runInContext(app.slice(start, end), context);
  vm.runInContext(fs.readFileSync(path.join(frontend, 'undo-redo-ui.js'), 'utf8'), context);
  return { calls, fields, get refreshes() { return refreshes; } };
}

test('BDD reconnect invokes its native transaction without a frontend history checkpoint', async () => {
  const ui = fixture();
  await ui.fields.get('apply-target').onclick();
  assert.equal(ui.calls.length, 1);
  assert.equal(ui.calls[0].command, 'reconnect_bdd_relationship');
  assert.equal(ui.calls[0].args.side, 'target');
  assert.equal(ui.refreshes, 1);
});

test('rejected BDD reconnect never checkpoints or refreshes', async () => {
  const ui = fixture(true);
  await assert.rejects(ui.fields.get('apply-source').onclick(), /invalid endpoint/);
  assert.deepEqual(ui.calls.map(call => call.command), ['reconnect_bdd_relationship']);
  assert.equal(ui.refreshes, 0);
});

test('Requirement reconnect relies on the native transaction on success and rejection', async () => {
  for (const reject of [false, true]) {
    const ui = fixture(reject, 'Copy');
    if (reject) await assert.rejects(ui.fields.get('apply-target').onclick(), /invalid endpoint/);
    else await ui.fields.get('apply-target').onclick();
    assert.deepEqual(ui.calls.map(call => call.command), ['reconnect_traceability_relationship']);
    assert.equal(ui.refreshes, reject ? 0 : 1);
  }
});
