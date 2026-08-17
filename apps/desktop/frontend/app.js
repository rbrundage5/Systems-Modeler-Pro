const invoke = window.__TAURI__?.core?.invoke;
const SVG_NS = 'http://www.w3.org/2000/svg';

const state = {
  snapshot: null,
  selectedElementId: null,
  selectedPackageId: null,
  selectedDiagramId: null,
  selectedRelationshipId: null,
  pendingRelationship: null,
};

const $ = (id) => document.getElementById(id);

function requireInvoke() {
  if (!invoke) throw new Error('Tauri command bridge is unavailable. Run this UI through the desktop application.');
  return invoke;
}

async function refresh() {
  state.snapshot = await requireInvoke()('workspace_snapshot');
  if (state.selectedRelationshipId && !state.snapshot?.project?.relationships?.some((r) => r.id === state.selectedRelationshipId)) {
    state.selectedRelationshipId = null;
  }
  render();
}

function render() {
  renderRepository();
  renderCanvas();
  renderProperties();
  renderStatus();
}

function renderStatus(message) {
  const status = $('status');
  if (message) return void (status.textContent = message);
  if (state.pendingRelationship) {
    status.textContent = `${state.pendingRelationship.kind}: source selected. Click the target Block on the BDD.`;
    return;
  }
  if (!state.snapshot?.project) return void (status.textContent = 'No project open');
  const file = state.snapshot.current_file ? ` · ${state.snapshot.current_file}` : ' · unsaved';
  status.textContent = `${state.snapshot.project.name} · Rust semantic model${file}`;
}

function renderRepository() {
  const host = $('repository');
  host.innerHTML = '';
  const project = state.snapshot?.project;
  if (!project) return void (host.innerHTML = '<div class="muted">No project open.</div>');

  const root = document.createElement('div');
  root.className = 'tree-root';
  root.textContent = project.name;
  host.appendChild(root);

  const byOwner = new Map();
  for (const element of project.elements) {
    if (!element.owner_id) continue;
    if (!byOwner.has(element.owner_id)) byOwner.set(element.owner_id, []);
    byOwner.get(element.owner_id).push(element);
  }
  function appendChildren(ownerId, container, depth) {
    const children = (byOwner.get(ownerId) || []).sort((a, b) => a.name.localeCompare(b.name));
    for (const element of children) {
      const row = document.createElement('button');
      row.className = 'tree-row';
      if (state.selectedElementId === element.id || state.selectedPackageId === element.id) row.classList.add('selected');
      row.style.paddingLeft = `${12 + depth * 16}px`;
      row.innerHTML = `<span class="kind">${element.kind === 'Package' ? '▣' : '◇'}</span><span>${escapeHtml(element.name)}</span><span class="type-tag">${escapeHtml(element.kind)}</span>`;
      row.onclick = () => {
        state.selectedRelationshipId = null;
        if (element.kind === 'Package') {
          state.selectedPackageId = element.id;
          state.selectedElementId = null;
        } else state.selectedElementId = element.id;
        render();
      };
      container.appendChild(row);
      appendChildren(element.id, container, depth + 1);
    }
  }
  appendChildren(project.root_id, host, 0);

  if (state.snapshot.diagrams.length) {
    const heading = document.createElement('div');
    heading.className = 'tree-heading';
    heading.textContent = 'Diagrams';
    host.appendChild(heading);
    for (const diagram of state.snapshot.diagrams) {
      const row = document.createElement('button');
      row.className = 'tree-row diagram-row';
      if (state.selectedDiagramId === diagram.id) row.classList.add('selected');
      row.innerHTML = `<span class="kind">▤</span><span>${escapeHtml(diagram.name)}</span><span class="type-tag">BDD</span>`;
      row.onclick = () => {
        state.selectedDiagramId = diagram.id;
        state.selectedRelationshipId = null;
        state.pendingRelationship = null;
        render();
      };
      host.appendChild(row);
    }
  }
}

function marker(defs, id, path, options = {}) {
  const node = document.createElementNS(SVG_NS, 'marker');
  node.setAttribute('id', id);
  node.setAttribute('markerWidth', options.width || '12');
  node.setAttribute('markerHeight', options.height || '12');
  node.setAttribute('refX', options.refX || '10');
  node.setAttribute('refY', options.refY || '6');
  node.setAttribute('orient', 'auto-start-reverse');
  node.setAttribute('markerUnits', 'strokeWidth');
  const shape = document.createElementNS(SVG_NS, 'path');
  shape.setAttribute('d', path);
  shape.setAttribute('fill', options.fill || '#f8f8f6');
  shape.setAttribute('stroke', '#111');
  shape.setAttribute('stroke-width', '1');
  node.appendChild(shape);
  defs.appendChild(node);
}

function applyAssociationEndDecoration(polyline, relationship) {
  const decoratedEnd = (relationship.association_ends || []).find((end) => end.aggregation === 'shared' || end.aggregation === 'composite');
  if (!decoratedEnd) return;
  const markerId = decoratedEnd.aggregation === 'composite' ? 'composite-diamond' : 'shared-diamond';
  if (decoratedEnd.classifier_id === relationship.source_id) polyline.setAttribute('marker-start', `url(#${markerId})`);
  else if (decoratedEnd.classifier_id === relationship.target_id) polyline.setAttribute('marker-end', `url(#${markerId})`);
}

function endpointLabel(end) {
  if (!end) return '';
  return [end.role_name, end.multiplicity].filter(Boolean).join(' ');
}

function addEndpointLabel(svg, point, text, side) {
  if (!text) return;
  const label = document.createElementNS(SVG_NS, 'text');
  label.classList.add('relationship-label');
  label.setAttribute('x', point.x + (side === 'start' ? 8 : -8));
  label.setAttribute('y', point.y - 7);
  label.setAttribute('text-anchor', side === 'start' ? 'start' : 'end');
  label.textContent = text;
  svg.appendChild(label);
}

function createRelationshipLayer(frame, diagram, project) {
  const svg = document.createElementNS(SVG_NS, 'svg');
  svg.classList.add('relationship-layer');
  svg.setAttribute('width', '100%');
  svg.setAttribute('height', '100%');
  svg.setAttribute('aria-label', 'BDD relationships');

  const defs = document.createElementNS(SVG_NS, 'defs');
  marker(defs, 'open-triangle', 'M 1 1 L 11 6 L 1 11 Z', { fill: '#f8f8f6', refX: '11' });
  marker(defs, 'open-arrow', 'M 1 1 L 11 6 L 1 11', { fill: 'none', refX: '11' });
  marker(defs, 'shared-diamond', 'M 1 6 L 6 1 L 11 6 L 6 11 Z', { fill: '#f8f8f6', refX: '11' });
  marker(defs, 'composite-diamond', 'M 1 6 L 6 1 L 11 6 L 6 11 Z', { fill: '#111', refX: '11' });
  svg.appendChild(defs);

  const relationships = new Map((project.relationships || []).map((r) => [r.id, r]));
  for (const edge of diagram.edges || []) {
    const relationship = relationships.get(edge.relationship_id);
    if (!relationship || !edge.points?.length) continue;
    const polyline = document.createElementNS(SVG_NS, 'polyline');
    polyline.setAttribute('points', edge.points.map((point) => `${point.x},${point.y}`).join(' '));
    polyline.setAttribute('fill', 'none');
    polyline.classList.add('bdd-relationship', `relationship-${relationship.kind.toLowerCase()}`);
    if (state.selectedRelationshipId === relationship.id) polyline.classList.add('selected');
    applyAssociationEndDecoration(polyline, relationship);
    if (relationship.kind === 'Generalization' || relationship.kind === 'Realization') polyline.setAttribute('marker-end', 'url(#open-triangle)');
    if (relationship.kind === 'Dependency') polyline.setAttribute('marker-end', 'url(#open-arrow)');
    polyline.onclick = (event) => {
      event.stopPropagation();
      state.selectedRelationshipId = relationship.id;
      state.selectedElementId = null;
      state.pendingRelationship = null;
      render();
    };
    const title = document.createElementNS(SVG_NS, 'title');
    title.textContent = relationship.kind;
    polyline.appendChild(title);
    svg.appendChild(polyline);

    if (relationship.association_ends?.length === 2) {
      addEndpointLabel(svg, edge.points[0], endpointLabel(relationship.association_ends[0]), 'start');
      addEndpointLabel(svg, edge.points[edge.points.length - 1], endpointLabel(relationship.association_ends[1]), 'end');
    }
  }
  frame.appendChild(svg);
}

function renderCanvas() {
  const canvas = $('canvas');
  canvas.innerHTML = '';
  const project = state.snapshot?.project;
  if (!project) return void (canvas.innerHTML = '<div class="empty-state"><h1>Systems Modeler Pro</h1><div>Create or open a project to begin.</div></div>');
  const diagram = state.snapshot.diagrams.find((item) => item.id === state.selectedDiagramId);
  if (!diagram) return void (canvas.innerHTML = '<div class="empty-state"><h1>Model ready</h1><div>Create or select a Block Definition Diagram.</div></div>');

  const frame = document.createElement('div');
  frame.className = 'diagram-frame';
  frame.innerHTML = `<div class="diagram-header">bdd [package] ${escapeHtml(diagram.name)}</div>`;
  canvas.appendChild(frame);
  createRelationshipLayer(frame, diagram, project);

  const elementsById = new Map(project.elements.map((e) => [e.id, e]));
  for (const node of diagram.nodes) {
    const element = elementsById.get(node.element_id);
    if (!element) continue;
    const box = document.createElement('button');
    box.className = 'bdd-block';
    if (state.selectedElementId === element.id) box.classList.add('selected');
    if (state.pendingRelationship?.sourceElementId === element.id) box.classList.add('relationship-source');
    box.style.left = `${node.x}px`;
    box.style.top = `${node.y}px`;
    box.style.width = `${node.width}px`;
    box.style.height = `${node.height}px`;
    box.innerHTML = `<div class="stereotype">«block»</div><div class="block-name">${escapeHtml(element.name)}</div><div class="compartment">values</div>`;
    box.onclick = async (event) => {
      event.stopPropagation();
      if (state.pendingRelationship && state.pendingRelationship.sourceElementId !== element.id) {
        const pending = state.pendingRelationship;
        state.pendingRelationship = null;
        await runCommand(`Creating ${pending.kind}…`, () => requireInvoke()('create_bdd_relationship', {
          diagramId: state.selectedDiagramId,
          kind: pending.kind,
          sourceElementId: pending.sourceElementId,
          targetElementId: element.id,
        }));
        state.selectedElementId = element.id;
        await refresh();
        return;
      }
      state.selectedRelationshipId = null;
      state.selectedElementId = element.id;
      render();
    };
    frame.appendChild(box);
  }

  frame.onclick = () => {
    state.selectedElementId = null;
    state.selectedRelationshipId = null;
    state.pendingRelationship = null;
    render();
  };
}

function blockOptions(project, selectedId) {
  return project.elements.filter((e) => e.kind === 'Block').map((e) => `<option value="${escapeAttr(e.id)}"${e.id === selectedId ? ' selected' : ''}>${escapeHtml(e.name)}</option>`).join('');
}

function associationEndEditor(end, index) {
  return `<fieldset class="relationship-end"><legend>End ${index + 1}</legend>
    <label>Role name<input id="end-role-${index}" value="${escapeAttr(end.role_name || '')}"></label>
    <label>Multiplicity<input id="end-multiplicity-${index}" value="${escapeAttr(end.multiplicity || '1')}" placeholder="1, 0..1, 1..*, *"></label>
    <label>Aggregation<select id="end-aggregation-${index}">
      <option value="none"${end.aggregation === 'none' ? ' selected' : ''}>none</option>
      <option value="shared"${end.aggregation === 'shared' ? ' selected' : ''}>shared (aggregation)</option>
      <option value="composite"${end.aggregation === 'composite' ? ' selected' : ''}>composite (composition)</option>
    </select></label>
    <label class="check-row"><input id="end-navigable-${index}" type="checkbox"${end.navigable ? ' checked' : ''}> Navigable</label>
    <button class="primary" id="apply-end-${index}">Apply end</button>
  </fieldset>`;
}

function renderRelationshipProperties(panel, project, relationship) {
  panel.innerHTML = `<div class="property-heading">Relationship</div>
    <label>Type<input value="${escapeAttr(relationship.kind)}" disabled></label>
    <label>Stable ID<input value="${escapeAttr(relationship.external_id)}" disabled></label>
    <label>Source<select id="relationship-source">${blockOptions(project, relationship.source_id)}</select></label>
    <button id="apply-source" class="primary">Reconnect source</button>
    <label>Target<select id="relationship-target">${blockOptions(project, relationship.target_id)}</select></label>
    <button id="apply-target" class="primary">Reconnect target</button>
    ${(relationship.association_ends || []).map(associationEndEditor).join('')}
    <button id="delete-relationship" class="danger">Delete relationship</button>`;

  const reconnect = async (side) => {
    const elementId = $(`relationship-${side}`).value;
    await runCommand(`Reconnecting ${side}…`, () => requireInvoke()('reconnect_bdd_relationship', {
      diagramId: state.selectedDiagramId,
      relationshipId: relationship.id,
      side,
      elementId,
    }));
    await refresh();
  };
  $('apply-source').onclick = () => reconnect('source');
  $('apply-target').onclick = () => reconnect('target');

  (relationship.association_ends || []).forEach((end, index) => {
    $(`apply-end-${index}`).onclick = async () => {
      await runCommand('Updating association end…', () => requireInvoke()('update_association_end', {
        relationshipId: relationship.id,
        endId: end.id,
        roleName: $(`end-role-${index}`).value,
        multiplicity: $(`end-multiplicity-${index}`).value,
        navigable: $(`end-navigable-${index}`).checked,
        aggregation: $(`end-aggregation-${index}`).value,
      }));
      await refresh();
    };
  });

  $('delete-relationship').onclick = async () => {
    if (!confirm(`Delete ${relationship.kind} relationship?`)) return;
    await runCommand('Deleting relationship…', () => requireInvoke()('delete_bdd_relationship', {
      diagramId: state.selectedDiagramId,
      relationshipId: relationship.id,
    }));
    state.selectedRelationshipId = null;
    await refresh();
  };
}

function renderProperties() {
  const panel = $('properties');
  const project = state.snapshot?.project;
  if (!project) return void (panel.innerHTML = '<div class="muted">Create or open a project to inspect properties.</div>');

  const relationship = project.relationships?.find((item) => item.id === state.selectedRelationshipId);
  if (relationship) return renderRelationshipProperties(panel, project, relationship);

  const element = project.elements.find((item) => item.id === state.selectedElementId);
  if (!element) return void (panel.innerHTML = '<div class="muted">Select a Block, package, or relationship.</div>');
  panel.innerHTML = `<label>Type<input value="${escapeAttr(element.kind)}" disabled></label>
    <label>Name<input id="property-name" value="${escapeAttr(element.name)}"></label>
    <label>Stable ID<input value="${escapeAttr(element.external_id)}" disabled></label>
    <button id="rename-element" class="primary">Apply name</button>`;
  $('rename-element').onclick = async () => {
    const name = $('property-name').value.trim();
    if (!name) return;
    await runCommand('Renaming element…', () => requireInvoke()('rename_element', { elementId: element.id, name }));
    await refresh();
  };
}

async function runCommand(progressMessage, action) {
  try {
    renderStatus(progressMessage);
    return await action();
  } catch (error) {
    const message = error?.message || String(error);
    state.pendingRelationship = null;
    renderStatus(message);
    alert(message);
    throw error;
  }
}

async function createProject() {
  const name = prompt('Project name', 'Vehicle Model');
  if (!name) return;
  await runCommand('Creating project…', () => requireInvoke()('new_project', { name }));
  Object.assign(state, { selectedElementId: null, selectedPackageId: null, selectedDiagramId: null, selectedRelationshipId: null, pendingRelationship: null });
  await refresh();
}

async function openProject() {
  const suggested = state.snapshot?.current_file || 'Vehicle Model.smproj';
  const path = prompt('Project file path (.smproj)', suggested);
  if (!path) return;
  await runCommand('Opening project…', () => requireInvoke()('open_project_file', { path }));
  Object.assign(state, { selectedElementId: null, selectedPackageId: null, selectedDiagramId: null, selectedRelationshipId: null, pendingRelationship: null });
  await refresh();
  if (state.snapshot.diagrams.length) state.selectedDiagramId = state.snapshot.diagrams[0].id;
  render();
}

async function saveProjectAs() {
  if (!state.snapshot?.project) return alert('Create or open a project first.');
  const suggested = state.snapshot.current_file || `${state.snapshot.project.name}.smproj`;
  const path = prompt('Save project as (.smproj)', suggested);
  if (!path) return;
  await runCommand('Saving project…', () => requireInvoke()('save_project_file', { path }));
  await refresh();
}
async function saveProject() {
  if (!state.snapshot?.project) return alert('Create or open a project first.');
  if (!state.snapshot.current_file) return saveProjectAs();
  await runCommand('Saving project…', () => requireInvoke()('save_current_project'));
  await refresh();
}
async function createPackage() {
  if (!state.snapshot?.project) return alert('Create a project first.');
  const name = prompt('Package name', 'Structure');
  if (!name) return;
  const ownerId = state.selectedPackageId || state.snapshot.project.root_id;
  const id = await runCommand('Creating package…', () => requireInvoke()('create_package', { ownerId, name }));
  state.selectedPackageId = id;
  state.selectedElementId = null;
  state.selectedRelationshipId = null;
  await refresh();
}
async function createBlock() {
  if (!state.snapshot?.project) return alert('Create a project first.');
  const ownerId = state.selectedPackageId;
  if (!ownerId) return alert('Select a package in the Model Repository first.');
  const name = prompt('Block name', 'New Block');
  if (!name) return;
  const id = await runCommand('Creating Block…', () => requireInvoke()('create_block', { ownerId, name }));
  state.selectedElementId = id;
  state.selectedRelationshipId = null;
  await refresh();
}
async function createBdd() {
  if (!state.snapshot?.project) return alert('Create a project first.');
  const ownerId = state.selectedPackageId || state.snapshot.project.root_id;
  const name = prompt('BDD name', 'System Structure');
  if (!name) return;
  state.selectedDiagramId = await runCommand('Creating BDD…', () => requireInvoke()('create_bdd', { ownerId, name }));
  state.selectedRelationshipId = null;
  state.pendingRelationship = null;
  await refresh();
}
async function placeSelectedBlock() {
  if (!state.selectedDiagramId) return alert('Create or select a BDD first.');
  if (!state.selectedElementId) return alert('Select a Block in the repository first.');
  const element = state.snapshot.project.elements.find((item) => item.id === state.selectedElementId);
  if (!element || element.kind !== 'Block') return alert('Only Blocks can be placed on this BDD.');
  const existing = state.snapshot.diagrams.find((d) => d.id === state.selectedDiagramId)?.nodes.length || 0;
  await runCommand('Placing Block…', () => requireInvoke()('place_element_on_bdd', {
    diagramId: state.selectedDiagramId, elementId: state.selectedElementId,
    x: 70 + (existing % 3) * 230, y: 90 + Math.floor(existing / 3) * 180,
  }));
  await refresh();
}
function startRelationship() {
  if (!state.selectedDiagramId) return alert('Create or select a BDD first.');
  if (!state.selectedElementId) return alert('Select the source Block first.');
  const element = state.snapshot?.project?.elements.find((item) => item.id === state.selectedElementId);
  if (!element || element.kind !== 'Block') return alert('BDD relationships require Block endpoints.');
  const diagram = state.snapshot.diagrams.find((item) => item.id === state.selectedDiagramId);
  if (!diagram?.nodes.some((node) => node.element_id === element.id)) return alert('Place the source Block on the selected BDD first.');
  state.selectedRelationshipId = null;
  state.pendingRelationship = { kind: $('relationship-kind').value, sourceElementId: element.id };
  render();
}

function escapeHtml(value) { return String(value).replace(/[&<>\"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '\"': '&quot;' }[c])); }
function escapeAttr(value) { return escapeHtml(value); }

$('new-project').onclick = createProject;
$('open-project').onclick = openProject;
$('save-project').onclick = saveProject;
$('save-project-as').onclick = saveProjectAs;
$('new-package').onclick = createPackage;
$('new-block').onclick = createBlock;
$('new-bdd').onclick = createBdd;
$('place-block').onclick = placeSelectedBlock;
$('start-relationship').onclick = startRelationship;

refresh().catch((error) => renderStatus(error.message));
