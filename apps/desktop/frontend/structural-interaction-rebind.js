(() => {
  const style = document.createElement('style');
  style.textContent = '.smp-structural-bound{touch-action:none}.smp-structural-bound.smp-dragging{cursor:grabbing!important}';
  document.head.appendChild(style);

  function invokeCommit(command, args) {
    return runCommand('Updating diagram presentation…', () => requireInvoke()(command, args))
      .then(async () => { await refresh(); })
      .catch((error) => console.error('Structural presentation update failed', error));
  }

  function ensureHandle(node) {
    let handle = node.querySelector(':scope > .smp-resize-handle');
    if (handle) return handle;
    handle = document.createElement('span');
    handle.className = 'smp-resize-handle';
    handle.title = 'Drag to resize';
    handle.setAttribute('aria-label', 'Resize element');
    node.appendChild(handle);
    return handle;
  }

  function bind(node, config) {
    if (!node || node.dataset.smpStructuralRebind === '1') return;
    node.dataset.smpStructuralRebind = '1';
    node.classList.add('smp-structural-bound', 'smp-interactive-presentation');
    if (getComputedStyle(node).position === 'static') node.style.position = 'absolute';
    const handle = ensureHandle(node);

    node.addEventListener('pointerdown', (event) => {
      if (event.button !== 0 || event.target.closest?.('.smp-resize-handle, .constraint-parameter') || config.disabled()) return;
      event.preventDefault();
      event.stopPropagation();
      config.select();
      const original = config.geometry();
      const startX = event.clientX;
      const startY = event.clientY;
      let next = { ...original };
      node.classList.add('smp-dragging');
      node.setPointerCapture?.(event.pointerId);
      const move = (pointer) => {
        next.x = Math.max(0, original.x + pointer.clientX - startX);
        next.y = Math.max(42, original.y + pointer.clientY - startY);
        node.style.left = `${next.x}px`;
        node.style.top = `${next.y}px`;
      };
      const up = () => {
        node.removeEventListener('pointermove', move);
        node.removeEventListener('pointerup', up);
        node.removeEventListener('pointercancel', up);
        node.classList.remove('smp-dragging');
        void config.commit(next);
      };
      node.addEventListener('pointermove', move);
      node.addEventListener('pointerup', up);
      node.addEventListener('pointercancel', up);
    }, true);

    handle.addEventListener('pointerdown', (event) => {
      if (event.button !== 0 || config.disabled()) return;
      event.preventDefault();
      event.stopPropagation();
      config.select();
      const original = config.geometry();
      const startX = event.clientX;
      const startY = event.clientY;
      let next = { ...original };
      handle.setPointerCapture?.(event.pointerId);
      const move = (pointer) => {
        next.width = Math.max(config.minWidth, original.width + pointer.clientX - startX);
        next.height = Math.max(config.minHeight, original.height + pointer.clientY - startY);
        node.style.width = `${next.width}px`;
        node.style.height = `${next.height}px`;
      };
      const up = () => {
        handle.removeEventListener('pointermove', move);
        handle.removeEventListener('pointerup', up);
        handle.removeEventListener('pointercancel', up);
        void config.commit(next);
      };
      handle.addEventListener('pointermove', move);
      handle.addEventListener('pointerup', up);
      handle.addEventListener('pointercancel', up);
    }, true);
  }

  function installBdd() {
    const diagram = state.snapshot?.diagrams?.find((item) => String(item.id) === String(state.selectedDiagramId));
    if (!diagram) return;
    const nodes = [...document.querySelectorAll('#canvas .bdd-block')];
    nodes.forEach((node, index) => {
      const presentationId = node.dataset.presentationId;
      const presentation = (presentationId
        ? diagram.nodes?.find((item) => String(item.id) === String(presentationId))
        : null) || diagram.nodes?.[index];
      if (!presentation) return;
      bind(node, {
        minWidth: 80,
        minHeight: 50,
        geometry: () => ({ ...presentation }),
        disabled: () => !!state.pendingRelationship || !!state.paletteTool,
        select: () => { state.selectedElementId = presentation.element_id; state.selectedRelationshipId = null; },
        commit: (next) => invokeCommit(
          diagram.family === 'parametric'
            ? 'update_parametric_presentation_geometry'
            : 'update_bdd_presentation_geometry', {
          diagramId: diagram.id,
          presentationId: presentation.id,
          x: next.x, y: next.y, width: next.width, height: next.height,
        }),
      });
    });
  }

  function installIbd() {
    const diagram = state.snapshot?.ibd_diagrams?.find((item) => String(item.id) === String(state.selectedDiagramId));
    if (!diagram) return;
    [...document.querySelectorAll('#canvas .ibd-property')].forEach((node, index) => {
      const presentation = diagram.properties?.[index];
      if (!presentation) return;
      bind(node, {
        minWidth: 80,
        minHeight: 50,
        geometry: () => ({ ...presentation }),
        disabled: () => !!state.pendingRelationship || !!state.paletteTool,
        select: () => { state.selectedElementId = presentation.element_id; state.selectedRelationshipId = null; },
        commit: (next) => invokeCommit('update_ibd_property_geometry', {
          diagramId: diagram.id,
          presentationId: presentation.id,
          x: next.x, y: next.y, width: next.width, height: next.height,
        }),
      });
    });
  }

  function installUseCaseSubjectBoundary() {
    const diagram = state.snapshot?.diagrams?.find(
      (item) => String(item.id) === String(state.selectedDiagramId) && item.family === 'use-case',
    );
    const boundary = diagram?.subject_boundary;
    if (!diagram || !boundary) return;
    const node = document.querySelector(
      `#canvas .use-case-subject-boundary[data-subject-boundary-id="${CSS.escape(String(boundary.id))}"]`,
    );
    bind(node, {
      minWidth: 280,
      minHeight: 220,
      geometry: () => ({ ...boundary }),
      disabled: () => !!state.pendingRelationship || !!state.paletteTool,
      select: () => {
        Object.assign(state, {
          selectedUseCaseSubjectBoundaryId: boundary.id,
          selectedElementId: null,
          selectedRelationshipId: null,
        });
      },
      commit: (next) => invokeCommit('update_use_case_subject_boundary_geometry', {
        diagramId: diagram.id,
        boundaryId: boundary.id,
        x: next.x, y: next.y, width: next.width, height: next.height,
      }),
    });
  }

  function install() {
    installBdd();
    installIbd();
    installUseCaseSubjectBoundary();
  }

  const canvas = document.getElementById('canvas');
  if (canvas) {
    const observer = new MutationObserver(() => queueMicrotask(install));
    observer.observe(canvas, { childList: true, subtree: true });
  }
  const baseRender = render;
  render = function renderWithStructuralRebind() {
    baseRender();
    install();
  };
  queueMicrotask(install);
})();
