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

  let drag = null;
  let suppressNextClick = false;

  window.addEventListener('blur', cleanup);

  const RESIZE_CONTROL_SELECTOR = '.smp-resize-handle, .smp-svg-resize-handle, .lifeline-resize-handle, .sysml-frame-resize';

  function presentationTarget(target) {
    return target?.closest?.(
      '[data-smp-presentation-id], .bdd-block, .ibd-property, .ibd-port, '
      + '.activity-node, .activity-edge, [data-vertex-id], [data-transition-id], '
      + '[data-lifeline-id], [data-message-id], [data-execution-id], [data-fragment-id], '
      + `[data-invariant-id], ${RESIZE_CONTROL_SELECTOR}, .sysml-frame-label`,
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

  // An explicit resize handle is stronger user intent than a pending placement or
  // relationship tool. Cancel only those transient authoring modes before the
  // established family resize handler receives the pointerdown. This prevents
  // stale cross-family tool state from silently disabling resize while retaining
  // the existing BDD/IBD/Parametric/Behavior/Activity resize implementations.
  canvas.addEventListener('pointerdown', (event) => {
    if (!event.target.closest?.(RESIZE_CONTROL_SELECTOR)) return;
    Object.assign(state, {
      paletteTool: null,
      pendingRelationship: null,
      behaviorTool: null,
      behaviorPending: null,
      activityTool: null,
      activityPendingFlow: null,
    });
  }, true);

  // ESC and other authoritative clear operations emit this from the shared host.
  canvas.addEventListener('smp:selection-changed', () => {
    window.smpStandardEditing?.setSelections?.([]);
  });

  // A renderer's ordinary empty-frame click clears selection. Suppress only the
  // compatibility click generated after a completed marquee drag so the newly
  // selected set survives. Normal empty-canvas clicks continue to clear.
  canvas.addEventListener('click', (event) => {
    if (!suppressNextClick) return;
    suppressNextClick = false;
    event.preventDefault();
    event.stopImmediatePropagation();
  }, true);

  // Space/Ctrl/Meta panning is intercepted by the shared workspace capture
  // handler before this bubble listener. Marquee therefore does not own another
  // keyboard controller and cannot compete with viewport panning.
  //
  // Critically, do not capture the pointer here. Palette click-to-place relies on
  // the family renderer receiving the ordinary click on its SVG/frame. Pointer
  // capture begins only after the user has actually crossed the marquee threshold.
  canvas.addEventListener('pointerdown', (event) => {
    if (event.button !== 0
      || event.ctrlKey
      || event.metaKey
      || canvas.classList.contains('pan-active')
      || canvas.classList.contains('is-panning')
      || !emptyDiagramSurface(event.target)) return;

    drag = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      lastX: event.clientX,
      lastY: event.clientY,
      moved: false,
      additive: event.shiftKey,
      box: null,
    };
  }, false);

  canvas.addEventListener('pointermove', (event) => {
    if (!drag || event.pointerId !== drag.pointerId) return;
    drag.lastX = event.clientX;
    drag.lastY = event.clientY;
    if (!drag.moved && Math.hypot(drag.lastX - drag.startX, drag.lastY - drag.startY) >= 4) {
      drag.moved = true;
      const box = document.createElement('div');
      box.className = 'smp-marquee-selection';
      document.body.appendChild(box);
      drag.box = box;
      canvas.setPointerCapture?.(event.pointerId);
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
    completed.box?.remove();
    if (!completed.moved) return;

    event.preventDefault();
    event.stopPropagation();
    const rect = bounds(completed.startX, completed.startY, event.clientX, event.clientY);
    const hits = selectedPresentations(rect);
    const existing = completed.additive
      ? (window.smpStandardEditing?.selections?.() || [])
      : [];
    window.smpStandardEditing?.setSelections?.([...existing, ...hits]);
    suppressNextClick = true;
  }, false);

  canvas.addEventListener('pointercancel', cleanup, false);
})();
