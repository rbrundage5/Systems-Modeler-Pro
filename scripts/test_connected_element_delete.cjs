const assert = require('node:assert/strict');
const { test } = require('node:test');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

test('Delete from Model sends the usage ID once and preserves the selection on rejection', async () => {
  for (const reject of [false, true]) {
    const calls = [], errors = [], confirmations = [];
    let refreshes = 0;
    const node = () => ({ style: {}, classList: { add() {}, remove() {}, toggle() {} },
      setAttribute() {}, appendChild() {}, addEventListener() {}, querySelector() { return null; } });
    const document = { head: node(), body: node(), createElement: node,
      querySelector() { return null; }, querySelectorAll() { return []; },
      getElementById() { return null; }, addEventListener() {} };
    const window = { smpState: { snapshot: { project: { elements: [{ id: 'part', type_id: 'shared-type' }] },
      ibd_diagrams: [{ id: 'diagram', properties: [{ id: 'presentation', element_id: 'part', ports: [] }] }] } },
      smpDialogs: { notify: message => errors.push(message), confirm: async value => { confirmations.push(value); return true; } },
      smpRendererHost: { context: () => ({ diagramId: 'diagram', family: { id: 'ibd' } }), publishInteraction: async () => {} } };
    const context = { document, window, queueMicrotask() {}, renderStatus() {},
      refresh: async () => { refreshes++; }, requireInvoke: () => async (command, args) => {
        calls.push({ command, args });
        if (reject) throw new Error('Surviving behavior requires this usage');
      } };
    vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../apps/desktop/frontend/standard-editing-ui.js'), 'utf8'), context);
    window.smpStandardEditing.setSelections([{ kind: 'IbdProperty', id: 'presentation' }]);
    await window.smpStandardEditing.deleteFromModel();
    assert.equal(calls.length, 1);
    assert.equal(calls[0].command, 'delete_model_element');
    assert.equal(calls[0].args.elementId, 'part');
    assert.match(confirmations[0].description, /dependent relationships/);
    assert.equal(window.smpStandardSelections.length, reject ? 1 : 0);
    assert.equal(refreshes, reject ? 0 : 1);
    assert.equal(errors.length, reject ? 1 : 0);
  }
});
