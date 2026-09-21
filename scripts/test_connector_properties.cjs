const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');
const flush = () => new Promise(resolve => setImmediate(resolve));
const deferred = () => { let resolve; const promise = new Promise(done => { resolve = done; }); return { promise, resolve }; };
const specification = { name: 'original', kind: 'Delegation', source_presentation_id: 'external', target_presentation_id: 'internal',
  endpoints: [{ presentation_id: 'external', label: 'Boundary: Project::System::port [external]' },
    { presentation_id: 'internal', label: 'unit: Project::Unit::port [internal]' },
    { presentation_id: 'second', label: 'unit2: Project::Unit::port [second]' }] };

function fixture({ load, apply } = {}) {
  const fields = new Map(), calls = [];
  let refreshes = 0, selected = 'connector';
  const document = { activeElement: null, createElement: tag => new Field(tag, '') };
  class Field {
    constructor(tag, attributes) { this.tagName = tag.toUpperCase(); this.disabled = /disabled/.test(attributes); this.value = ''; this.children = []; this.textContent = ''; }
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
  vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../apps/desktop/frontend/connector-properties.js'), 'utf8'), { window, document });
  function render(id = 'connector') {
    selected = id;
    window.smpConnectorProperties.render({ container, projectId: 'project', diagram: { id: 'diagram' },
      relationship: { id, name: id === 'connector' ? 'original' : 'different', connector: { kind: 'Delegation' } },
      isCurrent: () => selected === id,
      invoke: async (command, args) => {
        calls.push({ command, args: JSON.parse(JSON.stringify(args)) });
        return command === 'ibd_connector_specification' ? (load ? load(args) : specification) : (apply ? apply(args) : true);
      }, refresh: async () => { refreshes++; }, route() {} });
  }
  const field = name => fields.get(`connector-${name}`);
  render();
  return { window, document, render, field, calls, refreshes: () => refreshes,
    submit: () => field('specification-form').onsubmit({ preventDefault() {} }) };
}

test('connector Apply sends one complete Rust transaction and qualified endpoint choices', async () => {
  const ui = fixture(); await flush();
  ui.field('name').value = 'Renamed'; ui.field('kind').value = 'Assembly'; ui.field('source').value = 'internal'; ui.field('target').value = 'second';
  assert.match(ui.field('source').children[0].textContent, /Project::System/);
  await ui.submit();
  assert.equal(ui.calls.length, 2);
  assert.equal(ui.calls[1].command, 'update_ibd_connector_specification');
  assert.deepEqual(ui.calls[1].args.edit, { name: 'Renamed', kind: 'Assembly', source_presentation_id: 'internal', target_presentation_id: 'second' });
  assert.equal(ui.refreshes(), 1);
});

test('rejected connector edit retains every draft field across a rerender', async () => {
  const ui = fixture({ apply: () => { throw new Error('Other IBD must present the replacement endpoint'); } }); await flush();
  ui.field('name').value = 'Draft'; ui.field('target').value = 'second'; await ui.submit();
  assert.match(ui.field('error').textContent, /must present/);
  assert.equal(ui.document.activeElement, ui.field('error'));
  assert.equal(ui.field('apply').disabled, false);
  ui.render(); await flush();
  assert.equal(ui.field('name').value, 'Draft'); assert.equal(ui.field('target').value, 'second');
  assert.equal(ui.refreshes(), 0);
});

test('late endpoint response cannot replace the newly selected connector form', async () => {
  const pending = deferred();
  const ui = fixture({ load: args => args.relationshipId === 'connector' ? pending.promise : specification });
  await flush(); ui.render('other'); await flush();
  pending.resolve({ ...specification, target_presentation_id: 'second' }); await flush();
  assert.equal(ui.field('name').value, 'different'); assert.equal(ui.field('target').value, 'internal');
});

test('pending connector Apply disables editing and suppresses repeated submission', async () => {
  const pending = deferred(); const ui = fixture({ apply: () => pending.promise }); await flush();
  const first = ui.submit(); await flush(); await ui.submit();
  assert.equal(ui.calls.filter(call => call.command.startsWith('update_')).length, 1);
  assert.equal(ui.field('name').disabled, true); assert.equal(ui.field('reload').disabled, true);
  pending.resolve(true); await first; assert.equal(ui.refreshes(), 1);
});

test('endpoint loading failure keeps Apply disabled and Reload Saved retries', async () => {
  let fail = true;
  const ui = fixture({ load: () => { if (fail) throw new Error('Read failed'); return specification; } }); await flush();
  assert.equal(ui.field('apply').disabled, true); assert.equal(ui.field('error').textContent, 'Read failed');
  fail = false; ui.field('reload').onclick(); await flush();
  assert.equal(ui.field('apply').disabled, false); assert.equal(ui.field('target').value, 'internal');
});
