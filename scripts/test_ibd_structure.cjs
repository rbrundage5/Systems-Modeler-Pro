// Production renderer/dispatch contracts. Native persistence is tested in Rust.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const { test } = require('node:test');
function fixture() {
  class Node {
    constructor(tag) { this.tagName = tag; this.children = []; this.style = {}; this.dataset = {}; this.classList = { add() {} }; }
    appendChild(node) { this.children.push(node); }
    prepend(node) { this.children.unshift(node); }
    setAttribute(name, value) { this[name] = value; }
  }
  const part = (id, path) => ({ id, element_id: path.at(-1), property_path: path, ports: [], x: 100, y: 100, width: 220, height: 100 });
  const diagram = { id: 'ibd', name: 'Vehicle', context_block_id: 'vehicle', properties: [part('front', ['engine']), part('rear', ['rearEngine']), part('front-pistons', ['engine', 'pistons']), part('rear-pistons', ['rearEngine', 'pistons'])], boundary_ports: [], connectors: [] };
  const elements = ['engine', 'rearEngine', 'pistons'].map(id => ({ id, name: id, kind: 'PartProperty', owner_id: id === 'pistons' ? 'Engine' : 'vehicle' }));
  const project = { elements: [...elements, { id: 'Engine', name: 'Engine' }], relationships: [] };
  const panel = new Node('section'); const canvas = new Node('main'); const commands = [];
  const state = { snapshot: { project, ibd_diagrams: [diagram] }, selectedDiagramId: diagram.id };
  const context = { state, window: {}, console, SVG_NS: 'svg', document: { createElement: tag => new Node(tag), createElementNS: (_, tag) => new Node(tag), addEventListener() {} },
    $: id => id === 'properties' ? panel : canvas, escapeHtml: String, featureNotation: (_, element) => element.name,
    renderContext() {}, renderStatus() {}, renderDiagramTabs() {}, renderRepository() {}, renderCanvas() {}, renderPalette() {}, renderProperties() {}, render() {},
    refresh: async () => {}, runCommand: async (_, op) => op(), requireInvoke: () => async (command, args) => commands.push({command, args}),
  };
  vm.createContext(context);
  for (const name of ['ibd-ui.js', 'undo-redo-ui.js']) vm.runInContext(fs.readFileSync(`${__dirname}/../apps/desktop/frontend/${name}`, 'utf8'), context);
  return { context, diagram, panel, canvas, commands, state };
}

test('repeated terminal properties select and dispatch the actual occurrence', async () => {
  const ui = fixture();
  ui.context.renderIbdCanvas(ui.canvas, ui.diagram, ui.state.snapshot.project);
  const boxes = ui.canvas.children[0].children.filter(node => node.className?.startsWith('ibd-property'));
  await boxes.find(node => node.dataset.presentationId === 'rear-pistons').onclick({stopPropagation() {}});
  assert.equal(ui.state.selectedElementId, 'pistons');
  assert.equal(ui.state.selectedIbdPresentationId, 'rear-pistons');
  ui.context.renderProperties();
  const controls = ui.panel.children[0];
  assert.match(controls.children[0].textContent, /every usage/);
  assert.match(controls.children[0].textContent, /rearEngine \/ pistons/);
  await controls.children[1].onclick();
  assert.equal(ui.commands.length, 1, 'native operation owns the checkpoint');
  assert.equal(ui.commands[0].command, 'set_ibd_structure_expanded');
  assert.equal(ui.commands[0].args.presentationId, 'rear-pistons');
});

test('collapse hides only descendants of its contextual path and their connectors', () => {
  const ui = fixture(); ui.diagram.properties[0].collapsed = true;
  assert.equal(ui.context.ibdPropertyVisible(ui.diagram, ui.diagram.properties[2]), false);
  assert.equal(ui.context.ibdPropertyVisible(ui.diagram, ui.diagram.properties[3]), true);
  assert.equal(ui.context.ibdEndpointVisible(ui.diagram, 'front-pistons'), false);
  assert.equal(ui.context.ibdEndpointVisible(ui.diagram, 'rear-pistons'), true);
  ui.context.renderIbdCanvas(ui.canvas, ui.diagram, ui.state.snapshot.project);
  const ids = ui.canvas.children[0].children.map(node => node.dataset.presentationId);
  assert.equal(ids.includes('front-pistons'), false);
  assert.equal(ids.includes('rear-pistons'), true);
});

test('show existing parts dispatches without guessing types or changing the snapshot', async () => {
  const ui = fixture(); const before = JSON.stringify(ui.state.snapshot);
  await ui.context.updateIbdStructure(ui.diagram);
  assert.equal(ui.commands.length, 1);
  assert.equal(ui.commands[0].command, 'show_ibd_existing_parts');
  assert.equal(JSON.stringify(ui.state.snapshot), before);
});
