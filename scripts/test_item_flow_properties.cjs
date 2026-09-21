const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');
const flush = () => new Promise(resolve => setImmediate(resolve));
const deferred = () => { let resolve; const promise = new Promise(done => { resolve = done; }); return { promise, resolve }; };
const specification = { name: 'Data', direction: 'Forward', conveyed_item_ids: ['signal'],
  source_label: 'System::external', target_label: 'System::unit / Unit::internal',
  classifiers: [{ id: 'signal', label: 'Project::Signal [signal]' }, { id: 'value', label: 'Project::Value [value]' }] };

function fixture({ load, apply } = {}) {
  const fields = new Map(), calls = [];
  let refreshes = 0, current = true;
  const document = { activeElement: null, createElement: tag => new Field(tag, '') };
  class Field {
    constructor(tag, attributes) { this.tagName = tag.toUpperCase(); this.disabled = /disabled/.test(attributes); this.value = ''; this.children = []; this.textContent = ''; }
    get selectedOptions() { return this.children.filter(option => option.selected); }
    replaceChildren() { this.children = []; }
    appendChild(child) { this.children.push(child); }
    focus() { document.activeElement = this; }
  }
  const container = {
    set innerHTML(html) {
      fields.clear();
      for (const match of html.matchAll(/<(\w+)\b([^>]*\bid="([^"]+)"[^>]*)>/g)) fields.set(match[3], new Field(match[1], match[2]));
    },
    querySelector(selector) { return fields.get(selector.slice(1)); },
  };
  const window = {};
  vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../apps/desktop/frontend/item-flow-properties.js'), 'utf8'), { window, document });
  const options = { container, projectId: 'project', diagramId: 'diagram', connectorId: 'connector',
    flows: [{ id: 'one', name: 'Data', item_flow: {} }, { id: 'two', name: 'Other', item_flow: {} }],
    isCurrent: () => current,
    invoke: async (command, args) => {
      calls.push({ command, args: JSON.parse(JSON.stringify(args)) });
      return command === 'ibd_item_flow_specification' ? (load ? load(args) : specification) : (apply ? apply(args) : true);
    }, refresh: async () => { refreshes++; } };
  const field = name => fields.get(`flow-${name}`);
  const render = () => window.smpItemFlowProperties.render(options);
  render();
  return { document, field, options, render, calls, refreshes: () => refreshes,
    leave: () => { current = false; window.smpItemFlowProperties.deactivate(); },
    submit: () => field('specification-form').onsubmit({ preventDefault() {} }) };
}

test('ItemFlow Apply sends one complete specification with multiple classifier IDs and direction', async () => {
  const ui = fixture(); await flush();
  assert.match(ui.field('endpoints').textContent, /System::external.*Unit::internal/);
  assert.match(ui.field('classifiers').children[1].textContent, /Project::Value/);
  ui.field('name').value = 'Return'; ui.field('direction').value = 'Reverse'; ui.field('classifiers').children[1].selected = true;
  await ui.submit();
  assert.equal(ui.calls.length, 2);
  assert.deepEqual(ui.calls[1], { command: 'update_ibd_item_flow_specification', args: { relationshipId: 'one',
    edit: { name: 'Return', direction: 'Reverse', conveyed_item_ids: ['signal', 'value'] } } });
  assert.equal(ui.refreshes(), 1);
});

test('rejected ItemFlow edit preserves fields, selected types and focused error across rerender', async () => {
  const ui = fixture({ apply: () => { throw new Error('ItemFlow requires a conveyed item'); } }); await flush();
  ui.field('name').value = 'Draft'; ui.field('direction').value = 'Reverse'; ui.field('classifiers').children[0].selected = false;
  await ui.submit(); assert.equal(ui.document.activeElement, ui.field('error'));
  ui.render(); await flush();
  assert.equal(ui.field('name').value, 'Draft'); assert.equal(ui.field('direction').value, 'Reverse');
  assert.equal(ui.field('classifiers').selectedOptions.length, 0);
  assert.match(ui.field('error').textContent, /requires a conveyed/); assert.equal(ui.refreshes(), 0);
});

test('late ItemFlow reads cannot replace the selected flow', async () => {
  const pending = deferred();
  const ui = fixture({ load: args => args.relationshipId === 'one' ? pending.promise : { ...specification, name: 'Second' } });
  await flush(); ui.field('selection').value = 'two'; ui.field('selection').onchange(); await flush();
  pending.resolve(specification); await flush();
  assert.equal(ui.field('name').value, 'Second'); assert.equal(ui.field('selection').value, 'two');
});

test('pending ItemFlow writes disable fields and prevent repeated Apply', async () => {
  const pending = deferred(); const ui = fixture({ apply: () => pending.promise }); await flush();
  const first = ui.submit(); await flush(); await ui.submit();
  assert.equal(ui.field('selection').disabled, true); assert.equal(ui.field('classifiers').disabled, true);
  assert.equal(ui.calls.filter(call => call.command.startsWith('update_')).length, 1);
  pending.resolve(true); await first;
});

test('ItemFlow loading failure keeps Apply disabled and Reload Saved retries', async () => {
  let fails = true; const ui = fixture({ load: () => { if (fails) throw new Error('Read failed'); return specification; } }); await flush();
  assert.equal(ui.field('apply').disabled, true); assert.equal(ui.field('error').textContent, 'Read failed');
  fails = false; ui.field('reload').onclick(); await flush(); assert.equal(ui.field('apply').disabled, false);
});

test('saved ItemFlow changes reload a clean form after undo or external refresh', async () => {
  let saved = specification;
  const ui = fixture({ load: () => saved }); await flush();
  ui.options.flows[0].name = 'Restored'; saved = { ...specification, name: 'Restored', direction: 'Reverse' };
  ui.render(); await flush();
  assert.equal(ui.field('name').value, 'Restored'); assert.equal(ui.field('direction').value, 'Reverse');
  assert.equal(ui.calls.filter(call => call.command === 'ibd_item_flow_specification').length, 2);
});
