(() => {
  let candidate = null;
  let active = null;
  let ghost = null;
  let targetFrame = null;
  const BDD_CLASSIFIERS = new Set(['Block', 'InterfaceBlock', 'ValueType', 'DataType', 'Enumeration', 'ConstraintBlock', 'Package']);

  function makeInternalDrags() {
    document.querySelectorAll('[draggable="true"]').forEach((node) => {
      node.dataset.smpInternalDrag = 'true';
      node.draggable = false;
    });
  }

  const observer = new MutationObserver(makeInternalDrags);
  observer.observe(document.documentElement, { childList: true, subtree: true, attributes: true, attributeFilter: ['draggable'] });
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
      if (!element || !BDD_CLASSIFIERS.has(element.kind)) return null;
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
    targetFrame = frame;
    targetFrame?.classList.add('palette-target');
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
    const source = event.target instanceof Element ? event.target.closest('[data-smp-internal-drag="true"]') : null;
    if (!source) return;
    candidate = { source, x: event.clientX, y: event.clientY, pointerId: event.pointerId };
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
    setTarget(frameAt(event.clientX, event.clientY));
  }, true);

  document.addEventListener('pointerup', async (event) => {
    if (!active) {
      candidate = null;
      return;
    }
    event.preventDefault();
    const payload = active;
    const frame = frameAt(event.clientX, event.clientY);
    try {
      if (frame) {
        const point = diagramCoordinates(frame, event);
        if (payload.type === 'palette') await createPaletteElementAt(payload.item, point.x, point.y);
        else await placeExistingElementAt(payload.elementId, point.x, point.y);
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

// PR26A Package Diagram adapter. Install after every legacy wrapper has loaded so
// Package Diagram remains a thin presentation layer over Rust-owned semantics.
window.addEventListener('DOMContentLoaded', () => {
  'use strict';

  const style = document.createElement('style');
  style.textContent = `
    .package-diagram .package-node{overflow:visible!important;background:#fbfbf8!important;border:1.5px solid #222!important;border-radius:1px!important;text-align:left!important;padding:28px 12px 10px!important;box-sizing:border-box!important}
    .package-diagram .package-node::before{content:"";position:absolute;left:-1.5px;top:-19px;width:76px;height:20px;background:#fbfbf8;border:1.5px solid #222;border-bottom:0;border-radius:3px 8px 0 0;box-sizing:border-box}
    .package-diagram .package-node-stereotype{display:block;font-size:11px;text-align:center;color:#444;margin-bottom:4px}
    .package-diagram .package-node-name{display:block;font-weight:700;text-align:center;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
    .package-diagram .package-node.selected{outline:2px solid currentColor;outline-offset:2px}
    .package-diagram .package-hint{position:absolute;left:24px;top:54px;color:#666;font-size:12px;pointer-events:none}
  `;
  document.head.appendChild(style);

  const selectedPackageDiagram = () => state.snapshot?.diagrams?.find(
    (diagram) => diagram.id === state.selectedDiagramId && diagram.family === 'package',
  ) || null;

  const baseLoadPalette = loadPalette;
  loadPalette = async function loadPackagePalette() {
    if (selectedPackageDiagram()) {
      Object.assign(state, { paletteItems: [] });
      return;
    }
    return baseLoadPalette();
  };

  const baseRenderPalette = renderPalette;
  renderPalette = function renderPackagePalette() {
    if (!selectedPackageDiagram()) return baseRenderPalette();
    const host = $('palette');
    host.innerHTML = '<div class="palette-hint"><b>Package Diagram</b><br>Drag existing Package elements from the Model Repository onto the diagram. Package relationships are intentionally deferred to the next package-semantics increment.</div>';
  };

  const basePlaceExistingElementAt = placeExistingElementAt;
  placeExistingElementAt = async function placeExistingPackageElement(elementId, x, y) {
    const diagram = selectedPackageDiagram();
    if (!diagram) return basePlaceExistingElementAt(elementId, x, y);
    const element = state.snapshot?.project?.elements?.find((candidate) => candidate.id === elementId);
    if (element?.kind !== 'Package') throw new Error('Package Diagrams can present existing Package elements only.');
    await runCommand('Placing existing Package…', () => requireInvoke()('place_on_package_diagram', {
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

  function renderPackageCanvas() {
    const diagram = selectedPackageDiagram();
    if (!diagram) return;
    const canvas = $('canvas');
    const project = state.snapshot?.project;
    if (!project) return;
    const owner = project.elements.find((element) => element.id === diagram.owner_id);
    const elements = new Map(project.elements.map((element) => [element.id, element]));
    canvas.innerHTML = '';
    const frame = document.createElement('div');
    frame.className = 'diagram-frame package-diagram';
    frame.innerHTML = `<div class="diagram-header">pkg [package] ${escapeHtml(owner?.name || project.name)} [${escapeHtml(diagram.name)}]</div>`;
    canvas.appendChild(frame);

    if (!diagram.nodes?.length) {
      const hint = document.createElement('div');
      hint.className = 'package-hint';
      hint.textContent = 'Drag Package elements from the repository onto this diagram.';
      frame.appendChild(hint);
    }

    for (const node of diagram.nodes || []) {
      const element = elements.get(node.element_id);
      if (element?.kind !== 'Package') continue;
      const presentation = document.createElement('button');
      presentation.type = 'button';
      presentation.className = 'bdd-block package-node';
      presentation.dataset.semanticKind = 'Package';
      presentation.dataset.presentationId = node.id;
      if (element.id === state.selectedElementId) presentation.classList.add('selected');
      Object.assign(presentation.style, {
        left: `${node.x}px`,
        top: `${node.y}px`,
        width: `${node.width}px`,
        height: `${node.height}px`,
      });
      presentation.innerHTML = `<span class="package-node-stereotype">«package»</span><span class="package-node-name">${escapeHtml(element.name)}</span>`;
      presentation.onclick = (event) => {
        event.stopPropagation();
        Object.assign(state, {
          selectedElementId: element.id,
          selectedPackageId: null,
          selectedRelationshipId: null,
          pendingRelationship: null,
          paletteTool: null,
        });
        render();
      };
      frame.appendChild(presentation);
    }

    frame.onclick = () => {
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

  const baseRenderContext = renderContext;
  renderContext = function renderPackageContext() {
    const diagram = selectedPackageDiagram();
    if (!diagram) return baseRenderContext();
    $('active-diagram-summary').textContent = `${diagram.name} · Package Diagram`;
    $('palette-title').textContent = 'Elements (Package)';
  };

  const baseRenderStatus = renderStatus;
  renderStatus = function renderPackageStatus(message) {
    const diagram = selectedPackageDiagram();
    if (!diagram) return baseRenderStatus(message);
    if (message) $('status').textContent = message;
    else $('status').textContent = `${state.snapshot.project.name} · Package Diagram: ${diagram.name}`;
    $('model-counts').textContent = `Elements: ${state.snapshot.project.elements.length}   Relationships: ${state.snapshot.project.relationships.length}   Diagram: ${diagram.name} (PKG)`;
  };

  const baseRenderDiagramTabs = renderDiagramTabs;
  renderDiagramTabs = function renderPackageDiagramTabs() {
    baseRenderDiagramTabs();
    const structural = state.snapshot?.diagrams || [];
    [...$('diagram-tabs').querySelectorAll('.diagram-tab')].forEach((tab, index) => {
      if (structural[index]?.family === 'package') tab.textContent = `${structural[index].name} · PKG`;
    });
  };

  const baseRenderRepository = renderRepository;
  renderRepository = function renderPackageRepository() {
    baseRenderRepository();
    const diagrams = state.snapshot?.diagrams || [];
    document.querySelectorAll('#repository .diagram-row[data-diagram-id]').forEach((row) => {
      const diagram = diagrams.find((candidate) => String(candidate.id) === String(row.dataset.diagramId));
      if (diagram?.family !== 'package') return;
      const tag = row.querySelector('.type-tag');
      if (tag) tag.textContent = 'PKG';
    });
  };

  window.smpCreatePackageDiagram = async function createPackageDiagram() {
    if (!state.snapshot?.project) {
      window.smpDialogs?.notify?.('Create a project first.', 'warning');
      return;
    }
    const project = state.snapshot.project;
    const ownerId = state.selectedPackageId || project.root_id;
    const owner = project.elements.find((element) => element.id === ownerId);
    if (!owner || !['Model', 'Package'].includes(owner.kind)) {
      window.smpDialogs?.notify?.('Select a Package or the model root before creating a Package Diagram.', 'warning');
      return;
    }
    const definition = await window.smpDialogs?.edit?.({
      title: 'Create Package Diagram',
      description: `Diagram owner: ${owner.name}`,
      fields: [{ id: 'name', label: 'Diagram name', value: `${owner.name} Packages`, required: true }],
      confirmLabel: 'Create',
    });
    if (!definition) return;
    const diagramId = await runCommand('Creating Package Diagram…', () => requireInvoke()('create_package_diagram', {
      ownerId,
      name: definition.values.name,
    }));
    Object.assign(state, {
      selectedDiagramId: diagramId,
      selectedElementId: null,
      selectedPackageId: null,
      selectedRelationshipId: null,
      pendingRelationship: null,
      paletteTool: null,
    });
    await refresh();
    await selectDiagram(diagramId);
  };

  window.smpRendererHost?.registerSelectionRenderer?.(
    'package',
    ['selectedElementId', 'selectedRelationshipId'],
    ['paletteTool', 'pendingRelationship'],
    { renderCanvas: renderPackageCanvas },
  );

  render();
});
