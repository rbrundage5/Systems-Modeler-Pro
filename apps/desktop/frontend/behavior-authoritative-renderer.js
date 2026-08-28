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
      frame.appendChild(node);      frame.appendChild(node);
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
      timelineResize.onclick = (event) => {
        event.preventDefault();
        event.stopPropagation();
      };
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
          if (event.button !== 0 || state.behaviorPending || state.behaviorTool) return;
          event.preventDefault();
          event.stopPropagation();
          const begin = window.smpBeginPresentationGesture;
          if (typeof begin !== 'function') return;
          const originalEnd = timelineEnd;
          let nextEnd = originalEnd;
          begin(event, {
            owner: timelineResize,
            disabled: () => !!state.behaviorPending || !!state.behaviorTool,
            onMove: (_dx, dy) => {
              nextEnd = Math.max(timelineStart + 80, originalEnd + dy);
              timeline.style.height = `${nextEnd - timelineStart}px`;
              node.style.height = `${Math.max(120, nextEnd - 60)}px`;
              timelineResize.style.top = `${Math.max(50, nextEnd - 66)}px`;
            },
            onCancel: () => render(),
            onCommit: async () => {
              await window.smpCommitPresentationGeometry('resize_sequence_lifeline_timeline', {
                diagramId: diagram.id,
                lifelineIdValue: String(lifeline.id),
                timelineStartY: timelineStart,
                timelineEndY: nextEnd,
              });
            },
          });
        };
        node.onpointerdown = (event) => {
          if (event.button !== 0 || event.target.closest?.('.lifeline-resize-handle')) return;
          if (state.behaviorPending || state.behaviorTool) return;
          event.preventDefault();
          event.stopPropagation();
          const begin = window.smpBeginPresentationGesture;
          if (typeof begin !== 'function') return;
          const original = presentation.x;
          let nextX = original;
          begin(event, {
            owner: node,
            disabled: () => !!state.behaviorPending || !!state.behaviorTool,
            onStart: () => node.classList.add('smp-dragging'),
            onMove: (dx) => {
              nextX = Math.max(70, original + dx);
              node.style.left = `${nextX - 65}px`;
              window.smpPreviewSequenceLifelineGeometry?.(inter, lifeline.id, nextX);
            },
            onCancel: () => {
              node.classList.remove('smp-dragging');
              render();
            },
            onCommit: async () => {
              node.classList.remove('smp-dragging');
              await runCommand('Moving Lifeline…', () => requireInvoke()('move_sequence_lifeline', {
                diagramId: diagram.id,
                lifelineIdValue: String(lifeline.id),
                x: nextX,
              }));
              await refresh();
            },
          });
        };
      }
      frame.appendChild(node);      frame.appendChild(node);
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
      const hit=line.cloneNode(false);hit.className.baseVal='sequence-message-hit';hit.removeAttribute('marker-end');hit.removeAttribute('stroke-dasharray');
      const selectMessage=(event)=>{event.stopPropagation();state.selectedBehaviorItem={type:'Message',id:message.id,semantic:message};render();};line.onclick=selectMessage;hit.onclick=selectMessage;
      const dragMessage=(event)=>{if(state.behaviorTool||state.behaviorPending)return;event.preventDefault();const startY=event.clientY,original=messageOrder(message,index);event.currentTarget.setPointerCapture?.(event.pointerId);event.currentTarget.onpointerup=async(up)=>{const order=Math.max(1,original+Math.round((up.clientY-startY)/4));await runCommand('Moving Message occurrence…',()=>requireInvoke()('update_sequence_message',{diagramId:diagram.id,messageIdValue:message.id,sort:message.sort,name:message.name||'',signatureId:message.signature?.Operation||message.signature?.Signal||null,arguments:message.arguments||[],order}));await refresh();};};line.onpointerdown=dragMessage;hit.onpointerdown=dragMessage;
      svg.appendChild(hit);
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

  // PR32_STATE_MACHINE_EXECUTION_BEGIN
  Object.assign(state, { stateMachineExecutionSnapshot: null, stateMachineExecutionRunning: false });
  let stateMachineRunGeneration = 0;

  function executionDiagram() {
    return state.behaviorSnapshot?.diagrams?.find((item) =>
      String(item.id) === String(state.selectedBehaviorDiagramId) && item.kind === 'StateMachine') || null;
  }

  function executionSnapshot() {
    const diagram = executionDiagram();
    const snapshot = state.stateMachineExecutionSnapshot;
    return snapshot && diagram && String(snapshot.state_machine_id) === String(diagram.semantic_id)
      ? snapshot
      : null;
  }

  function visualizeStateMachineExecution() {
    document.querySelectorAll('.state-vertex').forEach((node) =>
      node.classList.remove('runtime-active-state', 'runtime-waiting-state', 'runtime-final-state'));
    document.querySelectorAll('.state-transition').forEach((edge) =>
      edge.classList.remove('runtime-enabled-transition', 'runtime-fired-transition'));
    const snapshot = executionSnapshot();
    if (!snapshot) return;
    const active = new Set((snapshot.active_states || []).map((item) => String(item.state_id)));
    const waiting = new Set((snapshot.waiting_state_ids || []).map(String));
    const enabled = new Set((snapshot.enabled_transition_ids || []).map(String));
    const finalRegions = new Set((snapshot.final_region_ids || []).map(String));
    document.querySelectorAll('.state-vertex[data-vertex-id]').forEach((node) => {
      const id = String(node.dataset.vertexId);
      node.classList.toggle('runtime-active-state', active.has(id));
      node.classList.toggle('runtime-waiting-state', waiting.has(id));
      node.classList.toggle('runtime-final-state',
        node.dataset.semanticKind === 'FinalState' && finalRegions.has(String(node.dataset.regionId)));
    });
    document.querySelectorAll('.state-transition[data-transition-id]').forEach((edge) => {
      const id = String(edge.dataset.transitionId);
      edge.classList.toggle('runtime-enabled-transition', enabled.has(id));
      edge.classList.toggle('runtime-fired-transition', id === String(snapshot.last_transition_id || ''));
    });
  }

  function renderStateMachineExecutionPanel() {
    const host = document.querySelector('.diagram-workspace');
    const snapshot = executionSnapshot();
    let panel = document.querySelector('.state-machine-execution-panel');
    if (!host || !snapshot) { panel?.remove(); return; }
    if (!panel) {
      panel = document.createElement('aside');
      panel.className = 'state-machine-execution-panel';
      panel.dataset.workspaceOverlay = 'true';
      host.appendChild(panel);
    }
    const execution = snapshot.execution;
    const diagnostics = (execution.diagnostics || []).slice(-4);
    const trace = (execution.trace || []).slice(-6);
    panel.innerHTML = `<div class="state-machine-execution-heading"><strong>${escapeHtml(execution.state)}</strong><span>${execution.simulation_time} ns</span></div><div class="state-machine-execution-metrics"><span>Step ${execution.steps_executed}</span><span>${snapshot.pending_event_count || 0} pending</span><span>${snapshot.active_region_ids?.length || 0} region(s)</span></div><div class="state-machine-current-event">Current event: ${escapeHtml(snapshot.current_event?.name || 'None')}</div>${diagnostics.length ? `<div class="state-machine-execution-diagnostics">${diagnostics.map((item) => `<div class="runtime-${String(item.severity).toLowerCase()}">${escapeHtml(item.message)}</div>`).join('')}</div>` : ''}<div class="state-machine-execution-trace">${trace.map((item) => `<div><span>${item.simulation_time}</span>${escapeHtml(item.message)}</div>`).join('')}</div>${window.renderStructuralRuntimeInspector?.(snapshot) || ''}`;
  }

  function refreshStateMachineExecution() {
    const visible = Boolean(executionDiagram());
    let group = document.querySelector('.state-machine-execution-ribbon-group');
    if (!group) {
      group = document.createElement('section');
      group.className = 'ribbon-group state-machine-execution-ribbon-group';
      const controls = [['runtime','◎','Runtime'],['initialize','◇','Initialize'],['run','▶','Run'],['step','▸','Step'],['pause','Ⅱ','Pause'],['resume','▷','Resume'],['reset','↺','Reset'],['terminate','■','Terminate'],['signal','⇢','Signal']];
      group.innerHTML = `<div class="ribbon-actions state-machine-execution-actions">${controls.map(([command, icon, label]) => `<button class="ribbon-command" data-state-machine-execution="${command}"><span class="command-icon">${icon}</span><span>${label}</span></button>`).join('')}</div><div class="ribbon-label">State Machine Execution</div>`;
      group.addEventListener('click', handleStateMachineExecutionCommand);
      document.querySelector('.ribbon')?.insertBefore(group, document.querySelector('.ribbon-context'));
    }
    group.hidden = !visible;
    const current = executionSnapshot()?.execution?.state || 'Not initialized';
    const initialized = current !== 'Not initialized';
    group.querySelectorAll('[data-state-machine-execution]').forEach((button) => {
      const command = button.dataset.stateMachineExecution;
      button.disabled = !visible
        || (command === 'run' && (!initialized || !['Initialized', 'Paused'].includes(current)))
        || (command === 'step' && (!initialized || current === 'Running' || ['Completed', 'Failed', 'Terminated'].includes(current)))
        || (command === 'pause' && current !== 'Running')
        || (command === 'resume' && current !== 'Paused')
        || (['reset', 'terminate', 'signal'].includes(command) && !initialized);
    });
    visualizeStateMachineExecution();
    renderStateMachineExecutionPanel();
  }

  async function invokeStateMachineExecution(command, extra = {}) {
    const diagram = executionDiagram();
    if (!diagram) throw new Error('Open a State Machine diagram first.');
    const snapshot = await requireInvoke()(command, { diagramId: diagram.id, ...extra });
    Object.assign(state, { stateMachineExecutionSnapshot: snapshot });
    refreshStateMachineExecution();
    return snapshot;
  }

  async function initializeStateMachineExecution() {
    stateMachineRunGeneration += 1;
    Object.assign(state, { stateMachineExecutionRunning: false });
    await invokeStateMachineExecution('initialize_state_machine_execution');
  }

  async function runStateMachineExecution(resume) {
    if (!executionSnapshot()) await initializeStateMachineExecution();
    await invokeStateMachineExecution(resume ? 'resume_state_machine_execution' : 'run_state_machine_execution');
    const generation = ++stateMachineRunGeneration;
    Object.assign(state, { stateMachineExecutionRunning: true });
    while (generation === stateMachineRunGeneration && executionSnapshot()?.execution?.state === 'Running') {
      const before = executionSnapshot()?.execution?.revision;
      await invokeStateMachineExecution('step_state_machine_execution');
      if (executionSnapshot()?.execution?.state !== 'Running') break;
      if (executionSnapshot()?.execution?.revision === before) {
        await invokeStateMachineExecution('pause_state_machine_execution');
        break;
      }
      await new Promise((resolve) => requestAnimationFrame(resolve));
    }
    if (generation === stateMachineRunGeneration) {
      Object.assign(state, { stateMachineExecutionRunning: false });
      refreshStateMachineExecution();
    }
  }

  async function handleStateMachineExecutionCommand(event) {
    const command = event.target.closest?.('[data-state-machine-execution]')?.dataset.stateMachineExecution;
    if (!command) return;
    try {
      if (command === 'runtime') {
        await window.smpOpenStructuralRuntimeConfiguration?.('stateMachine', executionDiagram().id);
        Object.assign(state, { stateMachineExecutionSnapshot: null });
        refreshStateMachineExecution();
      } else if (command === 'initialize') await initializeStateMachineExecution();
      else if (command === 'run') await runStateMachineExecution(false);
      else if (command === 'step') {
        stateMachineRunGeneration += 1;
        if (!executionSnapshot()) await initializeStateMachineExecution();
        await invokeStateMachineExecution('step_state_machine_execution');
      } else if (command === 'pause') {
        stateMachineRunGeneration += 1;
        Object.assign(state, { stateMachineExecutionRunning: false });
        await invokeStateMachineExecution('pause_state_machine_execution');
      } else if (command === 'resume') await runStateMachineExecution(true);
      else if (command === 'reset') {
        stateMachineRunGeneration += 1;
        await invokeStateMachineExecution(executionSnapshot() ? 'reset_state_machine_execution' : 'initialize_state_machine_execution');
      } else if (command === 'terminate') {
        stateMachineRunGeneration += 1;
        await invokeStateMachineExecution('terminate_state_machine_execution');
      } else if (command === 'signal') {
        if (!executionSnapshot()) await initializeStateMachineExecution();
        const candidates = (state.snapshot?.project?.elements || [])
          .filter((item) => item.kind === 'Signal').map((item) => ({ id: item.id, label: item.name }));
        if (!candidates.length) throw new Error('Create a Signal before queuing a SignalEvent.');
        const choice = await window.smpDialogs?.choose({ title: 'Queue SignalEvent', description: 'Choose a modeled Signal for the shared runtime event queue.', candidates, confirmLabel: 'Queue' });
        if (choice?.selectedId) await invokeStateMachineExecution('queue_state_machine_signal', { signalId: choice.selectedId });
      }
    } catch (error) {
      Object.assign(state, { stateMachineExecutionRunning: false });
      window.smpDialogs?.notify?.(error?.message || String(error), 'error');
      refreshStateMachineExecution();
    }
  }

  window.smpRefreshStateMachineExecution = refreshStateMachineExecution;

  async function loadStateMachineExecutionSnapshot() {
    const diagram = executionDiagram();
    if (!diagram) {
      Object.assign(state, { stateMachineExecutionSnapshot: null });
      refreshStateMachineExecution();
      return;
    }
    try {
      const snapshot = await requireInvoke()('state_machine_execution_snapshot', { diagramId: diagram.id });
      Object.assign(state, { stateMachineExecutionSnapshot: snapshot });
    } catch (_error) {
      Object.assign(state, { stateMachineExecutionSnapshot: null });
    }
    refreshStateMachineExecution();
  }

  document.addEventListener('click', (event) => {
    const target = event.target.closest?.('.diagram-tab, .diagram-row');
    if (!target || !/\bSTM\b/.test(target.textContent || '')) return;
    queueMicrotask(loadStateMachineExecutionSnapshot);
  }, true);

  const newProjectWithStateMachineExecution = $('new-project')?.onclick;
  const openProjectWithStateMachineExecution = $('open-project')?.onclick;
  if ($('new-project')) $('new-project').onclick = async () => {
    await newProjectWithStateMachineExecution?.();
    await requireInvoke()('clear_state_machine_executions');
    await requireInvoke()('clear_sequence_executions');
    await requireInvoke()('clear_parametric_executions');
    Object.assign(state, { stateMachineExecutionSnapshot: null, parametricExecutionSnapshot: null });
    refreshStateMachineExecution();
  };
  if ($('open-project')) $('open-project').onclick = async () => {
    await openProjectWithStateMachineExecution?.();
    await requireInvoke()('clear_state_machine_executions');
    await requireInvoke()('clear_sequence_executions');
    await requireInvoke()('clear_parametric_executions');
    Object.assign(state, { stateMachineExecutionSnapshot: null, parametricExecutionSnapshot: null });
    refreshStateMachineExecution();
  };
  // PR32_STATE_MACHINE_EXECUTION_END

  // PR34_SEQUENCE_EXECUTION_BEGIN
  Object.assign(state, { sequenceExecutionSnapshot: null, sequenceExecutionRunning: false });
  let runGeneration = 0;

  function sequenceExecutionDiagram() {
    return state.behaviorSnapshot?.diagrams?.find((diagram) =>
      String(diagram.id) === String(state.selectedBehaviorDiagramId)
      && diagram.kind === 'Sequence') || null;
  }

  function sequenceExecutionSnapshot() {
    const diagram = sequenceExecutionDiagram();
    const snapshot = state.sequenceExecutionSnapshot;
    return snapshot && diagram && String(snapshot.interaction_id) === String(diagram.semantic_id)
      ? snapshot
      : null;
  }

  function visualizeSequenceExecution() {
    document.querySelectorAll('.sequence-message').forEach((message) =>
      message.classList.remove('runtime-active-message', 'runtime-completed-message'));
    const snapshot = sequenceExecutionSnapshot();
    if (!snapshot) return;
    const completed = new Set((snapshot.completed_message_ids || []).map(String));
    document.querySelectorAll('.sequence-message[data-message-id]').forEach((message) => {
      const id = String(message.dataset.messageId);
      message.classList.toggle('runtime-active-message', id === String(snapshot.active_message_id || ''));
      message.classList.toggle('runtime-completed-message', completed.has(id));
    });
  }

  function renderSequenceExecutionPanel() {
    const host = document.querySelector('.diagram-workspace');
    const snapshot = sequenceExecutionSnapshot();
    let panel = document.querySelector('.sequence-execution-panel');
    if (!host || !snapshot) {
      panel?.remove();
      return;
    }
    if (!panel) {
      panel = document.createElement('aside');
      panel.className = 'sequence-execution-panel';
      panel.dataset.workspaceOverlay = 'true';
      host.appendChild(panel);
    }
    const execution = snapshot.execution;
    const diagnostics = (execution.diagnostics || []).slice(-4);
    const trace = (execution.trace || []).slice(-6);
    panel.innerHTML = `<div class="sequence-execution-heading"><strong>${escapeHtml(execution.state)}</strong><span>${execution.simulation_time} ns</span></div>
      <div class="sequence-execution-metrics"><span>Step ${execution.steps_executed}</span><span>${snapshot.completed_message_ids?.length || 0} message(s)</span><span>${snapshot.lifeline_bindings?.length || 0} participant(s)</span></div>
      ${diagnostics.length ? `<div class="sequence-execution-diagnostics">${diagnostics.map((item) => `<div class="runtime-${String(item.severity).toLowerCase()}">${escapeHtml(item.message)}</div>`).join('')}</div>` : ''}
      <div class="sequence-execution-trace">${trace.map((item) => `<div><span>${item.simulation_time}</span>${escapeHtml(item.message)}</div>`).join('')}</div>
      ${window.renderStructuralRuntimeInspector?.(snapshot) || ''}`;
  }

  function refreshSequenceExecution() {
    const visible = Boolean(sequenceExecutionDiagram());
    let group = document.querySelector('.sequence-execution-ribbon-group');
    if (!group) {
      group = document.createElement('section');
      group.className = 'ribbon-group sequence-execution-ribbon-group';
      const controls = [
        ['runtime', '◎', 'Runtime'], ['initializeSequenceExecution', '◇', 'Initialize'],
        ['runSequenceExecution', '▶', 'Run'], ['step', '▸', 'Step'], ['pause', 'Ⅱ', 'Pause'],
        ['resume', '▷', 'Resume'], ['reset', '↺', 'Reset'], ['terminate', '■', 'Terminate'],
      ];
      group.innerHTML = `<div class="ribbon-actions sequence-execution-actions">${controls.map(([command, icon, label]) => `<button class="ribbon-command" data-sequence-execution="${command}"><span class="command-icon">${icon}</span><span>${label}</span></button>`).join('')}</div><div class="ribbon-label">Sequence Execution</div>`;
      group.addEventListener('click', handleSequenceExecutionCommand);
      document.querySelector('.ribbon')?.insertBefore(group, document.querySelector('.ribbon-context'));
    }
    group.hidden = !visible;
    const current = sequenceExecutionSnapshot()?.execution?.state || 'Not initialized';
    const initialized = current !== 'Not initialized';
    group.querySelectorAll('[data-sequence-execution]').forEach((button) => {
      const command = button.dataset.sequenceExecution;
      button.disabled = !visible
        || (command === 'runSequenceExecution' && (!initialized || !['Initialized', 'Paused'].includes(current)))
        || (command === 'step' && (!initialized || current === 'Running' || ['Completed', 'Failed', 'Terminated'].includes(current)))
        || (command === 'pause' && current !== 'Running')
        || (command === 'resume' && current !== 'Paused')
        || (['reset', 'terminate'].includes(command) && !initialized);
    });
    visualizeSequenceExecution();
    renderSequenceExecutionPanel();
  }

  async function invokeSequenceExecution(command) {
    const diagram = sequenceExecutionDiagram();
    if (!diagram) throw new Error('Open a Sequence diagram first.');
    const snapshot = await requireInvoke()(command, { diagramId: diagram.id });
    Object.assign(state, { sequenceExecutionSnapshot: snapshot });
    refreshSequenceExecution();
    return snapshot;
  }

  async function initializeSequenceExecution() {
    runGeneration += 1;
    Object.assign(state, { sequenceExecutionRunning: false });
    await invokeSequenceExecution('initialize_sequence_execution');
  }

  async function runSequenceExecution(resume) {
    if (!sequenceExecutionSnapshot()) await initializeSequenceExecution();
    await invokeSequenceExecution(resume ? 'resume_sequence_execution' : 'run_sequence_execution');
    const generation = ++runGeneration;
    Object.assign(state, { sequenceExecutionRunning: true });
    while (generation === runGeneration && sequenceExecutionSnapshot()?.execution?.state === 'Running') {
      await invokeSequenceExecution('step_sequence_execution');
      if (sequenceExecutionSnapshot()?.execution?.state !== 'Running') break;
      await new Promise((resolve) => requestAnimationFrame(resolve));
    }
    if (generation === runGeneration) {
      Object.assign(state, { sequenceExecutionRunning: false });
      refreshSequenceExecution();
    }
  }

  async function handleSequenceExecutionCommand(event) {
    const command = event.target.closest?.('[data-sequence-execution]')?.dataset.sequenceExecution;
    if (!command) return;
    try {
      if (command === 'runtime') {
        await window.smpOpenStructuralRuntimeConfiguration?.('sequence', sequenceExecutionDiagram().id);
        Object.assign(state, { sequenceExecutionSnapshot: null });
        refreshSequenceExecution();
      } else if (command === 'initializeSequenceExecution') await initializeSequenceExecution();
      else if (command === 'runSequenceExecution') await runSequenceExecution(false);
      else if (command === 'step') {
        runGeneration += 1;
        if (!sequenceExecutionSnapshot()) await initializeSequenceExecution();
        await invokeSequenceExecution('step_sequence_execution');
      } else if (command === 'pause') {
        runGeneration += 1;
        Object.assign(state, { sequenceExecutionRunning: false });
        await invokeSequenceExecution('pause_sequence_execution');
      } else if (command === 'resume') await runSequenceExecution(true);
      else if (command === 'reset') {
        runGeneration += 1;
        await invokeSequenceExecution(sequenceExecutionSnapshot() ? 'reset_sequence_execution' : 'initialize_sequence_execution');
      } else if (command === 'terminate') {
        runGeneration += 1;
        await invokeSequenceExecution('terminate_sequence_execution');
      }
    } catch (error) {
      Object.assign(state, { sequenceExecutionRunning: false });
      window.smpDialogs?.notify?.(error?.message || String(error), 'error');
      refreshSequenceExecution();
    }
  }

  async function loadSequenceExecutionSnapshot() {
    const diagram = sequenceExecutionDiagram();
    if (!diagram) {
      Object.assign(state, { sequenceExecutionSnapshot: null });
      refreshSequenceExecution();
      return;
    }
    try {
      const snapshot = await requireInvoke()('sequence_execution_snapshot', { diagramId: diagram.id });
      Object.assign(state, { sequenceExecutionSnapshot: snapshot });
    } catch (_error) {
      Object.assign(state, { sequenceExecutionSnapshot: null });
    }
    refreshSequenceExecution();
  }

  window.smpRefreshSequenceExecution = refreshSequenceExecution;
  // PR34_SEQUENCE_EXECUTION_END


  renderCanvas = function renderAuthoritativeBehaviorCanvas() {
    const diagram = activeDiagram();
    if (!diagram) return previousRenderCanvas();
    const canvas = $('canvas');
    canvas.innerHTML = '';
    if (diagram.kind === 'StateMachine') renderStateMachine(canvas, diagram);
    else renderSequence(canvas, diagram);
    queueMicrotask(loadStateMachineExecutionSnapshot);
    queueMicrotask(loadSequenceExecutionSnapshot);
  };
})();

// PR33_STRUCTURAL_RUNTIME_INSPECTOR_BEGIN
(() => {
  'use strict';

  const html = (value) => String(value ?? '')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#039;');

  const idText = (value) => String(value ?? '');

  function runtimeValueText(value) {
    if (value == null || value === 'Unset') return 'unset';
    if (typeof value !== 'object') return String(value);
    const [kind, content] = Object.entries(value)[0] || ['Unset', 'unset'];
    if (kind === 'ElementReference') return `element ${content}`;
    return String(content);
  }

  function renderBehavior(snapshot, instanceId) {
    if (idText(snapshot.runtime_instance_id) !== instanceId) return '';
    if (Array.isArray(snapshot.call_frames)) {
      const frames = snapshot.call_frames.map((frame) => html(frame.activity_name)).join(' → ');
      return frames
        ? `<div class="structural-runtime-behavior"><b>Activity</b><span>${frames}</span></div>`
        : '';
    }
    if (Array.isArray(snapshot.active_states)) {
      const states = snapshot.active_states.map((state) => html(state.state_name)).join(', ');
      return `<div class="structural-runtime-behavior"><b>State</b><span>${states || 'No active state'}</span></div>`;
    }
    return '';
  }

  function renderInstance(snapshot, runtime, instance, children, valueDefinitions, depth) {
    const execution = snapshot.execution;
    const instanceId = idText(instance.id);
    const values = (execution.runtime_values || [])
      .filter((entry) => idText(entry.key?.instance_id) === instanceId)
      .map((entry) => {
        const definition = valueDefinitions.get(idText(entry.key?.semantic_element_id));
        const unit = definition?.unit_symbol ? ` ${html(definition.unit_symbol)}` : '';
        return `<li><b>${html(definition?.name || entry.key?.semantic_element_id)}</b> = ${html(runtimeValueText(entry.value))}${unit}</li>`;
      });
    const ports = (runtime.ports || [])
      .filter((port) => idText(port.owner_instance_id) === instanceId)
      .map((port) => {
        const contracts = (port.flow_contracts || [])
          .map((flow) => `${html(String(flow.direction).toLowerCase())} ${html(flow.name)} : ${html(flow.type_name)}`)
          .join('; ');
        return `<li title="Port ${html(port.semantic_port_id)}"><b>${html(port.qualified_path.split('.').at(-1))}</b> : ${html(port.type_name)} <span class="structural-runtime-tag">${html(port.kind)}</span>${port.is_conjugated ? ' <span class="structural-runtime-tag">conjugated</span>' : ''}${contracts ? `<small>${contracts}</small>` : ''}</li>`;
      });
    const links = (runtime.connector_links || [])
      .filter((link) => idText(link.source?.instance_id) === instanceId || idText(link.target?.instance_id) === instanceId)
      .map((link) => {
        const outgoing = idText(link.source?.instance_id) === instanceId;
        const peer = outgoing ? link.target : link.source;
        return `<li title="Connector ${html(link.semantic_connector_id)}">${outgoing ? '→' : '←'} ${html(peer?.qualified_path)} <span class="structural-runtime-tag">${html(link.kind)}</span></li>`;
      });
    const events = (execution.scheduled_events || [])
      .filter((scheduled) => idText(scheduled.event?.target_runtime_instance_id) === instanceId)
      .map((scheduled) => `<li><b>${html(scheduled.event?.name)}</b> at ${html(scheduled.due_time)} ns${scheduled.event?.target_port_id ? ` → Port ${html(scheduled.event.target_port_id)}` : ''}</li>`);
    const nested = (children.get(instanceId) || [])
      .map((child) => renderInstance(snapshot, runtime, child, children, valueDefinitions, depth + 1))
      .join('');
    const usage = instance.semantic_usage_id
      ? `<span>usage ${html(instance.name)} · ${html(instance.semantic_usage_id)}</span>`
      : '<span>configured/root occurrence</span>';
    return `<details class="structural-runtime-instance" data-depth="${depth}" ${depth < 2 ? 'open' : ''}>
      <summary><span class="structural-runtime-path">${html(instance.qualified_path)}</span><span>: ${html(instance.classifier_name || instance.classifier_id)}</span></summary>
      <div class="structural-runtime-identity" title="Runtime instance ${html(instanceId)}"><span>runtime ${html(instanceId)}</span>${usage}<span>classifier ${html(instance.classifier_id)}</span></div>
      ${renderBehavior(snapshot, instanceId)}
      ${values.length ? `<div class="structural-runtime-section"><b>Values</b><ul>${values.join('')}</ul></div>` : ''}
      ${ports.length ? `<div class="structural-runtime-section"><b>Ports</b><ul>${ports.join('')}</ul></div>` : ''}
      ${links.length ? `<div class="structural-runtime-section"><b>Connector endpoints</b><ul>${links.join('')}</ul></div>` : ''}
      ${events.length ? `<div class="structural-runtime-section"><b>Pending addressed events</b><ul>${events.join('')}</ul></div>` : ''}
      ${nested ? `<div class="structural-runtime-children">${nested}</div>` : ''}
    </details>`;
  }

  window.renderStructuralRuntimeInspector = function renderStructuralRuntimeInspector(snapshot) {
    const runtime = snapshot?.execution?.structural_runtime;
    if (!runtime) return '';
    const instances = runtime.instances || [];
    const byId = new Map(instances.map((instance) => [idText(instance.id), instance]));
    const children = new Map();
    for (const instance of instances) {
      const ownerId = idText(instance.owner_runtime_instance_id);
      if (!ownerId) continue;
      const owned = children.get(ownerId) || [];
      owned.push(instance);
      children.set(ownerId, owned);
    }
    for (const owned of children.values()) {
      owned.sort((left, right) => String(left.qualified_path).localeCompare(String(right.qualified_path)));
    }
    const valueDefinitions = new Map((runtime.value_definitions || [])
      .map((definition) => [idText(definition.semantic_property_id), definition]));
    const roots = (runtime.root_instance_ids || [])
      .map((id) => byId.get(idText(id)))
      .filter(Boolean);
    return `<section class="structural-runtime-inspector" aria-label="System runtime inspection">
      <div class="structural-runtime-heading"><strong>System Runtime</strong><span>${instances.length} instance(s) · ${(runtime.connector_links || []).length} link(s)</span></div>
      <div class="structural-runtime-tree">${roots.map((instance) => renderInstance(snapshot, runtime, instance, children, valueDefinitions, 0)).join('')}</div>
    </section>`;
  };
})();
// PR33_STRUCTURAL_RUNTIME_INSPECTOR_END


// PR33_STRUCTURAL_RUNTIME_CONFIGURATION_BEGIN
(() => {
  'use strict';

  const invoke = () => {
    const command = window.__TAURI__?.core?.invoke;
    if (!command) throw new Error('Tauri command bridge is unavailable.');
    return command;
  };
  const esc = (value) => String(value ?? '')
    .replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;').replaceAll("'", '&#039;');

  function parseArray(text, label) {
    const source = String(text || '').trim();
    if (!source) return [];
    let value;
    try { value = JSON.parse(source); } catch (error) {
      throw new Error(`${label} must be valid JSON: ${error.message}`);
    }
    if (!Array.isArray(value)) throw new Error(`${label} must be a JSON array.`);
    return value;
  }

  function selectionFromDialog(dialog) {
    return {
      root_semantic_id: dialog.querySelector('[data-runtime-root]').value || null,
      structural_configuration: {
        root_instance_name: dialog.querySelector('[data-runtime-root-name]').value.trim() || null,
        populations: parseArray(dialog.querySelector('[data-runtime-populations]').value, 'Population decisions'),
        reference_bindings: parseArray(dialog.querySelector('[data-runtime-references]').value, 'Reference bindings'),
        configured_instance_specification_ids: parseArray(dialog.querySelector('[data-runtime-instances]').value, 'Configured InstanceSpecification IDs'),
      },
      runtime_instance_path: dialog.querySelector('[data-runtime-path]').value.trim() || null,
    };
  }

  function commands(kind) {
    if (kind === 'activity') return {
      get: 'activity_execution_runtime_selection',
      preview: 'preview_activity_execution_runtime',
      configure: 'configure_activity_execution_runtime',
      label: 'Activity',
    };
    if (kind === 'sequence') return {
      get: 'sequence_execution_runtime_selection',
      preview: 'preview_sequence_execution_runtime',
      configure: 'configure_sequence_execution_runtime',
      label: 'Sequence',
    };
    if (kind === 'parametric') return {
      get: 'parametric_execution_runtime_selection',
      preview: 'preview_parametric_execution_runtime',
      configure: 'configure_parametric_execution_runtime',
      label: 'Parametric',
    };
    return {
      get: 'state_machine_execution_runtime_selection',
      preview: 'preview_state_machine_execution_runtime',
      configure: 'configure_state_machine_execution_runtime',
      label: 'State Machine',
    };
  }

  function runtimeRootOptions(current) {
    const elements = window.smpState?.snapshot?.project?.elements || [];
    const supported = new Set(['Block', 'AssociationBlock', 'PartProperty', 'InstanceSpecification']);
    return elements
      .filter((element) => supported.has(element.kind))
      .sort((left, right) => `${left.name}:${left.kind}`.localeCompare(`${right.name}:${right.kind}`))
      .map((element) => `<option value="${esc(element.id)}" ${String(current || '') === String(element.id) ? 'selected' : ''}>${esc(element.name)} · ${esc(element.kind)}</option>`)
      .join('');
  }

  window.smpOpenStructuralRuntimeConfiguration = async function openStructuralRuntimeConfiguration(kind, diagramId) {
    const api = commands(kind);
    const existing = await invoke()(api.get, { diagramId });
    document.querySelector('.structural-runtime-config-backdrop')?.remove();
    const backdrop = document.createElement('div');
    backdrop.className = 'structural-runtime-config-backdrop';
    const structural = existing?.structural_configuration || {};
    backdrop.innerHTML = `<section class="structural-runtime-config-dialog" role="dialog" aria-modal="true" aria-label="${esc(api.label)} runtime configuration">
      <header><div><strong>${esc(api.label)} Runtime Context</strong><p>Choose the structural system occurrence this execution runs on. Rust validates and owns the resulting runtime graph.</p></div><button type="button" data-runtime-close aria-label="Close">×</button></header>
      <label>Structural execution root<select data-runtime-root><option value="">Behavior context (default)</option>${runtimeRootOptions(existing?.root_semantic_id)}</select></label>
      <label>Root occurrence name<input data-runtime-root-name value="${esc(structural.root_instance_name || '')}" placeholder="Optional engineer-facing runtime root name" /></label>
      <label>Behavior runtime occurrence<input data-runtime-path list="structural-runtime-compatible-paths" value="${esc(existing?.runtime_instance_path || '')}" placeholder="Auto-select when exactly one compatible occurrence exists" /><datalist id="structural-runtime-compatible-paths"></datalist></label>
      <details><summary>Advanced structural configuration</summary>
        <p class="muted">These are transient runtime decisions. They do not modify PartProperty multiplicity, ReferenceProperty ownership, or authored InstanceSpecifications.</p>
        <label>Population decisions (JSON array)<textarea data-runtime-populations rows="4">${esc(JSON.stringify(structural.populations || [], null, 2))}</textarea></label>
        <label>Reference bindings (JSON array)<textarea data-runtime-references rows="5">${esc(JSON.stringify(structural.reference_bindings || [], null, 2))}</textarea></label>
        <label>Additional configured InstanceSpecification IDs (JSON array)<textarea data-runtime-instances rows="3">${esc(JSON.stringify(structural.configured_instance_specification_ids || [], null, 2))}</textarea></label>
      </details>
      <div class="structural-runtime-config-preview" data-runtime-preview>Preview has not been run.</div>
      <footer><button type="button" data-runtime-preview-button>Preview</button><button type="button" data-runtime-apply>Apply Runtime Context</button><button type="button" data-runtime-close>Cancel</button></footer>
    </section>`;
    document.body.appendChild(backdrop);
    const dialog = backdrop.querySelector('.structural-runtime-config-dialog');
    const previewHost = dialog.querySelector('[data-runtime-preview]');
    const list = dialog.querySelector('#structural-runtime-compatible-paths');
    const close = () => backdrop.remove();
    dialog.querySelectorAll('[data-runtime-close]').forEach((button) => button.onclick = close);
    backdrop.addEventListener('click', (event) => { if (event.target === backdrop) close(); });

    async function preview() {
      const selection = selectionFromDialog(dialog);
      const result = await invoke()(api.preview, { diagramId, selection });
      list.innerHTML = (result.compatible_runtime_instance_paths || [])
        .map((path) => `<option value="${esc(path)}"></option>`).join('');
      const runtime = result.structural_runtime;
      previewHost.innerHTML = runtime
        ? `<b>${runtime.instances?.length || 0} runtime instance(s)</b> · ${runtime.ports?.length || 0} Port(s) · ${runtime.connector_links?.length || 0} connector link(s)<br/>Compatible behavior occurrence(s): ${esc((result.compatible_runtime_instance_paths || []).join(', ') || 'none')}`
        : 'This behavior has no structural runtime context. Existing non-structural execution will be preserved.';
      if (!dialog.querySelector('[data-runtime-path]').value && result.selected_runtime_instance_path) {
        dialog.querySelector('[data-runtime-path]').value = result.selected_runtime_instance_path;
      }
      return result;
    }

    dialog.querySelector('[data-runtime-preview-button]').onclick = async () => {
      try { await preview(); }
      catch (error) { previewHost.textContent = error?.message || String(error); previewHost.classList.add('runtime-error'); }
    };
    dialog.querySelector('[data-runtime-apply]').onclick = async () => {
      try {
        const selection = selectionFromDialog(dialog);
        await invoke()(api.configure, { diagramId, selection });
        if (kind === 'activity') {
          window.smpState.activityExecutionSnapshot = null;
          window.smpRefreshActivityExecution?.();
        } else if (kind === 'sequence') {
          window.smpState.sequenceExecutionSnapshot = null;
          window.smpRefreshSequenceExecution?.();
        } else if (kind === 'parametric') {
          window.smpState.parametricExecutionSnapshot = null;
          window.smpRefreshParametricExecution?.();
        } else {
          window.smpState.stateMachineExecutionSnapshot = null;
          window.smpRefreshStateMachineExecution?.();
        }
        window.smpDialogs?.notify?.('Runtime context configured. Initialize execution to build the validated structural runtime.', 'info');
        close();
      } catch (error) {
        previewHost.textContent = error?.message || String(error);
        previewHost.classList.add('runtime-error');
      }
    };
    try { await preview(); } catch (error) { previewHost.textContent = error?.message || String(error); }
  };
})();
// PR33_STRUCTURAL_RUNTIME_CONFIGURATION_END
