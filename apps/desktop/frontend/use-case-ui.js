(() => {
  'use strict';

  const USE_CASE_KINDS = new Set(['Actor', 'UseCase']);
  const USE_CASE_RELATIONSHIPS = new Set(['Association', 'Include', 'Extend', 'Generalization']);

  function selectedUseCaseDiagram() {
    return (state.snapshot?.diagrams || []).find(
      (diagram) => diagram.id === state.selectedDiagramId && diagram.family === 'use-case',
    );
  }

  const baseLoadPalette = loadPalette;
  loadPalette = async function loadUseCasePalette() {
    const diagram = (state.snapshot?.diagrams || []).find((item) => item.id === state.selectedDiagramId);
    if (diagram?.family !== 'use-case') return baseLoadPalette();
    Object.assign(state, { paletteItems: await requireInvoke()('diagram_palette', { diagramType: 'UseCase' }) });
  };

  function actorMarkup(element) {
    return `<svg class="actor-figure" viewBox="0 0 80 105" aria-hidden="true">
      <circle cx="40" cy="17" r="13"></circle>
      <path d="M40 30 V66 M16 43 H64 M40 66 L19 94 M40 66 L61 94"></path>
    </svg><div class="actor-name">${escapeHtml(element.name)}</div>`;
  }

  function useCaseMarkup(element) {
    const points = element.extension_points || [];
    return `<div class="use-case-name">${escapeHtml(element.name)}</div>${points.length ? `<div class="extension-point-compartment"><div class="compartment-title">extension points</div>${points.map((point) => `<div>${escapeHtml(point)}</div>`).join('')}</div>` : ''}`;
  }

  function subjectBoundary(frame, diagram, project) {
    const useCaseNodes = (diagram.nodes || []).filter((node) => project.elements.find(
      (element) => element.id === node.element_id && element.kind === 'UseCase',
    ));
    if (!useCaseNodes.length) return;
    const padding = 42;
    const left = Math.max(35, Math.min(...useCaseNodes.map((node) => node.x)) - padding);
    const top = Math.max(58, Math.min(...useCaseNodes.map((node) => node.y)) - padding);
    const right = Math.max(...useCaseNodes.map((node) => node.x + node.width)) + padding;
    const bottom = Math.max(...useCaseNodes.map((node) => node.y + node.height)) + padding;
    const context = project.elements.find((element) => element.id === diagram.semantic_context_id);
    const boundary = document.createElement('section');
    boundary.className = 'use-case-subject-boundary';
    boundary.style.left = `${left}px`;
    boundary.style.top = `${top}px`;
    boundary.style.width = `${Math.max(280, right - left)}px`;
    boundary.style.height = `${Math.max(220, bottom - top)}px`;
    boundary.innerHTML = `<header>${escapeHtml(context?.name || diagram.name)}</header>`;
    frame.appendChild(boundary);
  }

  async function completeRelationship(diagram, targetElement) {
    const pending = { ...state.pendingRelationship };
    if (!pending.sourceElementId) {
      state.pendingRelationship.sourceElementId = targetElement.id;
      Object.assign(state, { selectedElementId: targetElement.id });
      render();
      return;
    }
    if (pending.sourceElementId === targetElement.id) return;
    Object.assign(state, { pendingRelationship: null });
    await runCommand(`Creating ${pending.kind}…`, () => requireInvoke()('create_use_case_relationship', {
      diagramId: diagram.id,
      kind: pending.kind,
      sourceElementId: pending.sourceElementId,
      targetElementId: targetElement.id,
      condition: null,
      extensionLocation: null,
    }));
    Object.assign(state, { selectedElementId: targetElement.id });
    await refresh();
  }

  function renderExtendDetails(frame, diagram, project) {
    const svg = frame.querySelector('.relationship-layer');
    if (!svg) return;
    const relationships = new Map(project.relationships.map((relationship) => [relationship.id, relationship]));
    for (const edge of diagram.edges || []) {
      const relationship = relationships.get(edge.relationship_id);
      if (relationship?.kind !== 'Extend' || !edge.points?.length) continue;
      const details = [
        relationship.extension_condition ? `[${relationship.extension_condition}]` : '',
        relationship.extension_location ? `at ${relationship.extension_location}` : '',
      ].filter(Boolean).join(' ');
      if (!details) continue;
      const anchor = edge.label_anchor || edge.points[Math.floor(edge.points.length / 2)];
      const label = document.createElementNS(SVG_NS, 'text');
      label.classList.add('relationship-label', 'extend-detail-label');
      label.setAttribute('x', String(anchor.x));
      label.setAttribute('y', String(anchor.y + 14));
      label.setAttribute('text-anchor', 'middle');
      label.textContent = details;
      svg.appendChild(label);
    }
  }

  const baseRenderCanvas = renderCanvas;
  renderCanvas = function renderUseCaseCanvas() {
    const diagram = selectedUseCaseDiagram();
    if (!diagram) return baseRenderCanvas();
    const canvas = $('canvas');
    const project = state.snapshot?.project;
    canvas.innerHTML = '';
    const frame = document.createElement('div');
    frame.className = 'diagram-frame use-case-diagram';
    frame.innerHTML = `<div class="diagram-header">uc [package] ${escapeHtml(diagram.name)}</div>`;
    canvas.appendChild(frame);
    subjectBoundary(frame, diagram, project);
    createRelationshipLayer(frame, diagram, project);
    renderExtendDetails(frame, diagram, project);

    frame.ondragover = (event) => {
      if (![...event.dataTransfer.types].some((type) => [
        'application/x-smp-palette-id',
        'application/x-smp-element-id',
        'application/x-smp-repository-element-id',
      ].includes(type))) return;
      event.preventDefault();
      frame.classList.add('palette-target');
    };
    frame.ondragleave = () => frame.classList.remove('palette-target');
    frame.ondrop = async (event) => {
      event.preventDefault();
      event.stopPropagation();
      frame.classList.remove('palette-target');
      const point = diagramCoordinates(frame, event);
      const paletteId = event.dataTransfer.getData('application/x-smp-palette-id');
      const elementId = event.dataTransfer.getData('application/x-smp-element-id')
        || event.dataTransfer.getData('application/x-smp-repository-element-id');
      if (paletteId) {
        const item = state.paletteItems.find((candidate) => candidate.id === paletteId);
        if (item) await createPaletteElementAt(item, point.x, point.y);
      } else if (elementId) await placeExistingElementAt(elementId, point.x, point.y);
    };

    const elements = new Map(project.elements.map((element) => [element.id, element]));
    for (const node of diagram.nodes || []) {
      const element = elements.get(node.element_id);
      if (!element || !USE_CASE_KINDS.has(element.kind)) continue;
      const presentation = document.createElement('button');
      presentation.type = 'button';
      presentation.className = `bdd-block ${element.kind === 'Actor' ? 'actor-presentation' : 'use-case-presentation'}`;
      presentation.dataset.semanticKind = element.kind;
      presentation.dataset.presentationId = node.id;
      if (state.selectedElementId === element.id) presentation.classList.add('selected');
      if (state.pendingRelationship?.sourceElementId === element.id) presentation.classList.add('relationship-source');
      Object.assign(presentation.style, {
        left: `${node.x}px`, top: `${node.y}px`, width: `${node.width}px`, height: `${node.height}px`,
      });
      presentation.innerHTML = element.kind === 'Actor' ? actorMarkup(element) : useCaseMarkup(element);
      presentation.onclick = async (event) => {
        event.stopPropagation();
        if (state.pendingRelationship) return completeRelationship(diagram, element);
        Object.assign(state, {
          selectedElementId: element.id,
          selectedRelationshipId: null,
          paletteTool: null,
        });
        render();
      };
      frame.appendChild(presentation);
    }
    frame.onclick = async (event) => {
      if (state.paletteTool?.category === 'element') {
        const point = diagramCoordinates(frame, event);
        await createPaletteElementAt(state.paletteTool, point.x, point.y);
        return;
      }
      Object.assign(state, {
        selectedElementId: null,
        selectedRelationshipId: null,
        pendingRelationship: null,
        paletteTool: null,
      });
      render();
    };
  };

  const baseCreatePaletteElementAt = createPaletteElementAt;
  createPaletteElementAt = async function createUseCasePaletteElement(item, x, y) {
    const diagram = selectedUseCaseDiagram();
    if (!diagram) return baseCreatePaletteElementAt(item, x, y);
    if (!USE_CASE_KINDS.has(item.semantic_kind)) throw new Error(`${item.label} is not a Use Case element.`);
    const definition = await window.smpDialogs?.edit({
      title: `Create ${item.label}`,
      fields: [{ id: 'name', label: `${item.label} name`, value: `New ${item.label}`, required: true }],
      confirmLabel: 'Create',
    });
    if (!definition) return;
    const elementId = await runCommand(`Creating ${item.label}…`, () => requireInvoke()('create_use_case_element', {
      kind: item.semantic_kind,
      ownerId: diagram.owner_id,
      name: definition.values.name,
    }));
    await runCommand(`Placing ${item.label}…`, () => requireInvoke()('place_on_use_case_diagram', {
      diagramId: diagram.id, elementId, x, y,
    }));
    Object.assign(state, { selectedElementId: elementId, paletteTool: null });
    await refresh();
  };

  const basePlaceExistingElementAt = placeExistingElementAt;
  placeExistingElementAt = async function placeExistingUseCaseElement(elementId, x, y) {
    const diagram = selectedUseCaseDiagram();
    if (!diagram) return basePlaceExistingElementAt(elementId, x, y);
    await runCommand('Placing existing Actor or Use Case…', () => requireInvoke()('place_on_use_case_diagram', {
      diagramId: diagram.id, elementId, x, y,
    }));
    Object.assign(state, { selectedElementId: elementId });
    await refresh();
  };

  function endpointOptions(project, relationship, side) {
    const selected = side === 'source' ? relationship.source_id : relationship.target_id;
    const otherId = side === 'source' ? relationship.target_id : relationship.source_id;
    const other = project.elements.find((element) => element.id === otherId);
    return project.elements.filter((element) => {
      if (!USE_CASE_KINDS.has(element.kind)) return false;
      if (relationship.kind === 'Association') return element.kind !== other?.kind;
      if (relationship.kind === 'Generalization') return element.kind === other?.kind;
      return element.kind === 'UseCase';
    }).map((element) => `<option value="${escapeAttr(element.id)}"${element.id === selected ? ' selected' : ''}>${escapeHtml(element.name)} (${element.kind})</option>`).join('');
  }

  function renderUseCaseRelationshipProperties(panel, project, relationship) {
    const extended = project.elements.find((element) => element.id === relationship.target_id);
    const extensionPoints = (extended?.extension_points || []).map(
      (point) => `<option value="${escapeAttr(point)}"${point === relationship.extension_location ? ' selected' : ''}>${escapeHtml(point)}</option>`,
    ).join('');
    panel.innerHTML = `<div class="property-heading">${escapeHtml(relationship.kind)}</div>
      <label>Source<select id="uc-relationship-source">${endpointOptions(project, relationship, 'source')}</select></label>
      <button id="apply-uc-source" class="primary">Reconnect source</button>
      <label>Target<select id="uc-relationship-target">${endpointOptions(project, relationship, 'target')}</select></label>
      <button id="apply-uc-target" class="primary">Reconnect target</button>
      ${relationship.kind === 'Extend' ? `<label>Condition<input id="extend-condition" value="${escapeAttr(relationship.extension_condition || '')}"></label><label>Extension point<select id="extend-location"><option value="">None</option>${extensionPoints}</select></label><button id="apply-extend" class="primary">Apply Extend</button>` : ''}
      <label>Stable ID<input value="${escapeAttr(relationship.external_id)}" disabled></label>
      <button id="delete-uc-relationship" class="danger">Delete relationship</button>`;
    for (const side of ['source', 'target']) {
      $(`apply-uc-${side}`).onclick = async () => {
        await runCommand(`Reconnecting ${side}…`, () => requireInvoke()('reconnect_use_case_relationship', {
          diagramId: state.selectedDiagramId,
          relationshipId: relationship.id,
          side,
          elementId: $(`uc-relationship-${side}`).value,
        }));
        await refresh();
      };
    }
    if (relationship.kind === 'Extend') {
      $('apply-extend').onclick = async () => {
        await runCommand('Updating Extend specification…', () => requireInvoke()('update_extend_specification', {
          relationshipId: relationship.id,
          condition: $('extend-condition').value,
          extensionLocation: $('extend-location').value,
        }));
        await refresh();
      };
    }
    $('delete-uc-relationship').onclick = async () => {
      if (!confirm(`Delete ${relationship.kind} relationship?`)) return;
      await runCommand('Deleting Use Case relationship…', () => requireInvoke()('delete_use_case_relationship', {
        relationshipId: relationship.id,
      }));
      Object.assign(state, { selectedRelationshipId: null });
      await refresh();
    };
  }

  const baseRenderProperties = renderProperties;
  renderProperties = function renderUseCaseProperties() {
    const diagram = selectedUseCaseDiagram();
    if (!diagram) return baseRenderProperties();
    const panel = $('properties');
    const project = state.snapshot?.project;
    const relationship = project.relationships.find((candidate) => candidate.id === state.selectedRelationshipId);
    if (relationship && USE_CASE_RELATIONSHIPS.has(relationship.kind)) {
      return renderUseCaseRelationshipProperties(panel, project, relationship);
    }
    const element = project.elements.find((candidate) => candidate.id === state.selectedElementId);
    if (!element || !USE_CASE_KINDS.has(element.kind)) {
      panel.innerHTML = '<div class="muted">Select an Actor, Use Case, or relationship.</div>';
      return;
    }
    if (element.kind === 'Actor') {
      panel.innerHTML = `<div class="property-heading">Actor</div><label>Name<input id="uc-element-name" value="${escapeAttr(element.name)}"></label><label>Documentation<textarea id="uc-documentation" rows="7">${escapeHtml(element.documentation || '')}</textarea></label><label>Stable ID<input value="${escapeAttr(element.external_id)}" disabled></label><button id="apply-actor" class="primary">Apply Actor</button>`;
      $('apply-actor').onclick = async () => {
        await runCommand('Updating Actor…', () => requireInvoke()('update_actor_details', {
          elementId: element.id,
          name: $('uc-element-name').value,
          documentation: $('uc-documentation').value,
        }));
        await refresh();
      };
      return;
    }
    const subjects = project.elements.filter((candidate) => [
      'Block', 'AssociationBlock', 'InterfaceBlock', 'ConstraintBlock',
    ].includes(candidate.kind)).map((candidate) => `<option value="${escapeAttr(candidate.id)}"${candidate.id === element.represented_classifier_id ? ' selected' : ''}>${escapeHtml(candidate.name)} (${candidate.kind})</option>`).join('');
    panel.innerHTML = `<div class="property-heading">Use Case</div>
      <label>Name<input id="uc-element-name" value="${escapeAttr(element.name)}"></label>
      <label>Documentation<textarea id="uc-documentation" rows="4">${escapeHtml(element.documentation || '')}</textarea></label>
      <label>Specification<textarea id="uc-specification" rows="8">${escapeHtml(element.use_case_specification || '')}</textarea></label>
      <label>Extension points<textarea id="uc-extension-points" rows="5" placeholder="One named extension point per line">${escapeHtml((element.extension_points || []).join('\n'))}</textarea></label>
      <label>Represented subject<select id="uc-subject"><option value="">None</option>${subjects}</select></label>
      <label>Stable ID<input value="${escapeAttr(element.external_id)}" disabled></label>
      <button id="apply-use-case" class="primary">Apply Use Case</button>`;
    $('apply-use-case').onclick = async () => {
      await runCommand('Updating Use Case specification…', () => requireInvoke()('update_use_case_specification', {
        elementId: element.id,
        name: $('uc-element-name').value,
        documentation: $('uc-documentation').value,
        specification: $('uc-specification').value,
        extensionPoints: $('uc-extension-points').value.split(/\r?\n/),
        representedClassifierId: $('uc-subject').value || null,
      }));
      await refresh();
    };
  };

  const baseRenderContext = renderContext;
  renderContext = function renderUseCaseContext() {
    const diagram = selectedUseCaseDiagram();
    if (!diagram) return baseRenderContext();
    $('active-diagram-summary').textContent = `${diagram.name} · Use Case Diagram`;
    $('palette-title').textContent = 'Elements (Use Case)';
  };

  const baseRenderStatus = renderStatus;
  renderStatus = function renderUseCaseStatus(message) {
    const diagram = selectedUseCaseDiagram();
    if (!diagram) return baseRenderStatus(message);
    if (message) $('status').textContent = message;
    else if (state.pendingRelationship) $('status').textContent = state.pendingRelationship.sourceElementId
      ? `${state.pendingRelationship.kind}: source selected. Click the validated target.`
      : `${state.pendingRelationship.kind}: click source, then target.`;
    else $('status').textContent = `${state.snapshot.project.name} · Use Case: ${diagram.name}`;
    $('model-counts').textContent = `Elements: ${state.snapshot.project.elements.length}   Relationships: ${state.snapshot.project.relationships.length}   Diagram: ${diagram.name} (UC)`;
  };

  async function createUseCaseDiagram() {
    if (!state.snapshot?.project) return window.smpDialogs?.notify?.('Create a project first.', 'warning');
    const ownerId = state.selectedPackageId || state.snapshot.project.root_id;
    const candidates = state.snapshot.project.elements.filter((element) => [
      'Block', 'AssociationBlock', 'InterfaceBlock', 'ConstraintBlock',
    ].includes(element.kind)).map((element) => ({ id: element.id, label: `${element.name} (${element.kind})` }));
    const definition = await window.smpDialogs?.choose({
      title: 'Create Use Case Diagram',
      description: 'Optionally select the represented system subject. Repository ownership remains separate.',
      fields: [{ id: 'name', label: 'Diagram name', value: 'System Use Cases', required: true }],
      candidates,
      confirmLabel: 'Create',
    });
    if (!definition) return;
    const selectedDiagramId = await runCommand('Creating Use Case Diagram…', () => requireInvoke()('create_use_case_diagram', {
      ownerId,
      name: definition.values.name,
      semanticContextId: definition.selectedId || null,
    }));
    Object.assign(state, {
      selectedDiagramId,
      selectedRelationshipId: null,
      pendingRelationship: null,
      paletteTool: null,
    });
    await refresh();
    await selectDiagram(selectedDiagramId);
  }

  window.smpCreateUseCaseDiagram = createUseCaseDiagram;
})();
