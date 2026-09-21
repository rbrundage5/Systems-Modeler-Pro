// Drive the production shared HTML gesture controller; no semantic engine is mocked.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');
const flush = () => new Promise(resolve => setImmediate(resolve));

function fixture(family = 'bdd', scale = 1) {
  const commands = [];
  const previews = [];
  let refreshes = 0;
  class Node {
    constructor(classes = '') {
      this.classes = new Set(classes.split(' ').filter(Boolean));
      this.classList = {
        contains: name => this.classes.has(name),
        add: (...names) => names.forEach(name => this.classes.add(name)),
        remove: (...names) => names.forEach(name => this.classes.delete(name)),
        toggle: (name, value) => value ? this.classes.add(name) : this.classes.delete(name),
      };
      this.dataset = {}; this.style = {}; this.children = []; this.listeners = new Map();
    }
    set className(value) { this.classes = new Set(value.split(' ').filter(Boolean)); }
    addEventListener(name, listener) { const list = this.listeners.get(name) || []; list.push(listener); this.listeners.set(name, list); }
    removeEventListener(name, listener) { this.listeners.set(name, (this.listeners.get(name) || []).filter(item => item !== listener)); }
    appendChild(child) { this.children.push(child); child.parentElement = this; return child; }
    setAttribute() {}
    matches(selector) { return selector.split(',').some(item => this.classes.has(item.trim().slice(1))); }
    closest(selector) {
      for (let current = this; current; current = current.parentElement) if (current.matches(selector)) return current;
      return null;
    }
    contains(other) { return other === this || this.children.some(child => child.contains(other)); }
    getBoundingClientRect() { return { width: 1000 * scale, height: 800 * scale }; }
    setPointerCapture() {}
    fire(name, event) { for (const listener of [...(this.listeners.get(name) || [])]) { listener(event); if (event.immediateStopped) break; } }
  }
  const canvas = new Node('workspace-renderer-surface'); canvas.offsetWidth = 1000; canvas.offsetHeight = 800;
  const kind = family === 'ibd' ? 'ibd-property' : family === 'state' ? 'state-vertex' : 'bdd-block';
  const node = canvas.appendChild(new Node(kind));
  node.dataset.presentationId = 'presentation'; node.dataset.vertexId = 'vertex';
  Object.assign(node.style, { left: '100px', top: '120px', width: '190px', height: '110px' });
  const original = { id: 'presentation', element_id: 'element', vertex_id: 'vertex', x: 100, y: 120, width: 190, height: 110, ports: [] };
  const diagram = { id: 'diagram', family, nodes: [original], properties: [original], kind: 'StateMachine', state_nodes: [original] };
  const state = {
    selectedDiagramId: family === 'state' ? null : 'diagram', selectedBehaviorDiagramId: family === 'state' ? 'diagram' : null,
    snapshot: { diagrams: family === 'ibd' || family === 'state' ? [] : [diagram], ibd_diagrams: family === 'ibd' ? [diagram] : [] },
    behaviorSnapshot: { diagrams: family === 'state' ? [diagram] : [] },
  };
  const document = {
    createElement: () => new Node(), head: new Node(), getElementById: id => id === 'canvas' ? canvas : null,
    querySelector: () => null,
    querySelectorAll: selector => selector === `#canvas .${kind}` ? [node] : [],
  };
  const context = {
    state, document, Element: Node, console, CSS: { escape: String },
    MutationObserver: class { observe() {} }, queueMicrotask() {}, getComputedStyle: () => ({ position: 'absolute' }),
    window: { smpPreviewStateTransitionGeometry: (_, __, geometry) => previews.push({ ...geometry }) },
    runCommand: async (_, action) => action(), requireInvoke: () => async (command, args) => { commands.push({ command, args }); },
    refresh: async () => { refreshes++; }, render() {}, renderStatus() {},
  };
  vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../apps/desktop/frontend/diagram-interaction.js'), 'utf8'), context);
  context.window.smpInstallPresentationGeometry();
  const handle = node.children.find(child => child.classList.contains('smp-resize-handle'));
  function event(target, x, y, pointerId = 1) {
    return { target, button: 0, clientX: x, clientY: y, pointerId,
      preventDefault() { this.defaultPrevented = true; }, stopPropagation() {},
      stopImmediatePropagation() { this.immediateStopped = true; } };
  }
  function start(resize = false) {
    const target = resize ? handle : node;
    canvas.fire('pointerdown', event(target, 10, 20));
    return {
      move: (x, y, id = 1) => target.fire('pointermove', event(target, x, y, id)),
      end: name => target.fire(name, event(target, 50, 60)),
    };
  }
  return { node, canvas, commands, original, previews, start, event, get refreshes() { return refreshes; } };
}

for (const family of ['bdd', 'requirement', 'package', 'use-case', 'parametric', 'ibd', 'state']) {
  test(`${family}: cancelled move restores preview and leaves authored state untouched`, async () => {
    const ui = fixture(family);
    const gesture = ui.start(); gesture.move(50, 60);
    assert.equal(ui.node.style.left, '140px');
    gesture.end('pointercancel'); await flush();
    assert.equal(ui.node.style.left, '100px'); assert.equal(ui.node.style.top, '120px');
    assert.equal(ui.commands.length, 0); assert.equal(ui.refreshes, 0);
    assert.equal(ui.node.classList.contains('smp-dragging'), false);
    if (family === 'state') assert.deepEqual(ui.previews.at(-1), ui.original);
    const click = ui.event(ui.node, 50, 60); ui.canvas.fire('click', click);
    assert.equal(click.defaultPrevented, undefined, 'the next normal selection click must work');
  });
}

test('lost capture restores resized geometry and ignores a late pointerup', async () => {
  const ui = fixture(); const gesture = ui.start(true); gesture.move(50, 60);
  assert.equal(ui.node.style.width, '230px');
  gesture.end('lostpointercapture'); gesture.end('pointerup'); await flush();
  assert.equal(ui.node.style.width, '190px'); assert.equal(ui.node.style.height, '110px');
  assert.equal(ui.commands.length, 0);
});

test('successful zoomed drag still commits once through the existing Rust command', async () => {
  const ui = fixture('bdd', 2); const gesture = ui.start(); gesture.move(50, 60); gesture.end('pointerup'); await flush();
  assert.equal(ui.commands.length, 1); assert.equal(ui.commands[0].command, 'update_bdd_presentation_geometry');
  assert.equal(ui.commands[0].args.x, 120); assert.equal(ui.commands[0].args.y, 140);
  assert.equal(ui.refreshes, 1);
});

test('sub-threshold pointer movement and unrelated pointer IDs do not mutate geometry', async () => {
  const ui = fixture(); const gesture = ui.start(); gesture.move(50, 60, 2); gesture.move(11, 21); gesture.end('pointerup'); await flush();
  assert.equal(ui.commands.length, 0); assert.equal(ui.node.style.left, '100px'); assert.equal(ui.node.style.top, '120px');
});
