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

function propertyLabel(project, element) { return featureNotation(project, element); }

function connectorHelp(kind, sourceChosen = false) {
  if (kind === 'Assembly') {
    return sourceChosen
      ? 'Assembly: first internal endpoint selected. Now click a SECOND internal Part/Reference Property or one of its ports. Do not use a port on the outer IBD boundary.'
      : 'Assembly: click the first INTERNAL Part/Reference Property or one of its ports, then click a second INTERNAL endpoint. Assembly connects internal roles; it does not start/end on the outer Block boundary.';
  }
  if (kind === 'Delegation') {
    return sourceChosen
      ? 'Delegation: first endpoint selected. Now click the opposite kind of endpoint: one outer Block boundary port and one internal Part/Reference Property or nested port.'
      : 'Delegation: connect ONE port on the outer IBD/Block boundary to ONE internal Part/Reference Property or nested port.';
  }
  return 'Select valid connector endpoints.';
}

function friendlyConnectorError(kind, error) {
  const raw = error?.message || String(error);
  if (raw.includes('assembly connector requires internal ends')) {
    return 'Assembly Connector requires two INTERNAL endpoints. Click a Part/Reference Property or a port attached to an internal property for endpoint 1, then another internal property/port for endpoint 2. Do not select an outer Block boundary port.';
  }
  if (raw.includes('delegation connector requires exactly one boundary end and one internal end')) {
    return 'Delegation Connector requires exactly ONE outer Block boundary port and ONE internal Part/Reference Property or nested port. Select one of each.';
  }
  if (raw.includes('connector endpoint types are incompatible')) {
    return 'The selected connector endpoints have incompatible semantic types. Choose ports/properties with the same compatible type (or a valid generalized type relationship).';
  }
  if (raw.includes('connector endpoint must be')) {
    return 'That item is not a legal IBD connector endpoint. Select a Part Property, Reference Property, Proxy Port, or Full Port.';
  }
  return `${kind} Connector could not be created: ${raw}`;
}

function clearPendingConnector() {
  state.pendingRelationship = null;
  render();
}

async function commitIbdConnector(diagram, targetPresentationId) {
  const pending = { ...state.pendingRelationship };
  if (!pending?.sourcePresentationId) {
    state.pendingRelationship = { ...pending, sourcePresentationId: targetPresentationId };
    render();
    return;
  }
  if (pending.sourcePresentationId === targetPresentationId) {
    alert('Choose a different second endpoint. A Connector cannot connect an endpoint to itself.');
    return;
  }
  state.pendingRelationship = null;
  try {
    await runCommand(`Creating ${pending.kind} Connector…`, () => requireInvoke()('create_ibd_connector', {
      diagramId: diagram.id,
      kind: pending.kind,
      sourcePresentationId: pending.sourcePresentationId,
      targetPresentationId,
      name: null,
    }));
    await refresh();
  } catch (error) {
    alert(friendlyConnectorError(pending.kind, error));
    state.pendingRelationship = { kind: pending.kind, sourcePresentationId: null };
    render();
  }
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
  else if (state.pendingRelationship) status.textContent = connectorHelp(state.pendingRelationship.kind, !!state.pendingRelationship.sourcePresentationId);
  else status.textContent = `${state.snapshot.project.name} · IBD: ${ibd.name}`;
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
  svg.setAttribute('width', '100%');
  svg.setAttribute('height', '100%');
  svg.setAttribute('aria-label', 'IBD connectors');
  const relationships = new Map(project.relationships.map((r) => [r.id, r]));
  for (const edge of diagram.connectors) {
    const relationship = relationships.get(edge.relationship_id);
    if (!relationship || !edge.points?.length) continue;
    const polyline = document.createElementNS(SVG_NS, 'polyline');
    polyline.setAttribute('points', edge.points.map((p) => `${p.x},${p.y}`).join(' '));
    polyline.setAttribute('fill', 'none');
    polyline.classList.add('ibd-connector');
    if (state.selectedRelationshipId === relationship.id) polyline.classList.add('selected');
    const title = document.createElementNS(SVG_NS, 'title');
    title.textContent = `${relationship.kind} — click to select`;
    polyline.appendChild(title);
    polyline.onclick = (event) => {
      event.stopPropagation();
      state.selectedRelationshipId = relationship.id;
      state.selectedElementId = null;
      state.pendingRelationship = null;
      render();
    };
    svg.appendChild(polyline);
    const midpoint = edge.points[Math.floor(edge.points.length / 2)];
    if (relationship.name) {
      const text = document.createElementNS(SVG_NS, 'text');
      text.classList.add('relationship-label');
      text.setAttribute('x', midpoint.x + 5);
      text.setAttribute('y', midpoint.y - 6);
      text.textContent = relationship.name;
      svg.appendChild(text);
    }
  }
  frame.appendChild(svg);
}

function renderIbdPort(frame, diagram, port, project, boundary = false) {
  const element = ibdElement(project, port.element_id);
  if (!element) return;
  const node = document.createElement('button');
  node.className = `ibd-port ${element.kind === 'ProxyPort' ? 'proxy-port' : 'full-port'} ${boundary ? 'boundary-port' : 'nested-port'}`;
  if (state.selectedElementId === element.id) node.classList.add('selected');
  if (state.pendingRelationship?.sourcePresentationId === port.id) node.classList.add('connector-source');
  node.style.left = `${port.x - port.size / 2}px`;
  node.style.top = `${port.y - port.size / 2}px`;
  node.style.width = `${port.size}px`;
  node.style.height = `${port.size}px`;
  node.title = `${boundary ? 'Boundary' : 'Internal'} ${element.kind}: ${element.name} : ${typeName(project, element)}${element.is_conjugated ? ' ~' : ''}`;
  node.onclick = async (event) => {
    event.stopPropagation();
    if (state.pendingRelationship && ['Assembly', 'Delegation'].includes(state.pendingRelationship.kind)) {
      await commitIbdConnector(diagram, port.id);
      return;
    }
    state.selectedElementId = element.id;
    state.selectedRelationshipId = null;
    render();
  };
  frame.appendChild(node);
}

function renderConnectorGuide(frame) {
  if (!state.pendingRelationship || !['Assembly', 'Delegation'].includes(state.pendingRelationship.kind)) return;
  const guide = document.createElement('div');
  guide.className = 'ibd-connector-guide';
  guide.innerHTML = `<strong>${escapeHtml(state.pendingRelationship.kind)} Connector</strong><span>${escapeHtml(connectorHelp(state.pendingRelationship.kind, !!state.pendingRelationship.sourcePresentationId))}</span><button type="button">Cancel</button>`;
  guide.querySelector('button').onclick = (event) => { event.stopPropagation(); clearPendingConnector(); };
  frame.appendChild(guide);
}

function renderIbdCanvas(canvas, diagram, project) {
  const context = ibdElement(project, diagram.context_block_id);
  const frame = document.createElement('div');
  frame.className = 'diagram-frame ibd-frame';
  frame.innerHTML = `<div class="diagram-header">ibd [${escapeHtml(context?.name || 'block')}] ${escapeHtml(diagram.name)}</div>`;
  canvas.appendChild(frame);
  renderIbdConnectorLayer(frame, diagram, project);
  for (const property of diagram.properties) {
    const element = ibdElement(project, property.element_id);
    if (!element) continue;
    const box = document.createElement('button');
    box.className = `ibd-property ${element.kind === 'ReferenceProperty' ? 'reference-property' : 'part-property'}`;
    if (state.selectedElementId === element.id) box.classList.add('selected');
    if (state.pendingRelationship?.sourcePresentationId === property.id) box.classList.add('connector-source');
    box.style.left = `${property.x}px`;
    box.style.top = `${property.y}px`;
    box.style.width = `${property.width}px`;
    box.style.height = `${property.height}px`;
    box.innerHTML = `<div class="ibd-property-name">${escapeHtml(propertyLabel(project, element))}</div>`;
    box.title = `${element.kind}: click to select${state.pendingRelationship ? ' or use as connector endpoint' : ''}`;
    box.onclick = async (event) => {
      event.stopPropagation();
      if (state.pendingRelationship && ['Assembly', 'Delegation'].includes(state.pendingRelationship.kind)) {
        await commitIbdConnector(diagram, property.id);
        return;
      }
      state.selectedElementId = element.id;
      state.selectedRelationshipId = null;
      render();
    };
    frame.appendChild(box);
    for (const port of property.ports) renderIbdPort(frame, diagram, port, project, false);
  }
  for (const port of diagram.boundary_ports) renderIbdPort(frame, diagram, port, project, true);
  renderConnectorGuide(frame);
  frame.onclick = () => {
    state.selectedElementId = null;
    state.selectedRelationshipId = null;
    if (!state.pendingRelationship) state.paletteTool = null;
    render();
  };
}

function bindBddIbdDrilldown() {
  const project = state.snapshot?.project;
  const diagram = state.snapshot?.diagrams?.find((candidate) => candidate.id === state.selectedDiagramId);
  if (!project || !diagram) return;
  const boxes = [...document.querySelectorAll('#canvas .bdd-block')];
  boxes.forEach((box, index) => {
    const node = diagram.nodes[index];
    if (!node) return;
    const element = project.elements.find((candidate) => candidate.id === node.element_id);
    if (!element || !['Block', 'AssociationBlock'].includes(element.kind)) return;
    box.title = `${box.title ? `${box.title} · ` : ''}Double-click to open this Block's IBD`;
    box.ondblclick = async (event) => {
      event.preventDefault();
      event.stopPropagation();
      const child = (state.snapshot.ibd_diagrams || []).find((candidate) => candidate.context_block_id === element.id);
      if (!child) {
        alert(`${element.name} does not have an IBD yet. Select the Block and use Home → IBD to create its Internal Block Diagram.`);
        return;
      }
      await selectDiagram(child.id);
    };
  });
}

const baseRenderCanvasPr11 = renderCanvas;
renderCanvas = function renderCanvasPr11() {
  const ibd = selectedIbd();
  if (!ibd) {
    baseRenderCanvasPr11();
    bindBddIbdDrilldown();
    return;
  }
  const canvas = $('canvas');
  canvas.innerHTML = '';
  renderIbdCanvas(canvas, ibd, state.snapshot.project);
};

async function addExistingPortToSelectedProperty(item) {
  const diagram = selectedIbd();
  const project = state.snapshot?.project;
  if (!diagram || !project) return;
  const propertyPresentation = diagram.properties.find((presentation) => presentation.element_id === state.selectedElementId);
  if (!propertyPresentation) {
    alert(`Select an internal Part Property or Reference Property first, then choose ${item.label}.`);
    return;
  }
  const propertyElement = ibdElement(project, propertyPresentation.element_id);
  const typeId = propertyElement?.type_id;
  if (!typeId) {
    alert('The selected property has no compatible type, so its nested ports cannot be resolved.');
    return;
  }
  const ports = project.elements.filter((element) => element.owner_id === typeId && element.kind === item.semantic_kind);
  if (!ports.length) {
    alert(`The type of ${propertyElement.name} has no ${item.label}. Create the port on that Block/type first, then return to the IBD.`);
    return;
  }
  const menu = ports.map((port, index) => `${index + 1}. ${port.name} : ${typeName(project, port)}`).join('\n');
  const answer = prompt(`Choose ${item.label} to show on ${propertyElement.name}:\n${menu}`, '1');
  if (!answer) return;
  const port = ports[Number(answer) - 1];
  if (!port) return alert('Invalid port selection.');
  const side = prompt('Attach port to which side? left, right, top, or bottom', 'right');
  if (!side) return;
  await runCommand(`Adding ${item.label} to IBD…`, () => requireInvoke()('add_nested_port_to_ibd', {
    diagramId: diagram.id,
    propertyPresentationId: propertyPresentation.id,
    portId: port.id,
    side: side.toLowerCase(),
  }));
  await refresh();
}

const baseRenderPalettePr11 = renderPalette;
renderPalette = function renderPalettePr11() {
  const ibd = selectedIbd();
  if (!ibd) return baseRenderPalettePr11();
  const host = $('palette');
  host.innerHTML = '';
  const intro = document.createElement('div');
  intro.className = 'palette-hint ibd-palette-hint';
  intro.textContent = 'Connectors: choose Assembly or Delegation, then click endpoint 1 and endpoint 2 directly on the diagram. Selected items are outlined.';
  host.appendChild(intro);
  for (const [category, title] of [['feature', 'Internal Structure'], ['relationship', 'Connections']]) {
    const items = state.paletteItems.filter((item) => item.category === category);
    const section = document.createElement('section');
    section.className = 'palette-section';
    section.innerHTML = `<div class="palette-section-title">${title}</div>`;
    for (const item of items) {
      const button = document.createElement('button');
      button.className = `palette-item ${category}`;
      if (state.pendingRelationship?.kind === item.relationship_kind) button.classList.add('active');
      button.innerHTML = `<span class="palette-symbol">${escapeHtml(paletteSymbol(item))}</span><span>${escapeHtml(item.label)}</span>`;
      if (item.relationship_kind === 'Assembly') button.title = 'Assembly: internal endpoint → internal endpoint.';
      if (item.relationship_kind === 'Delegation') button.title = 'Delegation: outer Block boundary port ↔ internal endpoint.';
      button.onclick = async () => {
        if (item.relationship_kind === 'Assembly' || item.relationship_kind === 'Delegation') {
          state.pendingRelationship = { kind: item.relationship_kind, sourcePresentationId: null };
          state.selectedRelationshipId = null;
          render();
          return;
        }
        if (item.relationship_kind === 'ItemFlow') {
          const relationship = state.snapshot.project.relationships.find((r) => r.id === state.selectedRelationshipId && r.kind === 'Connector');
          if (!relationship) return alert('Select an existing Connector line first, then click Item Flow.');
          const candidates = state.snapshot.project.elements.filter((e) => !['Model', 'Package', 'PartProperty', 'ReferenceProperty', 'ValueProperty', 'FlowProperty', 'ConstraintProperty', 'ProxyPort', 'FullPort', 'Operation', 'Parameter', 'Reception', 'Comment'].includes(e.kind));
          const menu = candidates.map((e, i) => `${i + 1}. ${e.name} (${e.kind})`).join('\n');
          const answer = prompt(`Conveyed classifier:\n${menu}`, '1');
          if (!answer) return;
          const itemId = candidates[Number(answer) - 1]?.id;
          if (!itemId) return alert('Invalid conveyed classifier selection.');
          await runCommand('Adding Item Flow…', () => requireInvoke()('add_item_flow_to_connector', { relationshipId: relationship.id, conveyedItemIds: [itemId] }));
          await refresh();
          return;
        }
        if (item.semantic_kind === 'ProxyPort' || item.semantic_kind === 'FullPort') {
          await addExistingPortToSelectedProperty(item);
          return;
        }
        if (item.semantic_kind === 'PartProperty' || item.semantic_kind === 'ReferenceProperty') {
          alert('Part/Reference Properties are semantic features of the IBD context Block. Create them on the Block (for example from the BDD Properties/palette), then use the IBD population workflow so the IBD presents the same semantic property rather than creating a duplicate.');
        }
      };
      section.appendChild(button);
    }
    host.appendChild(section);
  }
};

async function createIbdForSelectedBlock() {
  const project = state.snapshot?.project;
  if (!project) return;
  const block = project.elements.find((e) => e.id === state.selectedElementId && ['Block', 'AssociationBlock'].includes(e.kind));
  if (!block) return alert('Select a Block or AssociationBlock first.');
  const existing = (state.snapshot.ibd_diagrams || []).find((diagram) => diagram.context_block_id === block.id);
  if (existing) {
    await selectDiagram(existing.id);
    return;
  }
  const name = prompt('IBD name', `${block.name} Internal Structure`);
  if (!name) return;
  const ownerId = state.selectedPackageId || block.owner_id || project.root_id;
  const id = await runCommand('Creating IBD…', () => requireInvoke()('create_ibd', { contextBlockId: block.id, ownerId, name }));
  await requireInvoke()('populate_ibd_from_context', { diagramId: id });
  state.selectedDiagramId = id;
  await refresh();
}

async function routeSelectedIbd() {
  const diagram = selectedIbd();
  if (!diagram) return;
  await runCommand('Routing IBD…', () => requireInvoke()('route_ibd', { diagramId: diagram.id }));
  await refresh();
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
    $('properties').innerHTML = `<div class="property-heading">Connector</div><div class="property-help">Selected Connector. Use Item Flow in the IBD palette to add a conveyed classifier.</div><label>Stable ID<input value="${escapeAttr(relationship.external_id)}" disabled></label><button id="route-ibd-selection" class="primary">Route IBD</button>`;
    $('route-ibd-selection').onclick = routeSelectedIbd;
    return;
  }
  baseRenderPropertiesPr11();
};