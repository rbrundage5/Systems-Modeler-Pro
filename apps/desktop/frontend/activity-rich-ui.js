(() => {
  const RICH_NODE_TOOLS = new Set([
    'CallBehaviorAction', 'CallOperationAction', 'SendSignalAction', 'AcceptEventAction',
    'AcceptTimeEventAction', 'ActivityParameterNode',
  ]);
  const STRUCTURED_TOOLS = new Set([
    'ActivityPartition', 'StructuredActivityNode', 'ConditionalNode', 'LoopNode', 'SequenceNode',
    'ExpansionRegion', 'InterruptibleActivityRegion',
  ]);

  function activeDiagram() {
    return state.activitySnapshot?.diagrams?.find(
      (diagram) => String(diagram.id) === String(state.selectedActivityDiagramId),
    ) || null;
  }

  function activeActivity() {
    const diagram = activeDiagram();
    return diagram
      ? state.activitySnapshot?.repository?.activities?.[String(diagram.activity_id)] || null
      : null;
  }

  function projectElements(kind) {
    return (state.snapshot?.project?.elements || []).filter((element) => element.kind === kind);
  }

  function choose(candidates, label, allowNone = false) {
    if (!candidates.length) {
      if (allowNone) return null;
      alert(`Create a ${label} first.`);
      return undefined;
    }
    const lines = candidates.map((item, index) => `${index + 1}. ${item.name}`).join('\n');
    const prefix = allowNone ? '0. None\n' : '';
    const answer = prompt(`Choose ${label}:\n${prefix}${lines}`, allowNone ? '0' : '1');
    if (answer == null) return undefined;
    if (allowNone && Number(answer) === 0) return null;
    return candidates[Number(answer) - 1] || undefined;
  }

  function canvasPoint(svg, event) {
    const rect = svg.getBoundingClientRect();
    return {
      x: ((event.clientX - rect.left) / rect.width) * 1800,
      y: ((event.clientY - rect.top) / rect.height) * 1100,
    };
  }

  async function createRichAction(kind, point) {
    const diagram = activeDiagram();
    if (!diagram) return;
    let referenceId = null;
    let expression = null;
    let defaultName = kind.replace(/Action$/, '').replace(/([a-z])([A-Z])/g, '$1 $2');

    if (kind === 'CallBehaviorAction') {
      const currentId = String(diagram.activity_id);
      const candidates = Object.values(state.activitySnapshot?.repository?.activities || {})
        .filter((activity) => String(activity.id) !== currentId);
      const selected = choose(candidates, 'Activity');
      if (!selected) return;
      referenceId = selected.id;
      defaultName = selected.name;
    } else if (kind === 'CallOperationAction') {
      const selected = choose(projectElements('Operation'), 'Operation');
      if (!selected) return;
      referenceId = selected.id;
      defaultName = selected.name;
    } else if (kind === 'SendSignalAction') {
      const selected = choose(projectElements('Signal'), 'Signal');
      if (!selected) return;
      referenceId = selected.id;
      defaultName = `send ${selected.name}`;
    } else if (kind === 'AcceptEventAction') {
      const selected = choose(projectElements('Signal'), 'Signal', true);
      if (selected === undefined) return;
      referenceId = selected?.id || null;
      defaultName = selected ? `accept ${selected.name}` : 'Accept Event';
    } else if (kind === 'AcceptTimeEventAction') {
      expression = prompt('Time event expression', 'after 1s');
      if (expression == null) return;
      defaultName = 'Accept Time Event';
    }

    const name = prompt('Action name', defaultName) || defaultName;
    await runCommand(`Creating ${defaultName}…`, () => requireInvoke()('add_activity_action', {
      diagramId: diagram.id,
      kind,
      name,
      referenceId,
      expression,
      x: point.x,
      y: point.y,
    }));
  }

  async function createParameterNode(point) {
    const diagram = activeDiagram();
    const selected = choose(projectElements('Parameter'), 'Parameter');
    if (!diagram || !selected) return;
    await runCommand('Creating Activity Parameter Node…', () => requireInvoke()('add_activity_parameter_node', {
      diagramId: diagram.id,
      parameterId: selected.id,
      x: point.x,
      y: point.y,
    }));
  }

  async function createStructured(kind) {
    const diagram = activeDiagram();
    if (!diagram) return;
    if (kind === 'ActivityPartition') {
      const name = prompt('Partition name', 'Partition');
      if (!name) return;
      const represented = choose(
        (state.snapshot?.project?.elements || []).filter((element) => ['Block', 'PartProperty'].includes(element.kind)),
        'represented Block/Part',
        true,
      );
      if (represented === undefined) return;
      await runCommand('Creating Activity Partition…', () => requireInvoke()('add_activity_partition', {
        diagramId: diagram.id,
        name,
        representedElementId: represented?.id || null,
        isDimension: false,
        isExternal: false,
      }));
      return;
    }
    const name = prompt(`${kind} name`, kind.replace(/([a-z])([A-Z])/g, '$1 $2'));
    if (!name) return;
    await runCommand(`Creating ${kind}…`, () => requireInvoke()('add_structured_activity_node', {
      diagramId: diagram.id,
      kind,
      name,
      parentId: null,
    }));
  }

  document.addEventListener('click', async (event) => {
    const svg = event.target.closest?.('.activity-svg');
    const kind = state.activityTool?.semantic_kind;
    if (!svg || (!RICH_NODE_TOOLS.has(kind) && !STRUCTURED_TOOLS.has(kind))) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    const point = canvasPoint(svg, event);
    try {
      if (kind === 'ActivityParameterNode') await createParameterNode(point);
      else if (RICH_NODE_TOOLS.has(kind)) await createRichAction(kind, point);
      else await createStructured(kind);
      state.activityTool = null;
      state.activitySnapshot = await requireInvoke()('activity_snapshot');
      render();
    } catch (error) {
      console.error('Activity rich semantic creation failed', error);
      window.smpDialogs?.notify?.(error?.message || String(error), 'error');
    }
  }, true);

  function actionKind(node) {
    const action = node?.kind?.Action;
    const kind = action?.kind;
    if (!kind) return null;
    if (typeof kind === 'string') return kind;
    return Object.keys(kind)[0] || null;
  }

  function selectedNode() {
    return activeActivity()?.nodes?.find(
      (node) => String(node.id) === String(state.selectedActivityNodeId),
    ) || null;
  }

  function currentActionReference(action, kind) {
    if (!action?.kind || typeof action.kind === 'string') return null;
    if (kind === 'CallBehavior') return action.kind.CallBehavior?.activity_id || null;
    if (kind === 'CallOperation') return action.kind.CallOperation?.operation_id || null;
    if (kind === 'SendSignal') return action.kind.SendSignal?.signal_id || null;
    if (kind === 'AcceptEvent') return action.kind.AcceptEvent?.signal_id || null;
    return null;
  }

  function actionReferenceCandidates(kind) {
    if (kind === 'CallBehavior') {
      return Object.values(state.activitySnapshot?.repository?.activities || {});
    }
    if (kind === 'CallOperation') return projectElements('Operation');
    if (kind === 'SendSignal' || kind === 'AcceptEvent') return projectElements('Signal');
    return [];
  }

  function actionReferenceEditor(action, kind) {
    if (!['CallBehavior', 'CallOperation', 'SendSignal', 'AcceptEvent'].includes(kind)) return '';
    const current = String(currentActionReference(action, kind) || '');
    const label = kind === 'CallBehavior' ? 'Referenced Activity'
      : kind === 'CallOperation' ? 'Referenced Operation'
        : kind === 'SendSignal' ? 'Signal' : 'Accepted Signal/Event';
    const allowNone = kind === 'AcceptEvent';
    const options = actionReferenceCandidates(kind).map((item) => {
      const selected = String(item.id) === current ? 'selected' : '';
      return `<option value="${escapeAttr(item.id)}" ${selected}>${escapeHtml(item.name)}</option>`;
    }).join('');
    return `<label>${label}<select id="activity-action-reference">${allowNone ? '<option value="">Any receive event</option>' : ''}${options}</select></label>`;
  }

  const baseRenderProperties = renderProperties;
  renderProperties = function renderRichActivityProperties() {
    const diagram = activeDiagram();
    const node = selectedNode();
    if (!diagram || !node) return baseRenderProperties();
    const panel = $('properties');
    const action = node.kind?.Action;
    const kind = actionKind(node);
    const decision = node.kind?.Decision;
    const join = node.kind?.Join;
    const partitions = activeActivity()?.partitions || [];
    const structured = activeActivity()?.structured_nodes || [];
    const pinRows = (action?.pins || []).map((pin) =>
      `<div class="activity-pin-row"><span>${escapeHtml(pin.direction)}</span><strong>${escapeHtml(pin.name)}</strong><span>${escapeHtml(pin.type_id || 'untyped')}</span></div>`,
    ).join('');

    panel.innerHTML = `<div class="property-heading">${escapeHtml(kind || 'Activity Node')}</div>
      <label>Name<input id="activity-property-name" value="${escapeAttr(node.name || '')}"></label>
      <label>Semantic ID<input value="${escapeAttr(node.id)}" disabled></label>
      ${kind === 'Opaque' ? `<label>Execution expression/body<textarea id="activity-opaque-body">${escapeHtml(action.kind.Opaque.body || '')}</textarea></label>` : ''}
      ${actionReferenceEditor(action, kind)}
      ${kind === 'AcceptTimeEvent' ? `<label>Time expression<input id="activity-time-expression" value="${escapeAttr(action.kind.AcceptTimeEvent.expression || '')}"></label>` : ''}
      ${decision ? `<label>Decision input<input id="activity-decision-input" value="${escapeAttr(decision.decision_input || '')}"></label>` : ''}
      ${join ? `<label>Join specification<input id="activity-join-spec" value="${escapeAttr(join.join_specification || '')}"></label>` : ''}
      ${pinRows ? `<div class="property-heading small">Pins</div>${pinRows}` : ''}
      <label>Partition<select id="activity-partition"><option value="">None</option>${partitions.map((item) => `<option value="${escapeAttr(item.id)}" ${String(node.partition_id || '') === String(item.id) ? 'selected' : ''}>${escapeHtml(item.name)}</option>`).join('')}</select></label>
      <label>Structured parent<select id="activity-structured"><option value="">None</option>${structured.map((item) => `<option value="${escapeAttr(item.id)}" ${String(node.structured_node_id || '') === String(item.id) ? 'selected' : ''}>${escapeHtml(item.name)}</option>`).join('')}</select></label>
      <button id="activity-apply-semantics" class="primary">Apply Activity Semantics</button>`;

    $('activity-apply-semantics').onclick = async () => {
      const referenceEditor = $('activity-action-reference');
      await runCommand('Updating Activity semantics…', () => requireInvoke()('update_activity_node_semantics', {
        diagramId: diagram.id,
        activityNodeId: node.id,
        name: $('activity-property-name').value,
        opaqueBody: $('activity-opaque-body')?.value ?? null,
        decisionInput: $('activity-decision-input')?.value ?? null,
        joinSpecification: $('activity-join-spec')?.value ?? null,
        timeExpression: $('activity-time-expression')?.value ?? null,
        actionReferenceId: referenceEditor?.value || null,
        updateActionReference: Boolean(referenceEditor),
      }));
      await requireInvoke()('assign_activity_node_partition', {
        diagramId: diagram.id,
        activityNodeId: node.id,
        partitionId: $('activity-partition').value || null,
      });
      await requireInvoke()('assign_activity_node_structured_parent', {
        diagramId: diagram.id,
        activityNodeId: node.id,
        structuredNodeId: $('activity-structured').value || null,
      });
      state.activitySnapshot = await requireInvoke()('activity_snapshot');
      render();
    };
  };

  function svgElement(tag, attrs = {}) {
    const element = document.createElementNS('http://www.w3.org/2000/svg', tag);
    for (const [name, value] of Object.entries(attrs)) element.setAttribute(name, String(value));
    return element;
  }

  function presentationForNode(diagram, nodeId) {
    return diagram.nodes?.find((item) => String(item.activity_node_id) === String(nodeId)) || null;
  }

  function boundsForMembers(diagram, memberIds, fallbackIndex) {
    const boxes = memberIds.map((id) => presentationForNode(diagram, id)).filter(Boolean);
    if (!boxes.length) {
      return { x: 70 + fallbackIndex * 34, y: 70 + fallbackIndex * 28, width: 360, height: 220 };
    }
    const left = Math.min(...boxes.map((box) => box.x)) - 36;
    const top = Math.min(...boxes.map((box) => box.y)) - 54;
    const right = Math.max(...boxes.map((box) => box.x + box.width)) + 36;
    const bottom = Math.max(...boxes.map((box) => box.y + box.height)) + 36;
    return { x: left, y: top, width: right - left, height: bottom - top };
  }

  function appendFrame(layer, bounds, label, className) {
    const group = svgElement('g', { class: className });
    group.appendChild(svgElement('rect', {
      x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height, rx: 4,
    }));
    const text = svgElement('text', { x: bounds.x + 10, y: bounds.y + 18 });
    text.textContent = label;
    group.appendChild(text);
    layer.appendChild(group);
  }

  function renderSemanticRegions(svg, diagram, activity) {
    const layer = svgElement('g', { class: 'activity-semantic-regions' });
    (activity.partitions || []).forEach((partition, index) => {
      const members = activity.nodes.filter((node) => String(node.partition_id || '') === String(partition.id)).map((node) => node.id);
      const bounds = boundsForMembers(diagram, members, index);
      appendFrame(layer, bounds, partition.name || 'Partition', 'activity-partition-frame');
    });
    (activity.structured_nodes || []).forEach((structured, index) => {
      const members = activity.nodes.filter((node) => String(node.structured_node_id || '') === String(structured.id)).map((node) => node.id);
      const bounds = boundsForMembers(diagram, members, index + (activity.partitions || []).length);
      const kind = typeof structured.kind === 'string' ? structured.kind : Object.keys(structured.kind || {})[0] || 'Structured';
      appendFrame(layer, bounds, `${kind} · ${structured.name || ''}`, `activity-structured-frame activity-structured-${kind.toLowerCase()}`);
    });
    const defs = svg.querySelector('defs');
    if (defs?.nextSibling) svg.insertBefore(layer, defs.nextSibling);
    else svg.appendChild(layer);
  }

  function pinAnchor(presentation, pin, index, count) {
    const direction = pin.direction;
    if (direction === 'Input') {
      return { x: presentation.x, y: presentation.y + ((index + 1) * presentation.height) / (count + 1), side: 'left' };
    }
    if (direction === 'Output') {
      return { x: presentation.x + presentation.width, y: presentation.y + ((index + 1) * presentation.height) / (count + 1), side: 'right' };
    }
    return { x: presentation.x + ((index + 1) * presentation.width) / (count + 1), y: presentation.y + presentation.height, side: 'bottom' };
  }

  function renderPins(svg, diagram, activity) {
    for (const node of activity.nodes || []) {
      const action = node.kind?.Action;
      if (!action?.pins?.length) continue;
      const presentation = presentationForNode(diagram, node.id);
      if (!presentation) continue;
      for (const direction of ['Input', 'Output', 'Value']) {
        const pins = action.pins.filter((pin) => pin.direction === direction);
        pins.forEach((pin, index) => {
          const anchor = pinAnchor(presentation, pin, index, pins.length);
          const group = svgElement('g', {
            class: `activity-pin-anchor activity-pin-${direction.toLowerCase()}`,
            'data-pin-token': `pin:${pin.id}`,
            'data-owner-node-id': node.id,
          });
          group.appendChild(svgElement('rect', { x: anchor.x - 5, y: anchor.y - 5, width: 10, height: 10 }));
          const labelX = anchor.side === 'left' ? anchor.x + 9 : anchor.side === 'right' ? anchor.x - 9 : anchor.x + 8;
          const label = svgElement('text', {
            x: labelX,
            y: anchor.side === 'bottom' ? anchor.y + 16 : anchor.y - 8,
            'text-anchor': anchor.side === 'right' ? 'end' : 'start',
          });
          label.textContent = pin.name || direction;
          group.appendChild(label);
          svg.appendChild(group);
        });
      }
    }
  }

  Object.assign(state, {
    activityExecutionSnapshot: null,
    activityExecutionRunning: false,
  });
  let executionRunGeneration = 0;

  function executionSnapshot() {
    const snapshot = state.activityExecutionSnapshot;
    return snapshot && String(snapshot.root_activity_id) === String(activeDiagram()?.activity_id)
      ? snapshot
      : null;
  }

  function executionState() {
    return executionSnapshot()?.execution?.state || 'Not initialized';
  }

  function clearExecutionVisualization(svg) {
    if (!svg) return;
    for (const node of svg.querySelectorAll('.activity-node[data-activity-node-id]')) {
      node.classList.remove('runtime-active', 'runtime-enabled', 'runtime-waiting', 'runtime-completed', 'runtime-failed');
    }
    for (const edge of svg.querySelectorAll('.activity-flow[data-activity-edge-id]')) {
      edge.classList.remove('runtime-active', 'runtime-completed');
    }
    svg.querySelectorAll('.activity-runtime-token-badge').forEach((badge) => badge.remove());
  }

  async function invokeExecution(command) {
    const diagram = activeDiagram();
    if (!diagram) throw new Error('Open an Activity Diagram first.');
    const snapshot = await requireInvoke()(command, { diagramId: diagram.id });
    Object.assign(state, { activityExecutionSnapshot: snapshot });
    refreshExecutionUi();
    return snapshot;
  }

  async function initializeExecution() {
    executionRunGeneration += 1;
    Object.assign(state, { activityExecutionRunning: false });
    await invokeExecution('initialize_activity_execution');
  }

  async function stepExecution() {
    executionRunGeneration += 1;
    Object.assign(state, { activityExecutionRunning: false });
    if (!executionSnapshot()) await initializeExecution();
    await invokeExecution('step_activity_execution');
  }

  async function runExecution(resume) {
    if (!executionSnapshot()) await initializeExecution();
    const command = resume ? 'resume_activity_execution' : 'run_activity_execution';
    await invokeExecution(command);
    const generation = ++executionRunGeneration;
    Object.assign(state, { activityExecutionRunning: true });
    while (generation === executionRunGeneration && executionState() === 'Running') {
      await invokeExecution('step_activity_execution');
      if (executionState() !== 'Running') break;
      await new Promise((resolve) => requestAnimationFrame(resolve));
    }
    if (generation === executionRunGeneration) {
      Object.assign(state, { activityExecutionRunning: false });
      refreshExecutionUi();
    }
  }

  async function pauseExecution() {
    executionRunGeneration += 1;
    Object.assign(state, { activityExecutionRunning: false });
    if (executionState() === 'Running') await invokeExecution('pause_activity_execution');
  }

  async function resetExecution() {
    executionRunGeneration += 1;
    Object.assign(state, { activityExecutionRunning: false });
    if (!executionSnapshot()) return initializeExecution();
    return invokeExecution('reset_activity_execution');
  }

  async function terminateExecution() {
    executionRunGeneration += 1;
    Object.assign(state, { activityExecutionRunning: false });
    if (executionSnapshot()) await invokeExecution('terminate_activity_execution');
  }

  function ensureExecutionRibbon(visible) {
    const ribbon = document.querySelector('.ribbon');
    if (!ribbon) return;
    let group = ribbon.querySelector('.activity-execution-ribbon-group');
    if (!group) {
      group = document.createElement('section');
      group.className = 'ribbon-group activity-execution-ribbon-group';
      group.innerHTML = `<div class="ribbon-actions activity-execution-actions">
        <button class="ribbon-command" data-activity-execution="runtime"><span class="command-icon">◎</span><span>Runtime</span></button>
        <button class="ribbon-command" data-activity-execution="initialize"><span class="command-icon">◇</span><span>Initialize</span></button>
        <button class="ribbon-command" data-activity-execution="run"><span class="command-icon">▶</span><span>Run</span></button>
        <button class="ribbon-command" data-activity-execution="step"><span class="command-icon">▸</span><span>Step</span></button>
        <button class="ribbon-command" data-activity-execution="pause"><span class="command-icon">Ⅱ</span><span>Pause</span></button>
        <button class="ribbon-command" data-activity-execution="resume"><span class="command-icon">▷</span><span>Resume</span></button>
        <button class="ribbon-command" data-activity-execution="reset"><span class="command-icon">↺</span><span>Reset</span></button>
        <button class="ribbon-command" data-activity-execution="terminate"><span class="command-icon">■</span><span>Terminate</span></button>
      </div><div class="ribbon-label">Activity Execution</div>`;
      group.onclick = async (event) => {
        const command = event.target.closest?.('[data-activity-execution]')?.dataset.activityExecution;
        if (!command) return;
        try {
          if (command === 'runtime') {
            await window.smpOpenStructuralRuntimeConfiguration?.('activity', activeDiagram().id);
            Object.assign(state, { activityExecutionSnapshot: null });
            refreshExecutionUi();
          } else if (command === 'initialize') await initializeExecution();
          else if (command === 'run') await runExecution(false);
          else if (command === 'step') await stepExecution();
          else if (command === 'pause') await pauseExecution();
          else if (command === 'resume') await runExecution(true);
          else if (command === 'reset') await resetExecution();
          else if (command === 'terminate') await terminateExecution();
        } catch (error) {
          Object.assign(state, { activityExecutionRunning: false });
          window.smpDialogs?.notify?.(error?.message || String(error), 'error');
          refreshExecutionUi();
        }
      };
      ribbon.insertBefore(group, ribbon.querySelector('.ribbon-context'));
    }
    group.hidden = !visible;
    const current = executionState();
    const initialized = current !== 'Not initialized';
    const running = current === 'Running';
    const paused = current === 'Paused';
    for (const button of group.querySelectorAll('[data-activity-execution]')) {
      const command = button.dataset.activityExecution;
      button.disabled = !visible
        || (command === 'run' && (!initialized || !['Initialized', 'Paused'].includes(current)))
        || (command === 'step' && (!initialized || running || ['Completed', 'Failed', 'Terminated'].includes(current)))
        || (command === 'pause' && !running)
        || (command === 'resume' && !paused)
        || (['reset', 'terminate'].includes(command) && !initialized);
    }
  }

  function runtimeEndpointId(endpoint) {
    return endpoint?.Node || endpoint?.Pin || null;
  }

  function applyExecutionVisualization(svg, activity) {
    clearExecutionVisualization(svg);
    const snapshot = executionSnapshot();
    if (!snapshot) return;
    const rootNodes = snapshot.nodes.filter(
      (node) => String(node.activity_id) === String(snapshot.root_activity_id),
    );
    const nodeStates = new Map(rootNodes.map((node) => [String(node.node_id), node.state]));
    const activeNodes = new Set((snapshot.active_node_ids || []).map(String));
    const activeEdges = new Set((snapshot.active_edge_ids || []).map(String));
    const completedEdges = new Set((snapshot.completed_edge_ids || []).map(String));
    for (const node of svg.querySelectorAll('.activity-node[data-activity-node-id]')) {
      const id = String(node.dataset.activityNodeId);
      node.classList.toggle('runtime-active', activeNodes.has(id));
      for (const runtimeState of ['Enabled', 'Waiting', 'Completed', 'Failed']) {
        node.classList.toggle(`runtime-${runtimeState.toLowerCase()}`, nodeStates.get(id) === runtimeState);
      }
    }
    for (const edge of svg.querySelectorAll('.activity-flow[data-activity-edge-id]')) {
      const id = String(edge.dataset.activityEdgeId);
      edge.classList.toggle('runtime-active', activeEdges.has(id));
      edge.classList.toggle('runtime-completed', completedEdges.has(id));
    }

    const nodeTokenCounts = new Map();
    for (const store of snapshot.token_stores || []) {
      if (String(store.activity_id) !== String(snapshot.root_activity_id)) continue;
      const endpointId = runtimeEndpointId(store.endpoint);
      if (!endpointId) continue;
      const ownerNode = activity.nodes.find((node) => {
        if (String(node.id) === String(endpointId)) return true;
        return (node.kind?.Action?.pins || []).some((pin) => String(pin.id) === String(endpointId));
      });
      if (!ownerNode) continue;
      const key = String(ownerNode.id);
      nodeTokenCounts.set(key, (nodeTokenCounts.get(key) || 0) + (store.tokens?.length || 0));
    }
    for (const [nodeId, count] of nodeTokenCounts) {
      const presentation = presentationForNode(activeDiagram(), nodeId);
      if (!presentation) continue;
      const badge = svgElement('g', { class: 'activity-runtime-token-badge' });
      badge.appendChild(svgElement('circle', {
        cx: presentation.x + presentation.width - 5,
        cy: presentation.y + 5,
        r: 10,
      }));
      const label = svgElement('text', {
        x: presentation.x + presentation.width - 5,
        y: presentation.y + 9,
        'text-anchor': 'middle',
      });
      label.textContent = String(count);
      badge.appendChild(label);
      svg.appendChild(badge);
    }
  }

  function renderExecutionPanel() {
    const host = document.querySelector('.diagram-workspace');
    const snapshot = executionSnapshot();
    let panel = document.querySelector('.activity-execution-panel');
    if (!host || !snapshot) {
      panel?.remove();
      return;
    }
    if (!panel) {
      panel = document.createElement('aside');
      panel.className = 'activity-execution-panel';
      panel.dataset.workspaceOverlay = 'true';
      host.appendChild(panel);
    } else if (panel.parentElement !== host) {
      host.appendChild(panel);
    }
    const execution = snapshot.execution;
    const diagnostics = (execution.diagnostics || []).slice(-4);
    const trace = (execution.trace || []).slice(-6);
    panel.innerHTML = `<div class="activity-execution-heading"><strong>${escapeHtml(execution.state)}</strong><span>${execution.simulation_time} ns</span></div>
      <div class="activity-execution-metrics"><span>Step ${execution.steps_executed}</span><span>${snapshot.call_frames?.length || 0} frame(s)</span><span>${(snapshot.token_stores || []).reduce((sum, store) => sum + (store.tokens?.length || 0), 0)} token(s)</span></div>
      ${diagnostics.length ? `<div class="activity-execution-diagnostics">${diagnostics.map((item) => `<div class="runtime-${String(item.severity).toLowerCase()}">${escapeHtml(item.message)}</div>`).join('')}</div>` : ''}
      <div class="activity-execution-trace">${trace.map((item) => `<div><span>${item.simulation_time}</span>${escapeHtml(item.message)}</div>`).join('')}</div>
      ${window.renderStructuralRuntimeInspector?.(snapshot) || ''}`;
  }

  window.smpRefreshActivityExecution = () => refreshExecutionUi();

  function refreshExecutionUi() {
    const diagram = activeDiagram();
    const activity = activeActivity();
    const svg = document.querySelector('.activity-svg');
    ensureExecutionRibbon(Boolean(diagram));
    if (svg) clearExecutionVisualization(svg);
    if (diagram && activity && svg) applyExecutionVisualization(svg, activity);
    renderExecutionPanel();
  }

  const baseRenderCanvas = renderCanvas;
  renderCanvas = function renderRichActivityCanvas() {
    baseRenderCanvas();
    const diagram = activeDiagram();
    const activity = activeActivity();
    const svg = document.querySelector('.activity-svg');
    ensureExecutionRibbon(Boolean(diagram));
    if (!diagram || !activity || !svg) {
      renderExecutionPanel();
      return;
    }
    renderSemanticRegions(svg, diagram, activity);
    renderPins(svg, diagram, activity);
    applyExecutionVisualization(svg, activity);
    renderExecutionPanel();
  };

  document.addEventListener('click', async (event) => {
    const pin = event.target.closest?.('.activity-pin-anchor');
    if (!pin || !activeDiagram()) return;
    if (state.activityPendingFlow?.kind !== 'ObjectFlow') {
      state.selectedActivityNodeId = pin.dataset.ownerNodeId || null;
      return;
    }
    event.preventDefault();
    event.stopImmediatePropagation();
    const token = pin.dataset.pinToken;
    if (!state.activityPendingFlow.source) {
      state.activityPendingFlow.source = token;
      state.selectedActivityNodeId = pin.dataset.ownerNodeId || null;
      render();
      return;
    }
    const source = state.activityPendingFlow.source;
    const diagram = activeDiagram();
    state.activityPendingFlow = null;
    try {
      await runCommand('Creating ObjectFlow…', () => requireInvoke()('add_activity_edge', {
        diagramId: diagram.id,
        kind: 'ObjectFlow',
        sourceActivityNodeId: source,
        targetActivityNodeId: token,
        guard: null,
        weight: null,
      }));
      state.selectedActivityNodeId = null;
      state.activitySnapshot = await requireInvoke()('activity_snapshot');
      render();
    } catch (error) {
      console.error('Activity pin ObjectFlow creation failed', error);
      window.smpDialogs?.notify?.(error?.message || String(error), 'error');
    }
  }, true);
})();
