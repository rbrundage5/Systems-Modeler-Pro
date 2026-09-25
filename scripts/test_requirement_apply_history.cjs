const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');
const frontend = path.join(__dirname, '../apps/desktop/frontend');
const app = fs.readFileSync(path.join(frontend, 'app.js'), 'utf8');
const start = app.indexOf("    $('update-requirement').onclick = async () => {");
const end = app.indexOf('    return;', start);
assert.ok(start > 0 && end > start);
for (const reject of [false, true]) {
  test(`Requirement Apply ${reject ? 'rejects without consuming history or draft' : 'uses only native history'}`, async () => {
    const fields = new Map(['update-requirement', 'property-name', 'requirement-id', 'requirement-text', 'requirement-documentation'].map(id => [id, { value: `draft-${id}` }]));
    const calls = [];
    let refreshed = 0;
    const context = {
      console, window: {}, document: { addEventListener() {} }, state: {},
      element: { id: 'requirement' }, $: id => fields.get(id),
      runCommand: async (_, operation) => operation(),
      requireInvoke: () => async (command, args) => {
        calls.push({ command, args });
        if (reject && command === 'update_requirement') throw new Error('duplicate requirement ID');
      },
      refresh: async () => { refreshed++; }, renderStatus() {},
    };
    vm.createContext(context);
    vm.runInContext(app.slice(start, end), context);
    vm.runInContext(fs.readFileSync(path.join(frontend, 'undo-redo-ui.js'), 'utf8'), context);
    if (reject) await assert.rejects(fields.get('update-requirement').onclick(), /duplicate/);
    else await fields.get('update-requirement').onclick();
    assert.deepEqual(calls.map(call => call.command), ['update_requirement']);
    assert.equal(calls[0].args.details.elementId, 'requirement');
    assert.equal(refreshed, reject ? 0 : 1);
    assert.equal(fields.get('requirement-text').value, 'draft-requirement-text');
  });
}
