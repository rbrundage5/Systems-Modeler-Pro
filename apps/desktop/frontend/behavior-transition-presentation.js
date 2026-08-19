(() => {
  const previousRenderCanvas = renderCanvas;

  function activeStateMachineDiagram() {
    return state.behaviorSnapshot?.diagrams?.find(
      (diagram) => String(diagram.id) === String(state.selectedBehaviorDiagramId)
        && diagram.kind === 'StateMachine',
    ) || null;
  }

  function collectTransitions(regions, output = new Map()) {
    for (const region of regions || []) {
      for (const transition of region.transitions || []) {
        output.set(String(transition.id), transition);
      }
      for (const vertex of region.vertices || []) {
        const childRegions = vertex.kind?.State?.regions || [];
        if (childRegions.length) collectTransitions(childRegions, output);
      }
    }
    return output;
  }

  function boundaryPoint(fromRect, towardRect) {
    const cx = fromRect.x + fromRect.width / 2;
    const cy = fromRect.y + fromRect.height / 2;
    const tx = towardRect.x + towardRect.width / 2;
    const ty = towardRect.y + towardRect.height / 2;
    const dx = tx - cx;
    const dy = ty - cy;
    if (Math.abs(dx) < 0.001 && Math.abs(dy) < 0.001) return { x: cx, y: cy };

    const halfWidth = Math.max(1, fromRect.width / 2);
    const halfHeight = Math.max(1, fromRect.height / 2);
    const scaleX = Math.abs(dx) > 0.001 ? halfWidth / Math.abs(dx) : Number.POSITIVE_INFINITY;
    const scaleY = Math.abs(dy) > 0.001 ? halfHeight / Math.abs(dy) : Number.POSITIVE_INFINITY;
    const scale = Math.min(scaleX, scaleY);
    return { x: cx + dx * scale, y: cy + dy * scale };
  }

  function presentationRect(node) {
    return {
      x: node.offsetLeft,
      y: node.offsetTop,
      width: node.offsetWidth,
      height: node.offsetHeight,
    };
  }

  function repairTransitionGeometry() {
    const diagram = activeStateMachineDiagram();
    const frame = document.querySelector('.authoritative-behavior-frame.state-machine-frame');
    if (!diagram || !frame) return;
    const machine = state.behaviorSnapshot?.repository?.state_machines?.[String(diagram.semantic_id)];
    if (!machine) return;

    const transitions = collectTransitions(machine.regions || []);
    for (const line of frame.querySelectorAll('line.state-transition')) {
      const transition = transitions.get(String(line.dataset.transitionId));
      if (!transition || String(transition.source_id) === String(transition.target_id)) continue;
      const sourceNode = frame.querySelector(`.state-vertex[data-vertex-id="${CSS.escape(String(transition.source_id))}"]`);
      const targetNode = frame.querySelector(`.state-vertex[data-vertex-id="${CSS.escape(String(transition.target_id))}"]`);
      if (!sourceNode || !targetNode) continue;

      const sourceRect = presentationRect(sourceNode);
      const targetRect = presentationRect(targetNode);
      const start = boundaryPoint(sourceRect, targetRect);
      const end = boundaryPoint(targetRect, sourceRect);
      line.setAttribute('x1', start.x);
      line.setAttribute('y1', start.y);
      line.setAttribute('x2', end.x);
      line.setAttribute('y2', end.y);

      const label = line.nextElementSibling;
      if (label?.classList?.contains('behavior-edge-label')) {
        label.setAttribute('x', (start.x + end.x) / 2 + 6);
        label.setAttribute('y', (start.y + end.y) / 2 - 7);
      }
    }
  }

  renderCanvas = function renderCanvasWithVisibleStateTransitions() {
    previousRenderCanvas();
    repairTransitionGeometry();
  };
})();
