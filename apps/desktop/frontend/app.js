const invoke = window.__TAURI__?.core?.invoke;
const SVG_NS = 'http://www.w3.org/2000/svg';
const state = {
  snapshot: null,
  paletteItems: [],
  paletteTool: null,
  selectedElementId: null,
  selectedPackageId: null,
  selectedDiagramId: null,
  selectedRelationshipId: null,
  pendingRelationship: null,
  repositoryFilter: '',
};
window.smpState = state;
const $ = (id) => document.getElementById(id);
function requireInvoke() {
  if (!invoke) throw new Error('Tauri command bridge is unavailable. Run this UI through the desktop application.');
  return invoke;
}
async function loadPalette() {
  if (!state.selectedDiagramId) {
    state.paletteItems = [];
    return;
  }
  const diagram = state.snapshot?.diagrams?.find((item) => item.id === state.selectedDiagramId);
  state.paletteItems = await requireInvoke()('diagram_palette', { diagramType: diagram?.family === 'requirement' ? 'Requirement' : 'BDD' });
}
async function refresh() {
  state.snapshot = await requireInvoke()('workspace_snapshot');
  if (state.selectedRelationshipId && !state.snapshot?.project?.relationships?.some((r) => r.id === state.selectedRelationshipId)) {
    state.selectedRelationshipId = null;
  }
  if (state.selectedDiagramId && !state.snapshot?.diagrams?.some((d) => d.id === state.selectedDiagramId)) {
    state.selectedDiagramId = null;
  }
  await loadPalette();
  render();
}
function render() {
  renderRepository();
  renderPalette();
  renderDiagramTabs();
  renderCanvas();
  renderProperties();
  renderContext();
  renderStatus();
}
function renderStatus(message) {
  const status = $('status');
  const counts = $('model-counts');
  if (message) status.textContent = message;
  else if (state.pendingRelationship) {
    status.textContent = state.pendingRelationship.sourceElementId
      ? `${state.pendingRelationship.kind}: source selected. Click the target Block.`
      : `${state.pendingRelationship.kind}: click the source Block, then the target Block.`;
  } else if (state.paletteTool?.category === 'element') {
    status.textContent = `${state.paletteTool.label}: click or drop on the active BDD to create it.`;
  } else if (!state.snapshot?.project) status.textContent = 'No project open';
  else {
    const file = state.snapshot.current_file ? ` · ${state.snapshot.current_file}` : ' · unsaved';
    status.textContent = `${state.snapshot.project.name} · Rust semantic model${file}`;
  }
  const project = state.snapshot?.project;
  const diagram = state.snapshot?.diagrams?.find((d) => d.id === state.selectedDiagramId);
  counts.textContent = project
    ? `Elements: ${project.elements.length}   Relationships: ${project.relationships.length}${diagram ? `   Diagram: ${diagram.name} (BDD)` : ''}`
    : '';
}
function renderContext() {
  const diagram = state.snapshot?.diagrams?.find((d) => d.id === state.selectedDiagramId);
  const label = diagram?.family === 'requirement' ? 'Requirement Diagram' : 'BDD';
  $('active-diagram-summary').textContent = diagram ? `${diagram.name} · ${label}` : 'No diagram selected';
  $('palette-title').textContent = diagram ? `Elements (${label})` : 'Elements';
}
function repositoryMatches(element) {
  const filter = state.repositoryFilter.trim().toLowerCase();
  return !filter || element.name.toLowerCase().includes(filter) || element.kind.toLowerCase().includes(filter);
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
  function branchHasMatch(element) {
    if (repositoryMatches(element)) return true;
    return (byOwner.get(element.id) || []).some(branchHasMatch);
  }
  function appendChildren(ownerId, container, depth) {
    const children = (byOwner.get(ownerId) || []).sort((a, b) => a.name.localeCompare(b.name));
    for (const element of children) {
      if (!branchHasMatch(element)) continue;
      const row = document.createElement('button');
      row.className = 'tree-row';
      if (state.selectedElementId === element.id || state.selectedPackageId === element.id) row.classList.add('selected');
      row.style.paddingLeft = `${12 + depth * 16}px`;
      row.innerHTML = `<span class="kind">${element.kind === 'Package' ? '▣' : '◇'}</span><span>${escapeHtml(element.name)}</span><span class="type-tag">${escapeHtml(element.kind)}</span>`;
      if (element.kind === 'Block' || element.kind === 'Requirement' || element.kind === 'TestCase') {
        row.draggable = true;
        row.title = 'Drag onto a compatible diagram to create another presentation of this semantic element.';
        row.ondragstart = (event) => {
          event.dataTransfer.effectAllowed = 'copy';
          event.dataTransfer.setData('application/x-smp-element-id', element.id);
        };
      }
      row.onclick = () => {
        state.selectedRelationshipId = null;
        state.paletteTool = null;
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
      if (state.repositoryFilter && !diagram.name.toLowerCase().includes(state.repositoryFilter.toLowerCase())) continue;
      const row = document.createElement('button');
      row.className = 'tree-row diagram-row';
      if (state.selectedDiagramId === diagram.id) row.classList.add('selected');
      row.innerHTML = `<span class="kind">▤</span><span>${escapeHtml(diagram.name)}</span><span class="type-tag">${diagram.family === 'requirement' ? 'REQ' : 'BDD'}</span>`;
      row.onclick = () => selectDiagram(diagram.id);
      host.appendChild(row);
    }
  }
}
function paletteSymbol(item) {
  if (item.semantic_kind === 'Block') return '◇';
  const symbols = {
    Association: '──', Aggregation: '◇─', Composition: '◆─',
    Generalization: '▷─', Dependency: '⇢', Realization: '⇢▷',
  };
  return symbols[item.relationship_kind] || '·';
}
function renderPalette() {
  const host = $('palette');
  host.innerHTML = '';
  if (!state.selectedDiagramId) {
    host.innerHTML = '<div class="muted">Select a diagram to load its Rust-defined palette.</div>';
    return;
  }
  const groups = [
    ['element', 'Elements'],
    ['relationship', 'Relationships'],
  ];
  for (const [category, title] of groups) {
    const items = state.paletteItems.filter((item) => item.category === category);
    if (!items.length) continue;
    const section = document.createElement('section');
    section.className = 'palette-section';
    section.innerHTML = `<div class="palette-section-title">${title}</div>`;
    for (const item of items) {
      const button = document.createElement('button');
      button.className = `palette-item ${category}`;
      const active = category === 'element'
        ? state.paletteTool?.id === item.id
        : state.pendingRelationship?.kind === item.relationship_kind;
      if (active) button.classList.add('active');
      button.innerHTML = `<span class="palette-symbol">${escapeHtml(paletteSymbol(item))}</span><span>${escapeHtml(item.label)}</span>`;
      button.onclick = () => activatePaletteItem(item);
      if (item.draggable) {
        button.draggable = true;
        button.title = 'Click then click the diagram, or drag directly onto the diagram.';
        button.ondragstart = (event) => {
          event.dataTransfer.effectAllowed = 'copy';
          event.dataTransfer.setData('application/x-smp-palette-id', item.id);
        };
      }
      section.appendChild(button);
    }
    host.appendChild(section);
  }
  const hint = document.createElement('div');
  hint.className = 'palette-hint';
  hint.textContent = 'Palette tools come from Rust for the active diagram. Repository drag places an existing semantic element; palette placement creates a new semantic element.';
  host.appendChild(hint);
}
function activatePaletteItem(item) {
  state.selectedRelationshipId = null;
  if (item.category === 'relationship') {
    state.paletteTool = null;
    state.pendingRelationship = { kind: item.relationship_kind, sourceElementId: null };
  } else {
    state.pendingRelationship = null;
    state.paletteTool = item;
  }
  render();
}
async function selectDiagram(diagramId) {
  Object.assign(state, {
    selectedDiagramId: diagramId,
    selectedRelationshipId: null,
    pendingRelationship: null,
    paletteTool: null,
    selectedBehaviorDiagramId: null,
    selectedBehaviorItem: null,
    behaviorTool: null,
    behaviorPending: null,
    selectedActivityDiagramId: null,
    selectedActivityNodeId: null,
    selectedActivityEdgeId: null,
    activityTool: null,
    activityPendingFlow: null,
  });
  await loadPalette();
  const bdd = state.snapshot?.diagrams?.find((diagram) => diagram.id === diagramId);
  const ibd = state.snapshot?.ibd_diagrams?.find((diagram) => diagram.id === diagramId);
  await window.smpRendererHost?.activate({
    diagramId,
    familyId: ibd ? 'ibd' : (bdd?.family || 'bdd'),
    name: (ibd || bdd)?.name || 'Diagram',
    semanticContextId: ibd?.context_block_id || bdd?.owner_id || '',
  });
  render();
}
function renderDiagramTabs() {
  const host = $('diagram-tabs');
  host.innerHTML = '';
  for (const diagram of state.snapshot?.diagrams || []) {
    const tab = document.createElement('button');
    tab.className = 'diagram-tab';
    if (diagram.id === state.selectedDiagramId) tab.classList.add('active');
    tab.textContent = `${diagram.name} · ${diagram.family === 'requirement' ? 'REQ' : 'BDD'}`;
    tab.onclick = () => selectDiagram(diagram.id);
    host.appendChild(tab);
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
    if (['Dependency', 'DeriveRequirement', 'Satisfy', 'Verify', 'Refine', 'Trace', 'Copy'].includes(relationship.kind)) polyline.setAttribute('marker-end', 'url(#open-arrow)');
    polyline.onclick = (event) => {
      event.stopPropagation();
      state.selectedRelationshipId = relationship.id;
      state.selectedElementId = null;
      state.pendingRelationship = null;
      state.paletteTool = null;
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
function diagramCoordinates(frame, event) {
  const rect = frame.getBoundingClientRect();
  return {
    x: Math.max(10, event.clientX - rect.left - 90),
    y: Math.max(42, event.clientY - rect.top - 52),
  };
}
async function createPaletteElementAt(item, x, y) {
  const diagram = state.snapshot.diagrams.find((d) => d.id === state.selectedDiagramId);
  if (!diagram) throw new Error('Select a diagram first.');
  const name = prompt(`${item.label} name`, `New ${item.label}`);
  if (!name) return;
  let elementId;
  if (diagram.family === 'requirement' && item.semantic_kind === 'Requirement') {
    const definition = await window.smpDialogs?.edit({ title:'Create Requirement', fields:[{ id:'requirementId', label:'Requirement ID', value:'REQ-001', required:true }, { id:'text', label:'Requirement text', value:'The system shall ...', multiline:true, required:true }], confirmLabel:'Create' });
    if (!definition) return;
    elementId = await runCommand('Creating Requirement…', () => requireInvoke()('create_requirement', { ownerId: diagram.owner_id, name, requirementId:definition.values.requirementId, text:definition.values.text }));
  } else if (diagram.family === 'requirement' && item.semantic_kind === 'TestCase') {
    elementId = await runCommand('Creating Test Case…', () => requireInvoke()('create_test_case', { ownerId: diagram.owner_id, name }));
  } else if (diagram.family === 'requirement') {
    elementId = await runCommand(`Creating ${item.label}…`, () => requireInvoke()('create_bdd_element', { kind:item.semantic_kind, ownerId:diagram.owner_id, name }));
  } else if (item.semantic_kind === 'Block') {
    elementId = await runCommand(`Creating ${item.label}…`, () => requireInvoke()('create_block', { ownerId: diagram.owner_id, name }));
  } else throw new Error(`Palette item ${item.label} is not executable by the active Rust diagram engine.`);
  const placeCommand = diagram.family === 'requirement' ? 'place_on_requirement_diagram' : 'place_element_on_bdd';
  await runCommand(`Placing ${item.label}…`, () => requireInvoke()(placeCommand, {
    diagramId: diagram.id,
    elementId,
    x,
    y,
  }));
  state.selectedElementId = elementId;
  state.paletteTool = null;
  await refresh();
}
async function placeExistingElementAt(elementId, x, y) {
  const diagram = state.snapshot.diagrams.find((item) => item.id === state.selectedDiagramId);
  const command = diagram?.family === 'requirement' ? 'place_on_requirement_diagram' : 'place_element_on_bdd'; await runCommand('Placing existing element…', () => requireInvoke()(command, { diagramId: state.selectedDiagramId, elementId, x, y }));
  state.selectedElementId = elementId;
  await refresh();
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
    canvas.innerHTML = '<div class="empty-state"><h1>Model ready</h1><div>Create or select a Block Definition Diagram. Diagram elements are created from the palette.</div></div>';
    return;
  }
  const frame = document.createElement('div');
  frame.className = 'diagram-frame';
  frame.innerHTML = `<div class="diagram-header">${diagram.family === 'requirement' ? 'req' : 'bdd'} [package] ${escapeHtml(diagram.name)}</div>`;
  canvas.appendChild(frame);
  createRelationshipLayer(frame, diagram, project);
  frame.ondragover = (event) => {
    if (event.dataTransfer.types.includes('application/x-smp-palette-id') || event.dataTransfer.types.includes('application/x-smp-element-id')) {
      event.preventDefault();
      event.dataTransfer.dropEffect = 'copy';
      frame.classList.add('palette-target');
    }
  };
  frame.ondragleave = () => frame.classList.remove('palette-target');
  frame.ondrop = async (event) => {
    event.preventDefault();
    event.stopPropagation();
    frame.classList.remove('palette-target');
    const point = diagramCoordinates(frame, event);
    const paletteId = event.dataTransfer.getData('application/x-smp-palette-id');
    const elementId = event.dataTransfer.getData('application/x-smp-element-id');
    if (paletteId) {
      const item = state.paletteItems.find((value) => value.id === paletteId);
      if (item) await createPaletteElementAt(item, point.x, point.y);
    } else if (elementId) {
      await placeExistingElementAt(elementId, point.x, point.y);
    }
  };
  const elementsById = new Map(project.elements.map((e) => [e.id, e]));
  for (const node of diagram.nodes) {
    const element = elementsById.get(node.element_id);
    if (!element) continue;
    const box = document.createElement('button');
    box.className = 'bdd-block';
    box.dataset.semanticKind = element.kind;
    if (state.selectedElementId === element.id) box.classList.add('selected');
    if (state.pendingRelationship?.sourceElementId === element.id) box.classList.add('relationship-source');
    box.style.left = `${node.x}px`;
    box.style.top = `${node.y}px`;
    box.style.width = `${node.width}px`;
    box.style.height = `${node.height}px`;
    box.innerHTML = element.kind === 'Requirement'
      ? `<div class="stereotype">«requirement»</div><div class="block-name">${escapeHtml(element.name)}</div><div class="compartment"><div class="compartment-title">id</div><b>id</b> = ${escapeHtml(element.requirement_id || '')}</div><div class="compartment"><div class="compartment-title">text</div><b>text</b> = ${escapeHtml(element.requirement_text || '')}</div>${element.documentation ? `<div class="compartment"><div class="compartment-title">documentation</div>${escapeHtml(element.documentation)}</div>` : ''}`
      : `<div class="stereotype">«${element.kind === 'TestCase' ? 'testCase' : 'block'}»</div><div class="block-name">${escapeHtml(element.name)}</div><div class="compartment">${element.kind === 'Block' ? 'values' : 'verification'}</div>`;
    box.onclick = async (event) => {
      event.stopPropagation();
      if (state.pendingRelationship) {
        if (diagram.family !== 'requirement' && element.kind !== 'Block') return;
        if (!state.pendingRelationship.sourceElementId) {
          state.pendingRelationship.sourceElementId = element.id;
          state.selectedElementId = element.id;
          render();
          return;
        }
        if (state.pendingRelationship.sourceElementId !== element.id) {
          const pending = { ...state.pendingRelationship };
          state.pendingRelationship = null;
          const sourceNode = diagram.nodes.find((node) => node.element_id === pending.sourceElementId);
          const targetNode = diagram.nodes.find((node) => node.element_id === element.id);
          const command = diagram.family === 'requirement' ? 'create_traceability_relationship' : 'create_bdd_relationship';
          const args = diagram.family === 'requirement'
            ? { diagramId: state.selectedDiagramId, relationshipKind: pending.kind, sourceNodeId: sourceNode?.id, targetNodeId: targetNode?.id }
            : { diagramId: state.selectedDiagramId, kind: pending.kind, sourceElementId: pending.sourceElementId, targetElementId: element.id };
          await runCommand(`Creating ${pending.kind}…`, () => requireInvoke()(command, args));
          state.selectedElementId = element.id;
          await refresh();
          return;
        }
      }
      state.selectedRelationshipId = null;
      state.paletteTool = null;
      state.selectedElementId = element.id;
      render();
    };
    frame.appendChild(box);
  }
  frame.onclick = async (event) => {
    if (state.paletteTool?.category === 'element') {
      const point = diagramCoordinates(frame, event);
      await createPaletteElementAt(state.paletteTool, point.x, point.y);
      return;
    }
    state.selectedElementId = null;
    state.selectedRelationshipId = null;
    state.pendingRelationship = null;
    state.paletteTool = null;
    render();
  };
}
function blockOptions(project, selectedId) {
  return project.elements
    .filter((e) => e.kind === 'Block')
    .map((e) => `<option value="${escapeAttr(e.id)}"${e.id === selectedId ? ' selected' : ''}>${escapeHtml(e.name)}</option>`)
    .join('');
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
  if (!project) {
    panel.innerHTML = '<div class="muted">Create or open a project to inspect properties.</div>';
    return;
  }
  const relationship = project.relationships?.find((item) => item.id === state.selectedRelationshipId);
  if (relationship) return renderRelationshipProperties(panel, project, relationship);
  const element = project.elements.find((item) => item.id === state.selectedElementId);
  if (!element) {
    panel.innerHTML = '<div class="muted">Select an element or relationship. Create diagram elements from the Element Palette.</div>';
    return;
  }
  if (element.kind === 'Requirement') {
    panel.innerHTML = `<div class="property-heading">Requirement</div><label>Name<input id="property-name" value="${escapeAttr(element.name)}"></label><label>Requirement ID<input id="requirement-id" value="${escapeAttr(element.requirement_id || '')}"></label><label>Text<textarea id="requirement-text" rows="7">${escapeHtml(element.requirement_text || '')}</textarea></label><label>Documentation<textarea id="requirement-documentation" rows="5">${escapeHtml(element.documentation || '')}</textarea></label><label>Stable ID<input value="${escapeAttr(element.external_id)}" disabled></label><button id="update-requirement" class="primary">Apply Requirement</button>`;
    $('update-requirement').onclick = async () => {
      await runCommand('Updating Requirement…', () => requireInvoke()('update_requirement', { details:{ elementId: element.id, name: $('property-name').value, requirementId: $('requirement-id').value, text: $('requirement-text').value, documentation: $('requirement-documentation').value } })); await refresh();
    };
    return;
  }
  panel.innerHTML = `<div class="property-heading">${escapeHtml(element.kind)}</div>
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
    state.paletteTool = null;
    renderStatus(message);
    alert(message);
    throw error;
  }
}
async function createProject() {
  const name = prompt('Project name', 'Vehicle Model');
  if (!name) return;
  await runCommand('Creating project…', () => requireInvoke()('new_project', { name }));
  Object.assign(state, {
    paletteItems: [], paletteTool: null, selectedElementId: null, selectedPackageId: null,
    selectedDiagramId: null, selectedRelationshipId: null, pendingRelationship: null,
  });
  await refresh();
}
async function openProject() {
  const suggested = state.snapshot?.current_file || 'Vehicle Model.smproj';
  const path = prompt('Project file path (.smproj)', suggested);
  if (!path) return;
  await runCommand('Opening project…', () => requireInvoke()('open_project_file', { path }));
  Object.assign(state, {
    paletteItems: [], paletteTool: null, selectedElementId: null, selectedPackageId: null,
    selectedDiagramId: null, selectedRelationshipId: null, pendingRelationship: null,
  });
  await refresh();
  if (!state.selectedDiagramId && state.snapshot?.diagrams?.length) {
    state.selectedDiagramId = state.snapshot.diagrams[0].id;
    await loadPalette();
    render();
  }
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
async function createBdd() {
  if (!state.snapshot?.project) return alert('Create a project first.');
  const ownerId = state.selectedPackageId || state.snapshot.project.root_id;
  const name = prompt('BDD name', 'System Structure');
  if (!name) return;
  state.selectedDiagramId = await runCommand('Creating BDD…', () => requireInvoke()('create_bdd', { ownerId, name }));
  state.selectedRelationshipId = null;
  state.pendingRelationship = null;
  state.paletteTool = null;
  await refresh();
}
async function createRequirementDiagram() {
  if (!state.snapshot?.project) return window.smpDialogs?.notify('Create a project first.', 'warning');
  const ownerId = state.selectedPackageId || state.snapshot.project.root_id;
  const definition = await window.smpDialogs?.edit({ title:'Create Requirement Diagram', fields:[{ id:'name', label:'Diagram name', value:'System Requirements', required:true }], confirmLabel:'Create' });
  if (!definition) return;
  const selectedDiagramId = await runCommand('Creating Requirement Diagram…', () => requireInvoke()('create_requirement_diagram', { ownerId, name:definition.values.name }));
  Object.assign(state, { selectedDiagramId, selectedRelationshipId:null, pendingRelationship:null, paletteTool:null });
  await refresh();
}
window.smpCreateRequirementDiagram = createRequirementDiagram;
function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  })[c]);
}
function escapeAttr(value) { return escapeHtml(value); }
$('new-project').onclick = createProject;
$('open-project').onclick = openProject;
$('save-project').onclick = saveProject;
$('save-project-as').onclick = saveProjectAs;
$('new-package').onclick = createPackage;
$('new-bdd').onclick = createBdd;
$('new-requirement-diagram').onclick = createRequirementDiagram;
$('repository-search').oninput = (event) => {
  state.repositoryFilter = event.target.value;
  renderRepository();
};
$('collapse-palette').onclick = () => {
  const panel = document.querySelector('.palette-panel');
  panel.classList.toggle('collapsed');
  document.querySelector('.workspace').classList.toggle('palette-collapsed');
  $('collapse-palette').textContent = panel.classList.contains('collapsed') ? '»' : '«';
};
refresh().catch((error) => renderStatus(error.message));
