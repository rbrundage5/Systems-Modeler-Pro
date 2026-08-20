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
    .smp-resize-handle { position: absolute; right: -5px; bottom: -5px; width: 11px; height: 11px; border: 1px solid #315b86; background: #fff; cursor: nwse-resize; z-index: 20; box-sizing: border-box; }
    .smp-svg-resize-handle { fill: #fff; stroke: #315b86; stroke-width: 2; cursor: nwse-resize; pointer-events: all; }
    .activity-node.smp-dragging { opacity: .92; }
  `;
  document.head.appendChild(style);

  function commit(command, args, after) {
    return runCommand('Updating diagram presentation…', () => requireInvoke()(command, args))
      .then(async () => {
        await after();
        render();
      })
      .catch((error) => console.error('Presentation geometry update failed', error));
  }

  function bindHtmlGeometry(node, config) {
    if (!node || node.dataset.smpGeometryBound === '1') return;
    node.dataset.smpGeometryBound = '1';
    node.classList.add('smp-interactive-presentation');
    if (getComputedStyle(node).position === 'static') node.style.position = 'absolute';

    const handle = document.createElement('span');
    handle.className = 'smp-resize-handle';
    handle.title = 'Drag to resize';
    handle.setAttribute('aria-label', 'Resize element');
    node.appendChild(handle);

    node.onpointerdown = (event) => {
      if (event.button !== 0 || event.target.closest?.('.smp-resize-handle')) return;
      if (config.disabled?.()) return;
      event.preventDefault();
      event.stopPropagation();
      config.select?.();
      const startX = event.clientX;
      const startY = event.clientY;
      const original = config.geometry();
      let next = { ...original };
      node.classList.add('smp-dragging');
      node.setPointerCapture?.(event.pointerId);
      node.onpointermove = (move) => {
        next.x = Math.max(0, original.x + move.clientX - startX);
        next.y = Math.max(42, original.y + move.clientY - startY);
        node.style.left = `${next.x}px`;
        node.style.top = `${next.y}px`;
      };
      node.onpointerup = () => {
        node.onpointermove = null;
        node.onpointerup = null;
        node.classList.remove('smp-dragging');
        void config.commit(next);
      };
    };

    handle.onpointerdown = (event) => {
      if (event.button !== 0 || config.disabled?.()) return;
      event.preventDefault();
      event.stopPropagation();
      config.select?.();
      const startX = event.clientX;
      const startY = event.clientY;
      const original = config.geometry();
      let next = { ...original };
      handle.setPointerCapture?.(event.pointerId);
      handle.onpointermove = (move) => {
        next.width = Math.max(config.minWidth, original.width + move.clientX - startX);
        next.height = Math.max(config.minHeight, original.height + move.clientY - startY);
        node.style.width = `${next.width}px`;
        node.style.height = `${next.height}px`;
      };
      handle.onpointerup = () => {
        handle.onpointermove = null;
        handle.onpointerup = null;
        void config.commit(next);
      };
    };
  }

  function installBdd() {
    const diagram = state.snapshot?.diagrams?.find((item) => item.id === state.selectedDiagramId);
    if (!diagram) return;
    [...document.querySelectorAll('#canvas .bdd-block')].forEach((node, index) => {
      const presentation = diagram.nodes[index];
      if (!presentation) return;
      bindHtmlGeometry(node, {
        minWidth: MIN.BDD.width,
        minHeight: MIN.BDD.height,
        geometry: () => ({ ...presentation }),
        disabled: () => !!state.pendingRelationship || !!state.paletteTool,
        select: () => { state.selectedElementId = presentation.element_id; state.selectedRelationshipId = null; },
        commit: (next) => commit('update_bdd_presentation_geometry', {
          diagramId: diagram.id,
          presentationId: presentation.id,
          x: next.x, y: next.y, width: next.width, height: next.height,
        }, refresh),
      });
    });
  }

  function installIbd() {
    const diagram = (state.snapshot?.ibd_diagrams || []).find((item) => item.id === state.selectedDiagramId);
    if (!diagram) return;
    [...document.querySelectorAll('#canvas .ibd-property')].forEach((node, index) => {
      const presentation = diagram.properties[index];
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
        }, refresh),
      });
    });

    const ports = [...diagram.properties.flatMap((property) => property.ports || []), ...(diagram.boundary_ports || [])];
    [...document.querySelectorAll('#canvas .ibd-port')].forEach((node, index) => {
      const presentation = ports[index];
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
        }, refresh),
      });
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
        minHeight: MIN.StateMachine.height,
        geometry: () => ({ ...presentation }),
        disabled: () => !!state.behaviorPending || !!state.behaviorTool,
        select: () => {},
        commit: (next) => commit('update_state_presentation_geometry', {
          diagramId: diagram.id,
          stateVertexId: vertexId,
          x: next.x, y: next.y, width: next.width, height: next.height,
        }, refresh),
      });
    });
  }

  function installSequence() {
    document.querySelectorAll('#canvas .sequence-lifeline').forEach((node) => {
      node.classList.add('smp-interactive-presentation');
      node.title = `${node.title ? `${node.title} · ` : ''}Drag to move; use the lower handle to resize the timeline.`;
    });
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
        group.onpointerup = () => {
          group.onpointermove = null;
          group.onpointerup = null;
          group.classList.remove('smp-dragging');
          group.removeAttribute('transform');
          void commit('update_activity_presentation_geometry', {
            diagramId: diagram.id,
            presentationId: presentation.id,
            x: next.x, y: next.y, width: next.width, height: next.height,
          }, async () => { state.activitySnapshot = await requireInvoke()('activity_snapshot'); });
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
          next.height = Math.max(MIN.Activity.height, original.height + (move.clientY - startY) * scale.y);
          handle.setAttribute('x', original.x + next.width - 6);
          handle.setAttribute('y', original.y + next.height - 6);
        };
        handle.onpointerup = () => {
          handle.onpointermove = null;
          handle.onpointerup = null;
          void commit('update_activity_presentation_geometry', {
            diagramId: diagram.id,
            presentationId: presentation.id,
            x: original.x, y: original.y, width: next.width, height: next.height,
          }, async () => { state.activitySnapshot = await requireInvoke()('activity_snapshot'); });
        };
      };
    });
  }

  function install() {
    installBdd();
    installIbd();
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
})();
