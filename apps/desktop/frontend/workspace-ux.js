(() => {
  let candidate = null;
  let active = null;
  let ghost = null;
  let targetFrame = null;

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
      // Reuse the row's established selection behavior so the exact semantic
      // element ID is used even when names are duplicated in different owners.
      source.click();
      const elementId = state.selectedElementId;
      const element = state.snapshot?.project?.elements?.find((value) => value.id === elementId);
      if (!element || element.kind !== 'Block') return null;
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
    } finally {
      cleanup();
    }
  }, true);

  document.addEventListener('pointercancel', cleanup, true);
  window.addEventListener('blur', cleanup);
})();
