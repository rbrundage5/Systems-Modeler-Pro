(() => {
  'use strict';

  const canvas = document.getElementById('canvas');
  if (!canvas) return;

  const style = document.createElement('style');
  style.textContent = `
    .smp-marquee-selection {
      position: fixed;
      z-index: 99990;
      border: 1px solid rgba(47,111,173,.9);
      background: rgba(47,111,173,.09);
      pointer-events: none;
      box-sizing: border-box;
    }
  `;
  document.head.appendChild(style);

  let spacePressed = false;
  let drag = null;

  document.addEventListener('keydown', (event) => {
    if (event.code === 'Space') spacePressed = true;
  }, true);
  document.addEventListener('keyup', (event) => {
    if (event.code === 'Space') spacePressed = false;
  }, true);
  window.addEventListener('blur', () => {
    spacePressed = false;
    cleanup();
  });

  function presentationTarget(target) {
    return target?.closest?.(
      '[data-smp-presentation-id], .bdd-block, .ibd-property, .ibd-port, '
      + '.activity-node, .activity-edge, [data-vertex-id], [data-transition-id], '
      + '[data-lifeline-id], [data-message-id], [data-execution-id], [data-fragment-id], '
      + '[data-invariant-id], .sysml-frame-label, .sysml-frame-resize',
    );
  }

  function emptyDiagramSurface(target) {
    if (!(target instanceof Element) || !canvas.contains(target)) return false;
    if (presentationTarget(target)) return false;
    return target === canvas
      || target.classList.contains('workspace-transform-spacer')
      || Boolean(target.closest('.diagram-frame, .relationship-layer, .workspace-renderer-surface'));
  }

  function bounds(left, top, right, bottom) {
    return {
      left: Math.min(left, right),
      top: Math.min(top, bottom),
      right: Math.max(left, right),
      bottom: Math.max(top, bottom),
    };
  }

  function intersects(left, right) {
    return left.left <= right.right
      && left.right >= right.left
      && left.top <= right.bottom
      && left.bottom >= right.top;
  }

  function renderBox() {
    if (!drag?.box) return;
    const rect = bounds(drag.startX, drag.startY, drag.lastX, drag.lastY);
    Object.assign(drag.box.style, {
      left: `${rect.left}px`,
      top: `${rect.top}px`,
      width: `${Math.max(1, rect.right - rect.left)}px`,
      height: `${Math.max(1, rect.bottom - rect.top)}px`,
    });
  }

  function cleanup() {
    drag?.box?.remove();
    drag = null;
  }

  function selectedPresentations(rect) {
    const selected = new Map();
    document.querySelectorAll('#canvas [data-smp-presentation-id]').forEach((node) => {
      if (!(node instanceof Element)) return;
      const computed = getComputedStyle(node);
      if (computed.display === 'none' || computed.visibility === 'hidden') return;
      const nodeRect = node.getBoundingClientRect();
      if (nodeRect.width <= 0 || nodeRect.height <= 0 || !intersects(rect, nodeRect)) return;
      const id = node.dataset.smpPresentationId;
      if (!id) return;
      const kind = node.dataset.smpPresentationKind || 'Presentation';
      selected.set(`${kind}:${id}`, { kind, id });
    });
    return [...selected.values()];
  }

  // The shared host emits this event for ESC/clear-selection. Keep the standard
  // multi-selection layer synchronized so stale marquee outlines never survive
  // an authoritative workspace clear.
  canvas.addEventListener('smp:selection-changed', () => {
    window.smpStandardEditing?.setSelections?.([]);
  });

  canvas.addEventListener('pointerdown', (event) => {
    if (event.button !== 0
      || spacePressed
      || event.ctrlKey
      || event.metaKey
      || canvas.classList.contains('pan-active')
      || !emptyDiagramSurface(event.target)) return;

    const box = document.createElement('div');
    box.className = 'smp-marquee-selection';
    document.body.appendChild(box);
    drag = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      lastX: event.clientX,
      lastY: event.clientY,
      moved: false,
      additive: event.shiftKey,
      box,
    };
    canvas.setPointerCapture?.(event.pointerId);
    renderBox();
  }, false);

  canvas.addEventListener('pointermove', (event) => {
    if (!drag || event.pointerId !== drag.pointerId) return;
    drag.lastX = event.clientX;
    drag.lastY = event.clientY;
    if (!drag.moved && Math.hypot(drag.lastX - drag.startX, drag.lastY - drag.startY) >= 4) {
      drag.moved = true;
    }
    if (!drag.moved) return;
    event.preventDefault();
    event.stopPropagation();
    renderBox();
  }, false);

  canvas.addEventListener('pointerup', (event) => {
    if (!drag || event.pointerId !== drag.pointerId) return;
    const completed = drag;
    drag = null;
    completed.box.remove();
    if (!completed.moved) return;

    event.preventDefault();
    event.stopPropagation();
    const rect = bounds(completed.startX, completed.startY, event.clientX, event.clientY);
    const hits = selectedPresentations(rect);
    const existing = completed.additive
      ? (window.smpStandardEditing?.selections?.() || [])
      : [];
    window.smpStandardEditing?.setSelections?.([...existing, ...hits]);
  }, false);

  canvas.addEventListener('pointercancel', cleanup, false);
})();
