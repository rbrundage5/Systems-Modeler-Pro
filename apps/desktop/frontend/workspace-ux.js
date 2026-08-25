(() => {
  let candidate = null;
  let active = null;
  let ghost = null;
  let targetFrame = null;
  const STRUCTURAL_DRAG_KINDS = new Set([
    'Block', 'InterfaceBlock', 'ValueType', 'DataType', 'Enumeration',
    'ConstraintBlock', 'Package', 'ModelLibrary', 'Comment',
  ]);

  function makeInternalDrags() {
    document.querySelectorAll('[draggable="true"]').forEach((node) => {
      node.dataset.smpInternalDrag = 'true';
      node.draggable = false;
    });
  }

  const observer = new MutationObserver(makeInternalDrags);
  observer.observe(document.documentElement, {
    childList: true,
    subtree: true,
    attributes: true,
    attributeFilter: ['draggable'],
  });
  makeInternalDrags();

  function payloadFor(source) {
    if (source.classList.contains('palette-item')) {
      const label = source.querySelector('span:last-child')?.textContent?.trim();
      const item = state.paletteItems.find((value) => value.label === label);
      if (!item || item.category !== 'element') return null;
      return { type: 'palette', item, label: item.label };
    }
    if (source.classList.contains('tree-row')) {
      source.click();
      const elementId = state.selectedElementId || state.selectedPackageId;
      const element = state.snapshot?.project?.elements?.find((value) => value.id === elementId);
      if (!element || element.kind === 'Model'
        || !(element.packageable || element.kind === 'Comment' || STRUCTURAL_DRAG_KINDS.has(element.kind))) return null;
      return { type: 'repository', elementId, label: element.name };
    }
    return null;
  }

  function createGhost(label) {
    const node = document.createElement('div');
    node.className = 'modeler-drag-ghost';
    node.textContent = label;
    document.body.appendChild(node);
    return node;
  }

  function moveGhost(event) {
    if (!ghost) return;
    ghost.style.transform = `translate(${event.clientX + 14}px, ${event.clientY + 14}px)`;
  }

  function frameAt(x, y) {
    const hit = document.elementFromPoint(x, y);
    return hit instanceof Element ? hit.closest('.diagram-frame') : null;
  }

  function setTarget(frame) {
    if (targetFrame === frame) return;
    targetFrame?.classList.remove('palette-target');
    targetFrame?.classList.remove('repository-drop-target');
    targetFrame = frame;
    targetFrame?.classList.add(
      targetFrame.dataset.repositoryOwnerId ? 'repository-drop-target' : 'palette-target',
    );
  }

  function cleanup() {
    candidate = null;
    active = null;
    ghost?.remove();
    ghost = null;
    setTarget(null);
    document.body.classList.remove('modeler-dragging');
  }

  document.addEventListener('pointerdown', (event) => {
    if (event.button !== 0) return;
    const source = event.target instanceof Element
      ? event.target.closest('[data-smp-internal-drag="true"]')
      : null;
    if (!source) return;
    candidate = {
      source,
      x: event.clientX,
      y: event.clientY,
      pointerId: event.pointerId,
    };
  }, true);

  document.addEventListener('pointermove', (event) => {
    if (!candidate && !active) return;
    if (!active) {
      const distance = Math.hypot(event.clientX - candidate.x, event.clientY - candidate.y);
      if (distance < 5) return;
      const payload = payloadFor(candidate.source);
      if (!payload) {
        cleanup();
        return;
      }
      active = payload;
      candidate = null;
      ghost = createGhost(payload.label);
      document.body.classList.add('modeler-dragging');
    }
    event.preventDefault();
    moveGhost(event);
    const hit = document.elementFromPoint(event.clientX, event.clientY);
    const repositoryTarget = active.type === 'repository' && hit instanceof Element
      ? hit.closest('[data-repository-owner-id]')
      : null;
    setTarget(repositoryTarget || frameAt(event.clientX, event.clientY));
  }, true);

  document.addEventListener('pointerup', async (event) => {
    if (!active) {
      candidate = null;
      return;
    }
    event.preventDefault();
    const payload = active;
    const hit = document.elementFromPoint(event.clientX, event.clientY);
    const repositoryTarget = payload.type === 'repository' && hit instanceof Element
      ? hit.closest('[data-repository-owner-id]')
      : null;
    const frame = frameAt(event.clientX, event.clientY);
    try {
      if (repositoryTarget) {
        await requireInvoke()('move_repository_element', {
          elementId: payload.elementId,
          newOwnerId: repositoryTarget.dataset.repositoryOwnerId,
        });
        Object.assign(state, {
          selectedElementId: payload.elementId,
          selectedPackageId: null,
          selectedRelationshipId: null,
        });
        await refresh();
      } else if (frame) {
        const point = diagramCoordinates(frame, event);
        if (payload.type === 'palette') {
          await createPaletteElementAt(payload.item, point.x, point.y);
        } else {
          await placeExistingElementAt(payload.elementId, point.x, point.y);
        }
      }
    } catch (error) {
      console.error(error);
      window.smpDialogs?.notify?.(error?.message || String(error), 'error');
    } finally {
      cleanup();
    }
  }, true);

  document.addEventListener('pointercancel', cleanup, true);
  window.addEventListener('blur', cleanup);
})();

// Package Diagram reuses the shared workspace/palette/selection systems while
// every semantic mutation and final route remains Rust-authoritative.
window.addEventListener('DOMContentLoaded', () => {
  'use strict';

  const PACKAGE_NODE_KINDS = new Set(['Package', 'ModelLibrary', 'Comment']);
  const PACKAGE_NAMESPACES = new Set(['Model', 'Package', 'ModelLibrary']);
  const PACKAGE_RELATIONSHIPS = new Set([
    'PackageImport', 'ElementImport', 'PackageMerge', 'Dependency',
  ]);

  const style = document.createElement('style');
  style.textContent = `
    .package-diagram .package-node{overflow:visible!important;background:#fbfbf8!important;border:1.5px solid #222!important;border-radius:1px!important;text-align:left!important;padding:26px 12px 10px!important;box-sizing:border-box!important}
    .package-diagram .package-node::before{content:"";position:absolute;left:-1.5px;top:-19px;width:76px;height:20px;background:#fbfbf8;border:1.5px solid #222;border-bottom:0;border-radius:3px 8px 0 0;box-sizing:border-box}
    .package-diagram .package-node-stereotype{display:block;min-height:14px;font-size:11px;text-align:center;color:#444;margin-bottom:3px}
    .package-diagram .package-node-name{display:block;font-weight:700;text-align:center;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
    .package-diagram .package-node.selected,.package-diagram .package-comment.selected{outline:2px solid currentColor;outline-offset:2px}
    .package-diagram .package-comment{position:absolute;box-sizing:border-box;border:1.5px solid #555;background:#fffbe8;padding:12px;text-align:left;white-space:pre-wrap;overflow:hidden}
    .package-diagram .package-comment::after{content:"";position:absolute;right:-1px;top:-1px;width:16px;height:16px;background:linear-gradient(225deg,#fff 49%,#777 50%,#777 54%,#fffbe8 55%)}
    .package-diagram .package-hint{position:absolute;left:24px;top:54px;color:#666;font-size:12px;pointer-events:none}
    .package-diagram .relationship-packageimport,.package-diagram .relationship-elementimport,.package-diagram .relationship-packagemerge,.package-diagram .relationship-dependency{stroke-dasharray:7 5}
    .package-diagram .relationship-label{paint-order:stroke;stroke:#fff;stroke-width:4px;stroke-linejoin:round}
    .package-properties .technical-id{font-size:11px;color:#666;overflow-wrap:anywhere}
  `;
  document.head.appendChild(style);

  const selectedPackageDiagram = () => state.snapshot?.diagrams?.find(
    (diagram) => diagram.id === state.selectedDiagramId && diagram.family === 'package',
  ) || null;

  const elementById = (id) => state.snapshot?.project?.elements?.find(
    (element) => String(element.id) === String(id),
  ) || null;

  const qualifiedLabel = (element) => element
    ? `${element.qualified_name || element.name} (${element.kind})`
    : 'Missing semantic element';

  const packagePresentable = (element) => Boolean(
    element && (element.packageable || element.kind === 'Comment'),
  );

  function legalPackageEndpoint(kind, side, element) {
    if (!element) return false;
    if (kind === 'ElementImport') {
      return side === 'source' ? PACKAGE_NAMESPACES.has(element.kind) : Boolean(element.packageable);
    }
    if (kind === 'PackageMerge') {
      return ['Package', 'ModelLibrary'].includes(element.kind);
    }
    return PACKAGE_NAMESPACES.has(element.kind);
  }

  const baseRenderPalette = renderPalette;
  renderPalette = function renderPackagePalette() {
    const diagram = selectedPackageDiagram();
    if (!diagram) return baseRenderPalette();
    baseRenderPalette();
    $('palette').querySelectorAll('.palette-section-title').forEach((title) => {
      if (title.textContent === 'Classifiers') title.textContent = 'Package Elements';
    });
    const hint = $('palette').querySelector('.palette-hint');
    if (hint) {
      hint.textContent = 'Drag Package, Model Library, or Comment onto the canvas. Select a relationship, then choose its source and target.';
    }
  };

  const baseCreatePaletteElementAt = createPaletteElementAt;
  createPaletteElementAt = async function createPackagePaletteElementAt(item, x, y) {
    const diagram = selectedPackageDiagram();
    if (!diagram) return baseCreatePaletteElementAt(item, x, y);
    if (item.category !== 'element' || !PACKAGE_NODE_KINDS.has(item.semantic_kind)) {
      throw new Error(`${item.label} is not a creatable Package Diagram element.`);
    }
    const definition = await window.smpDialogs?.edit({
      title: `Create ${item.label}`,
      fields: [{
        id: 'name',
        label: item.semantic_kind === 'Comment' ? 'Comment title' : 'Name',
        value: item.semantic_kind === 'Comment' ? 'Comment' : `New ${item.label}`,
        required: true,
      }],
      confirmLabel: 'Create',
    });
    if (!definition) return;
    const elementId = await runCommand(`Creating ${item.label}…`, () => requireInvoke()('create_package_element', {
      diagramId: diagram.id,
      kind: item.semantic_kind,
      name: definition.values.name,
      x,
      y,
    }));
    Object.assign(state, {
      selectedElementId: elementId,
      selectedPackageId: null,
      selectedRelationshipId: null,
      paletteTool: null,
    });
    await refresh();
  };

  const basePlaceExistingElementAt = placeExistingElementAt;
  placeExistingElementAt = async function placeExistingPackageElement(elementId, x, y) {
    const diagram = selectedPackageDiagram();
    if (!diagram) return basePlaceExistingElementAt(elementId, x, y);
    const element = elementById(elementId);
    if (!packagePresentable(element)) {
      throw new Error(`${qualifiedLabel(element)} cannot be presented on a Package Diagram.`);
    }
    await runCommand(`Placing existing ${element.kind}…`, () => requireInvoke()('place_on_package_diagram', {
      diagramId: diagram.id,
      elementId,
      x,
      y,
    }));
    Object.assign(state, {
      selectedElementId: elementId,
      selectedPackageId: null,
      selectedRelationshipId: null,
    });
    await refresh();
  };

  async function relationshipOptions(kind) {
    if (!['PackageImport', 'ElementImport'].includes(kind)) {
      return { visibility: 'public', alias: null };
    }
    const definition = await window.smpDialogs?.edit({
      title: kind === 'PackageImport' ? 'Create Package Import' : 'Create Element Import',
      description: 'Visibility must be public or private.',
      fields: [
        { id: 'visibility', label: 'Visibility', value: 'public', required: true },
        ...(kind === 'ElementImport'
          ? [{ id: 'alias', label: 'Alias (optional)', value: '' }]
          : []),
      ],
      confirmLabel: 'Create',
    });
    if (!definition) return null;
    return {
      visibility: definition.values.visibility,
      alias: definition.values.alias || null,
    };
  }

  async function selectPackageNode(element) {
    const diagram = selectedPackageDiagram();
    if (!diagram) return;
    if (state.pendingRelationship) {
      const side = state.pendingRelationship.sourceElementId ? 'target' : 'source';
      if (!legalPackageEndpoint(state.pendingRelationship.kind, side, element)) {
        window.smpDialogs?.notify?.(
          `${element.name} (${element.kind}) is not a valid Package relationship endpoint.`,
          'error',
        );
        return;
      }
      if (!state.pendingRelationship.sourceElementId) {
        state.pendingRelationship.sourceElementId = element.id;
        state.selectedElementId = element.id;
        render();
        return;
      }
      if (state.pendingRelationship.sourceElementId === element.id) {
        window.smpDialogs?.notify?.('Choose a different target element.', 'warning');
        return;
      }
      const pending = { ...state.pendingRelationship };
      const options = await relationshipOptions(pending.kind);
      if (!options) return;
      state.pendingRelationship = null;
      const relationshipId = await runCommand(
        `Creating ${pending.kind}…`,
        () => requireInvoke()('create_package_relationship', {
          diagramId: diagram.id,
          kind: pending.kind,
          sourceElementId: pending.sourceElementId,
          targetElementId: element.id,
          visibility: options.visibility,
          alias: options.alias,
        }),
      );
      Object.assign(state, {
        selectedElementId: null,
        selectedRelationshipId: relationshipId,
        paletteTool: null,
      });
      await refresh();
      return;
    }
    Object.assign(state, {
      selectedElementId: element.id,
      selectedPackageId: null,
      selectedRelationshipId: null,
      paletteTool: null,
    });
    render();
  }

  function renderPackageCanvas() {
    const diagram = selectedPackageDiagram();
    if (!diagram) return;
    const canvas = $('canvas');
    const project = state.snapshot?.project;
    if (!project) return;
    const owner = elementById(diagram.owner_id);
    const elements = new Map(project.elements.map((element) => [element.id, element]));
    canvas.innerHTML = '';
    const frame = document.createElement('div');
    frame.className = 'diagram-frame package-diagram';
    frame.innerHTML = `<div class="diagram-header">pkg [package] ${escapeHtml(owner?.name || project.name)} [${escapeHtml(diagram.name)}]</div>`;
    canvas.appendChild(frame);
    createRelationshipLayer(frame, diagram, project);

    if (!diagram.nodes?.length) {
      const hint = document.createElement('div');
      hint.className = 'package-hint';
      hint.textContent = 'Drag a Package, Model Library, or Comment from the palette or repository.';
      frame.appendChild(hint);
    }

    for (const node of diagram.nodes || []) {
      const element = elements.get(node.element_id);
      if (!packagePresentable(element)) continue;
      const presentation = document.createElement('button');
      presentation.type = 'button';
      presentation.className = element.kind === 'Comment'
        ? 'bdd-block package-comment'
        : ['Package', 'ModelLibrary'].includes(element.kind)
          ? 'bdd-block package-node'
          : 'bdd-block packageable-node';
      presentation.dataset.semanticKind = element.kind;
      presentation.dataset.presentationId = node.id;
      if (element.id === state.selectedElementId) presentation.classList.add('selected');
      if (state.pendingRelationship?.sourceElementId === element.id) {
        presentation.classList.add('relationship-source');
      }
      Object.assign(presentation.style, {
        left: `${node.x}px`,
        top: `${node.y}px`,
        width: `${node.width}px`,
        height: `${node.height}px`,
        minHeight: '0',
      });
      if (element.kind === 'Comment') {
        presentation.textContent = element.documentation || element.name;
      } else if (['Package', 'ModelLibrary'].includes(element.kind)) {
        const stereotype = element.kind === 'ModelLibrary' ? '«modelLibrary»' : '';
        presentation.innerHTML = `<span class="package-node-stereotype">${stereotype}</span><span class="package-node-name">${escapeHtml(element.name)}</span>`;
      } else {
        const stereotype = typeof classifierStereotype === 'function'
          ? classifierStereotype(element.kind)
          : element.kind;
        const compartments = typeof classifierCompartments === 'function'
          ? classifierCompartments(project, element)
          : '';
        presentation.innerHTML = `<div class="stereotype">«${escapeHtml(stereotype)}»</div><div class="block-name">${escapeHtml(element.name)}</div>${compartments}`;
      }
      presentation.onclick = async (event) => {
        event.stopPropagation();
        try {
          await selectPackageNode(element);
        } catch (error) {
          window.smpDialogs?.notify?.(error?.message || String(error), 'error');
          await refresh();
        }
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
        selectedPackageId: null,
        selectedRelationshipId: null,
        pendingRelationship: null,
        paletteTool: null,
      });
      render();
    };
  }

  function ownerOptions(project, element) {
    return project.elements
      .filter((candidate) => ['Model', 'Package', 'ModelLibrary'].includes(candidate.kind)
        && candidate.id !== element.id)
      .map((candidate) => `<option value="${escapeAttr(candidate.id)}"${candidate.id === element.owner_id ? ' selected' : ''}>${escapeHtml(qualifiedLabel(candidate))}</option>`)
      .join('');
  }

  function endpointOptions(diagram, project, selectedId, relationshipKind, side) {
    const presented = new Set(diagram.nodes.map((node) => node.element_id));
    return project.elements
      .filter((element) => presented.has(element.id)
        && legalPackageEndpoint(relationshipKind, side, element))
      .map((element) => `<option value="${escapeAttr(element.id)}"${element.id === selectedId ? ' selected' : ''}>${escapeHtml(qualifiedLabel(element))}</option>`)
      .join('');
  }

  function renderPackageElementProperties(panel, project, element) {
    panel.innerHTML = `<section class="package-properties">
      <div class="property-heading">${escapeHtml(element.kind === 'ModelLibrary' ? 'Model Library' : element.kind)}</div>
      <label>Name<input id="pkg-element-name" value="${escapeAttr(element.name)}"></label>
      <label>Documentation<textarea id="pkg-element-documentation">${escapeHtml(element.documentation || '')}</textarea></label>
      <label>Owner / namespace<select id="pkg-element-owner">${ownerOptions(project, element)}</select></label>
      <label>Qualified name<input value="${escapeAttr(element.qualified_name || element.name)}" disabled></label>
      <label>Visibility<select id="pkg-element-visibility"><option value="public"${element.visibility !== 'private' ? ' selected' : ''}>public</option><option value="private"${element.visibility === 'private' ? ' selected' : ''}>private</option></select></label>
      <label>Semantic type<input value="${escapeAttr(element.kind === 'ModelLibrary' ? 'Package «modelLibrary»' : element.kind)}" disabled></label>
      <button id="pkg-apply-element" class="primary">Apply</button>
      <details><summary>Technical information</summary><div class="technical-id">Stable ID: ${escapeHtml(element.id)}</div></details>
    </section>`;
    $('pkg-apply-element').onclick = async () => {
      await runCommand(`Updating ${element.kind}…`, () => requireInvoke()('update_package_element', {
        elementId: element.id,
        name: $('pkg-element-name').value,
        documentation: $('pkg-element-documentation').value,
        visibility: $('pkg-element-visibility').value,
        ownerId: $('pkg-element-owner').value,
      }));
      await refresh();
    };
  }

  function renderPackageRelationshipProperties(panel, diagram, project, relationship) {
    const source = elementById(relationship.source_id);
    const target = elementById(relationship.target_id);
    const importRelationship = ['PackageImport', 'ElementImport'].includes(relationship.kind);
    panel.innerHTML = `<section class="package-properties">
      <div class="property-heading">${escapeHtml(relationship.kind.replace(/([a-z])([A-Z])/g, '$1 $2'))}</div>
      <label>${relationship.kind === 'PackageMerge' ? 'Receiving package' : ['PackageImport', 'ElementImport'].includes(relationship.kind) ? 'Importing namespace' : 'Source'}<select id="pkg-relationship-source">${endpointOptions(diagram, project, relationship.source_id, relationship.kind, 'source')}</select></label>
      <div class="muted">${escapeHtml(qualifiedLabel(source))}</div>
      <label>${relationship.kind === 'PackageMerge' ? 'Merged package' : relationship.kind === 'PackageImport' ? 'Imported package' : relationship.kind === 'ElementImport' ? 'Imported element' : 'Target'}<select id="pkg-relationship-target">${endpointOptions(diagram, project, relationship.target_id, relationship.kind, 'target')}</select></label>
      <div class="muted">${escapeHtml(qualifiedLabel(target))}</div>
      <label>Name<input id="pkg-relationship-name" value="${escapeAttr(relationship.name || '')}"></label>
      <label>Documentation<textarea id="pkg-relationship-documentation">${escapeHtml(relationship.documentation || '')}</textarea></label>
      ${importRelationship ? `<label>Visibility<select id="pkg-relationship-visibility"><option value="public"${relationship.visibility !== 'private' ? ' selected' : ''}>public</option><option value="private"${relationship.visibility === 'private' ? ' selected' : ''}>private</option></select></label>` : ''}
      ${relationship.kind === 'ElementImport' ? `<label>Alias<input id="pkg-element-import-alias" value="${escapeAttr(relationship.alias || '')}"></label>` : ''}
      <button id="pkg-apply-relationship" class="primary">Apply</button>
      <button id="pkg-delete-relationship" class="danger">Delete Relationship</button>
      <details><summary>Technical information</summary><div class="technical-id">Stable ID: ${escapeHtml(relationship.id)}</div></details>
    </section>`;
    $('pkg-apply-relationship').onclick = async () => {
      const sourceId = $('pkg-relationship-source').value;
      const targetId = $('pkg-relationship-target').value;
      await runCommand(`Updating ${relationship.kind}…`, () => requireInvoke()('update_package_relationship', {
        diagramId: diagram.id,
        relationshipId: relationship.id,
        sourceElementId: sourceId,
        targetElementId: targetId,
        name: $('pkg-relationship-name').value,
        documentation: $('pkg-relationship-documentation').value,
        visibility: $('pkg-relationship-visibility')?.value || 'public',
        alias: $('pkg-element-import-alias')?.value || null,
      }));
      await refresh();
    };
    $('pkg-delete-relationship').onclick = async () => {
      const accepted = window.smpDialogs?.confirm
        ? await window.smpDialogs.confirm({
            title: `Delete ${relationship.kind}`,
            description: `Delete the semantic relationship from ${source?.name || 'source'} to ${target?.name || 'target'} and remove its presentations?`,
            confirmLabel: 'Delete Relationship',
            destructive: true,
          })
        : confirm(`Delete ${relationship.kind}?`);
      if (!accepted) return;
      await runCommand(`Deleting ${relationship.kind}…`, () => requireInvoke()('delete_package_relationship', {
        relationshipId: relationship.id,
      }));
      state.selectedRelationshipId = null;
      await refresh();
    };
  }

  const baseRenderProperties = renderProperties;
  renderProperties = function renderPackageProperties() {
    const diagram = selectedPackageDiagram();
    if (!diagram) return baseRenderProperties();
    const project = state.snapshot?.project;
    const panel = $('properties');
    panel.innerHTML = '<div class="muted">Select a Package Diagram element or relationship.</div>';
    if (!project) return;
    const relationship = project.relationships.find(
      (candidate) => candidate.id === state.selectedRelationshipId
        && PACKAGE_RELATIONSHIPS.has(candidate.kind),
    );
    if (relationship) {
      renderPackageRelationshipProperties(panel, diagram, project, relationship);
      return;
    }
    const element = project.elements.find(
      (candidate) => candidate.id === state.selectedElementId
        && PACKAGE_NODE_KINDS.has(candidate.kind),
    );
    if (element) {
      renderPackageElementProperties(panel, project, element);
      return;
    }
    if (state.selectedElementId) baseRenderProperties();
  };

  const baseRenderContext = renderContext;
  renderContext = function renderPackageContext() {
    const diagram = selectedPackageDiagram();
    if (!diagram) return baseRenderContext();
    $('active-diagram-summary').textContent = `${diagram.name} · Package Diagram`;
    $('palette-title').textContent = 'Package Diagram';
  };

  const baseRenderStatus = renderStatus;
  renderStatus = function renderPackageStatus(message) {
    const diagram = selectedPackageDiagram();
    if (!diagram) return baseRenderStatus(message);
    if (message) $('status').textContent = message;
    else $('status').textContent = state.pendingRelationship?.sourceElementId
      ? `${state.pendingRelationship.kind}: choose the target package.`
      : `${state.snapshot.project.name} · Package Diagram: ${diagram.name}`;
    $('model-counts').textContent = `Elements: ${state.snapshot.project.elements.length}   Relationships: ${state.snapshot.project.relationships.length}   Diagram: ${diagram.name} (PKG)`;
  };

  window.smpRendererHost?.registerSelectionRenderer?.(
    'package',
    ['selectedElementId', 'selectedRelationshipId'],
    ['paletteTool', 'pendingRelationship'],
    { renderCanvas: renderPackageCanvas },
  );

  render();
});
