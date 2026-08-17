const invoke = window.__TAURI__?.core?.invoke;
const SVG_NS = 'http://www.w3.org/2000/svg';

const state = {
  snapshot: null,
  selectedElementId: null,
  selectedPackageId: null,
  selectedDiagramId: null,
  pendingRelationship: null,
};

const $ = (id) => document.getElementById(id);

function requireInvoke() {
  if (!invoke) throw new Error('Tauri command bridge is unavailable. Run this UI through the desktop application.');
  return invoke;
}

async function refresh() {
  state.snapshot = await requireInvoke()('workspace_snapshot');
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
  if (message) {
    status.textContent = message;
    return;
  }
  if (state.pendingRelationship) {
    status.textContent = `${state.pendingRelationship.kind}: source selected. Click the target Block on the BDD.`;
    return;
  }
  if (!state.snapshot?.project) {
    status.textContent = 'No project open';
    return;
  }
  const file = state.snapshot.current_file ? ` · ${state.snapshot.current_file}` : ' · unsaved';
  status.textContent = `${state.snapshot.project.name} · Rust semantic model${file}`;
}

function renderRepository() {
  const host = $('repository');
  host.innerHTML = '';
  const project = state.snapshot?.project;
  if (!project) {
    host.innerHTML = '<div class="muted">No project open.</div>';
    return;
  }

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
        if (element.kind === 'Package') {
          state.selectedPackageId = element.id;
          state.selectedElementId = null;
        } else {
          state.selectedElementId = element.id;
        }
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

function createRelationshipLayer(frame, diagram, project) {
  const svg = document.createElementNS(SVG_NS, 'svg');
  svg.classList.add('relationship-layer');
  svg.setAttribute('width', '100%');
  svg.setAttribute('height', '100%');
  svg.setAttribute('aria-label', 'BDD relationships');

  const defs = document.createElementNS(SVG_NS, 'defs');
  marker(defs, 'open-triangle', 'M 1 1 L 11 6 L 1 11 Z', { fill: '#f8f8f6', refX: '11' });
  marker(defs, 'open-arrow', 'M 1 1 L 11 6 L 1 11', { fill: 'none', refX: '11' });
  marker(defs, 'shared-diamond', 'M 1 6 L 6 1 L 11 6 L 6 11 Z', { fill: '#f8f8f6', refX: '1' });
  marker(defs, 'composite-diamond', 'M 1 6 L 6 1 L 11 6 L 6 11 Z', { fill: '#111', refX: '1' });
  svg.appendChild(defs);

  const relationships = new Map((project.relationships || []).map((relationship) => [relationship.id, relationship]));
  for (const edge of diagram.edges || []) {
    const relationship = relationships.get(edge.relationship_id);
    if (!relationship || !edge.points?.length) continue;
    const polyline = document.createElementNS(SVG_NS, 'polyline');
    polyline.setAttribute('points', edge.points.map((point) => `${point.x},${point.y}`).join(' '));
    polyline.setAttribute('fill', 'none');
    polyline.classList.add('bdd-relationship', `relationship-${relationship.kind.toLowerCase()}`);
    if (relationship.kind === 'Aggregation') polyline.setAttribute('marker-start', 'url(#shared-diamond)');
    if (relationship.kind === 'Composition') polyline.setAttribute('marker-start', 'url(#composite-diamond)');
    if (relationship.kind === 'Generalization' || relationship.kind === 'Realization') polyline.setAttribute('marker-end', 'url(#open-triangle)');
    if (relationship.kind === 'Dependency') polyline.setAttribute('marker-end', 'url(#open-arrow)');
    const title = document.createElementNS(SVG_NS, 'title');
    title.textContent = relationship.kind;
    polyline.appendChild(title);
    svg.appendChild(polyline);
  }
  frame.appendChild(svg);
}

function renderCanvas() {
  const canvas = $('canvas');
  canvas.innerHTML = '';
  const project = state.snapshot?.project;
  if (!project) {
    canvas.innerHTML = '<div class="empty-state"><h1>Systems Modeler Pro</h1><div>Create or open a project to begin.</div></div>';
    return;
  }
  const diagram = state.snapshot.diagrams.find((item) => item.id === state.selectedDiagramId);
  if (!diagram) {
    canvas.innerHTML = '<div class="empty-state"><h1>Model ready</h1><div>Create or select a Block Definition Diagram.</div></div>';
    return;
  }

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
      state.selectedElementId = element.id;
      render();
    };
    frame.appendChild(box);
  }

  frame.onclick = () => {
    state.selectedElementId = null;
    state.pendingRelationship = null;
    render();
  };
}

function renderProperties() {
  const panel = $('properties');
  const project = state.snapshot?.project;
  if (!project) {
    panel.innerHTML = '<div class="muted">Create or open a project to inspect properties.</div>';
    return;
  }
  const element = project.elements.find((item) => item.id === state.selectedElementId);
  if (!element) {
    panel.innerHTML = '<div class="muted">Select a Block or package.</div>';
    return;
  }
  panel.innerHTML = `
    <label>Type<input value="${escapeAttr(element.kind)}" disabled></label>
    <label>Name<input id="property-name" value="${escapeAttr(element.name)}"></label>
    <label>Stable ID<input value="${escapeAttr(element.external_id)}" disabled></label>
    <button id="rename-element" class="primary">Apply name</button>
  `;
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
  state.selectedElementId = null;
  state.selectedPackageId = null;
  state.selectedDiagramId = null;
  state.pendingRelationship = null;
  await refresh();
}

async function openProject() {
  const suggested = state.snapshot?.current_file || 'Vehicle Model.smproj';
  const path = prompt('Project file path (.smproj)', suggested);
  if (!path) return;
  await runCommand('Opening project…', () => requireInvoke()('open_project_file', { path }));
  state.selectedElementId = null;
  state.selectedPackageId = null;
  state.selectedDiagramId = null;
  state.pendingRelationship = null;
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
  await refresh();
}

async function createBdd() {
  if (!state.snapshot?.project) return alert('Create a project first.');
  const ownerId = state.selectedPackageId || state.snapshot.project.root_id;
  const name = prompt('BDD name', 'System Structure');
  if (!name) return;
  state.selectedDiagramId = await runCommand('Creating BDD…', () => requireInvoke()('create_bdd', { ownerId, name }));
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
    diagramId: state.selectedDiagramId,
    elementId: state.selectedElementId,
    x: 70 + (existing % 3) * 230,
    y: 90 + Math.floor(existing / 3) * 180,
  }));
  await refresh();
}

function startRelationship() {
  if (!state.selectedDiagramId) return alert('Create or select a BDD first.');
  if (!state.selectedElementId) return alert('Select the source Block first.');
  const element = state.snapshot?.project?.elements.find((item) => item.id === state.selectedElementId);
  if (!element || element.kind !== 'Block') return alert('BDD relationships in PR #5 require Block endpoints.');
  const diagram = state.snapshot.diagrams.find((item) => item.id === state.selectedDiagramId);
  if (!diagram?.nodes.some((node) => node.element_id === element.id)) return alert('Place the source Block on the selected BDD first.');
  state.pendingRelationship = {
    kind: $('relationship-kind').value,
    sourceElementId: element.id,
  };
  render();
}

function escapeHtml(value) {
  return String(value).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
}
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
