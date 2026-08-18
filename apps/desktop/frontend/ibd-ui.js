const baseLoadPalettePr11 = loadPalette;
loadPalette = async function loadPalettePr11() {
  if (!state.selectedDiagramId) { state.paletteItems = []; return; }
  const isIbd = (state.snapshot?.ibd_diagrams || []).some((d) => d.id === state.selectedDiagramId);
  state.paletteItems = await requireInvoke()('diagram_palette', { diagramType: isIbd ? 'IBD' : 'BDD' });
};

function selectedIbd() {
  return (state.snapshot?.ibd_diagrams || []).find((d) => d.id === state.selectedDiagramId);
}

function ibdElement(project, id) { return project.elements.find((e) => e.id === id); }

function propertyLabel(project, element) {
  return featureNotation(project, element);
}

function endpointElementId(diagram, presentationId) {
  const boundary = diagram.boundary_ports.find((p) => p.id === presentationId);
  if (boundary) return boundary.element_id;
  for (const property of diagram.properties) {
    if (property.id === presentationId) return property.element_id;
    const port = property.ports.find((p) => p.id === presentationId);
    if (port) return port.element_id;
  }
  return null;
}

function ibdEndpointPresentations(diagram) {
  const result = [];
  for (const port of diagram.boundary_ports) result.push({ id: port.id, label: `boundary:${port.element_id}` });
  for (const property of diagram.properties) {
    result.push({ id: property.id, label: `property:${property.element_id}` });
    for (const port of property.ports) result.push({ id: port.id, label: `port:${port.element_id}` });
  }
  return result;
}

const baseRenderContextPr11 = renderContext;
renderContext = function renderContextPr11() {
  const ibd = selectedIbd();
  if (!ibd) return baseRenderContextPr11();
  $('active-diagram-summary').textContent = `${ibd.name} · IBD`;
  $('palette-title').textContent = 'Elements (IBD)';
};

const baseRenderStatusPr11 = renderStatus;
renderStatus = function renderStatusPr11(message) {
  const ibd = selectedIbd();
  if (!ibd) return baseRenderStatusPr11(message);
  const status = $('status');
  if (message) status.textContent = message;
  else if (state.pendingRelationship) status.textContent = `${state.pendingRelationship.kind}: select valid IBD endpoints.`;
  else status.textContent = `${state.snapshot.project.name} · Rust IBD context ${ibd.context_block_id}`;
  $('model-counts').textContent = `Elements: ${state.snapshot.project.elements.length}   Relationships: ${state.snapshot.project.relationships.length}   Diagram: ${ibd.name} (IBD)`;
};

const baseRenderDiagramTabsPr11 = renderDiagramTabs;
renderDiagramTabs = function renderDiagramTabsPr11() {
  baseRenderDiagramTabsPr11();
  const host = $('diagram-tabs');
  for (const diagram of state.snapshot?.ibd_diagrams || []) {
    const tab = document.createElement('button');
    tab.className = 'diagram-tab';
    if (diagram.id === state.selectedDiagramId) tab.classList.add('active');
    tab.textContent = `${diagram.name} · IBD`;
    tab.onclick = () => selectDiagram(diagram.id);
    host.appendChild(tab);
  }
};

const baseRenderRepositoryPr11 = renderRepository;
renderRepository = function renderRepositoryPr11() {
  baseRenderRepositoryPr11();
  const host = $('repository');
  for (const diagram of state.snapshot?.ibd_diagrams || []) {
    if (state.repositoryFilter && !diagram.name.toLowerCase().includes(state.repositoryFilter.toLowerCase())) continue;
    const row = document.createElement('button');
    row.className = 'tree-row diagram-row';
    if (state.selectedDiagramId === diagram.id) row.classList.add('selected');
    row.innerHTML = `<span class="kind">▤</span><span>${escapeHtml(diagram.name)}</span><span class="type-tag">IBD</span>`;
    row.onclick = () => selectDiagram(diagram.id);
    host.appendChild(row);
  }
};

function renderIbdConnectorLayer(frame, diagram, project) {
  const svg = document.createElementNS(SVG_NS, 'svg');
  svg.classList.add('relationship-layer');
  svg.setAttribute('width', '100%'); svg.setAttribute('height', '100%');
  const relationships = new Map(project.relationships.map((r) => [r.id, r]));
  const itemFlowsByConnector = new Map();
  for (const relationship of project.relationships) {
    if (relationship.kind !== 'ItemFlow') continue;
    // Complete snapshots intentionally stay compact; ItemFlow conveyed labels are selected
    // through stable IDs during creation and can be enriched later without changing semantics.
  }
  for (const edge of diagram.connectors) {
    const relationship = relationships.get(edge.relationship_id);
    if (!relationship || !edge.points?.length) continue;
    const polyline = document.createElementNS(SVG_NS, 'polyline');
    polyline.setAttribute('points', edge.points.map((p) => `${p.x},${p.y}`).join(' '));
    polyline.setAttribute('fill', 'none');
    polyline.classList.add('ibd-connector');
    if (state.selectedRelationshipId === relationship.id) polyline.classList.add('selected');
    polyline.onclick = (event) => { event.stopPropagation(); state.selectedRelationshipId = relationship.id; state.selectedElementId = null; render(); };
    svg.appendChild(polyline);
    if (relationship.kind === 'Connector') {
      const midpoint = edge.points[Math.floor(edge.points.length / 2)];
      if (relationship.name) {
        const text = document.createElementNS(SVG_NS, 'text');
        text.classList.add('relationship-label'); text.setAttribute('x', midpoint.x + 5); text.setAttribute('y', midpoint.y - 6); text.textContent = relationship.name; svg.appendChild(text);
      }
    }
  }
  frame.appendChild(svg);
}

function renderIbdPort(frame, diagram, port, project, boundary = false) {
  const element = ibdElement(project, port.element_id);
  if (!element) return;
  const node = document.createElement('button');
  node.className = `ibd-port ${element.kind === 'ProxyPort' ? 'proxy-port' : 'full-port'} ${boundary ? 'boundary-port' : ''}`;
  node.style.left = `${port.x - port.size / 2}px`; node.style.top = `${port.y - port.size / 2}px`; node.style.width = `${port.size}px`; node.style.height = `${port.size}px`;
  node.title = `${element.name} : ${typeName(project, element)}${element.is_conjugated ? ' ~' : ''}`;
  node.onclick = async (event) => {
    event.stopPropagation();
    if (state.pendingRelationship && ['Assembly', 'Delegation'].includes(state.pendingRelationship.kind)) {
      if (!state.pendingRelationship.sourcePresentationId) { state.pendingRelationship.sourcePresentationId = port.id; render(); return; }
      const pending = { ...state.pendingRelationship }; state.pendingRelationship = null;
      await runCommand(`Creating ${pending.kind} Connector…`, () => requireInvoke()('create_ibd_connector', {
        diagramId: diagram.id, kind: pending.kind, sourcePresentationId: pending.sourcePresentationId, targetPresentationId: port.id, name: null,
      }));
      await refresh(); return;
    }
    state.selectedElementId = element.id; state.selectedRelationshipId = null; render();
  };
  frame.appendChild(node);
}

function renderIbdCanvas(canvas, diagram, project) {
  const context = ibdElement(project, diagram.context_block_id);
  const frame = document.createElement('div');
  frame.className = 'diagram-frame ibd-frame';
  frame.innerHTML = `<div class="diagram-header">ibd [${escapeHtml(context?.name || 'block')}] ${escapeHtml(diagram.name)}</div>`;
  canvas.appendChild(frame);
  renderIbdConnectorLayer(frame, diagram, project);
  for (const property of diagram.properties) {
    const element = ibdElement(project, property.element_id); if (!element) continue;
    const box = document.createElement('button');
    box.className = `ibd-property ${element.kind === 'ReferenceProperty' ? 'reference-property' : 'part-property'}`;
    box.style.left = `${property.x}px`; box.style.top = `${property.y}px`; box.style.width = `${property.width}px`; box.style.height = `${property.height}px`;
    box.innerHTML = `<div class="ibd-property-name">${escapeHtml(propertyLabel(project, element))}</div>`;
    box.onclick = (event) => { event.stopPropagation(); state.selectedElementId = element.id; state.selectedRelationshipId = null; render(); };
    frame.appendChild(box);
    for (const port of property.ports) renderIbdPort(frame, diagram, port, project, false);
  }
  for (const port of diagram.boundary_ports) renderIbdPort(frame, diagram, port, project, true);
  frame.onclick = () => { state.selectedElementId = null; state.selectedRelationshipId = null; state.pendingRelationship = null; render(); };
}

const baseRenderCanvasPr11 = renderCanvas;
renderCanvas = function renderCanvasPr11() {
  const ibd = selectedIbd();
  if (!ibd) return baseRenderCanvasPr11();
  const canvas = $('canvas'); canvas.innerHTML = '';
  renderIbdCanvas(canvas, ibd, state.snapshot.project);
};

const baseRenderPalettePr11 = renderPalette;
renderPalette = function renderPalettePr11() {
  const ibd = selectedIbd();
  if (!ibd) return baseRenderPalettePr11();
  const host = $('palette'); host.innerHTML = '';
  for (const [category, title] of [['feature', 'Internal Structure'], ['relationship', 'Connections']]) {
    const items = state.paletteItems.filter((item) => item.category === category);
    const section = document.createElement('section'); section.className = 'palette-section'; section.innerHTML = `<div class="palette-section-title">${title}</div>`;
    for (const item of items) {
      const button = document.createElement('button'); button.className = `palette-item ${category}`;
      button.innerHTML = `<span class="palette-symbol">${escapeHtml(paletteSymbol(item))}</span><span>${escapeHtml(item.label)}</span>`;
      button.onclick = async () => {
        if (item.relationship_kind === 'Assembly' || item.relationship_kind === 'Delegation') {
          state.pendingRelationship = { kind: item.relationship_kind, sourcePresentationId: null }; render(); return;
        }
        if (item.relationship_kind === 'ItemFlow') {
          const relationship = state.snapshot.project.relationships.find((r) => r.id === state.selectedRelationshipId && r.kind === 'Connector');
          if (!relationship) throw new Error('Select an existing Connector before adding an Item Flow.');
          const candidates = state.snapshot.project.elements.filter((e) => !['Model', 'Package', 'PartProperty', 'ReferenceProperty', 'ValueProperty', 'FlowProperty', 'ConstraintProperty', 'ProxyPort', 'FullPort', 'Operation', 'Parameter', 'Reception', 'Comment'].includes(e.kind));
          const menu = candidates.map((e, i) => `${i + 1}. ${e.name} (${e.kind})`).join('\n');
          const answer = prompt(`Conveyed classifier:\n${menu}`, '1'); if (!answer) return;
          const itemId = candidates[Number(answer) - 1]?.id; if (!itemId) throw new Error('Invalid conveyed classifier selection.');
          await runCommand('Adding Item Flow…', () => requireInvoke()('add_item_flow_to_connector', { relationshipId: relationship.id, conveyedItemIds: [itemId] }));
          await refresh(); return;
        }
        if (['PartProperty', 'ReferenceProperty', 'ProxyPort', 'FullPort'].includes(item.semantic_kind)) {
          alert('Create the semantic structural feature on the context Block/type, then use Populate IBD or the existing BDD feature workflow. This avoids presentation-only duplicates.');
        }
      };
      section.appendChild(button);
    }
    host.appendChild(section);
  }
};

async function createIbdForSelectedBlock() {
  const project = state.snapshot?.project; if (!project) return;
  const block = project.elements.find((e) => e.id === state.selectedElementId && ['Block', 'AssociationBlock'].includes(e.kind));
  if (!block) return alert('Select a Block or AssociationBlock first.');
  const name = prompt('IBD name', `${block.name} Internal Structure`); if (!name) return;
  const ownerId = state.selectedPackageId || block.owner_id || project.root_id;
  const id = await runCommand('Creating IBD…', () => requireInvoke()('create_ibd', { contextBlockId: block.id, ownerId, name }));
  await requireInvoke()('populate_ibd_from_context', { diagramId: id });
  state.selectedDiagramId = id; await refresh();
}

async function routeSelectedIbd() {
  const diagram = selectedIbd(); if (!diagram) return;
  await runCommand('Routing IBD…', () => requireInvoke()('route_ibd', { diagramId: diagram.id })); await refresh();
}

window.smpCreateIbdForSelectedBlock = createIbdForSelectedBlock;
window.smpRouteSelectedIbd = routeSelectedIbd;

const baseRenderPropertiesPr11 = renderProperties;
renderProperties = function renderPropertiesPr11() {
  const ibd = selectedIbd();
  if (!ibd) return baseRenderPropertiesPr11();
  const project = state.snapshot.project;
  const relationship = project.relationships.find((r) => r.id === state.selectedRelationshipId);
  if (relationship?.kind === 'Connector') {
    $('properties').innerHTML = `<div class="property-heading">Connector</div><label>Stable ID<input value="${escapeAttr(relationship.external_id)}" disabled></label><button id="route-ibd-selection" class="primary">Route IBD</button>`;
    $('route-ibd-selection').onclick = routeSelectedIbd;
    return;
  }
  baseRenderPropertiesPr11();
};
