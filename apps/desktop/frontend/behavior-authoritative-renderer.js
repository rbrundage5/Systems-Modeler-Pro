(() => {
  const previousRenderCanvas = renderCanvas;

  function activeDiagram() {
    return state.behaviorSnapshot?.diagrams?.find(
      (diagram) => String(diagram.id) === String(state.selectedBehaviorDiagramId),
    ) || null;
  }

  function projectElement(id) {
    return state.snapshot?.project?.elements?.find(
      (element) => String(element.id) === String(id),
    ) || null;
  }

  function repository() {
    return state.behaviorSnapshot?.repository || {};
  }

  function stateMachine(diagram) {
    return repository().state_machines?.[String(diagram.semantic_id)] || null;
  }

  function interaction(diagram) {
    return repository().interactions?.[String(diagram.semantic_id)] || null;
  }

  function flattenRegions(regions, vertices = [], transitions = []) {
    for (const region of regions || []) {
      for (const transition of region.transitions || []) transitions.push({ transition, region });
      for (const vertex of region.vertices || []) {
        vertices.push({ vertex, region });
        const children = vertex.kind?.State?.regions || [];
        if (children.length) flattenRegions(children, vertices, transitions);
      }
    }
    return { vertices, transitions };
  }

  function vertexKind(vertex) {
    if (vertex?.kind === 'FinalState' || vertex?.kind?.FinalState != null) return 'FinalState';
    if (vertex?.kind?.State != null || vertex?.kind === 'State') return 'State';
    return vertex?.kind?.Pseudostate || '';
  }

  function transitionLabel(transition) {
    const event = transition.trigger?.event;
    let trigger = '';
    if (event?.Signal) trigger = projectElement(event.Signal.signal_id)?.name || 'signal';
    else if (event?.Call) trigger = projectElement(event.Call.operation_id)?.name || 'operation';
    else if (event?.Time) trigger = `after(${event.Time.expression})`;
    else if (event?.Change) trigger = `when(${event.Change.expression})`;
    else if (event === 'AnyReceive' || event?.AnyReceive != null) trigger = 'all';
    return `${trigger}${transition.guard ? ` [${transition.guard}]` : ''}${transition.effect ? ` / ${transition.effect}` : ''}`.trim();
  }

  function statePresentationMap(diagram) {
    return new Map((diagram.state_nodes || []).map((item) => [String(item.vertex_id), item]));
  }

  function fallbackStatePresentation(index, kind) {
    const stateLike = kind === 'State';
    return {
      x: 60 + (index % 4) * 210,
      y: 90 + Math.floor(index / 4) * 145,
      width: stateLike ? 160 : 28,
      height: stateLike ? 90 : 28,
      fallback: true,
    };
  }

  function stateNodeMarkup(vertex, kind) {
    if (kind === 'State') {
      const semantic = vertex.kind?.State || {};
      const childRegions = semantic.regions || [];
      const behaviors = [
        semantic.entry ? `<span>entry / ${escapeHtml(semantic.entry)}</span>` : '',
        semantic.do_activity ? `<span>do / ${escapeHtml(semantic.do_activity)}</span>` : '',
        semantic.exit ? `<span>exit / ${escapeHtml(semantic.exit)}</span>` : '',
      ].join('');
      const regionHint = childRegions.length
        ? `<span class="authoritative-region-summary">${childRegions.map((region) => escapeHtml(region.name || 'Region')).join(' | ')}</span>`
        : '';
      return `<strong>${escapeHtml(vertex.name || 'State')}</strong>${behaviors}${regionHint}`;
    }
    if (kind === 'Initial') return '<span class="pseudo-dot"></span>';
    if (kind === 'FinalState') return '<span class="final-ring"><i></i></span>';
    if (kind === 'Choice') return '<span class="choice-diamond"></span>';
    if (kind === 'Fork' || kind === 'Join') return '<span class="fork-bar"></span>';
    return escapeHtml(kind.replace(/([A-Z])/g, ' $1').trim());
  }

  function renderStateMachine(canvas, diagram) {
    const machine = stateMachine(diagram);
    const frame = document.createElement('div');
    frame.className = 'diagram-frame behavior-frame state-machine-frame authoritative-behavior-frame';
    const context = projectElement(diagram.context_id);
    frame.innerHTML = `<div class="diagram-header">stm [${escapeHtml(context?.name || 'classifier')}] ${escapeHtml(diagram.name)}</div>`;
    if (!machine) {
      frame.innerHTML += '<div class="behavior-render-error">State Machine semantics are missing from the Rust snapshot.</div>';
      canvas.appendChild(frame);
      return;
    }

    const { vertices, transitions } = flattenRegions(machine.regions || []);
    const presentations = statePresentationMap(diagram);
    const positions = new Map();

    vertices.forEach(({ vertex, region }, index) => {
      const kind = vertexKind(vertex);
      const presentation = presentations.get(String(vertex.id)) || fallbackStatePresentation(index, kind);
      positions.set(String(vertex.id), presentation);
      const node = document.createElement('button');
      node.className = `state-vertex state-${kind.toLowerCase()}`;
      node.dataset.semanticKind = kind;
      node.dataset.vertexId = String(vertex.id);
      node.dataset.regionId = String(region.id);
      node.style.left = `${presentation.x}px`;
      node.style.top = `${presentation.y}px`;
      node.style.width = `${presentation.width}px`;
      node.style.height = `${presentation.height}px`;
      node.innerHTML = stateNodeMarkup(vertex, kind);
      if (presentation.fallback) {
        node.classList.add('presentation-fallback');
        node.title = 'Semantic vertex exists but its Rust presentation record is missing.';
      }
      if (
        state.selectedBehaviorItem?.type === 'Vertex'
        && String(state.selectedBehaviorItem.id) === String(vertex.id)
      ) node.classList.add('selected');
      if (String(state.behaviorPending?.source || '') === String(vertex.id)) node.classList.add('connector-source');
      node.onclick = (event) => {
        event.stopPropagation();
        state.selectedBehaviorItem = { type: 'Vertex', id: vertex.id, semantic: vertex };
        render();
      };
      node.onpointerdown = (event) => {
        if (state.behaviorPending || state.behaviorTool || presentation.fallback) return;
        const startX = event.clientX;
        const startY = event.clientY;
        const originalX = presentation.x;
        const originalY = presentation.y;
        node.setPointerCapture?.(event.pointerId);
        node.onpointermove = (move) => {
          presentation.x = originalX + move.clientX - startX;
          presentation.y = originalY + move.clientY - startY;
          node.style.left = `${presentation.x}px`;
          node.style.top = `${presentation.y}px`;
        };
        node.onpointerup = async () => {
          node.onpointermove = null;
          await runCommand('Moving State vertex…', () => requireInvoke()('move_state_vertex', {
            diagramId: diagram.id,
            stateVertexId: String(vertex.id),
            x: presentation.x,
            y: presentation.y,
          }));
          await refresh();
        };
      };
      frame.appendChild(node);
    });

    const svg = document.createElementNS(SVG_NS, 'svg');
    svg.classList.add('behavior-relationship-layer');
    svg.setAttribute('width', '100%');
    svg.setAttribute('height', '100%');
    const defs = document.createElementNS(SVG_NS, 'defs');
    marker(defs, 'authoritative-state-arrow', 'M 1 1 L 11 6 L 1 11', { fill: 'none', refX: '11' });
    svg.appendChild(defs);

    for (const { transition } of transitions) {
      const source = positions.get(String(transition.source_id));
      const target = positions.get(String(transition.target_id));
      if (!source || !target) continue;
      const x1 = source.x + source.width / 2;
      const y1 = source.y + source.height / 2;
      const x2 = target.x + target.width / 2;
      const y2 = target.y + target.height / 2;
      const storedRoute = (diagram.edge_routes || []).find((route) => String(route.semantic_id) === String(transition.id)); const routePoints = storedRoute?.points?.length >= 2 ? storedRoute.points : [{ x: x1, y: y1 }, { x: x2, y: y2 }];
      const line = document.createElementNS(SVG_NS, 'polyline');
      line.setAttribute('points', routePoints.map((point) => `${point.x},${point.y}`).join(' '));
      line.setAttribute('fill', 'none');
      line.setAttribute('marker-end', 'url(#authoritative-state-arrow)');
      line.classList.add('state-transition');
      line.dataset.transitionId = String(transition.id);
      if (
        state.selectedBehaviorItem?.type === 'Transition'
        && String(state.selectedBehaviorItem.id) === String(transition.id)
      ) line.classList.add('selected');
      line.onclick = (event) => {
        event.stopPropagation();
        state.selectedBehaviorItem = { type: 'Transition', id: transition.id, semantic: transition };
        render();
      };
      svg.appendChild(line);
      const labelText = transitionLabel(transition);
      if (labelText) {
        const label = document.createElementNS(SVG_NS, 'text');
        label.classList.add('behavior-edge-label');
        const first=routePoints[0],last=routePoints[routePoints.length-1],labelPoint=storedRoute?.label_anchor||{x:(first.x+last.x)/2,y:(first.y+last.y)/2};
        label.setAttribute('x', labelPoint.x + 6);
        label.setAttribute('y', labelPoint.y - 7);
        label.textContent = labelText;
        svg.appendChild(label);
      }
    }
    frame.insertBefore(svg, frame.children[1] || null);

    if (vertices.length && presentations.size !== vertices.length) {
      const warning = document.createElement('div');
      warning.className = 'behavior-render-warning';
      warning.textContent = `Presentation integrity warning: ${vertices.length} semantic vertices, ${presentations.size} Rust presentation records.`;
      frame.appendChild(warning);
    }
    canvas.appendChild(frame);
  }

  function lifelinePresentation(diagram, id, index) {
    return diagram.lifelines?.find((item) => String(item.lifeline_id) === String(id)) || {
      x: 150 + index * 210,
      timeline_start_y: 102,
      timeline_end_y: 840,
      fallback: true,
    };
  }

  function messageOrder(message, index) {
    return message.send_event?.order ?? message.receive_event?.order ?? ((index + 1) * 10);
  }

  function messageSignatureName(message) {
    const id = message.signature?.Operation || message.signature?.Signal;
    return id ? (projectElement(id)?.name || message.name || '') : (message.name || '');
  }

  function renderSequence(canvas, diagram) {
    const inter = interaction(diagram);
    const frame = document.createElement('div');
    frame.className = 'diagram-frame behavior-frame sequence-frame authoritative-behavior-frame';
    const context = projectElement(diagram.context_id);
    frame.innerHTML = `<div class="diagram-header">seq [${escapeHtml(context?.name || 'classifier')}] ${escapeHtml(diagram.name)}</div>`;
    if (!inter) {
      frame.innerHTML += '<div class="behavior-render-error">Sequence semantics are missing from the Rust snapshot.</div>';
      canvas.appendChild(frame);
      return;
    }

    const lifelinePositions = new Map();
    (inter.lifelines || []).forEach((lifeline, index) => {
      const presentation = lifelinePresentation(diagram, lifeline.id, index);
      lifelinePositions.set(String(lifeline.id), presentation.x);
      const node = document.createElement('button');
      node.className = 'sequence-lifeline';
      node.dataset.semanticKind = 'Lifeline';
      node.dataset.lifelineId = String(lifeline.id);
      const timelineStart = Number.isFinite(presentation.timeline_start_y) ? presentation.timeline_start_y : 102;
      const timelineEnd = Number.isFinite(presentation.timeline_end_y) ? presentation.timeline_end_y : 840;
      node.style.left = `${presentation.x - 65}px`;
      node.style.height = `${Math.max(120, timelineEnd - 60)}px`;
      node.innerHTML = `<div class="lifeline-head">${escapeHtml(lifeline.name || 'Lifeline')}</div><div class="lifeline-line"></div><span class="lifeline-resize-handle" title="Resize Lifeline timeline" aria-label="Resize Lifeline timeline"></span>`;
      const timeline = node.querySelector('.lifeline-line');
      timeline.style.top = `${Math.max(42, timelineStart - 60)}px`;
      timeline.style.bottom = 'auto';
      timeline.style.height = `${Math.max(80, timelineEnd - timelineStart)}px`;
      const timelineResize = node.querySelector('.lifeline-resize-handle');
      timelineResize.style.top = `${Math.max(50, timelineEnd - 66)}px`;
      if (presentation.fallback) node.classList.add('presentation-fallback');
      if (
        state.selectedBehaviorItem?.type === 'Lifeline'
        && String(state.selectedBehaviorItem.id) === String(lifeline.id)
      ) node.classList.add('selected');
      if (String(state.behaviorPending?.source || '') === String(lifeline.id)) node.classList.add('connector-source');
      node.onclick = (event) => {
        event.stopPropagation();
        state.selectedBehaviorItem = { type: 'Lifeline', id: lifeline.id, semantic: lifeline };
        render();
      };
      if (!presentation.fallback) {
        timelineResize.onpointerdown = (event) => {
          if (state.behaviorPending || state.behaviorTool) return;
          event.preventDefault();
          event.stopPropagation();
          const startY = event.clientY;
          const originalEnd = timelineEnd;
          let nextEnd = originalEnd;
          timelineResize.setPointerCapture?.(event.pointerId);
          timelineResize.onpointermove = (move) => {
            nextEnd = Math.max(timelineStart + 80, originalEnd + move.clientY - startY);
            timeline.style.height = `${nextEnd - timelineStart}px`;
            node.style.height = `${Math.max(120, nextEnd - 60)}px`;
            timelineResize.style.top = `${Math.max(50, nextEnd - 66)}px`;
          };
          timelineResize.onpointerup = async () => {
            timelineResize.onpointermove = null;
            timelineResize.onpointerup = null;
            await runCommand('Resizing Lifeline timeline…', () => requireInvoke()('resize_sequence_lifeline_timeline', {
              diagramId: diagram.id,
              lifelineIdValue: String(lifeline.id),
              timelineStartY: timelineStart,
              timelineEndY: nextEnd,
            }));
            await refresh();
          };
        };
        node.onpointerdown = (event) => {
          if (event.target.closest?.('.lifeline-resize-handle')) return;
          if (state.behaviorPending || state.behaviorTool) return;
          const start = event.clientX;
          const original = presentation.x;
          node.setPointerCapture?.(event.pointerId);
          node.onpointermove = (move) => {
            node.style.left = `${original + move.clientX - start - 65}px`;
          };
          node.onpointerup = async (up) => {
            node.onpointermove = null;
            await runCommand('Moving Lifeline…', () => requireInvoke()('move_sequence_lifeline', {
              diagramId: diagram.id,
              lifelineIdValue: String(lifeline.id),
              x: original + up.clientX - start,
            }));
            await refresh();
          };
        };
      }
      frame.appendChild(node);
    });

    const svg = document.createElementNS(SVG_NS, 'svg');
    svg.classList.add('sequence-message-layer');
    svg.setAttribute('width', '100%');
    svg.setAttribute('height', '100%');
    const defs = document.createElementNS(SVG_NS, 'defs');
    marker(defs, 'authoritative-seq-filled', 'M 1 1 L 11 6 L 1 11 Z', { fill: '#111', refX: '11' });
    marker(defs, 'authoritative-seq-open', 'M 1 1 L 11 6 L 1 11', { fill: 'none', refX: '11' });
    svg.appendChild(defs);

    (inter.messages || []).forEach((message, index) => {
      const sourceId = message.send_event?.lifeline_id;
      const targetId = message.receive_event?.lifeline_id;
      const sourceX = sourceId ? (lifelinePositions.get(String(sourceId)) ?? 70) : 70;
      const targetX = targetId ? (lifelinePositions.get(String(targetId)) ?? 1000) : 1000;
      const y = 110 + messageOrder(message, index) * 4;
      const storedRoute = (diagram.edge_routes || []).find((route) => String(route.semantic_id) === String(message.id)); const routePoints = storedRoute?.points?.length >= 2 ? storedRoute.points : [{ x: sourceX, y }, { x: targetX, y }];
      const line = document.createElementNS(SVG_NS, 'polyline');
      line.setAttribute('points', routePoints.map((point) => `${point.x},${point.y}`).join(' '));
      line.setAttribute('fill', 'none');
      line.classList.add('sequence-message', `message-${String(message.sort).toLowerCase()}`);
      line.dataset.messageId = String(message.id);
      const openArrow = ['Reply', 'AsynchCall', 'AsynchSignal'].includes(message.sort);
      line.setAttribute('marker-end', openArrow ? 'url(#authoritative-seq-open)' : 'url(#authoritative-seq-filled)');
      if (message.sort === 'Reply') line.setAttribute('stroke-dasharray', '6 4');
      if (
        state.selectedBehaviorItem?.type === 'Message'
        && String(state.selectedBehaviorItem.id) === String(message.id)
      ) line.classList.add('selected');
      line.onclick = (event) => {
        event.stopPropagation();
        state.selectedBehaviorItem = { type: 'Message', id: message.id, semantic: message };
        render();
      };
      svg.appendChild(line);
      const label = document.createElementNS(SVG_NS, 'text');
      label.classList.add('behavior-edge-label');
      const labelPoint = storedRoute?.label_anchor || routePoints[Math.floor(routePoints.length / 2)];
      label.setAttribute('x', labelPoint.x);
      label.setAttribute('y', labelPoint.y - 6);
      const name = messageSignatureName(message);
      label.textContent = `${name}${message.arguments?.length ? `(${message.arguments.join(', ')})` : ''}`;
      svg.appendChild(label);
    });
    frame.appendChild(svg);

    for (const execution of inter.executions || []) {
      const x = lifelinePositions.get(String(execution.lifeline_id)) ?? 140;
      const bar = document.createElement('div');
      bar.className = 'execution-spec';bar.dataset.semanticKind='ExecutionSpecification';
      bar.dataset.executionId = String(execution.id);
      bar.style.left = `${x - 7}px`;
      bar.style.top = `${110 + execution.start.order * 4}px`;
      bar.style.height = `${Math.max(30, (execution.finish.order - execution.start.order) * 4)}px`;
      frame.appendChild(bar);
    }

    for (const fragment of inter.fragments || []) {
      const xs = (fragment.covered_lifelines || []).map((id) => lifelinePositions.get(String(id))).filter((value) => Number.isFinite(value));
      if (!xs.length || !fragment.operands?.length) continue;
      const top = 110 + Math.min(...fragment.operands.map((operand) => operand.start_order)) * 4;
      const bottom = 110 + Math.max(...fragment.operands.map((operand) => operand.end_order)) * 4;
      const box = document.createElement('div');
      box.className = 'combined-fragment';box.dataset.semanticKind='CombinedFragment';
      box.dataset.fragmentId = String(fragment.id);
      box.style.left = `${Math.min(...xs) - 85}px`;
      box.style.width = `${Math.max(180, Math.max(...xs) - Math.min(...xs) + 170)}px`;
      box.style.top = `${top}px`;
      box.style.height = `${Math.max(50, bottom - top)}px`;
      box.innerHTML = `<div class="fragment-tag">${escapeHtml(String(fragment.operator).toLowerCase())}</div>`;
      frame.appendChild(box);
    }

    for (const invariant of inter.state_invariants || []) {
      const x = lifelinePositions.get(String(invariant.lifeline_id)) ?? 140;
      const box = document.createElement('button');
      box.className = 'state-invariant-box';box.dataset.semanticKind='StateInvariant';
      box.dataset.invariantId = String(invariant.id);
      box.style.left = `${x - 52}px`;
      box.style.top = `${110 + invariant.order * 4}px`;
      box.textContent = `{${invariant.constraint}}`;
      box.onclick = (event) => {
        event.stopPropagation();
        state.selectedBehaviorItem = { type: 'Invariant', id: invariant.id, semantic: invariant };
        render();
      };
      frame.appendChild(box);
    }

    canvas.appendChild(frame);
  }

  renderCanvas = function renderAuthoritativeBehaviorCanvas() {
    const diagram = activeDiagram();
    if (!diagram) return previousRenderCanvas();
    const canvas = $('canvas');
    canvas.innerHTML = '';
    if (diagram.kind === 'StateMachine') renderStateMachine(canvas, diagram);
    else renderSequence(canvas, diagram);
  };
})();
