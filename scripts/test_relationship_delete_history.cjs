const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');
const frontend = path.join(__dirname, '../apps/desktop/frontend');
const source = fs.readFileSync(path.join(frontend, 'app.js'), 'utf8');
const start = source.indexOf("  $('delete-relationship').onclick = async () => {");
const end = source.indexOf('\n  };', start) + '\n  };'.length;
assert.ok(start > 0 && end > start);

test('relationship delete preserves selection on rejection and never pre-checkpoints', async () => {
  for (const reject of [false, true]) {
    const calls = [], button = {};
    let refreshes = 0;
    const context = {
      window: {}, document: { addEventListener() {} }, confirm: () => true,
      state: { selectedDiagramId: 'view', selectedRelationshipId: 'link' },
      relationship: { id: 'link', kind: 'Association' }, $: () => button,
      requireInvoke: () => async command => {
        calls.push(command);
        if (reject && command === 'delete_bdd_relationship') throw new Error('still referenced');
      },
      runCommand: async (_, operation) => operation(),
      refresh: async () => { refreshes++; }, renderStatus() {},
    };
    vm.createContext(context);
    vm.runInContext(source.slice(start, end), context);
    vm.runInContext(fs.readFileSync(path.join(frontend, 'undo-redo-ui.js'), 'utf8'), context);
    if (reject) await assert.rejects(button.onclick(), /still referenced/);
    else await button.onclick();
    assert.deepEqual(calls, ['delete_bdd_relationship']);
    assert.equal(context.state.selectedRelationshipId, reject ? 'link' : null);
    assert.equal(refreshes, reject ? 0 : 1);
  }
});
