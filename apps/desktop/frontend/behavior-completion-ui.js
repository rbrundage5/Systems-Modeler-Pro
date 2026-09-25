(() => {
  state.behaviorPaletteCache = state.behaviorPaletteCache || {};
  state.behaviorTargetRegionId = null;

  const MESSAGE_TOOLS = new Set([
    'SynchCall', 'AsynchCall', 'AsynchSignal', 'Reply',
    'Create', 'Delete', 'Lost', 'Found',
  ]);
  const STATE_VERTEX_TOOLS = new Set([
    'State', 'CompositeState', 'OrthogonalState', 'Initial', 'FinalState',
    'Choice', 'Junction', 'Fork', 'Join', 'ShallowHistory', 'DeepHistory',
    'EntryPoint', 'ExitPoint', 'Terminate',
  ]);

  function activeBehaviorDiagram() {
    return state.behaviorSnapshot?.diagrams?.find(
      (diagram) => diagram.id === state.selectedBehaviorDiagramId,
    ) || null;
  }

  function repository() {
    return state.behaviorSnapshot?.repository || null;
  }

  function interaction(diagram = activeBehaviorDiagram()) {
    if (!diagram || diagram.kind !== 'Sequence') return null;
    return repository()?.interactions?.[diagram.semantic_id] || null;
  }

  function stateMachine(diagram = activeBehaviorDiagram()) {
    if (!diagram || diagram.kind !== 'StateMachine') return null;
    return repository()?.state_machines?.[diagram.semantic_id] || null;
  }

  function projectElement(id) {
    return state.snapshot?.project?.elements?.find((element) => element.id === id) || null;
  }

  function flattenedVertices(regions, output = []) {
    for (const region of regions || []) {
      for (const vertex of region.vertices || []) {
        output.push({ vertex, region });
        const childRegions = vertex.kind?.State?.regions;
        if (childRegions) flattenedVertices(childRegions, output);
      }
    }
    return output;
  }

  function vertexKind(vertex) {
    if (!vertex) return '';
    if (vertex.kind === 'FinalState' || vertex.kind?.FinalState != null) return 'FinalState';
    if (vertex.kind?.State != null || vertex.kind === 'State') return 'State';
    return vertex.kind?.Pseudostate || '';
  }

  function messageOrder(message) {
    return message.send_event?.order
      ?? Math.max(0, (message.receive_event?.order ?? 1) - 1);
  }

  function signatureId(message) {
    return message.signature?.Operation || message.signature?.Signal || null;
  }

  function lifelineX(diagram, id) {
    return diagram.lifelines?.find((item) => item.lifeline_id === id)?.x ?? 140;
  }

  function activateBehaviorTool(id) {
    state.selectedBehaviorItem = null;
    if (id === 'Transition') {
      state.behaviorTool = null;
      state.behaviorPending = { kind: 'TransitionComplete', source: null, regionId: null };
    } else if (id === 'Lost' || id === 'Found') {
      state.behaviorTool = null;
      state.behaviorPending = { kind: 'SingleEndedMessage', sort: id };
    } else if (MESSAGE_TOOLS.has(id)) {
      state.behaviorTool = null;
      state.behaviorPending = { kind: 'Message', sort: id, source: null };
    } else {
      state.behaviorPending = null;
      state.behaviorTool = id;
    }
    render();
  }

  const baseRenderPalette = renderPalette;
  renderPalette = function renderRustBehaviorPalette() {
    const diagram = activeBehaviorDiagram();
    if (!diagram) return baseRenderPalette();
    const host = $('palette');
    const items = state.behaviorPaletteCache[diagram.kind];
    host.innerHTML = '';
    if (!items) {
      host.innerHTML = '<div class="palette-hint">Loading Rust-defined behavior palette…</div>';
      requireInvoke()('diagram_palette', { diagramType: diagram.kind })
        .then((loaded) => {
          state.behaviorPaletteCache[diagram.kind] = loaded;
          renderPalette();
        })
        .catch((error) => {
          host.innerHTML = `<div class="palette-hint">${escapeHtml(error?.message || String(error))}</div>`;
        });
      return;
    }
    const hint = document.createElement('div');
    hint.className = 'palette-hint behavior-palette-hint';
    hint.textContent = diagram.kind === 'StateMachine'
      ? 'Rust owns State, Region, Transition, trigger, guard, effect, and validation semantics.'
      : 'Rust owns Lifeline paths, Message ordering/endpoints, executions, fragments, and invariants.';
    host.appendChild(hint);
    const section = document.createElement('section');
    section.className = 'palette-section';
    section.innerHTML = `<div class="palette-section-title">${diagram.kind === 'StateMachine' ? 'States & Transitions' : 'Interaction Elements'}</div>`;
    for (const item of items) {
      const button = document.createElement('button');
      button.className = `palette-item ${item.category === 'relationship' ? 'relationship' : 'element'}`;
      button.dataset.behaviorTool = item.id;
      if (
        state.behaviorTool === item.id
        || state.behaviorPending?.sort === item.id
        || (item.id === 'Transition' && state.behaviorPending?.kind === 'TransitionComplete')
      ) button.classList.add('active');
      const symbol = item.id === 'Transition' ? '→'
        : item.id === 'Lifeline' ? '┆'
          : ['State', 'CompositeState', 'OrthogonalState'].includes(item.id) ? '▢' : '•';
      button.innerHTML = `<span class="palette-symbol">${symbol}</span><span>${escapeHtml(item.label)}</span>`;
      button.onclick = () => activateBehaviorTool(item.id);
      section.appendChild(button);
    }
    host.appendChild(section);
  };

  async function createStateToolAt(frame, diagram, event) {
    const tool = state.behaviorTool;
    if (!STATE_VERTEX_TOOLS.has(tool)) return false;
    const rect = frame.getBoundingClientRect();
    const x = Math.max(12, event.clientX - rect.left - 70);
    const y = Math.max(42, event.clientY - rect.top - 35);
    const nameRequired = ['State', 'CompositeState', 'OrthogonalState'].includes(tool);
    const name = nameRequired
      ? prompt(`${tool === 'State' ? 'State' : tool.replace('State', ' State')} name`, 'State')
      : '';
    if (nameRequired && !name) return true;
    const regionIdValue = state.behaviorTargetRegionId || null;
    if (tool === 'CompositeState' || tool === 'OrthogonalState') {
      await runCommand(`Creating ${tool}…`, () => requireInvoke()('add_composite_state', {
        diagramId: diagram.id,
        regionIdValue,
        name,
        orthogonal: tool === 'OrthogonalState',
        x,
        y,
      }));
    } else {
      await runCommand(`Creating ${tool}…`, () => requireInvoke()('add_state_vertex', {
        diagramId: diagram.id,
        regionIdValue,
        kind: tool,
        name: name || '',
        x,
        y,
      }));
    }
    state.behaviorTool = null;
    await refresh();
    return true;
  }

  async function commitTransition(diagram, vertexId, regionId) {
    const pending = state.behaviorPending;
    if (!pending?.source) {
      state.behaviorPending = {
        kind: 'TransitionComplete',
        source: vertexId,
        regionId,
      };
      render();
      return;
    }
    const source = pending.source;
    const ownerRegion = pending.regionId === regionId ? regionId : null;
    state.behaviorPending = null;
    try {
      await runCommand('Creating Transition…', () => requireInvoke()('add_state_transition', {
        diagramId: diagram.id,
        regionIdValue: ownerRegion,
        sourceVertexId: source,
        targetVertexId: vertexId,
        kind: 'External',
        eventKind: 'None',
        eventReferenceId: null,
        eventExpression: null,
        guard: '',
        effect: '',
      }));
      await refresh();
    } catch (error) {
      state.behaviorPending = { kind: 'TransitionComplete', source: null, regionId: null };
      render();
    }
  }

  function decorateCompositeRegions(machine) {
    const vertices = flattenedVertices(machine?.regions);
    const nodes = [...document.querySelectorAll('.state-vertex')];
    nodes.forEach((node, index) => {
      const entry = vertices[index];
      if (!entry) return;
      node.dataset.vertexId = entry.vertex.id;
      node.dataset.regionId = entry.region.id;
      const regions = entry.vertex.kind?.State?.regions || [];
      if (!regions.length) return;
      node.classList.add('composite-state');
      const host = document.createElement('div');
      host.className = 'state-region-grid';
      for (const region of regions) {
        const regionView = document.createElement('div');
        regionView.className = 'state-region-cell';
        regionView.innerHTML = `<span>${escapeHtml(region.name || 'Region')}</span>`;
        host.appendChild(regionView);
      }
      node.appendChild(host);
    });
  }

  function decorateStateMachine(diagram) {
    const machine = stateMachine(diagram);
    const frame = document.querySelector('.state-machine-frame');
    if (!machine || !frame) return;
    decorateCompositeRegions(machine);
    frame.addEventListener('click', async (event) => {
      if (event.target !== frame) return;
      if (!STATE_VERTEX_TOOLS.has(state.behaviorTool)) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      await createStateToolAt(frame, diagram, event);
    }, true);
    for (const node of frame.querySelectorAll('.state-vertex')) {
      node.addEventListener('click', async (event) => {
        if (state.behaviorPending?.kind !== 'TransitionComplete') return;
        event.preventDefault();
        event.stopImmediatePropagation();
        await commitTransition(diagram, node.dataset.vertexId, node.dataset.regionId);
      }, true);
    }
  }

  async function createSingleEndedMessage(diagram, lifelineId, sort) {
    const name = prompt('Message name', sort === 'Lost' ? 'lost' : 'found') || '';
    const sourceLifelineId = sort === 'Lost' ? lifelineId : null;
    const targetLifelineId = sort === 'Found' ? lifelineId : null;
    state.behaviorPending = null;
    await runCommand(`Creating ${sort} Message…`, () => requireInvoke()('add_sequence_message', {
      diagramId: diagram.id,
      sourceLifelineId,
      targetLifelineId,
      sort,
      name,
      signatureId: null,
      arguments: [],
    }));
    await refresh();
  }

  function drawSingleEndedMarker(svg, x, y, messageId) {
    const circle = document.createElementNS(SVG_NS, 'circle');
    circle.setAttribute('cx', x);
    circle.setAttribute('cy', y);
    circle.setAttribute('r', '5');
    circle.classList.add('unknown-message-end');
    circle.dataset.messageId = messageId;
    svg.appendChild(circle);
  }

  function makeSelfMessage(svg, line, message, x, y) {
    line.style.display = 'none';
    const path = document.createElementNS(SVG_NS, 'path');
    path.setAttribute('d', `M ${x} ${y} h 46 v 26 h -46`);
    path.setAttribute('fill', 'none');
    path.setAttribute('marker-end', line.getAttribute('marker-end') || 'url(#seq-filled)');
    if (message.sort === 'Reply') path.setAttribute('stroke-dasharray', '6 4');
    path.classList.add('sequence-message', 'self-message', `message-${message.sort.toLowerCase()}`);
    path.dataset.messageId = message.id;
    path.onclick = (event) => {
      event.stopPropagation();
      state.selectedBehaviorItem = { type: 'Message', id: message.id, semantic: message };
      render();
    };
    svg.appendChild(path);
    return path;
  }

  function installMessageDrag(element, diagram, message) {
    element.onpointerdown = (event) => {
      if (state.behaviorTool || state.behaviorPending) return;
      event.preventDefault();
      const startY = event.clientY;
      const originalOrder = messageOrder(message);
      element.setPointerCapture?.(event.pointerId);
      element.onpointerup = async (up) => {
        element.onpointerup = null;
        const deltaOrder = Math.round((up.clientY - startY) / 4);
        const order = Math.max(1, originalOrder + deltaOrder);
        await runCommand('Moving Message occurrence…', () => requireInvoke()('update_sequence_message', {
          diagramId: diagram.id,
          messageIdValue: message.id,
          sort: message.sort,
          name: message.name || '',
          signatureId: signatureId(message),
          arguments: message.arguments || [],
          order,
        }));
        await refresh();
      };
    };
  }

  function renderStateInvariants(frame, diagram, inter) {
    for (const invariant of inter.state_invariants || []) {
      const box = document.createElement('button');
      box.className = 'state-invariant-box';
      box.style.left = `${lifelineX(diagram, invariant.lifeline_id) - 52}px`;
      box.style.top = `${110 + invariant.order * 4}px`;
      box.textContent = `{${invariant.constraint}}`;
      if (
        state.selectedBehaviorItem?.type === 'Invariant'
        && state.selectedBehaviorItem.id === invariant.id
      ) box.classList.add('selected');
      box.onclick = (event) => {
        event.stopPropagation();
        state.selectedBehaviorItem = { type: 'Invariant', id: invariant.id, semantic: invariant };
        render();
      };
      frame.appendChild(box);
    }
  }

  function decorateExecutions(diagram, inter) {
    const bars = [...document.querySelectorAll('.execution-spec')];
    bars.forEach((bar, index) => {
      const execution = inter.executions?.[index];
      if (!execution) return;
      bar.dataset.executionId = execution.id;
      bar.tabIndex = 0;
      if (
        state.selectedBehaviorItem?.type === 'Execution'
        && state.selectedBehaviorItem.id === execution.id
      ) bar.classList.add('selected');
      bar.onclick = (event) => {
        event.stopPropagation();
        state.selectedBehaviorItem = { type: 'Execution', id: execution.id, semantic: execution };
        render();
      };
      const resize = document.createElement('span');
      resize.className = 'execution-resize-handle';
      resize.onpointerdown = (event) => {
        event.stopPropagation();
        const startY = event.clientY;
        const originalFinish = execution.finish.order;
        resize.setPointerCapture?.(event.pointerId);
        resize.onpointerup = async (up) => {
          resize.onpointerup = null;
          const finishOrder = Math.max(
            execution.start.order + 1,
            originalFinish + Math.round((up.clientY - startY) / 4),
          );
          await runCommand('Resizing Execution Specification…', () => requireInvoke()(
            'update_execution_specification',
            {
              diagramId: diagram.id,
              executionIdValue: execution.id,
              startOrder: execution.start.order,
              finishOrder,
            },
          ));
          await refresh();
        };
      };
      bar.appendChild(resize);
    });
  }

  function decorateFragments(inter) {
    const boxes = [...document.querySelectorAll('.combined-fragment')];
    boxes.forEach((box, index) => {
      const fragment = inter.fragments?.[index];
      if (!fragment) return;
      box.dataset.fragmentId = fragment.id;
      box.tabIndex = 0;
      if (
        state.selectedBehaviorItem?.type === 'Fragment'
        && state.selectedBehaviorItem.id === fragment.id
      ) box.classList.add('selected');
      box.onclick = (event) => {
        event.stopPropagation();
        state.selectedBehaviorItem = { type: 'Fragment', id: fragment.id, semantic: fragment };
        render();
      };
      const minStart = Math.min(...fragment.operands.map((operand) => operand.start_order));
      for (const [operandIndex, operand] of fragment.operands.entries()) {
        if (operandIndex > 0) {
          const divider = document.createElement('div');
          divider.className = 'fragment-operand-divider';
          divider.style.top = `${(operand.start_order - minStart) * 4}px`;
          box.appendChild(divider);
        }
        if (operand.guard) {
          const guard = document.createElement('div');
          guard.className = 'fragment-operand-guard';
          guard.style.top = `${Math.max(20, (operand.start_order - minStart) * 4 + 4)}px`;
          guard.textContent = `[${operand.guard}]`;
          box.appendChild(guard);
        }
      }
    });
  }

  function decorateSequence(diagram) {
    const inter = interaction(diagram);
    const frame = document.querySelector('.sequence-frame');
    const svg = frame?.querySelector('.sequence-message-layer');
    if (!inter || !frame || !svg) return;
    const lifelineNodes = [...frame.querySelectorAll('.sequence-lifeline')];
    lifelineNodes.forEach((node, index) => {
      const lifeline = inter.lifelines?.[index];
      if (!lifeline) return;
      node.dataset.lifelineId = lifeline.id;
      node.addEventListener('click', async (event) => {
        const pending = state.behaviorPending;
        if (pending?.kind !== 'SingleEndedMessage') return;
        event.preventDefault();
        event.stopImmediatePropagation();
        await createSingleEndedMessage(diagram, lifeline.id, pending.sort);
      }, true);
    });
    const lines = [...svg.querySelectorAll('line.sequence-message')];
    (inter.messages || []).forEach((message, index) => {
      const line = lines[index];
      if (!line) return;
      line.dataset.messageId = message.id;
      const source = message.send_event?.lifeline_id;
      const target = message.receive_event?.lifeline_id;
      const y = 110 + messageOrder(message) * 4;
      let interactionElement = line;
      if (source && target && source === target) {
        interactionElement = makeSelfMessage(svg, line, message, lifelineX(diagram, source), y);
      }
      if (!source) drawSingleEndedMarker(svg, 70, y, message.id);
      if (!target) drawSingleEndedMarker(svg, frame.clientWidth - 70, y, message.id);
      if (
        state.selectedBehaviorItem?.type === 'Message'
        && state.selectedBehaviorItem.id === message.id
      ) interactionElement.classList.add('selected');
      installMessageDrag(interactionElement, diagram, message);
    });
    decorateExecutions(diagram, inter);
    decorateFragments(inter);
    renderStateInvariants(frame, diagram, inter);
  }

  function currentTransitionEvent(transition) {
    const event = transition.trigger?.event;
    if (!event) return { kind: 'None', referenceId: '', expression: '' };
    if (event.Signal) return { kind: 'Signal', referenceId: event.Signal.signal_id, expression: '' };
    if (event.Call) return { kind: 'Call', referenceId: event.Call.operation_id, expression: '' };
    if (event.Time) return { kind: 'Time', referenceId: '', expression: event.Time.expression || '' };
    if (event.Change) return { kind: 'Change', referenceId: '', expression: event.Change.expression || '' };
    return { kind: 'AnyReceive', referenceId: '', expression: '' };
  }

  function populateTriggerReference(eventKind, selectedId = '') {
    const host = $('behavior-trigger-detail');
    if (!host) return;
    if (eventKind === 'Signal' || eventKind === 'Call') {
      const required = eventKind === 'Signal' ? 'Signal' : 'Operation';
      const candidates = state.snapshot.project.elements.filter((element) => element.kind === required);
      host.innerHTML = `<label>${required}<select id="behavior-trigger-reference">${candidates
        .map((item) => `<option value="${escapeAttr(item.id)}"${item.id === selectedId ? ' selected' : ''}>${escapeHtml(item.name)}</option>`)
        .join('')}</select></label>`;
    } else if (eventKind === 'Time' || eventKind === 'Change') {
      host.innerHTML = `<label>${eventKind} expression<input id="behavior-trigger-expression" value="${escapeAttr(selectedId)}"></label>`;
    } else {
      host.innerHTML = '';
    }
  }

  function renderTransitionProperties(panel, diagram, transition) {
    const event = currentTransitionEvent(transition);
    panel.innerHTML = `<div class="property-heading">Transition</div>
      <label>Kind<select id="behavior-transition-kind">${['External', 'Internal', 'Local']
        .map((kind) => `<option${transition.kind === kind ? ' selected' : ''}>${kind}</option>`).join('')}</select></label>
      <label>Trigger<select id="behavior-trigger-kind">${['None', 'Signal', 'Call', 'Time', 'Change', 'AnyReceive']
        .map((kind) => `<option${event.kind === kind ? ' selected' : ''}>${kind}</option>`).join('')}</select></label>
      <div id="behavior-trigger-detail"></div>
      <label>Guard<input id="behavior-transition-guard" value="${escapeAttr(transition.guard || '')}"></label>
      <label>Effect<input id="behavior-transition-effect" value="${escapeAttr(transition.effect || '')}"></label>
      <button id="behavior-transition-apply" class="primary">Apply Transition</button>
      <div class="muted">Notation: trigger [guard] / effect. Rust rejects illegal Initial, Final, Fork, Join, and trigger/signature combinations.</div>`;
    populateTriggerReference(
      event.kind,
      event.kind === 'Time' || event.kind === 'Change' ? event.expression : event.referenceId,
    );
    $('behavior-trigger-kind').onchange = () => populateTriggerReference(
      $('behavior-trigger-kind').value,
      '',
    );
    $('behavior-transition-apply').onclick = async () => {
      const eventKind = $('behavior-trigger-kind').value;
      await runCommand('Updating Transition…', () => requireInvoke()('update_state_transition', {
        diagramId: diagram.id,
        transitionIdValue: transition.id,
        kind: $('behavior-transition-kind').value,
        eventKind,
        eventReferenceId: $('behavior-trigger-reference')?.value || null,
        eventExpression: $('behavior-trigger-expression')?.value || null,
        guard: $('behavior-transition-guard').value,
        effect: $('behavior-transition-effect').value,
      }));
      await refresh();
    };
  }

  function endpointOptions(inter, selected) {
    return `<option value="">(open / unknown)</option>${(inter.lifelines || [])
      .map((lifeline) => `<option value="${escapeAttr(lifeline.id)}"${lifeline.id === selected ? ' selected' : ''}>${escapeHtml(lifeline.name)}</option>`)
      .join('')}`;
  }

  function signatureOptions(message) {
    const kind = message.sort === 'AsynchSignal' ? 'Signal' : 'Operation';
    if (!['SynchCall', 'AsynchCall', 'AsynchSignal'].includes(message.sort)) return '';
    const selected = signatureId(message);
    return `<label>${kind}<select id="behavior-message-signature">${state.snapshot.project.elements
      .filter((element) => element.kind === kind)
      .map((element) => `<option value="${escapeAttr(element.id)}"${element.id === selected ? ' selected' : ''}>${escapeHtml(element.name)}</option>`)
      .join('')}</select></label>`;
  }

  function renderMessageProperties(panel, diagram, message) {
    const inter = interaction(diagram);
    panel.innerHTML = `<div class="property-heading">Message</div>
      <label>Sort<input value="${escapeAttr(message.sort)}" disabled></label>
      <label>Name<input id="behavior-message-name" value="${escapeAttr(message.name || '')}"></label>
      ${signatureOptions(message)}
      <label>Arguments<input id="behavior-message-arguments" value="${escapeAttr((message.arguments || []).join(', '))}"></label>
      <label>Occurrence order<input id="behavior-message-order" type="number" min="1" value="${messageOrder(message)}"></label>
      <label>Source<select id="behavior-message-source">${endpointOptions(inter, message.send_event?.lifeline_id || '')}</select></label>
      <label>Target<select id="behavior-message-target">${endpointOptions(inter, message.receive_event?.lifeline_id || '')}</select></label>
      <button id="behavior-message-apply" class="primary">Apply Message</button>
      <div class="muted">Lost Messages require an open target. Found Messages require an open source. Other Message sorts require both endpoints.</div>`;
    $('behavior-message-apply').onclick = async () => {
      const args = $('behavior-message-arguments').value.split(',').map((value) => value.trim()).filter(Boolean);
      await runCommand('Updating Message…', () => requireInvoke()('update_sequence_message', {
        diagramId: diagram.id,
        messageIdValue: message.id,
        sort: message.sort,
        name: $('behavior-message-name').value,
        signatureId: $('behavior-message-signature')?.value || null,
        arguments: args,
        order: Number($('behavior-message-order').value),
      }));
      await runCommand('Updating Message source…', () => requireInvoke()('reconnect_sequence_message', {
        diagramId: diagram.id,
        messageIdValue: message.id,
        side: 'source',
        lifelineIdValue: $('behavior-message-source').value || null,
      }));
      await runCommand('Updating Message target…', () => requireInvoke()('reconnect_sequence_message', {
        diagramId: diagram.id,
        messageIdValue: message.id,
        side: 'target',
        lifelineIdValue: $('behavior-message-target').value || null,
      }));
      await refresh();
    };
  }

  function renderExecutionProperties(panel, diagram, execution) {
    panel.innerHTML = `<div class="property-heading">Execution Specification</div>
      <label>Start order<input id="behavior-execution-start" type="number" min="0" value="${execution.start.order}"></label>
      <label>Finish order<input id="behavior-execution-finish" type="number" min="1" value="${execution.finish.order}"></label>
      <button id="behavior-execution-apply" class="primary">Apply Execution</button>`;
    $('behavior-execution-apply').onclick = async () => {
      await runCommand('Updating Execution Specification…', () => requireInvoke()(
        'update_execution_specification',
        {
          diagramId: diagram.id,
          executionIdValue: execution.id,
          startOrder: Number($('behavior-execution-start').value),
          finishOrder: Number($('behavior-execution-finish').value),
        },
      ));
      await refresh();
    };
  }

  function renderFragmentProperties(panel, diagram, fragment) {
    panel.innerHTML = `<div class="property-heading">Combined Fragment · ${escapeHtml(String(fragment.operator).toLowerCase())}</div>
      <div id="behavior-operands"></div>
      <button id="behavior-add-operand">Add Operand</button>`;
    const host = $('behavior-operands');
    for (const [index, operand] of fragment.operands.entries()) {
      const row = document.createElement('div');
      row.className = 'operand-editor';
      row.innerHTML = `<strong>Operand ${index + 1}</strong>
        <label>Guard<input id="operand-guard-${index}" value="${escapeAttr(operand.guard || '')}"></label>
        <label>Start<input id="operand-start-${index}" type="number" value="${operand.start_order}"></label>
        <label>End<input id="operand-end-${index}" type="number" value="${operand.end_order}"></label>
        <button id="operand-apply-${index}">Apply Operand</button>`;
      host.appendChild(row);
      $(`operand-apply-${index}`).onclick = async () => {
        await runCommand('Updating Interaction Operand…', () => requireInvoke()(
          'update_combined_fragment_operand',
          {
            diagramId: diagram.id,
            fragmentIdValue: fragment.id,
            operandIdValue: operand.id,
            guard: $(`operand-guard-${index}`).value,
            startOrder: Number($(`operand-start-${index}`).value),
            endOrder: Number($(`operand-end-${index}`).value),
          },
        ));
        await refresh();
      };
    }
    $('behavior-add-operand').onclick = async () => {
      const last = fragment.operands[fragment.operands.length - 1];
      const guard = prompt('Operand guard (optional)', '') || '';
      const startOrder = last?.end_order ?? 10;
      await runCommand('Adding Interaction Operand…', () => requireInvoke()(
        'add_combined_fragment_operand',
        {
          diagramId: diagram.id,
          fragmentIdValue: fragment.id,
          guard,
          startOrder,
          endOrder: startOrder + 20,
        },
      ));
      await refresh();
    };
  }

  function renderInvariantProperties(panel, diagram, invariant) {
    panel.innerHTML = `<div class="property-heading">State Invariant</div>
      <label>Constraint<input id="behavior-invariant-constraint" value="${escapeAttr(invariant.constraint)}"></label>
      <label>Order<input id="behavior-invariant-order" type="number" min="0" value="${invariant.order}"></label>
      <button id="behavior-invariant-apply" class="primary">Apply Invariant</button>`;
    $('behavior-invariant-apply').onclick = async () => {
      await runCommand('Updating State Invariant…', () => requireInvoke()('update_state_invariant', {
        diagramId: diagram.id,
        invariantIdValue: invariant.id,
        constraint: $('behavior-invariant-constraint').value,
        order: Number($('behavior-invariant-order').value),
      }));
      await refresh();
    };
  }

  function appendRegionControls(panel, vertex) {
    const regions = vertex.kind?.State?.regions || [];
    if (!regions.length) return;
    const section = document.createElement('div');
    section.className = 'region-target-picker';
    section.innerHTML = '<strong>Target Region for new vertices</strong>';
    for (const region of regions) {
      const button = document.createElement('button');
      button.textContent = region.name || 'Region';
      if (state.behaviorTargetRegionId === region.id) button.classList.add('primary');
      button.onclick = () => {
        state.behaviorTargetRegionId = region.id;
        renderProperties();
      };
      section.appendChild(button);
    }
    panel.appendChild(section);
  }

  const baseRenderProperties = renderProperties;
  renderProperties = function renderBehaviorCompletionProperties() {
    const diagram = activeBehaviorDiagram();
    if (!diagram) return baseRenderProperties();
    baseRenderProperties();
    const panel = $('properties');
    const item = state.selectedBehaviorItem;
    if (!item) return;
    if (item.type === 'Transition') {
      renderTransitionProperties(panel, diagram, item.semantic);
    } else if (item.type === 'Message') {
      renderMessageProperties(panel, diagram, item.semantic);
    } else if (item.type === 'Execution') {
      renderExecutionProperties(panel, diagram, item.semantic);
    } else if (item.type === 'Fragment') {
      renderFragmentProperties(panel, diagram, item.semantic);
    } else if (item.type === 'Invariant') {
      renderInvariantProperties(panel, diagram, item.semantic);
    } else if (item.type === 'Vertex' && vertexKind(item.semantic) === 'State') {
      appendRegionControls(panel, item.semantic);
    }
  };

  const baseRender = render;
  render = function renderBehaviorCompletion() {
    baseRender();
    const diagram = activeBehaviorDiagram();
    if (!diagram) return;
    if (diagram.kind === 'StateMachine') decorateStateMachine(diagram);
    else decorateSequence(diagram);
  };

  render();
})();
