const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');
const flush = () => new Promise(resolve => setImmediate(resolve));
const deferred = () => { let resolve; const promise = new Promise(done => { resolve = done; }); return { promise, resolve }; };
const flow = (id, direction) => ({ relationship_id: id, connector_id: 'connector', direction,
  conveyed_item_ids: ['signal'], conveyed_item_names: ['Signal'] });

function fixture(read) {
  class Node {
    constructor(tag) { this.tag = tag; this.attributes = {}; this.dataset = {}; this.children = []; this.classList = { add() {} }; }
    setAttribute(name, value) { this.attributes[name] = value; }
    appendChild(child) { this.children.push(child); }
    querySelectorAll(tag) { return this.children.flatMap(child => [...(child.tag === tag ? [child] : []), ...child.querySelectorAll(tag)]); }
  }
  const svg = new Node('svg');
  const diagram = { id: 'diagram', connectors: [{ relationship_id: 'connector', points: [{ x: 0, y: 0 }, { x: 100, y: 0 }] }] };
  const state = { snapshot: { project: { id: 'project', elements: [], relationships: [] } } };
  const context = {
    state, console, window: {}, SVG_NS: 'svg', localStorage: { getItem: () => null },
    document: { createElementNS: (_, tag) => new Node(tag) },
    requireInvoke: () => async command => { assert.equal(command, 'ibd_item_flow_notation'); return read(); },
    refresh: async () => {}, render() {}, renderStatus() {}, renderProperties() {},
    selectedIbd: () => diagram, renderIbdConnectorLayer() {},
    // Frame projection/connected dragging is covered by the production-renderer
    // gesture suite; these notation identity tests supply already displayed points.
    ibdConnectorDisplayPoints: (_, edge) => edge.points,
  };
  vm.createContext(context);
  vm.runInContext(fs.readFileSync(path.join(__dirname, '../apps/desktop/frontend/item-flow-ui.js'), 'utf8'), context);
  return { state, context, draw: () => {
    svg.children = [];
    context.renderIbdConnectorLayer({ querySelector: () => svg }, diagram, state.snapshot.project);
    return svg.querySelectorAll('polygon');
  } };
}

test('opposite ItemFlows conveying the same classifier retain both identities and arrow directions', async () => {
  const ui = fixture(() => [flow('forward', 'Forward'), flow('reverse', 'Reverse')]); await flush();
  const arrows = ui.draw(); assert.equal(arrows.length, 2);
  for (const arrow of arrows) {
    const points = arrow.attributes.points.split(' ').map(pair => pair.split(',').map(Number));
    const id = arrow.attributes['data-relationship-id'];
    assert.equal(points[0][0] > points[1][0], id === 'forward');
    assert.match(arrow.attributes['aria-label'], id === 'forward' ? /source to target/ : /target to source/);
  }
});

test('authoritative empty notation removes arrows after undo or deletion', async () => {
  let rows = [flow('one', 'Forward')];
  const ui = fixture(() => rows); await flush(); assert.equal(ui.draw().length, 1);
  rows = []; await ui.context.refresh(); assert.equal(ui.draw().length, 0);
  assert.equal(ui.state.itemFlowNotation.length, 0);
});

test('late initial notation cannot revive a flow after a newer refresh', async () => {
  const pending = deferred(); let calls = 0;
  const ui = fixture(() => ++calls === 1 ? pending.promise : []);
  await ui.context.refresh(); pending.resolve([flow('stale', 'Forward')]); await flush();
  assert.equal(ui.draw().length, 0);
});

test('notation from a previous project is ignored after project replacement', async () => {
  const pending = deferred(); const ui = fixture(() => pending.promise);
  ui.state.snapshot.project = { id: 'different-project', elements: [], relationships: [] };
  pending.resolve([flow('stale', 'Reverse')]); await flush();
  assert.equal(ui.state.itemFlowNotation.length, 0);
});
