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
})();
