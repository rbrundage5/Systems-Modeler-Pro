// Production occurrence controller contracts; Rust owns allocation and validation.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const { test } = require('node:test');

function fixture({ answers = [], apply, load } = {}) {
  const dialogs = [], calls = [], notices = [];
  let current = true, refreshes = 0;
  const window = { smpDialogs: {
    edit: async spec => { dialogs.push(spec); return answers.shift()?.(spec) ?? null; },
    notify: message => notices.push(message),
  } };
  vm.runInNewContext(fs.readFileSync(`${__dirname}/../apps/desktop/frontend/ibd-occurrence-ui.js`, 'utf8'), { window });
  const options = { diagramId: 'view', presentationId: 'occurrence', isCurrent: () => current,
    invoke: async (command, args) => { calls.push({ command, args }); return load ? load() : {
      groups: '', suggested_groups: 'unit1 = 1\nunit2 = 1', definition_id: 'shared-property', coverage: { constraint: '4', displayed: 2, status: 'Partial population view' },
    }; },
    commit: async (command, args) => { calls.push({ command, args }); return apply ? apply(command, args) : 'created'; },
    refresh: async () => { refreshes++; },
  };
  return { api: window.smpIbdOccurrences, options, dialogs, calls, notices, refreshes: () => refreshes, leave: () => { current = false; } };
}

test('occurrence editor sends aliases and counts to one native transaction, preserving a rejected draft', async () => {
  let attempt = 0;
  const ui = fixture({ answers: [
    () => ({ values: { groups: 'north = 5' } }),
    spec => { assert.equal(spec.fields[0].value, 'north = 5'); assert.match(spec.description, /exceeds/); return { values: { groups: 'north = 2\nsouth = 2' } }; },
  ], apply: () => { if (!attempt++) throw new Error('Allocation exceeds definition [4]'); } });
  await ui.api.edit(ui.options);
  assert.equal(ui.dialogs[0].fields[0].multiline, true);
  assert.equal(ui.dialogs[0].fields[0].value, 'unit1 = 1\nunit2 = 1');
  assert.equal(ui.calls[2].command, 'set_ibd_occurrence_groups');
  assert.equal(ui.calls[2].args.presentationId, 'occurrence');
  assert.equal(ui.calls[2].args.groups, 'north = 2\nsouth = 2');
  assert.equal(ui.refreshes(), 1);
});

test('cancel and a diagram switch while editing produce no mutation', async () => {
  const ui = fixture(); await ui.api.edit(ui.options);
  assert.equal(ui.calls.length, 1);
  const other = fixture({ answers: [() => { other.leave(); return { values: { groups: 'late = 1' } }; }] });
  await other.api.edit(other.options); assert.equal(other.calls.length, 1);
});

test('compact view dispatches null allocation without editing a semantic property', async () => {
  const ui = fixture(); await ui.api.compact(ui.options);
  assert.equal(ui.calls.length, 1);
  assert.equal(ui.calls[0].command, 'set_ibd_occurrence_groups');
  assert.equal(ui.calls[0].args.groups, null);
  assert.equal(ui.refreshes(), 1);
});

test('repeat-drop prompts for a new alias and keeps the stable property and containing occurrence IDs', async () => {
  const ui = fixture({ answers: [() => ({ values: { name: 'channelB', count: '1' } })] });
  const id = await ui.api.append(ui.options, { element_id: 'shared-property', parent_presentation_id: 'second-group', x: 140, y: 160 });
  assert.equal(id, 'created'); assert.equal(ui.dialogs[0].fields[0].label, 'Occurrence display name');
  assert.deepEqual(JSON.parse(JSON.stringify(ui.calls[0])), { command: 'append_ibd_occurrence', args: { diagramId: 'view', request: {
    element_id: 'shared-property', parent_presentation_id: 'second-group', x: 140, y: 160, name: 'channelB', count: '1',
  } } });
});

test('repeat-drop validation retains both inputs and cancellation releases the controller', async () => {
  const ui = fixture({ answers: [() => ({ values: { name: 'extra', count: '5' } }), spec => {
    assert.equal(spec.fields[0].value, 'extra'); assert.equal(spec.fields[1].value, '5'); return null;
  }], apply: command => { if (command === 'append_ibd_occurrence') throw new Error('Exceeds [4]'); } });
  await ui.api.append(ui.options, { element_id: 'shared-property' });
  assert.equal(ui.refreshes(), 0);
  await ui.api.compact(ui.options); assert.equal(ui.refreshes(), 1);
});

test('coverage renders the native partial report per containing instance', async () => {
  const ui = fixture(), target = {};
  await ui.api.coverage(target, ui.options);
  assert.match(target.textContent, /Partial population view: 2 displayed/);
  assert.match(target.textContent, /definition \[4\]. Counts apply per enclosing instance/);
});

test('an in-flight edit suppresses duplicate submissions', async () => {
  let release;
  const ui = fixture({ answers: [() => ({ values: { groups: 'first = 1' } })], apply: () => new Promise(resolve => { release = resolve; }) });
  const first = ui.api.edit(ui.options);
  await new Promise(resolve => setImmediate(resolve));
  await ui.api.edit(ui.options); await ui.api.compact(ui.options);
  assert.equal(ui.calls.length, 2);
  release(); await first;
});
