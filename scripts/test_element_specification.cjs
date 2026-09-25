// Execute the production Properties and history adapters with a small DOM fixture.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');
const flush = () => new Promise(resolve => setImmediate(resolve));

function fixture({ kind = 'PartProperty', choicesPromise, applyPromise, reject = false, legacy = false } = {}) {
  const fields = new Map();
  const calls = [];
  let refreshed = 0;
  let delegated = 0;
  const element = { id: 'part', external_id: 'EL-part', kind, name: 'original', type_id: 'old-type', multiplicity: '1', aggregation: 'composite', default_value: null };
  const choices = [
    { id: 'old-type', qualifiedName: 'Model::A::Unit', externalId: 'EL-old', kind: 'Block' },
    { id: 'new-type', qualifiedName: 'Model::B::Unit', externalId: 'EL-new', kind: 'Block' },
  ];
  const document = { activeElement: null, addEventListener() {} };
  class Field {
    constructor(tag, attrs, body = '') {
      this.tagName = tag.toUpperCase();
      this.value = attrs.match(/\bvalue="([^"]*)"/)?.[1] || (tag === 'textarea' ? body : '');
      this.checked = /\bchecked\b/.test(attrs);
      this.disabled = /\bdisabled\b/.test(attrs);
      this.textContent = '';
      this.innerHTML = body;
    }
    set innerHTML(html) {
      this.html = html;
      if (this.tagName === 'SELECT') this.value = html.match(/<option value="([^"]*)"/)?.[1] || '';
    }
    get innerHTML() { return this.html; }
    focus() { document.activeElement = this; }
  }
  const panel = {
    set innerHTML(html) {
      this.html = html;
      fields.clear();
      const pattern = /<(input|textarea|select|button|div)\b([^>]*\bid="([^"]+)"[^>]*)(?:>([\s\S]*?)<\/\1>|>)/g;
      for (const match of html.matchAll(pattern)) fields.set(match[3], new Field(match[1], match[2], match[4] || ''));
    },
    querySelector() { return null; },
  };
  const state = { selectedElementId: element.id, snapshot: { project: { elements: [element], relationships: [] } } };
  const invoke = async (command, args) => {
    calls.push({ command, args: args && JSON.parse(JSON.stringify(args)) });
    if (command === 'element_type_choices') return choicesPromise || choices;
    if (command === 'update_element_specification') {
      if (reject) throw new Error('Invalid multiplicity: lower bound exceeds upper bound');
      return applyPromise || true;
    }
    return null;
  };
  const context = {
    state, window: {}, document, console,
    $: id => id === 'properties' ? panel : fields.get(id),
    renderProperties: () => { delegated++; }, renderRelationshipProperties() {}, renderStatus() {},
    escapeHtml: String, escapeAttr: String, typeName: () => 'Unit',
    requireInvoke: () => invoke, runCommand: async (_, action) => action(), refresh: async () => { refreshed++; },
  };
  vm.createContext(context);
  for (const filename of [legacy ? 'bdd-completion-ui.js' : 'bdd-feature-editing.js', 'undo-redo-ui.js']) {
    let source = fs.readFileSync(path.join(__dirname, '../apps/desktop/frontend', filename), 'utf8');
    if (filename === 'bdd-completion-ui.js') {
      source = source.slice(source.indexOf('const renderStructuralProperties'), source.indexOf('async function saveProjectAsComplete'));
    }
    vm.runInContext(source, context);
  }
  context.renderProperties();
  return { fields, calls, state, context, element, panel, document, get refreshed() { return refreshed; }, get delegated() { return delegated; } };
}

test('structural usage selector dispatches explicit reference conversion once', async () => {
  const ui = fixture();
  await flush();
  assert.match(ui.panel.html, /Composite part/);
  assert.match(ui.panel.html, /Reference \(noncomposite\)/);
  assert.match(ui.panel.html, /every usage/);
  ui.fields.get('property-aggregation').value = 'none';
  await ui.fields.get('apply-element').onclick();
  const mutations = ui.calls.filter(call => call.command === 'update_element_specification');
  assert.equal(mutations.length, 1);
  assert.equal(mutations[0].args.elementId, 'part');
  assert.equal(mutations[0].args.edit.aggregation, 'none');
  assert.equal(ui.element.kind, 'PartProperty', 'only Rust may change semantic state');
});

test('legacy Apply combines name and metadata into a single Rust transaction', async () => {
  const ui = fixture({ kind: 'ValueType', legacy: true });
  ui.fields.get('property-name').value = 'renamed';
  ui.fields.get('property-documentation').value = 'updated documentation';
  ui.fields.get('property-unit').value = 'metres';
  await ui.fields.get('apply-element').onclick();
  assert.equal(ui.calls.length, 1);
  assert.equal(ui.calls[0].command, 'update_element_specification');
  assert.equal(ui.calls[0].args.edit.name, 'renamed');
  assert.equal(ui.calls[0].args.edit.documentation, 'updated documentation');
  assert.equal(ui.calls[0].args.edit.unitExternalId, 'metres');
  assert.equal(ui.refreshed, 1);
});

test('legacy rejection neither renames first nor refreshes away the draft', async () => {
  const ui = fixture({ kind: 'ValueType', legacy: true, reject: true });
  ui.fields.get('property-name').value = 'unsaved name';
  ui.fields.get('property-unit').value = 'invalid';
  await assert.rejects(ui.fields.get('apply-element').onclick(), /Invalid multiplicity/);
  assert.equal(ui.calls.length, 1);
  assert.equal(ui.calls[0].command, 'update_element_specification');
  assert.equal(ui.fields.get('property-name').value, 'unsaved name');
  assert.equal(ui.fields.get('property-unit').value, 'invalid');
  assert.equal(ui.refreshed, 0);
});

test('one Apply sends the entire draft through one Rust transaction with no frontend checkpoint', async () => {
  const ui = fixture(); await flush();
  ui.fields.get('property-name').value = 'renamed';
  ui.fields.get('property-type').value = 'new-type';
  ui.fields.get('property-multiplicity').value = '0..*';
  ui.fields.get('property-default').value = 'initial';
  await ui.fields.get('apply-element').onclick();
  const mutations = ui.calls.filter(call => call.command !== 'element_type_choices');
  assert.equal(mutations.length, 1);
  assert.equal(mutations[0].command, 'update_element_specification');
  assert.equal(mutations[0].args.elementId, 'part');
  assert.equal(mutations[0].args.edit.name, 'renamed');
  assert.equal(mutations[0].args.edit.typeId, 'new-type');
  assert.equal(mutations[0].args.edit.multiplicity, '0..*');
  assert.equal(mutations[0].args.edit.defaultValue, 'initial');
  assert.equal(ui.refreshed, 1);
});

test('Rust rejection keeps all draft fields and displays the error without refreshing', async () => {
  const ui = fixture({ reject: true }); await flush();
  ui.fields.get('property-name').value = 'draft';
  ui.fields.get('property-multiplicity').value = '2..1';
  ui.fields.get('property-default').value = 'unsaved';
  await ui.fields.get('apply-element').onclick();
  assert.equal(ui.fields.get('property-name').value, 'draft');
  assert.equal(ui.fields.get('property-multiplicity').value, '2..1');
  assert.equal(ui.fields.get('property-default').value, 'unsaved');
  assert.match(ui.fields.get('property-error').textContent, /Invalid multiplicity/);
  assert.equal(ui.document.activeElement, ui.fields.get('property-error'));
  assert.equal(ui.fields.get('apply-element').disabled, false);
  assert.equal(ui.refreshed, 0);
  assert.equal(ui.calls.some(call => call.command === 'history_checkpoint'), false);
});

test('compatible choices show qualified names and stable IDs and filter without losing selection', async () => {
  const ui = fixture(); await flush();
  const select = ui.fields.get('property-type');
  assert.match(select.innerHTML, /Model::A::Unit/);
  assert.match(select.innerHTML, /Model::B::Unit/);
  select.value = 'new-type';
  const search = ui.fields.get('property-type-search');
  search.value = 'EL-new'; search.oninput();
  assert.equal(select.value, 'new-type');
  assert.match(select.innerHTML, /EL-new/);
  assert.doesNotMatch(select.innerHTML, /EL-old/);
});

test('late type responses cannot change a newer selection or re-enable its Apply', async () => {
  let resolve;
  const ui = fixture({ choicesPromise: new Promise(done => { resolve = done; }) });
  const oldApply = ui.fields.get('apply-element');
  ui.state.selectedElementId = 'different';
  ui.panel.innerHTML = '<button id="apply-element" disabled>Apply</button>';
  const current = ui.fields.get('apply-element');
  resolve([]); await flush();
  assert.notEqual(oldApply, current);
  assert.equal(current.disabled, true);
});

test('duplicate Apply clicks submit once while a command is pending', async () => {
  let resolve;
  const ui = fixture({ applyPromise: new Promise(done => { resolve = done; }) }); await flush();
  const button = ui.fields.get('apply-element');
  const first = button.onclick();
  await button.onclick();
  assert.equal(ui.calls.filter(call => call.command === 'update_element_specification').length, 1);
  resolve(true); await first;
});

test('FullPort editor exposes an unset default but does not offer illegal conjugation', async () => {
  const ui = fixture({ kind: 'FullPort' }); await flush();
  assert.ok(ui.fields.get('property-default'));
  assert.equal(ui.fields.has('property-conjugated'), false);
  await ui.fields.get('apply-element').onclick();
  assert.equal(ui.calls.find(call => call.command === 'update_element_specification').args.edit.isConjugated, false);
});

test('type loading failures leave Apply disabled with an actionable error', async () => {
  const ui = fixture({ choicesPromise: Promise.reject(new Error('unavailable')) }); await flush();
  assert.equal(ui.fields.get('apply-element').disabled, true);
  assert.match(ui.fields.get('property-error').textContent, /Reselect this element to retry/);
});

test('requirements and test cases preserve their specialized editor', () => {
  for (const kind of ['Requirement', 'TestCase']) {
    const ui = fixture({ kind });
    assert.equal(ui.delegated, 1);
    assert.equal(ui.calls.length, 0);
  }
});

test('Reception keeps its established Signal field and submits the selected stable type ID', async () => {
  const ui = fixture({ kind: 'Reception', choicesPromise: Promise.resolve([
    { id: 'signal', qualifiedName: 'Model::Signals::Ready', externalId: 'EL-signal', kind: 'Signal' },
  ]) });
  await flush();
  const select = ui.fields.get('property-reception-signal');
  assert.ok(select);
  select.value = 'signal';
  await ui.fields.get('apply-element').onclick();
  assert.equal(ui.calls.find(call => call.command === 'update_element_specification').args.edit.typeId, 'signal');
});
