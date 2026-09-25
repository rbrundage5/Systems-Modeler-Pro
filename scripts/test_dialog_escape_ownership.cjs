// Simulated-DOM regression for the production shared dialog/workspace event handlers.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');

// Shared presentation cancellation must also preserve workspace/model state.
require('./test_cancelled_presentation_gesture.cjs');

class ClassList {
  constructor(owner) { this.owner = owner; this.values = new Set(); }
  add(...names) { names.forEach(name => this.values.add(name)); }
  remove(...names) { names.forEach(name => this.values.delete(name)); }
  contains(name) { return this.values.has(name); }
  toggle(name, force) {
    const enabled = force === undefined ? !this.contains(name) : force;
    enabled ? this.add(name) : this.remove(name);
    return enabled;
  }
}

function matches(element, selector) {
  if (selector.startsWith('.')) return element.classList.contains(selector.slice(1));
  if (selector.startsWith('[')) {
    const name = selector.slice(1, -1).split('=')[0];
    return Object.hasOwn(element.attributes, name);
  }
  return element.tagName.toLowerCase() === selector.toLowerCase();
}

class Element {
  constructor(document, tag = 'div') {
    this.ownerDocument = document; this.tagName = tag.toUpperCase(); this.children = [];
    this.parentElement = null; this.attributes = {}; this.dataset = {}; this.style = { setProperty() {} };
    this.classList = new ClassList(this); this.listeners = new Map(); this.value = ''; this.options = []; this.isConnected = true;
  }
  set className(value) { this.classList.values = new Set(value.split(/\s+/).filter(Boolean)); }
  get className() { return [...this.classList.values].join(' '); }
  set innerHTML(value) {
    this.children = [];
    if (value.includes('sysml-frame-label')) {
      const header = this.ownerDocument.createElement('header'); header.className = 'sysml-frame-label';
      header.appendChild(this.ownerDocument.createElement('span')); this.appendChild(header);
      const handle = this.ownerDocument.createElement('button'); handle.className = 'sysml-frame-resize'; this.appendChild(handle);
      return;
    }
    if (!value.includes('application-dialog')) return;
    const dialog = this.ownerDocument.createElement('section'); dialog.className = 'application-dialog'; dialog.setAttribute('role', 'dialog');
    for (const [tag, className, attribute] of [
      ['h2', '', ''], ['p', 'dialog-description', ''], ['div', 'dialog-fields', ''],
      ['div', 'dialog-candidates', ''], ['button', '', 'data-cancel'], ['button', '', 'data-confirm'],
    ]) {
      const child = this.ownerDocument.createElement(tag); child.className = className;
      if (attribute) child.setAttribute(attribute, ''); dialog.appendChild(child);
    }
    this.appendChild(dialog);
  }
  append(...children) { children.forEach(child => this.appendChild(child)); }
  appendChild(child) { child.parentElement = this; this.children.push(child); return child; }
  insertBefore(child, before) { child.parentElement = this; this.children.splice(Math.max(0, this.children.indexOf(before)), 0, child); return child; }
  remove() { if (this.parentElement) this.parentElement.children = this.parentElement.children.filter(child => child !== this); this.isConnected = false; }
  setAttribute(name, value) { this.attributes[name] = value; }
  getAttribute(name) { return this.attributes[name]; }
  addEventListener(type, listener) { const list = this.listeners.get(type) || []; list.push(listener); this.listeners.set(type, list); }
  removeEventListener(type, listener) { this.listeners.set(type, (this.listeners.get(type) || []).filter(item => item !== listener)); }
  querySelector(selector) { return this.querySelectorAll(selector)[0] || null; }
  querySelectorAll(selector) {
    const choices = selector.split(',').map(item => item.trim()); const found = [];
    const visit = element => { for (const child of element.children) { if (choices.some(choice => matches(child, choice))) found.push(child); visit(child); } };
    visit(this); return found;
  }
  closest(selector) { for (let current = this; current; current = current.parentElement) if (selector.split(',').some(choice => matches(current, choice.trim()))) return current; return null; }
  focus() { this.ownerDocument.activeElement = this; }
  reportValidity() { return true; }
  dispatchEvent() { return true; }
  scrollTo() {}
  setPointerCapture() {}
}

class EventTarget {
  constructor() { this.listeners = new Map(); }
  addEventListener(type, listener) { const list = this.listeners.get(type) || []; list.push(listener); this.listeners.set(type, list); }
  removeEventListener(type, listener) { this.listeners.set(type, (this.listeners.get(type) || []).filter(item => item !== listener)); }
}

const flush = () => new Promise(resolve => setImmediate(resolve));

async function fixture(options = {}) {
  const document = new EventTarget(); document.elements = new Map();
  document.createElement = tag => new Element(document, tag);
  document.body = document.createElement('body'); document.head = document.createElement('head');
  document.activeElement = null;
  document.getElementById = id => {
    if (!document.elements.has(id)) document.elements.set(id, document.createElement(id === 'canvas' ? 'main' : 'div'));
    return document.elements.get(id);
  };
  document.querySelector = selector => document.getElementById(selector);
  document.dispatchEvent = () => true;
  const canvas = document.getElementById('canvas');
  if (options.frame) {
    const root = document.createElement('div');
    root.offsetWidth = root.scrollWidth = 800; root.offsetHeight = root.scrollHeight = 600;
    canvas.appendChild(root);
  }
  const window = new EventTarget();
  window.alert = () => {};
  const calls = [];
  const requests = [];
  const invoke = async (command, args) => {
    calls.push(command); requests.push({ command, args });
    if (command === 'set_diagram_frame_preference') return options.saveFrame?.(args, window);
    if (command === 'diagram_family_registry') return [];
    if (command === 'semantic_presentation_stylesheet') return '';
    if (command === 'active_diagram_command_manifest') return [];
    if (command === 'get_panel_preferences') return { repositoryWidth: 200, elementsWidth: 200, propertiesWidth: 200, repositoryVisible: true, elementsVisible: true, propertiesVisible: true };
    if (command === 'activate_diagram') return { context: { diagramId: 'diagram', family: { id: options.family || 'bdd', displayName: 'BDD', accessibilityName: 'BDD', rendererId: 'bdd' }, name: 'Structure', frameLabel: 'bdd Structure' }, interaction: { revision: 1 }, commands: [] };
    if (command === 'get_viewport_preference') return { zoom: 1, panX: 0, panY: 0, gridVisible: true };
    if (command === 'get_diagram_frame_preference') return options.frame || null;
    if (command === 'clear_workspace_interaction') return { revision: 2 };
    return null;
  };
  window.__TAURI__ = { core: { invoke } };
  window.smpState = { selectedElementId: 'node-1', paletteTool: { id: 'block' }, snapshot: { ibd_diagrams: options.family === 'ibd' ? [{id:'diagram', context_frame:options.legacy ? null : options.frame}] : [] } };
  const context = {
    window, document, console, setTimeout, clearTimeout, queueMicrotask,
    MutationObserver: class { observe() {} }, CustomEvent: class {},
    getComputedStyle: () => ({ getPropertyValue: () => '200' }),
  };
  vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../apps/desktop/frontend/shared-dialogs.js'), 'utf8'), context);
  vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../apps/desktop/frontend/shared-workspace.js'), 'utf8'), context);
  await window.smpRendererHost.activate({ diagramId: 'diagram', familyId: 'bdd', name: 'Structure' });
  await flush(); calls.length = 0;

  function keydown(target) {
    const event = { key: 'Escape', code: 'Escape', target, defaultPrevented: false, immediateStopped: false,
      preventDefault() { this.defaultPrevented = true; }, stopImmediatePropagation() { this.immediateStopped = true; } };
    for (const listener of window.listeners.get('keydown') || []) { listener(event); if (event.immediateStopped) return event; }
    for (const listener of [...(document.listeners.get('keydown') || [])]) { listener(event); if (event.immediateStopped) break; }
    return event;
  }
  return { window, document, canvas, calls, requests, keydown };
}

test('dialog owns Escape without changing workspace interaction or authored history', async () => {
  const ui = await fixture();
  const invoking = ui.document.createElement('button'); invoking.focus();
  const result = ui.window.smpDialogs.edit({ title: 'Edit name', fields: [{ id: 'name', label: 'Name', value: 'Original' }] });
  const input = ui.document.activeElement; input.value = 'Unapplied';
  const event = ui.keydown(input); await flush();
  assert.equal(await result, null);
  assert.equal(ui.document.activeElement, invoking);
  assert.equal(ui.window.smpState.selectedElementId, 'node-1');
  assert.deepEqual(ui.window.smpState.paletteTool, { id: 'block' });
  assert.equal(ui.calls.includes('clear_workspace_interaction'), false);
  assert.equal(ui.calls.some(command => /history|rename|update|edit|delete|create|apply/i.test(command)), false);
  assert.equal(event.defaultPrevented, true, 'the dialog handler prevents the browser default');
  assert.equal(event.immediateStopped, false, 'the workspace must not stop propagation');
});

test('Escape outside dialogs retains workspace cancellation behavior without authored mutation', async () => {
  const ui = await fixture();
  const event = ui.keydown(ui.canvas); await flush(); await flush();
  assert.equal(ui.window.smpState.selectedElementId, null);
  assert.equal(ui.window.smpState.paletteTool, null);
  assert.equal(ui.calls.filter(command => command === 'clear_workspace_interaction').length, 1);
  assert.equal(ui.calls.some(command => /history|rename|update|edit|delete|create|apply/i.test(command)), false);
  assert.equal(event.defaultPrevented, true);
  assert.equal(event.immediateStopped, true);
});

require('./test_ibd_port_gesture.cjs');

async function frameEvent(ui, type, x, y) {
  const handle = ui.canvas.querySelector('.sysml-frame-resize');
  const event = { type, target: handle, pointerId: 1, button: 0, clientX: x, clientY: y, preventDefault() {}, stopPropagation() {} };
  await Promise.all((ui.canvas.listeners.get(type) || []).map(listener => listener(event)));
}

for (const cancelled of ['pointercancel', 'lostpointercapture']) {
  test(`IBD frame ${cancelled} restores its starting boundary without saving`, async () => {
    const frame = { x: 54, y: 70, width: 1018, height: 662, manuallySized: true };
    const ui = await fixture({ family: 'ibd', frame });
    await frameEvent(ui, 'pointerdown', 10, 20);
    await frameEvent(ui, 'pointermove', 110, 120);
    assert.equal(ui.window.smpRendererHost.frameGeometry().width, 1118);
    await frameEvent(ui, cancelled, 110, 120);
    assert.deepEqual({ ...ui.window.smpRendererHost.frameGeometry() }, frame);
    assert.equal(ui.calls.includes('set_diagram_frame_preference'), false);
  });
}

test('IBD frame commit awaits Rust before refreshing and undo restores the legacy frame', async () => {
  const frame = { x: 54, y: 70, width: 1018, height: 662, manuallySized: true };
  let release; let saved; let refreshed = false;
  const ui = await fixture({ family: 'ibd', frame, legacy: true, saveFrame: args => {
    saved = args.preference; return new Promise(resolve => { release = resolve; });
  } });
  ui.window.refresh = async () => { refreshed = true; ui.window.smpState.snapshot.ibd_diagrams[0].context_frame = saved; };
  await frameEvent(ui, 'pointerdown', 10, 20); await frameEvent(ui, 'pointermove', 110, 120);
  const finishing = frameEvent(ui, 'pointerup', 110, 120); await flush();
  assert.equal(refreshed, false); assert.equal(saved.width, 1118);
  release(); await finishing;
  assert.equal(refreshed, true); assert.equal(ui.window.smpRendererHost.frameGeometry().width, 1118);
  ui.window.smpState.snapshot.ibd_diagrams[0].context_frame = null;
  assert.deepEqual({ ...ui.window.smpRendererHost.frameGeometry() }, frame);
});

test('rejected IBD frame resize restores the original boundary', async () => {
  const frame = { x: 54, y: 70, width: 1018, height: 662, manuallySized: true };
  const ui = await fixture({ family: 'ibd', frame, saveFrame: async () => { throw new Error('route rejected'); } });
  await frameEvent(ui, 'pointerdown', 10, 20); await frameEvent(ui, 'pointermove', 110, 120);
  await frameEvent(ui, 'pointerup', 110, 120);
  assert.deepEqual({ ...ui.window.smpRendererHost.frameGeometry() }, frame);
  assert.equal(ui.calls.filter(command => command === 'set_diagram_frame_preference').length, 1);
});
// Keep the Properties/history regressions in the existing frontend CI entry point.
require('./test_element_specification.cjs');


test('a frame click or sub-threshold movement does not adopt legacy geometry or create history', async () => {
  const frame = { x: 54, y: 70, width: 1018, height: 662, manuallySized: true };
  const ui = await fixture({ family: 'ibd', frame, legacy: true });
  await frameEvent(ui, 'pointerdown', 10, 20);
  await frameEvent(ui, 'pointermove', 11, 21);
  await frameEvent(ui, 'pointerup', 11, 21);
  assert.equal(ui.calls.includes('set_diagram_frame_preference'), false);
  assert.equal(ui.window.smpState.snapshot.ibd_diagrams[0].context_frame, null);
});

require('./test_standard_editing_clipboard.cjs');
require('./test_connector_properties.cjs');
require('./test_item_flow_notation.cjs');
require('./test_item_flow_properties.cjs');
