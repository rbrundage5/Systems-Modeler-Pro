const assert = require('node:assert/strict');
const { test } = require('node:test');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

function clipboardFixture(family, reject = false) {
  const calls = [], errors = [];
  let refreshes = 0;
  const node = () => ({ style: {}, classList: { add() {}, remove() {}, toggle() {} },
    setAttribute() {}, appendChild() {}, addEventListener() {}, querySelector() { return null; } });
  const document = { head: node(), body: node(), createElement: node,
    querySelector() { return null; }, querySelectorAll() { return []; },
    getElementById() { return null; }, addEventListener() {} };
  const frame = { x: 80, y: 70, width: 800, height: 600, manuallySized: true };
  const window = { smpState: {}, smpDialogs: { notify: (message) => errors.push(message) },
    smpRendererHost: { context: () => ({ diagramId: 'diagram', family: { id: family } }),
      frameGeometry: () => frame, publishInteraction: async () => {} } };
  const context = { document, window, queueMicrotask() {}, renderStatus() {},
    refresh: async () => { refreshes++; },
    requireInvoke: () => async (command, args) => {
      calls.push({ command, args });
      if (reject) throw new Error('Port parent is missing');
      return { changed: 1, selections: [{ kind: 'IbdPort', id: 'pasted' }] };
    } };
  vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../apps/desktop/frontend/standard-editing-ui.js'), 'utf8'), context);
  return { window, frame, calls, errors, refreshes: () => refreshes };
}

test('IBD paste supplies the visible frame to Rust without computing geometry', async () => {
  const ui = clipboardFixture('ibd');
  assert.equal(await ui.window.smpStandardEditing.run('paste'), true);
  assert.equal(ui.calls[0].command, 'paste_selection');
  assert.deepEqual(ui.calls[0].args.framePreference, ui.frame);
  assert.equal(ui.calls[0].args.diagramId, 'diagram');
  assert.equal(ui.window.smpStandardSelections[0].id, 'pasted');
});

test('other families and copy retain their existing command arguments', async () => {
  for (const family of ['bdd', 'requirement', 'package', 'use-case', 'parametric', 'activity', 'state-machine', 'sequence']) {
    const ui = clipboardFixture(family);
    await ui.window.smpStandardEditing.run('paste');
    assert.equal(Object.hasOwn(ui.calls[0].args, 'framePreference'), false);
  }
  const ui = clipboardFixture('ibd');
  ui.window.smpStandardEditing.setSelections([{ kind: 'IbdPort', id: 'original' }]);
  await ui.window.smpStandardEditing.run('copy');
  assert.equal(Object.hasOwn(ui.calls[0].args, 'framePreference'), false);
});

test('rejected paste retains selection and reports the Rust diagnostic', async () => {
  const ui = clipboardFixture('ibd', true);
  ui.window.smpStandardEditing.setSelections([{ kind: 'IbdPort', id: 'original' }]);
  assert.equal(await ui.window.smpStandardEditing.run('paste'), false);
  assert.equal(ui.window.smpStandardSelections[0].id, 'original');
  assert.equal(ui.refreshes(), 0);
  assert.deepEqual(ui.errors, ['Port parent is missing']);
});
