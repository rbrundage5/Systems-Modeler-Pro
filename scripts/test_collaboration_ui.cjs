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
    if (!selector.startsWith('.')) return [];
    return this.children.filter(child => child.classes.has(selector.slice(1)));
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
  const selectNames = new Set([
    'project', 'element', 'relationship-source', 'relationship-target', 'relationship-owner',
    'relationship', 'bdd-owner', 'bdd-diagram', 'bdd-element', 'bdd-relationship', 'bdd-edge',
    'requirement-owner', 'requirement-target',
  ]);
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
  const relationshipEdit = find('[data-relationship-edit]');
  relationshipEdit.elements = { kind: new Element('select') };
  relationshipEdit.elements.kind.value = 'Dependency';
  const requirementEdit = find('[data-requirement-edit]');
  requirementEdit.elements = {
    name: new Element('input'), requirementId: new Element('input'), text: new Element('textarea'),
  };
  const dialog = new Element('dialog');
  dialog.querySelector = find;
  dialog.querySelectorAll = () => [
    ...controls.values(), ...Object.values(connect.elements), ...Object.values(edit.elements),
    ...Object.values(relationshipEdit.elements),
    ...Object.values(requirementEdit.elements),
  ];
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
      }, relationships: {} },
      diagrams: [{ id: 'bdd', name: 'Structure', owner: 'root', nodes: [
        { id: 'node', element: 'block', x: 72, y: 90, width: 190, height: 115 },
      ], edges: [] }],
    },
  };
  const calls = [];
  let poll;
  let handler = async () => structuredClone(view);
  const invoke = async (command, payload) => { calls.push({ command, payload }); return handler(command, payload); };
  vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../apps/desktop/frontend/collaboration-ui.js'), 'utf8'), {
    window: { __TAURI__: { core: { invoke } } }, document,
    Option: function (text, value) { return { text, value }; },
    crypto: { randomUUID: () => 'new-id' }, setInterval(callback) { poll = callback; },
  });
  connect.elements.server.value = 'https://example.test';
  connect.elements.token.value = 'a'.repeat(64);
  connect.onsubmit(event());
  await flush();
  find('[data-open]').onclick();
  await flush();
  calls.length = 0;
  return { find, edit, relationshipEdit, requirementEdit, view, calls, poll, dialog, setHandler: value => { handler = value; }, canvas: find('[data-bdd-canvas]') };
}

function requirementDraft(ui) {
  ui.requirementEdit.elements.name.value = 'Response time';
  ui.requirementEdit.elements.requirementId.value = 'REQ-1';
  ui.requirementEdit.elements.text.value = 'Respond within 50 ms.\nPreserve this second line.';
  ui.requirementEdit.listeners.input();
}

test('shared requirement creation submits full text against the captured revision', async () => {
  const ui = await fixture();
  requirementDraft(ui);
  await ui.requirementEdit.onsubmit(event());
  const call = ui.calls.find(item => item.command === 'collaboration_edit');
  assert.equal(call.payload.expectedRevision, 7);
  assert.equal(call.payload.edit.CreateRequirement.owner, 'root');
  assert.equal(call.payload.edit.CreateRequirement.requirement_id, 'REQ-1');
  assert.equal(call.payload.edit.CreateRequirement.text, 'Respond within 50 ms.\nPreserve this second line.');
  assert.equal(ui.requirementEdit.elements.text.value, '');
});

test('requirement conflicts preserve drafts and require explicit review before rebasing', async () => {
  const ui = await fixture();
  ui.view.snapshot.project.elements.req = {
    id: 'req', kind: 'Requirement', name: 'Original', requirement_id: 'REQ-1', requirement_text: 'Original text.',
  };
  await ui.find('[data-refresh]').onclick();
  ui.find('[data-requirement-load]').onclick();
  requirementDraft(ui);
  ui.view.snapshot.revision = 8;
  ui.view.snapshot.project.elements.req.requirement_text = 'Other editor revision.';
  await ui.find('[data-refresh]').onclick();
  assert.match(ui.find('[data-requirement-current]').textContent, /Other editor revision/);
  assert.match(ui.requirementEdit.elements.text.value, /Respond within 50 ms/);
  ui.setHandler(async command => {
    if (command === 'collaboration_edit') throw new Error('Refresh and review the project before editing.');
    return structuredClone(ui.view);
  });
  await ui.requirementEdit.onsubmit(event());
  assert.equal(ui.calls.filter(item => item.command === 'collaboration_edit').at(-1).payload.expectedRevision, 7);
  assert.match(ui.requirementEdit.elements.text.value, /Respond within 50 ms/);
  ui.find('[data-requirement-rebase]').onclick();
  ui.setHandler(async () => structuredClone(ui.view));
  await ui.requirementEdit.onsubmit(event());
  const call = ui.calls.filter(item => item.command === 'collaboration_edit').at(-1);
  assert.equal(call.payload.expectedRevision, 8);
  assert.equal(call.payload.edit.UpdateRequirement.element, 'req');
  assert.equal(call.payload.edit.UpdateRequirement.requirement_id, 'REQ-1');
});

test('requirement drafts block background refresh and project switching', async () => {
  const ui = await fixture();
  ui.dialog.open = true;
  requirementDraft(ui);
  ui.poll();
  await flush();
  assert.equal(ui.calls.length, 0);
  await ui.find('[data-open]').onclick();
  assert.equal(ui.calls.filter(item => item.command === 'collaboration_open').length, 0);
  assert.match(ui.find('[data-message]').textContent, /Save or clear/);
  assert.match(ui.requirementEdit.elements.text.value, /Respond within 50 ms/);
});

test('viewers can inspect requirement text but cannot submit changes', async () => {
  const ui = await fixture();
  ui.view.projects[0].role = 'viewer';
  ui.view.snapshot.project.elements.req = {
    id: 'req', kind: 'Requirement', name: 'Read only', requirement_id: 'REQ-1', requirement_text: '<example> & text',
  };
  await ui.find('[data-refresh]').onclick();
  ui.find('[data-requirement-load]').onclick();
  assert.equal(ui.requirementEdit.elements.text.value, '<example> & text');
  assert.equal(ui.find('[data-requirement-submit]').disabled, true);
  await ui.requirementEdit.onsubmit(event());
  assert.equal(ui.calls.filter(item => item.command === 'collaboration_edit').length, 0);
});

test('uncertain requirement submission retains draft until exact pending retry succeeds', async () => {
  const ui = await fixture();
  requirementDraft(ui);
  ui.setHandler(async command => {
    if (command === 'collaboration_edit') { ui.view.pending = true; throw new Error('Network interrupted'); }
    if (command === 'collaboration_retry') ui.view.pending = false;
    return structuredClone(ui.view);
  });
  await ui.requirementEdit.onsubmit(event());
  assert.match(ui.requirementEdit.elements.text.value, /Respond within 50 ms/);
  assert.equal(ui.find('[data-requirement-reset]').disabled, true);
  await ui.requirementEdit.onsubmit(event());
  assert.equal(ui.calls.filter(item => item.command === 'collaboration_edit').length, 1);
  await ui.find('[data-retry]').onclick();
  assert.equal(ui.requirementEdit.elements.text.value, '');
  assert.equal(ui.calls.filter(item => item.command === 'collaboration_retry').length, 1);
});

test('requirement text is disabled during an in-flight request', async () => {
  const ui = await fixture();
  requirementDraft(ui);
  let complete;
  ui.setHandler(() => new Promise(resolve => { complete = resolve; }));
  const submitted = ui.requirementEdit.onsubmit(event());
  assert.equal(ui.requirementEdit.elements.text.disabled, true);
  complete(structuredClone(ui.view));
  await submitted;
  assert.equal(ui.requirementEdit.elements.text.disabled, false);
});

test('test cases can be created for existing Verify relationship controls', async () => {
  const ui = await fixture();
  ui.edit.elements.operation.value = 'CreateTestCase';
  ui.edit.elements.name.value = 'Response verification';
  ui.edit.onsubmit(event());
  await flush();
  const call = ui.calls.find(item => item.command === 'collaboration_edit');
  assert.equal(call.payload.edit.CreateTestCase.owner, 'root');
  assert.equal(call.payload.edit.CreateTestCase.name, 'Response verification');
});

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

test('relationship creation submits kind, endpoints, owner, and visible revision', async () => {
  const ui = await fixture();
  ui.relationshipEdit.elements.kind.value = 'Generalization';
  ui.find('[data-relationship-source]').value = 'block';
  ui.find('[data-relationship-target]').value = 'root';
  ui.find('[data-relationship-owner]').value = 'root';
  ui.relationshipEdit.onsubmit(event());
  await flush();
  const calls = ui.calls.filter(call => call.command === 'collaboration_edit');
  assert.equal(calls.length, 1);
  assert.equal(calls[0].payload.expectedRevision, 7);
  const relationship = calls[0].payload.edit.CreateRelationship;
  assert.equal(relationship.kind, 'Generalization');
  assert.equal(relationship.source, 'block');
  assert.equal(relationship.target, 'root');
  assert.equal(relationship.owner, 'root');
});

test('relationship deletion uses the selected stable relationship identity', async () => {
  const ui = await fixture();
  ui.view.snapshot.project.relationships = {
    relationship: {
      id: 'relationship', kind: 'Dependency', source_id: 'block', target_id: 'root', owner_id: 'root',
    },
  };
  ui.find('[data-refresh]').onclick();
  await flush();
  ui.find('[data-relationship]').value = 'relationship';
  ui.find('[data-relationship-delete]').onclick();
  await flush();
  const calls = ui.calls.filter(call => call.command === 'collaboration_edit');
  assert.equal(calls.at(-1).payload.expectedRevision, 7);
  assert.equal(calls.at(-1).payload.edit.DeleteRelationship.relationship, 'relationship');
});

test('a semantic relationship can be presented on a BDD at the visible revision', async () => {
  const ui = await fixture();
  ui.view.snapshot.project.elements.target = { id: 'target', name: 'Controller', kind: 'Block' };
  ui.view.snapshot.project.relationships.relationship = {
    id: 'relationship', kind: 'Dependency', source_id: 'block', target_id: 'target', owner_id: 'root',
  };
  ui.view.snapshot.diagrams[0].nodes.push({
    id: 'target-node', element: 'target', x: 400, y: 90, width: 190, height: 115,
  });
  ui.find('[data-refresh]').onclick();
  await flush();
  assert.equal(ui.find('[data-bdd-relationship]').value, 'relationship');
  ui.find('[data-bdd-edge-place]').onclick();
  await flush();
  const call = ui.calls.filter(item => item.command === 'collaboration_edit').at(-1);
  assert.equal(call.payload.expectedRevision, 7);
  assert.equal(call.payload.edit.PresentBddRelationship.diagram, 'bdd');
  assert.equal(call.payload.edit.PresentBddRelationship.edge, 'new-id');
  assert.equal(call.payload.edit.PresentBddRelationship.relationship, 'relationship');
});

test('server-routed BDD edges render behind nodes with notation and selection', async () => {
  const ui = await fixture();
  ui.view.snapshot.project.elements.target = { id: 'target', name: 'Controller', kind: 'Block' };
  ui.view.snapshot.project.relationships.relationship = {
    id: 'relationship', kind: 'Satisfy', source_id: 'block', target_id: 'target', owner_id: 'root',
  };
  ui.view.snapshot.diagrams[0].nodes.push({
    id: 'target-node', element: 'target', x: 400, y: 90, width: 190, height: 115,
  });
  ui.view.snapshot.diagrams[0].edges.push({
    id: 'edge', relationship: 'relationship', source_node: 'node', target_node: 'target-node',
    points: [{ x: 262, y: 147.5 }, { x: 400, y: 147.5 }],
    label_anchor: { x: 331, y: 115.5 },
  });
  ui.find('[data-refresh]').onclick();
  await flush();
  const edge = ui.canvas.children.find(child => child.classes.has('collaboration-bdd-edge'));
  const line = edge.children.find(child => child.tag === 'polyline');
  const label = edge.children.find(child => child.tag === 'text');
  assert.equal(line.attributes.points, '262,147.5 400,147.5');
  assert.equal(line.attributes['marker-end'], 'url(#collaboration-bdd-arrow)');
  assert.equal(label.textContent, '«satisfy»');
  edge.listeners.pointerdown(event());
  assert.equal(ui.find('[data-bdd-edge]').value, 'edge');
  assert.equal(ui.find('[data-bdd-edge-remove]').disabled, false);
});

test('BDD edge removal and rerouting use stable presentation identity', async () => {
  const ui = await fixture();
  ui.view.snapshot.project.elements.target = { id: 'target', name: 'Controller', kind: 'Block' };
  ui.view.snapshot.project.relationships.relationship = {
    id: 'relationship', kind: 'Generalization', source_id: 'block', target_id: 'target', owner_id: 'root',
  };
  ui.view.snapshot.diagrams[0].nodes.push({
    id: 'target-node', element: 'target', x: 400, y: 90, width: 190, height: 115,
  });
  ui.view.snapshot.diagrams[0].edges.push({
    id: 'edge', relationship: 'relationship', source_node: 'node', target_node: 'target-node',
    points: [{ x: 262, y: 147.5 }, { x: 400, y: 147.5 }],
    label_anchor: { x: 331, y: 115.5 },
  });
  ui.find('[data-refresh]').onclick();
  await flush();
  const edgeSelect = ui.find('[data-bdd-edge]');
  edgeSelect.value = 'edge';
  edgeSelect.onchange({ target: edgeSelect });
  ui.find('[data-bdd-edge-remove]').onclick();
  await flush();
  let call = ui.calls.filter(item => item.command === 'collaboration_edit').at(-1);
  assert.equal(call.payload.expectedRevision, 7);
  assert.equal(call.payload.edit.RemoveBddEdge.diagram, 'bdd');
  assert.equal(call.payload.edit.RemoveBddEdge.edge, 'edge');
  ui.find('[data-bdd-route]').onclick();
  await flush();
  call = ui.calls.filter(item => item.command === 'collaboration_edit').at(-1);
  assert.equal(call.payload.expectedRevision, 7);
  assert.equal(call.payload.edit.RouteBddDiagram.diagram, 'bdd');
});
