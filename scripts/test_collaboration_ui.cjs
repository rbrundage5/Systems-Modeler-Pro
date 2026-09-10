// Behavioral regression checks for the actual shared-project UI, without a browser.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');

class Element {
  constructor(tag = 'div') {
    this.tag = tag;
    this.children = [];
    this.value = '';
    this.disabled = false;
    this.dataset = {};
    this.attributes = {};
    this.listeners = {};
    this.classes = new Set();
    this.classList = {
      add: name => this.classes.add(name),
      toggle: (name, enabled) => enabled ? this.classes.add(name) : this.classes.delete(name),
    };
  }
  append(...children) { this.children.push(...children); }
  replaceChildren(...children) {
    this.children = children;
    if (this.tag === 'select') this.value = children[0]?.value || '';
  }
  setAttribute(key, value) { this.attributes[key] = value; }
  removeAttribute(key) { delete this.attributes[key]; }
  addEventListener(name, listener) { this.listeners[name] = listener; }
  querySelectorAll(selector) {
    return this.children.filter(child => selector === '.collaboration-bdd-node' && child.classes.has('collaboration-bdd-node'));
  }
  setPointerCapture() {}
  releasePointerCapture() {}
  createSVGPoint() { return { x: 0, y: 0 }; }
  getScreenCTM() { return null; }
  showModal() { this.open = true; }
  close() { this.open = false; }
}
const flush = () => new Promise(resolve => setImmediate(resolve));
const event = (x = 72, y = 90) => ({ pointerId: 1, button: 0, clientX: x, clientY: y, preventDefault() {} });

async function fixture() {
  const controls = new Map();
  const selectNames = new Set(['project', 'element', 'bdd-owner', 'bdd-diagram', 'bdd-element']);
  const find = selector => {
    if (!controls.has(selector)) {
      const name = selector.slice(6, -1);
      controls.set(selector, new Element(selectNames.has(name) ? 'select' : 'div'));
    }
    return controls.get(selector);
  };
  const connect = find('[data-connect]');
  connect.elements = { server: new Element('input'), token: new Element('input') };
  const edit = find('[data-edit]');
  edit.elements = { name: new Element('input'), operation: new Element('select') };
  edit.elements.operation.value = 'CreateBlock';
  const dialog = new Element('dialog');
  dialog.querySelector = find;
  dialog.querySelectorAll = () => [...controls.values(), ...Object.values(connect.elements), ...Object.values(edit.elements)];
  const body = new Element();
  const document = {
    activeElement: null, body,
    querySelector: () => new Element(),
    createElement: tag => tag === 'dialog' ? dialog : new Element(tag),
    createElementNS: (_, tag) => new Element(tag),
  };
  const view = {
    projects: [{ id: 'project', role: 'editor' }], pending: false, needs_refresh: false,
    snapshot: {
      revision: 7,
      project: { id: 'project', name: 'Shared', root_id: 'root', elements: {
        root: { id: 'root', name: 'Model', kind: 'Model' },
        block: { id: 'block', name: 'Motor', kind: 'Block' },
      } },
      diagrams: [{ id: 'bdd', name: 'Structure', owner: 'root', nodes: [
        { id: 'node', element: 'block', x: 72, y: 90, width: 190, height: 115 },
      ] }],
    },
  };
  const calls = [];
  let handler = async () => structuredClone(view);
  const invoke = async (command, payload) => { calls.push({ command, payload }); return handler(command, payload); };
  vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../apps/desktop/frontend/collaboration-ui.js'), 'utf8'), {
    window: { __TAURI__: { core: { invoke } } }, document,
    Option: function (text, value) { return { text, value }; },
    crypto: { randomUUID: () => 'new-id' }, setInterval() {},
  });
  connect.elements.server.value = 'https://example.test';
  connect.elements.token.value = 'a'.repeat(64);
  connect.onsubmit(event());
  await flush();
  find('[data-open]').onclick();
  await flush();
  calls.length = 0;
  return { find, edit, view, calls, setHandler: value => { handler = value; }, canvas: find('[data-bdd-canvas]') };
}

test('a rejected semantic edit retains the typed intention', async () => {
  const ui = await fixture();
  ui.edit.elements.name.value = 'Intended motor';
  ui.setHandler(async command => {
    if (command === 'collaboration_edit') throw new Error('409: Refresh required');
    return structuredClone({ ...ui.view, needs_refresh: true });
  });
  ui.edit.onsubmit(event());
  await flush();
  assert.equal(ui.edit.elements.name.value, 'Intended motor');
  assert.match(ui.find('[data-message]').textContent, /409/);
});

test('a click selecting a node does not submit a geometry edit', async () => {
  const ui = await fixture();
  ui.canvas.children[0].listeners.pointerdown(event());
  ui.canvas.listeners.pointerup(event());
  await flush();
  assert.equal(ui.calls.filter(call => call.command === 'collaboration_edit').length, 0);
});

test('an in-flight refresh prevents a new canvas gesture', async () => {
  const ui = await fixture();
  let complete;
  ui.setHandler(() => new Promise(resolve => { complete = resolve; }));
  ui.find('[data-refresh]').onclick();
  const group = ui.canvas.children[0];
  group.listeners.pointerdown(event());
  ui.canvas.listeners.pointermove(event(150, 180));
  assert.equal(group.attributes.transform, 'translate(72 90)');
  ui.canvas.listeners.pointerup(event(150, 180));
  complete(structuredClone(ui.view));
  await flush();
  assert.equal(ui.calls.filter(call => call.command === 'collaboration_edit').length, 0);
});

test('a movement submits the revision and geometry of the visible gesture', async () => {
  const ui = await fixture();
  ui.canvas.children[0].listeners.pointerdown(event());
  ui.canvas.listeners.pointermove(event(150, 180));
  ui.canvas.listeners.pointerup(event(150, 180));
  await flush();
  const calls = ui.calls.filter(call => call.command === 'collaboration_edit');
  assert.equal(calls.length, 1);
  assert.equal(calls[0].payload.expectedRevision, 7);
  assert.equal(calls[0].payload.edit.UpdateBddNodeGeometry.x, 150);
  assert.equal(calls[0].payload.edit.UpdateBddNodeGeometry.y, 180);
});


test('a BDD name draft survives an explicit refresh after blur', async () => {
  const ui = await fixture();
  const name = ui.find('[data-bdd-name]');
  name.value = 'Intended structure';
  name.listeners.input();
  ui.find('[data-refresh]').onclick();
  await flush();
  assert.equal(name.value, 'Intended structure');
});

test('remote node geometry outside the initial canvas remains in view', async () => {
  const ui = await fixture();
  ui.view.snapshot.diagrams[0].nodes[0].y = 890;
  ui.find('[data-refresh]').onclick();
  await flush();
  const bounds = ui.canvas.attributes.viewBox.split(' ').map(Number);
  assert.ok(bounds[1] + bounds[3] >= 890 + 115);
});
