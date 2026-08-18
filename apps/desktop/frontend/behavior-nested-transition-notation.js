(() => {
  function activeDiagram() {
    return state.behaviorSnapshot?.diagrams?.find(
      (diagram) => diagram.id === state.selectedBehaviorDiagramId,
    ) || null;
  }

  function machine(diagram) {
    if (!diagram || diagram.kind !== 'StateMachine') return null;
    return state.behaviorSnapshot?.repository?.state_machines?.[diagram.semantic_id] || null;
  }

  function nestedTransitions(regions, depth = 0, output = []) {
    for (const region of regions || []) {
      if (depth > 0) {
        for (const transition of region.transitions || []) output.push(transition);
      }
      for (const vertex of region.vertices || []) {
        const childRegions = vertex.kind?.State?.regions;
        if (childRegions) nestedTransitions(childRegions, depth + 1, output);
      }
    }
    return output;
  }

  function presentation(diagram, vertexId) {
    return diagram.state_nodes?.find((node) => node.vertex_id === vertexId) || null;
  }

  function projectElement(id) {
    return state.snapshot?.project?.elements?.find((element) => element.id === id) || null;
  }

  function transitionLabel(transition) {
    const event = transition.trigger?.event;
    let trigger = '';
    if (event?.Signal) trigger = projectElement(event.Signal.signal_id)?.name || 'signal';
    else if (event?.Call) trigger = projectElement(event.Call.operation_id)?.name || 'operation';
    else if (event?.Time) trigger = `after(${event.Time.expression})`;
    else if (event?.Change) trigger = `when(${event.Change.expression})`;
    else if (event === 'AnyReceive' || event?.AnyReceive != null) trigger = 'all';
    const guard = transition.guard ? ` [${transition.guard}]` : '';
    const effect = transition.effect ? ` / ${transition.effect}` : '';
    return `${trigger}${guard}${effect}`.trim();
  }

  function drawNestedTransitions() {
    const diagram = activeDiagram();
    const semantic = machine(diagram);
    const svg = document.querySelector('.state-machine-frame .behavior-relationship-layer');
    if (!diagram || !semantic || !svg) return;

    for (const transition of nestedTransitions(semantic.regions)) {
      if (svg.querySelector(`[data-transition-id="${CSS.escape(transition.id)}"]`)) continue;
      const source = presentation(diagram, transition.source_id);
      const target = presentation(diagram, transition.target_id);
      if (!source || !target) continue;
      const x1 = source.x + source.width / 2;
      const y1 = source.y + source.height / 2;
      const x2 = target.x + target.width / 2;
      const y2 = target.y + target.height / 2;
      const line = document.createElementNS(SVG_NS, 'line');
      line.setAttribute('x1', x1);
      line.setAttribute('y1', y1);
      line.setAttribute('x2', x2);
      line.setAttribute('y2', y2);
      line.setAttribute('marker-end', 'url(#state-arrow)');
      line.classList.add('state-transition', 'nested-state-transition');
      line.dataset.transitionId = transition.id;
      if (
        state.selectedBehaviorItem?.type === 'Transition'
        && state.selectedBehaviorItem.id === transition.id
      ) line.classList.add('selected');
      line.onclick = (event) => {
        event.stopPropagation();
        state.selectedBehaviorItem = {
          type: 'Transition',
          id: transition.id,
          semantic: transition,
        };
        render();
      };
      svg.appendChild(line);

      const label = transitionLabel(transition);
      if (label) {
        const text = document.createElementNS(SVG_NS, 'text');
        text.classList.add('behavior-edge-label', 'nested-state-transition-label');
        text.setAttribute('x', (x1 + x2) / 2 + 5);
        text.setAttribute('y', (y1 + y2) / 2 - 7);
        text.textContent = label;
        svg.appendChild(text);
      }
    }
  }

  const baseRender = render;
  render = function renderNestedStateTransitions() {
    baseRender();
    queueMicrotask(drawNestedTransitions);
  };
})();
