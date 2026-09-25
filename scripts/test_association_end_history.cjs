const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');
const frontend = path.join(__dirname, '../apps/desktop/frontend');
const app = fs.readFileSync(path.join(frontend, 'app.js'), 'utf8');
const start = app.indexOf('  (relationship.association_ends || []).forEach');
const end = app.indexOf("  $('delete-relationship').onclick", start);
assert.ok(start > 0 && end > start);

for (const reject of [false, true]) {
  test(`association-end Apply ${reject ? 'rejection preserves history' : 'delegates one native transaction'}`, async () => {
    const calls = [];
    let refreshes = 0;
    const fields = new Map([
      ['apply-end-0', {}], ['end-role-0', { value: 'primary' }],
      ['end-multiplicity-0', { value: '2' }],
      ['end-navigable-0', { checked: true }], ['end-aggregation-0', { value: 'composite' }],
    ]);
    const context = {
      console, window: {}, document: { addEventListener() {} }, state: {},
      relationship: { id: 'composition', association_ends: [{ id: 'part-end' }] },
      $: id => fields.get(id), runCommand: async (_, operation) => operation(),
      requireInvoke: () => async (command, args) => {
        calls.push({ command, args });
        if (reject && command === 'update_association_end') throw new Error('incompatible dependent model');
      },
      refresh: async () => { refreshes++; }, renderStatus() {},
    };
    vm.createContext(context);
    vm.runInContext(app.slice(start, end), context);
    vm.runInContext(fs.readFileSync(path.join(frontend, 'undo-redo-ui.js'), 'utf8'), context);
    if (reject) await assert.rejects(fields.get('apply-end-0').onclick(), /incompatible/);
    else await fields.get('apply-end-0').onclick();
    assert.deepEqual(calls.map(item => item.command), ['update_association_end']);
    assert.equal(calls[0].args.roleName, 'primary');
    assert.equal(calls[0].args.multiplicity, '2');
    assert.equal(refreshes, reject ? 0 : 1);
  });
}
