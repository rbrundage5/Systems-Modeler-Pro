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
      ${kind === 'Opaque' ? `<label>Body<textarea id="activity-opaque-body">${escapeHtml(action.kind.Opaque.body || '')}</textarea></label>` : ''}
      ${kind === 'AcceptTimeEvent' ? `<label>Time expression<input id="activity-time-expression" value="${escapeAttr(action.kind.AcceptTimeEvent.expression || '')}"></label>` : ''}
      ${decision ? `<label>Decision input<input id="activity-decision-input" value="${escapeAttr(decision.decision_input || '')}"></label>` : ''}
      ${join ? `<label>Join specification<input id="activity-join-spec" value="${escapeAttr(join.join_specification || '')}"></label>` : ''}
      ${pinRows ? `<div class="property-heading small">Pins</div>${pinRows}` : ''}
      <label>Partition<select id="activity-partition"><option value="">None</option>${partitions.map((item) => `<option value="${escapeAttr(item.id)}" ${String(node.partition_id || '') === String(item.id) ? 'selected' : ''}>${escapeHtml(item.name)}</option>`).join('')}</select></label>
      <label>Structured parent<select id="activity-structured"><option value="">None</option>${structured.map((item) => `<option value="${escapeAttr(item.id)}" ${String(node.structured_node_id || '') === String(item.id) ? 'selected' : ''}>${escapeHtml(item.name)}</option>`).join('')}</select></label>
      <button id="activity-apply-semantics" class="primary">Apply Activity Semantics</button>`;

    $('activity-apply-semantics').onclick = async () => {
      await runCommand('Updating Activity semantics…', () => requireInvoke()('update_activity_node_semantics', {
        diagramId: diagram.id,
        activityNodeId: node.id,
        name: $('activity-property-name').value,
        opaqueBody: $('activity-opaque-body')?.value ?? null,
        decisionInput: $('activity-decision-input')?.value ?? null,
        joinSpecification: $('activity-join-spec')?.value ?? null,
        timeExpression: $('activity-time-expression')?.value ?? null,
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

  const baseRenderCanvas = renderCanvas;
  renderCanvas = function renderRichActivityCanvas() {
    baseRenderCanvas();
    const diagram = activeDiagram();
    const activity = activeActivity();
    const svg = document.querySelector('.activity-svg');
    if (!diagram || !activity || !svg) return;
    renderSemanticRegions(svg, diagram, activity);
    renderPins(svg, diagram, activity);
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
    }
  }, true);
})();