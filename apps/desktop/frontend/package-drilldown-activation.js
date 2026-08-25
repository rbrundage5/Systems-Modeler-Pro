(() => {
  'use strict';

  const canvas = document.getElementById('canvas');
  if (!canvas) return;

  const DOUBLE_CLICK_WINDOW_MS = 500;
  let lastActivation = null;

  function packagePresentation(target) {
    if (!(target instanceof Element)) return null;
    return target.closest('.package-diagram [data-presentation-id]');
  }

  function semanticElementFor(presentation) {
    const state = window.smpState;
    const diagram = state?.snapshot?.diagrams?.find(
      (candidate) => String(candidate.id) === String(state.selectedDiagramId)
        && candidate.family === 'package',
    );
    const node = diagram?.nodes?.find(
      (candidate) => String(candidate.id) === String(presentation.dataset.presentationId),
    );
    return state?.snapshot?.project?.elements?.find(
      (candidate) => String(candidate.id) === String(node?.element_id),
    ) || null;
  }

  function hasExistingDrillDownTarget(element) {
    if (!element) return false;
    const state = window.smpState;
    const currentDiagramId = String(state?.selectedDiagramId || '');
    const elementId = String(element.id);

    if ((state?.snapshot?.diagrams || []).some((diagram) =>
      String(diagram.id) !== currentDiagramId
      && (String(diagram.owner_id || '') === elementId
        || String(diagram.semantic_context_id || '') === elementId))) return true;

    if ((state?.snapshot?.ibd_diagrams || []).some(
      (diagram) => String(diagram.context_block_id || '') === elementId,
    )) return true;

    if ((state?.behaviorSnapshot?.diagrams || []).some(
      (diagram) => String(diagram.context_id || '') === elementId,
    )) return true;

    return (state?.activitySnapshot?.diagrams || []).some((diagram) =>
      String(diagram.context_id || diagram.owner_id || '') === elementId);
  }

  // A normal Package selection re-renders its presentation on the first click.
  // Native dblclick therefore cannot reliably target the same DOM node twice.
  // Track the stable presentation id across that re-render and delegate the
  // second click to the presentation's existing drill-down handler.
  canvas.addEventListener('click', (event) => {
    const presentation = packagePresentation(event.target);
    if (!presentation
      || window.smpState?.pendingRelationship
      || window.smpState?.paletteTool) {
      lastActivation = null;
      return;
    }

    const key = String(presentation.dataset.presentationId || '');
    if (!key) return;
    const now = performance.now();
    const isDoubleActivation = lastActivation
      && lastActivation.key === key
      && now - lastActivation.at <= DOUBLE_CLICK_WINDOW_MS;
    lastActivation = { key, at: now };
    if (!isDoubleActivation) return;

    lastActivation = null;
    event.preventDefault();
    event.stopImmediatePropagation();

    const element = semanticElementFor(presentation);
    if (!hasExistingDrillDownTarget(element)) {
      window.smpDialogs?.notify?.(
        `No existing diagram is owned by or context-bound to ${element?.name || 'this element'}.`,
        'info',
      );
      return;
    }

    if (typeof presentation.ondblclick !== 'function') {
      window.smpDialogs?.notify?.('Drill-down is unavailable for this presentation.', 'warning');
      return;
    }

    Promise.resolve(presentation.ondblclick(event)).catch((error) => {
      window.smpDialogs?.notify?.(error?.message || String(error), 'error');
    });
  }, true);
})();
