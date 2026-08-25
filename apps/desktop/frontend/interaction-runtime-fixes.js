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

  function boundaryPoint(rect, toward) {
    const center = pointForState(rect);
    const dx = toward.x - center.x;
    const dy = toward.y - center.y;
    if (Math.abs(dx) < 0.001 && Math.abs(dy) < 0.001) return center;
    const scaleX = Math.abs(dx) > 0.001
      ? Math.max(1, rect.width / 2) / Math.abs(dx)
      : Number.POSITIVE_INFINITY;
    const scaleY = Math.abs(dy) > 0.001
      ? Math.max(1, rect.height / 2) / Math.abs(dy)
      : Number.POSITIVE_INFINITY;
    const scale = Math.min(scaleX, scaleY);
    return { x: center.x + dx * scale, y: center.y + dy * scale };
  }

  function previewRoute(diagram, transition, source, target, movedVertexId, next) {
    const stored = (diagram.edge_routes || []).find(
      (route) => String(route.semantic_id) === String(transition.id),
    );
    const points = (stored?.points?.length >= 2
      ? stored.points
      : [pointForState(source), pointForState(target)])
      .map((point) => ({ x: point.x, y: point.y }));
    const movedSource = String(transition.source_id) === String(movedVertexId);
    const movedTarget = String(transition.target_id) === String(movedVertexId);
    if (movedSource && movedTarget) {
      const original = diagram.state_nodes?.find(
        (item) => String(item.vertex_id) === String(movedVertexId),
      );
      const dx = next.x - (original?.x ?? next.x);
      const dy = next.y - (original?.y ?? next.y);
      return points.map((point) => ({ x: point.x + dx, y: point.y + dy }));
    }
    if (movedSource) {
      points[0] = boundaryPoint(source, points[1] || pointForState(target));
    }
    if (movedTarget) {
      const last = points.length - 1;
      points[last] = boundaryPoint(target, points[last - 1] || pointForState(source));
    }
    return points;
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
      const line = document.querySelector(
        `#canvas .state-transition[data-transition-id="${CSS.escape(String(transition.id))}"]`,
      );
      if (!line) continue;
      const points = previewRoute(diagram, transition, source, target, movedVertexId, next);
      line.setAttribute('points', points.map((point) => `${point.x},${point.y}`).join(' '));
    }
  }

  window.smpPreviewStateTransitionGeometry = updateStateTransitionGeometry;

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

  function installRuntimeInteractionFixes() {
    bindSequenceConnectedDrag();
  }

  const baseRender = render;
  render = function renderWithRuntimeInteractionFixes() {
    baseRender();
    installRuntimeInteractionFixes();
  };
  queueMicrotask(installRuntimeInteractionFixes);
})();
