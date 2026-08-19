(() => {
  function behaviorDiagram() {
    return state.behaviorSnapshot?.diagrams?.find(
      (diagram) => String(diagram.id) === String(state.selectedBehaviorDiagramId),
    ) || null;
  }

  function flattenTransitions(regions, output = []) {
    for (const region of regions || []) {
      for (const transition of region.transitions || []) output.push(transition);
      for (const vertex of region.vertices || []) {
        const children = vertex.kind?.State?.regions || [];
        if (children.length) flattenTransitions(children, output);
      }
    }
    return output;
  }

  function pointForState(presentation) {
    return {
      x: presentation.x + presentation.width / 2,
      y: presentation.y + presentation.height / 2,
    };
  }

  function updateStateTransitionGeometry(diagram, movedVertexId, next) {
    const machine = state.behaviorSnapshot?.repository?.state_machines?.[String(diagram.semantic_id)];
    if (!machine) return;
    const positions = new Map((diagram.state_nodes || []).map((item) => [String(item.vertex_id), item]));
    positions.set(String(movedVertexId), next);
    for (const transition of flattenTransitions(machine.regions || [])) {
      if (String(transition.source_id) !== String(movedVertexId)
          && String(transition.target_id) !== String(movedVertexId)) continue;
      const source = positions.get(String(transition.source_id));
      const target = positions.get(String(transition.target_id));
      if (!source || !target) continue;
      const sourcePoint = pointForState(source);
      const targetPoint = pointForState(target);
      const line = document.querySelector(
        `#canvas .state-transition[data-transition-id="${CSS.escape(String(transition.id))}"]`,
      );
      if (!line) continue;
      line.setAttribute('x1', sourcePoint.x);
      line.setAttribute('y1', sourcePoint.y);
      line.setAttribute('x2', targetPoint.x);
      line.setAttribute('y2', targetPoint.y);
    }
  }

  function bindStateConnectedDrag() {
    const diagram = behaviorDiagram();
    if (!diagram || diagram.kind !== 'StateMachine') return;
    document.querySelectorAll('#canvas .state-vertex').forEach((node) => {
      const vertexId = node.dataset.vertexId;
      const presentation = diagram.state_nodes?.find(
        (item) => String(item.vertex_id) === String(vertexId),
      );
      if (!presentation) return;
      node.onpointerdown = (event) => {
        if (event.button !== 0 || event.target.closest?.('.smp-resize-handle')) return;
        if (state.behaviorPending || state.behaviorTool) return;
        event.preventDefault();
        event.stopPropagation();
        const startX = event.clientX;
        const startY = event.clientY;
        const original = { ...presentation };
        let next = { ...original };
        node.classList.add('smp-dragging');
        node.setPointerCapture?.(event.pointerId);
        node.onpointermove = (move) => {
          next.x = Math.max(0, original.x + move.clientX - startX);
          next.y = Math.max(42, original.y + move.clientY - startY);
          node.style.left = `${next.x}px`;
          node.style.top = `${next.y}px`;
          updateStateTransitionGeometry(diagram, vertexId, next);
        };
        node.onpointerup = async () => {
          node.onpointermove = null;
          node.onpointerup = null;
          node.classList.remove('smp-dragging');
          presentation.x = next.x;
          presentation.y = next.y;
          await runCommand('Moving State vertex…', () => requireInvoke()('update_state_presentation_geometry', {
            diagramId: diagram.id,
            stateVertexId: String(vertexId),
            x: next.x,
            y: next.y,
            width: next.width,
            height: next.height,
          }));
          await refresh();
        };
      };
    });
  }

  function updateSequenceMessages(interaction, lifelineId, x) {
    for (const message of interaction?.messages || []) {
      const line = document.querySelector(
        `#canvas .sequence-message[data-message-id="${CSS.escape(String(message.id))}"]`,
      );
      if (!line) continue;
      if (String(message.send_event?.lifeline_id || '') === String(lifelineId)) line.setAttribute('x1', x);
      if (String(message.receive_event?.lifeline_id || '') === String(lifelineId)) line.setAttribute('x2', x);
    }
  }

  function bindSequenceConnectedDrag() {
    const diagram = behaviorDiagram();
    if (!diagram || diagram.kind !== 'Sequence') return;
    const interaction = state.behaviorSnapshot?.repository?.interactions?.[String(diagram.semantic_id)];
    if (!interaction) return;
    document.querySelectorAll('#canvas .sequence-lifeline').forEach((node) => {
      const lifelineId = node.dataset.lifelineId;
      const presentation = diagram.lifelines?.find(
        (item) => String(item.lifeline_id) === String(lifelineId),
      );
      if (!presentation) return;
      node.onpointerdown = (event) => {
        if (event.button !== 0 || event.target.closest?.('.lifeline-resize-handle')) return;
        if (state.behaviorPending || state.behaviorTool) return;
        event.preventDefault();
        event.stopPropagation();
        const startX = event.clientX;
        const originalX = presentation.x;
        let nextX = originalX;
        node.classList.add('smp-dragging');
        node.setPointerCapture?.(event.pointerId);
        node.onpointermove = (move) => {
          nextX = Math.max(70, originalX + move.clientX - startX);
          node.style.left = `${nextX - 65}px`;
          updateSequenceMessages(interaction, lifelineId, nextX);
        };
        node.onpointerup = async () => {
          node.onpointermove = null;
          node.onpointerup = null;
          node.classList.remove('smp-dragging');
          presentation.x = nextX;
          await runCommand('Moving Lifeline…', () => requireInvoke()('move_sequence_lifeline', {
            diagramId: diagram.id,
            lifelineIdValue: String(lifelineId),
            x: nextX,
          }));
          await refresh();
        };
      };
    });
  }

  function applyActivityShapeGeometry(group, geometry) {
    const { x, y, width: w, height: h } = geometry;
    if (group.classList.contains('activity-initial')) {
      const circle = group.querySelector('circle');
      if (circle) {
        circle.setAttribute('cx', x + w / 2);
        circle.setAttribute('cy', y + h / 2);
        circle.setAttribute('r', Math.max(2, Math.min(w, h) / 2 - 2));
      }
      return;
    }
    if (group.classList.contains('activity-activityfinal')) {
      const circles = group.querySelectorAll('circle');
      circles[0]?.setAttribute('cx', x + w / 2);
      circles[0]?.setAttribute('cy', y + h / 2);
      circles[0]?.setAttribute('r', Math.max(2, Math.min(w, h) / 2 - 2));
      circles[1]?.setAttribute('cx', x + w / 2);
      circles[1]?.setAttribute('cy', y + h / 2);
      circles[1]?.setAttribute('r', Math.max(1, Math.min(w, h) / 2 - 7));
      return;
    }
    if (group.classList.contains('activity-flowfinal')) {
      const circle = group.querySelector('circle');
      if (circle) {
        circle.setAttribute('cx', x + w / 2);
        circle.setAttribute('cy', y + h / 2);
        circle.setAttribute('r', Math.max(2, Math.min(w, h) / 2 - 2));
      }
      const lines = group.querySelectorAll('line');
      lines[0]?.setAttribute('x1', x + 6);
      lines[0]?.setAttribute('y1', y + 6);
      lines[0]?.setAttribute('x2', x + w - 6);
      lines[0]?.setAttribute('y2', y + h - 6);
      lines[1]?.setAttribute('x1', x + w - 6);
      lines[1]?.setAttribute('y1', y + 6);
      lines[1]?.setAttribute('x2', x + 6);
      lines[1]?.setAttribute('y2', y + h - 6);
      return;
    }
    if (group.classList.contains('activity-decision') || group.classList.contains('activity-merge')) {
      const polygon = group.querySelector('polygon');
      polygon?.setAttribute('points', `${x + w / 2},${y} ${x + w},${y + h / 2} ${x + w / 2},${y + h} ${x},${y + h / 2}`);
      return;
    }
    if (group.classList.contains('activity-fork') || group.classList.contains('activity-join')) {
      const rect = group.querySelector('rect:not(.smp-svg-resize-handle)');
      if (rect) {
        rect.setAttribute('x', x);
        rect.setAttribute('y', y);
        rect.setAttribute('width', w);
        rect.setAttribute('height', h);
      }
      return;
    }
    const rect = group.querySelector('rect:not(.smp-svg-resize-handle)');
    if (rect) {
      rect.setAttribute('x', x);
      rect.setAttribute('y', y);
      rect.setAttribute('width', w);
      rect.setAttribute('height', h);
    }
    const text = group.querySelector('text');
    if (text) {
      text.setAttribute('x', x + w / 2);
      text.setAttribute('y', y + h / 2 + 4);
    }
  }

  function bindActivityVisibleResize() {
    const diagram = state.activitySnapshot?.diagrams?.find(
      (item) => String(item.id) === String(state.selectedActivityDiagramId),
    );
    const svg = document.querySelector('#canvas .activity-svg');
    if (!diagram || !svg) return;
    const viewBox = svg.viewBox.baseVal;
    const unitsPerPixel = () => {
      const rect = svg.getBoundingClientRect();
      return {
        x: viewBox.width / Math.max(1, rect.width),
        y: viewBox.height / Math.max(1, rect.height),
      };
    };
    svg.querySelectorAll('.activity-node').forEach((group) => {
      const semanticId = group.dataset.activityNodeId;
      const presentation = diagram.nodes?.find(
        (item) => String(item.activity_node_id) === String(semanticId),
      );
      const handle = group.querySelector('.smp-svg-resize-handle');
      if (!presentation || !handle) return;
      handle.onpointerdown = (event) => {
        if (event.button !== 0 || state.activityPendingFlow || state.activityTool) return;
        event.preventDefault();
        event.stopPropagation();
        const startX = event.clientX;
        const startY = event.clientY;
        const original = { ...presentation };
        let next = { ...original };
        const barLike = group.classList.contains('activity-fork') || group.classList.contains('activity-join');
        handle.setPointerCapture?.(event.pointerId);
        handle.onpointermove = (move) => {
          const scale = unitsPerPixel();
          next.width = Math.max(24, original.width + (move.clientX - startX) * scale.x);
          if (barLike) {
            next.height = Math.max(8, Math.min(24, original.height + (move.clientY - startY) * scale.y));
          } else {
            next.height = Math.max(24, original.height + (move.clientY - startY) * scale.y);
          }
          applyActivityShapeGeometry(group, next);
          handle.setAttribute('x', next.x + next.width - 6);
          handle.setAttribute('y', next.y + next.height - 6);
        };
        handle.onpointerup = async () => {
          handle.onpointermove = null;
          handle.onpointerup = null;
          presentation.width = next.width;
          presentation.height = next.height;
          await runCommand('Resizing Activity node…', () => requireInvoke()('update_activity_presentation_geometry', {
            diagramId: diagram.id,
            presentationId: presentation.id,
            x: next.x,
            y: next.y,
            width: next.width,
            height: next.height,
          }));
          state.activitySnapshot = await requireInvoke()('activity_snapshot');
          render();
        };
      };
    });
  }

  function installRuntimeInteractionFixes() {
    bindStateConnectedDrag();
    bindSequenceConnectedDrag();
    bindActivityVisibleResize();
  }

  const baseRender = render;
  render = function renderWithRuntimeInteractionFixes() {
    baseRender();
    installRuntimeInteractionFixes();
  };
  queueMicrotask(installRuntimeInteractionFixes);
})();
