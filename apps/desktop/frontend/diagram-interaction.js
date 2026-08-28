(() => {
  const MIN = {
    BDD: { width: 80, height: 50 },
    IBD: { width: 80, height: 50 },
    StateMachine: { width: 24, height: 24 },
    Activity: { width: 24, height: 24 },
  };

  const style = document.createElement('style');
  style.textContent = `
    .smp-interactive-presentation { cursor: move !important; touch-action: none; }
    .smp-interactive-presentation.smp-dragging { opacity: .92; cursor: grabbing !important; }
    .smp-resize-handle { position: absolute; right: 2px; bottom: 2px; width: 12px; height: 12px; border: 1px solid #315b86; background: #fff; cursor: nwse-resize; z-index: 20; box-sizing: border-box; pointer-events: auto; }
    .smp-svg-resize-handle { fill: #fff; stroke: #315b86; stroke-width: 2; cursor: nwse-resize; pointer-events: all; }
    .activity-node.smp-dragging { opacity: .92; }
  `;
  document.head.appendChild(style);

  async function commit(command, args) {
    try {
      await runCommand('Updating diagram presentation…', () => requireInvoke()(command, args));
    } catch (error) {
      const message = error?.message || String(error);
      console.error('Presentation geometry update failed', error);
      window.smpDialogs?.notify?.(`Presentation geometry update failed: ${message}`, 'error');
      renderStatus?.(`Presentation geometry update failed: ${message}`);
    } finally {
      // The pointer preview is disposable. Always replace it with the snapshot
      // returned after the Rust mutation completes, including on rejection.
      await refresh();
    }
  }

  window.smpCommitPresentationGeometry = commit;

  function surfaceScale(node) {
    const surface = node.closest?.('.workspace-renderer-surface') || node.parentElement;
    const rect = surface?.getBoundingClientRect?.();
    const width = surface?.offsetWidth || rect?.width || 1;
    const height = surface?.offsetHeight || rect?.height || 1;
    const x = rect?.width && width ? rect.width / width : 1;
    const y = rect?.height && height ? rect.height / height : 1;
    return { x: Math.max(x || 1, 0.0001), y: Math.max(y || 1, 0.0001) };
  }

  const DRAG_THRESHOLD_PX = 3;

  function cancelTransientAuthoring() {
    Object.assign(state, {
      paletteTool: null,
      pendingRelationship: null,
      behaviorTool: null,
      behaviorPending: null,
      activityTool: null,
      activityPendingFlow: null,
    });
  }

  function beginPointerGesture(event, options) {
    if (event.button !== 0) return false;
    const pointerId = event.pointerId;
    const startX = event.clientX;
    const startY = event.clientY;
    const owner = options.owner;
    const scale = options.scale || { x: 1, y: 1 };
    let started = false;
    let finished = false;

    const cleanup = () => {
      window.removeEventListener('pointermove', onMove, true);
      window.removeEventListener('pointerup', onUp, true);
      window.removeEventListener('pointercancel', onCancel, true);
      try { owner?.releasePointerCapture?.(pointerId); } catch (_) {}
    };

    const startIfNeeded = (move) => {
      if (started) return true;
      const dx = move.clientX - startX;
      const dy = move.clientY - startY;
      if (Math.hypot(dx, dy) < DRAG_THRESHOLD_PX) return false;
      options.prepare?.();
      if (options.disabled?.()) {
        finished = true;
        cleanup();
        options.onCancel?.();
        return false;
      }
      started = true;
      options.onStart?.();
      return true;
    };

    const onMove = (move) => {
      if (finished || move.pointerId !== pointerId || !startIfNeeded(move)) return;
      move.preventDefault();
      move.stopPropagation();
      options.onMove?.(
        (move.clientX - startX) / Math.max(scale.x || 1, 0.0001),
        (move.clientY - startY) / Math.max(scale.y || 1, 0.0001),
        move,
      );
    };

    const finish = async (up, cancelled) => {
      if (finished || up.pointerId !== pointerId) return;
      finished = true;
      cleanup();
      if (!started) return;
      up.preventDefault();
      up.stopPropagation();
      if (cancelled) options.onCancel?.();
      else await options.onCommit?.();
    };
    const onUp = (up) => { void finish(up, false); };
    const onCancel = (cancel) => { void finish(cancel, true); };

    window.addEventListener('pointermove', onMove, true);
    window.addEventListener('pointerup', onUp, true);
    window.addEventListener('pointercancel', onCancel, true);
    try { owner?.setPointerCapture?.(pointerId); } catch (_) {}
    return true;
  }

  window.smpBeginPresentationGesture = beginPointerGesture;

  function bindHtmlGeometry(node, config) {
    if (!node || node.dataset.smpGeometryBound === '1') return;
    node.dataset.smpGeometryBound = '1';
    node.classList.add('smp-interactive-presentation');
    if (getComputedStyle(node).position === 'static') node.style.position = 'absolute';

    const handle = document.createElement('span');
    handle.className = 'smp-resize-handle';
    handle.title = 'Drag to resize';
    handle.setAttribute('aria-label', 'Resize element');
    handle.addEventListener('click', (event) => {
      event.preventDefault();
      event.stopPropagation();
    });
    node.appendChild(handle);

    let suppressNextClick = false;
    node.addEventListener('click', (event) => {
      if (!suppressNextClick) return;
      suppressNextClick = false;
      event.preventDefault();
      event.stopImmediatePropagation();
    }, true);

    node.addEventListener('pointerdown', (event) => {
      if (event.button !== 0 || event.target.closest?.('.smp-resize-handle, .constraint-parameter')) return;
      event.preventDefault();
      event.stopPropagation();
      config.select?.();
      const original = config.geometry();
      let next = { ...original };
      beginPointerGesture(event, {
        owner: node,
        scale: surfaceScale(node),
        prepare: cancelTransientAuthoring,
        disabled: config.disabled,
        onStart: () => {
          suppressNextClick = true;
          node.classList.add('smp-dragging');
        },
        onMove: (dx, dy) => {
          next.x = Math.max(0, original.x + dx);
          next.y = Math.max(42, original.y + dy);
          node.style.left = `${next.x}px`;
          node.style.top = `${next.y}px`;
          config.preview?.(next);
        },
        onCancel: () => node.classList.remove('smp-dragging'),
        onCommit: async () => {
          node.classList.remove('smp-dragging');
          await config.commit(next);
        },
      });
    });

    handle.addEventListener('pointerdown', (event) => {
      if (event.button !== 0) return;
      event.preventDefault();
      event.stopPropagation();
      config.select?.();
      const original = config.geometry();
      let next = { ...original };
      beginPointerGesture(event, {
        owner: handle,
        scale: surfaceScale(node),
        prepare: cancelTransientAuthoring,
        disabled: config.disabled,
        onStart: () => { suppressNextClick = true; },
        onMove: (dx, dy) => {
          next.width = Math.max(config.minWidth, original.width + dx);
          next.height = Math.max(config.minHeight, original.height + dy);
          node.style.width = `${next.width}px`;
          node.style.height = `${next.height}px`;
          config.preview?.(next);
        },
        onCommit: async () => { await config.commit(next); },
      });
    });
  }

  function installBdd() {
    const diagram = state.snapshot?.diagrams?.find((item) => item.id === state.selectedDiagramId);
    if (!diagram) return;
    [...document.querySelectorAll('#canvas .bdd-block')].forEach((node, index) => {
      const presentationId = node.dataset.presentationId;
      const presentation = (presentationId
        ? diagram.nodes?.find((item) => String(item.id) === String(presentationId))
        : null) || diagram.nodes?.[index];
      if (!presentation) return;
      bindHtmlGeometry(node, {
        minWidth: MIN.BDD.width,
        minHeight: MIN.BDD.height,
        geometry: () => ({ ...presentation }),
        disabled: () => !!state.pendingRelationship || !!state.paletteTool,
        select: () => { state.selectedElementId = presentation.element_id; state.selectedRelationshipId = null; },
        commit: (next) => commit(
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
    const diagram = (state.snapshot?.ibd_diagrams || []).find((item) => item.id === state.selectedDiagramId);
    if (!diagram) return;
    [...document.querySelectorAll('#canvas .ibd-property')].forEach((node, index) => {
      const presentationId = node.dataset.presentationId;
      const presentation = (presentationId
        ? diagram.properties?.find((item) => String(item.id) === String(presentationId))
        : null) || diagram.properties?.[index];
      if (!presentation) return;
      bindHtmlGeometry(node, {
        minWidth: MIN.IBD.width,
        minHeight: MIN.IBD.height,
        geometry: () => ({ ...presentation }),
        disabled: () => !!state.pendingRelationship || !!state.paletteTool,
        select: () => { state.selectedElementId = presentation.element_id; state.selectedRelationshipId = null; },
        commit: (next) => commit('update_ibd_property_geometry', {
          diagramId: diagram.id,
          presentationId: presentation.id,
          x: next.x, y: next.y, width: next.width, height: next.height,
        }),
      });
    });

    const ports = [...diagram.properties.flatMap((property) => property.ports || []), ...(diagram.boundary_ports || [])];
    [...document.querySelectorAll('#canvas .ibd-port')].forEach((node, index) => {
      const presentationId = node.dataset.presentationId;
      const presentation = (presentationId
        ? ports.find((item) => String(item.id) === String(presentationId))
        : null) || ports[index];
      if (!presentation) return;
      bindHtmlGeometry(node, {
        minWidth: 10,
        minHeight: 10,
        geometry: () => ({
          x: presentation.x - presentation.size / 2,
          y: presentation.y - presentation.size / 2,
          width: presentation.size,
          height: presentation.size,
        }),
        disabled: () => !!state.pendingRelationship || !!state.paletteTool,
        select: () => { state.selectedElementId = presentation.element_id; state.selectedRelationshipId = null; },
        commit: (next) => commit('update_ibd_port_geometry', {
          diagramId: diagram.id,
          presentationId: presentation.id,
          x: next.x + next.width / 2,
          y: next.y + next.height / 2,
          size: Math.max(10, Math.max(next.width, next.height)),
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
    bindHtmlGeometry(node, {
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
      commit: (next) => commit('update_use_case_subject_boundary_geometry', {
        diagramId: diagram.id,
        boundaryId: boundary.id,
        x: next.x, y: next.y, width: next.width, height: next.height,
      }),
    });
  }

  function installStateMachine() {
    const diagram = state.behaviorSnapshot?.diagrams?.find((item) => String(item.id) === String(state.selectedBehaviorDiagramId));
    if (!diagram || diagram.kind !== 'StateMachine') return;
    [...document.querySelectorAll('#canvas .state-vertex')].forEach((node) => {
      const vertexId = node.dataset.vertexId;
      const presentation = diagram.state_nodes?.find((item) => String(item.vertex_id) === String(vertexId));
      if (!presentation) return;
      bindHtmlGeometry(node, {
        minWidth: MIN.StateMachine.width,
        minHeight: node.classList.contains('submachine-state') ? 90 : MIN.StateMachine.height,
        geometry: () => ({ ...presentation }),
        disabled: () => !!state.behaviorPending || !!state.behaviorTool,
        select: () => {},
        preview: (next) => window.smpPreviewStateTransitionGeometry?.(diagram, vertexId, next),
        commit: (next) => commit('update_state_presentation_geometry', {
          diagramId: diagram.id,
          stateVertexId: vertexId,
          x: next.x, y: next.y, width: next.width, height: next.height,
        }),
      });
    });
  }

  function installSequence() {
    document.querySelectorAll('#canvas .sequence-lifeline').forEach((node) => {
      node.classList.add('smp-interactive-presentation');
      node.title = `${node.title ? `${node.title} · ` : ''}Drag to move; use the lower handle to resize the timeline.`;
    });
  }

  function applyActivityShapeGeometry(group, geometry) {
    const { x, y, width, height } = geometry;
    const centerX = x + width / 2;
    const centerY = y + height / 2;
    const radius = Math.max(2, Math.min(width, height) / 2 - 2);
    if (group.classList.contains('activity-initial')) {
      const circle = group.querySelector('circle');
      if (circle) {
        circle.setAttribute('cx', centerX);
        circle.setAttribute('cy', centerY);
        circle.setAttribute('r', radius);
      }
      return;
    }
    if (group.classList.contains('activity-activityfinal')) {
      const circles = group.querySelectorAll('circle');
      circles[0]?.setAttribute('cx', centerX);
      circles[0]?.setAttribute('cy', centerY);
      circles[0]?.setAttribute('r', radius);
      circles[1]?.setAttribute('cx', centerX);
      circles[1]?.setAttribute('cy', centerY);
      circles[1]?.setAttribute('r', Math.max(1, Math.min(width, height) / 2 - 7));
      return;
    }
    if (group.classList.contains('activity-flowfinal')) {
      const circle = group.querySelector('circle');
      if (circle) {
        circle.setAttribute('cx', centerX);
        circle.setAttribute('cy', centerY);
        circle.setAttribute('r', radius);
      }
      const lines = group.querySelectorAll('line');
      lines[0]?.setAttribute('x1', x + 6);
      lines[0]?.setAttribute('y1', y + 6);
      lines[0]?.setAttribute('x2', x + width - 6);
      lines[0]?.setAttribute('y2', y + height - 6);
      lines[1]?.setAttribute('x1', x + width - 6);
      lines[1]?.setAttribute('y1', y + 6);
      lines[1]?.setAttribute('x2', x + 6);
      lines[1]?.setAttribute('y2', y + height - 6);
      return;
    }
    if (group.classList.contains('activity-decision') || group.classList.contains('activity-merge')) {
      group.querySelector('polygon')?.setAttribute(
        'points',
        `${centerX},${y} ${x + width},${centerY} ${centerX},${y + height} ${x},${centerY}`,
      );
      return;
    }
    const rect = group.querySelector('rect:not(.smp-svg-resize-handle)');
    if (rect) {
      rect.setAttribute('x', x);
      rect.setAttribute('y', y);
      rect.setAttribute('width', width);
      rect.setAttribute('height', height);
    }
    if (group.classList.contains('activity-fork') || group.classList.contains('activity-join')) return;
    const text = group.querySelector('text');
    if (text) {
      text.setAttribute('x', centerX);
      text.setAttribute('y', centerY + 4);
    }
  }

  function installActivity() {
    const diagram = state.activitySnapshot?.diagrams?.find((item) => String(item.id) === String(state.selectedActivityDiagramId));
    if (!diagram) return;
    const svg = document.querySelector('#canvas .activity-svg');
    if (!svg) return;
    const viewBox = svg.viewBox.baseVal;
    const unitsPerPixel = () => {
      const rect = svg.getBoundingClientRect();
      return { x: viewBox.width / Math.max(1, rect.width), y: viewBox.height / Math.max(1, rect.height) };
    };
    svg.querySelectorAll('.activity-node').forEach((group) => {
      if (group.dataset.smpGeometryBound === '1') return;
      const semanticId = group.dataset.activityNodeId;
      const presentation = diagram.nodes?.find((item) => String(item.activity_node_id) === String(semanticId));
      if (!presentation) return;
      group.dataset.smpGeometryBound = '1';
      group.classList.add('smp-interactive-presentation');
      const handle = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
      handle.classList.add('smp-svg-resize-handle');
      handle.setAttribute('x', presentation.x + presentation.width - 6);
      handle.setAttribute('y', presentation.y + presentation.height - 6);
      handle.setAttribute('width', '12');
      handle.setAttribute('height', '12');
      handle.onclick = (event) => {
        event.preventDefault();
        event.stopPropagation();
      };
      group.appendChild(handle);

      group.onpointerdown = (event) => {
        if (event.button !== 0 || event.target === handle || state.activityPendingFlow || state.activityTool) return;
        event.preventDefault();
        event.stopPropagation();
        state.selectedActivityNodeId = semanticId;
        const startX = event.clientX;
        const startY = event.clientY;
        const original = { ...presentation };
        let next = { ...original };
        group.classList.add('smp-dragging');
        group.setPointerCapture?.(event.pointerId);
        group.onpointermove = (move) => {
          const scale = unitsPerPixel();
          const dx = (move.clientX - startX) * scale.x;
          const dy = (move.clientY - startY) * scale.y;
          next.x = Math.max(0, original.x + dx);
          next.y = Math.max(42, original.y + dy);
          group.setAttribute('transform', `translate(${next.x - original.x} ${next.y - original.y})`);
        };
        group.onpointerup = async () => {
          group.onpointermove = null;
          group.onpointerup = null;
          group.classList.remove('smp-dragging');
          group.removeAttribute('transform');
          await commit('update_activity_presentation_geometry', {
            diagramId: diagram.id,
            presentationId: presentation.id,
            x: next.x, y: next.y, width: next.width, height: next.height,
          });
        };
      };

      handle.onpointerdown = (event) => {
        if (event.button !== 0 || state.activityPendingFlow || state.activityTool) return;
        event.preventDefault();
        event.stopPropagation();
        const startX = event.clientX;
        const startY = event.clientY;
        const original = { ...presentation };
        let next = { ...original };
        handle.setPointerCapture?.(event.pointerId);
        handle.onpointermove = (move) => {
          const scale = unitsPerPixel();
          next.width = Math.max(MIN.Activity.width, original.width + (move.clientX - startX) * scale.x);
          const barLike = group.classList.contains('activity-fork') || group.classList.contains('activity-join');
          next.height = barLike
            ? Math.max(20, Math.min(24, original.height + (move.clientY - startY) * scale.y))
            : Math.max(MIN.Activity.height, original.height + (move.clientY - startY) * scale.y);
          applyActivityShapeGeometry(group, next);
          handle.setAttribute('x', original.x + next.width - 6);
          handle.setAttribute('y', original.y + next.height - 6);
        };
        handle.onpointerup = async () => {
          handle.onpointermove = null;
          handle.onpointerup = null;
          await commit('update_activity_presentation_geometry', {
            diagramId: diagram.id,
            presentationId: presentation.id,
            x: original.x, y: original.y, width: next.width, height: next.height,
          });
        };
      };
    });
  }

  function install() {
    installBdd();
    installIbd();
    installUseCaseSubjectBoundary();
    installStateMachine();
    installSequence();
    installActivity();
  }

  const baseRender = render;
  render = function renderWithSharedPresentationInteraction() {
    baseRender();
    install();
  };
  queueMicrotask(install);
  const canvas = document.getElementById('canvas');
  if (canvas) {
    let installQueued = false;
    new MutationObserver(() => {
      if (installQueued) return;
      installQueued = true;
      queueMicrotask(() => {
        installQueued = false;
        install();
      });
    }).observe(canvas, { childList: true, subtree: true });
  }
})();
