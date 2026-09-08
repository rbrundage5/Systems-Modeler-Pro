(() => {
  state.itemFlowNotation = state.itemFlowNotation || [];
  state.lastItemFlowMessage = null;

  const LABEL_VISIBILITY_KEY = 'smp.ibd-label-visibility.v1';

  function labelVisibilityState() {
    try { return JSON.parse(localStorage.getItem(LABEL_VISIBILITY_KEY) || '{}'); }
    catch { return {}; }
  }

  function labelVisibilityKey(diagramId, relationshipId, kind) {
    const projectId = state.snapshot?.project?.id || 'project';
    return `${projectId}:${diagramId}:${relationshipId}:${kind}`;
  }

  function labelVisible(diagramId, relationshipId, kind) {
    const preferences = labelVisibilityState();
    return preferences[labelVisibilityKey(diagramId, relationshipId, kind)] !== false;
  }

  function setLabelVisible(diagramId, relationshipId, kind, visible) {
    const preferences = labelVisibilityState();
    preferences[labelVisibilityKey(diagramId, relationshipId, kind)] = !!visible;
    localStorage.setItem(LABEL_VISIBILITY_KEY, JSON.stringify(preferences));
  }

  function mergeItemFlows(serverFlows) {
    const merged = new Map();
    for (const flow of state.itemFlowNotation || []) merged.set(flow.relationship_id, flow);
    for (const flow of serverFlows || []) merged.set(flow.relationship_id, flow);
    state.itemFlowNotation = [...merged.values()];
  }

  function conveyedNamesFromIds(ids) {
    const project = state.snapshot?.project;
    return (ids || []).map((id) => project?.elements?.find((element) => element.id === id)?.name || id);
  }

  function removeHiddenConnectorLabels(svg, diagram, project) {
    for (const edge of diagram.connectors || []) {
      if (labelVisible(diagram.id, edge.relationship_id, 'connector-name')) continue;
      const relationship = project.relationships?.find((candidate) => candidate.id === edge.relationship_id);
      if (!relationship?.name || !edge.points?.length) continue;
      const midpoint = edge.label_anchor || edge.points[Math.floor(edge.points.length / 2)];
      const expectedX = midpoint.x + 5;
      const expectedY = midpoint.y - 6;
      const label = [...svg.querySelectorAll('text.relationship-label')].find((candidate) => {
        if (candidate.textContent !== relationship.name) return false;
        const x = Number(candidate.getAttribute('x'));
        const y = Number(candidate.getAttribute('y'));
        return Number.isFinite(x) && Number.isFinite(y)
          && Math.abs(x - expectedX) < 0.75
          && Math.abs(y - expectedY) < 0.75;
      });
      label?.remove();
    }
  }

  // Intercept only Item Flow creation so successful Rust creation is visible
  // immediately. The authoritative Rust query below subsequently reconciles it.
  const baseRequireInvokeItemFlow = requireInvoke;
  requireInvoke = function requireInvokeWithItemFlowFeedback() {
    const invokeFn = baseRequireInvokeItemFlow();
    return async (command, args = {}) => {
      const result = await invokeFn(command, args);
      if (command === 'add_item_flow_to_connector') {
        const names = conveyedNamesFromIds(args.conveyedItemIds);
        mergeItemFlows([{
          relationship_id: result,
          connector_id: args.relationshipId,
          conveyed_item_ids: [...(args.conveyedItemIds || [])],
          conveyed_item_names: names,
        }]);
        state.lastItemFlowMessage = `Item Flow created: ${names.join(', ') || 'conveyed item'} on selected Connector`;
      }
      return result;
    };
  };

  const baseRefreshItemFlow = refresh;
  refresh = async function refreshWithItemFlowNotation() {
    await baseRefreshItemFlow();
    try {
      const serverFlows = await baseRequireInvokeItemFlow()('ibd_item_flow_notation');
      mergeItemFlows(serverFlows);
    } catch (error) {
      console.error('Unable to load Item Flow notation data', error);
    }
    if (selectedIbd()) render();
  };

  const baseRenderStatusItemFlow = renderStatus;
  renderStatus = function renderStatusWithItemFlowFeedback(message) {
    if (!message && state.lastItemFlowMessage && selectedIbd()) {
      const status = $('status');
      if (status) status.textContent = state.lastItemFlowMessage;
      const counts = $('model-counts');
      if (counts && state.snapshot?.project) {
        const ibd = selectedIbd();
        counts.textContent = `Elements: ${state.snapshot.project.elements.length}   Relationships: ${state.snapshot.project.relationships.length}   Diagram: ${ibd.name} (IBD)`;
      }
      return;
    }
    baseRenderStatusItemFlow(message);
  };

  const baseRenderIbdConnectorLayer = renderIbdConnectorLayer;
  renderIbdConnectorLayer = function renderIbdConnectorLayerWithItemFlows(frame, diagram, project) {
    baseRenderIbdConnectorLayer(frame, diagram, project);
    const svg = frame.querySelector('svg.relationship-layer');
    if (!svg) return;

    removeHiddenConnectorLabels(svg, diagram, project);

    const flowsByConnector = new Map();
    const seenFlowKeys = new Set();
    for (const flow of state.itemFlowNotation || []) {
      const conveyedKey = [...(flow.conveyed_item_ids || [])].map(String).sort().join(',');
      const key = `${flow.connector_id}|${conveyedKey}`;
      if (seenFlowKeys.has(key)) continue;
      seenFlowKeys.add(key);
      if (!flowsByConnector.has(flow.connector_id)) flowsByConnector.set(flow.connector_id, []);
      flowsByConnector.get(flow.connector_id).push(flow);
    }

    for (const edge of diagram.connectors) {
      const flows = flowsByConnector.get(edge.relationship_id) || [];
      if (!flows.length || !edge.points || edge.points.length < 2) continue;

      const segmentIndex = Math.max(1, Math.floor(edge.points.length / 2));
      const from = edge.points[segmentIndex - 1];
      const to = edge.points[segmentIndex];
      const dx = to.x - from.x;
      const dy = to.y - from.y;
      const length = Math.hypot(dx, dy) || 1;
      const ux = dx / length;
      const uy = dy / length;
      const px = -uy;
      const py = ux;
      const centerX = (from.x + to.x) / 2;
      const centerY = (from.y + to.y) / 2;

      flows.forEach((flow, index) => {
        const lane = (index - (flows.length - 1) / 2) * 18;
        const tipX = centerX + px * lane;
        const tipY = centerY + py * lane;
        const arrowLength = 14;
        const arrowHalfWidth = 7;
        const baseX = tipX - ux * arrowLength;
        const baseY = tipY - uy * arrowLength;

        const arrow = document.createElementNS(SVG_NS, 'polygon');
        arrow.classList.add('ibd-item-flow-arrow');
        arrow.setAttribute(
          'points',
          `${tipX},${tipY} ${baseX + px * arrowHalfWidth},${baseY + py * arrowHalfWidth} ${baseX - px * arrowHalfWidth},${baseY - py * arrowHalfWidth}`,
        );
        arrow.setAttribute('aria-label', 'Item Flow direction');
        const title = document.createElementNS(SVG_NS, 'title');
        title.textContent = `Item Flow: ${(flow.conveyed_item_names || []).join(', ') || 'conveyed item'}`;
        arrow.appendChild(title);
        svg.appendChild(arrow);

        if (labelVisible(diagram.id, edge.relationship_id, 'item-flow-labels')) {
          const label = document.createElementNS(SVG_NS, 'text');
          label.classList.add('item-flow-label');
          label.setAttribute('x', tipX + px * 14 + 5);
          label.setAttribute('y', tipY + py * 14 - 5);
          label.textContent = (flow.conveyed_item_names || []).join(', ') || 'item';
          svg.appendChild(label);
        }
      });
    }
  };

  const baseRenderPropertiesItemFlow = renderProperties;
  renderProperties = function renderPropertiesWithItemFlows() {
    baseRenderPropertiesItemFlow();
    const diagram = selectedIbd();
    if (!diagram || !state.selectedRelationshipId) return;
    const selected = state.snapshot?.project?.relationships?.find(
      (relationship) => relationship.id === state.selectedRelationshipId && relationship.kind === 'Connector',
    );
    if (!selected) return;
    const flows = (state.itemFlowNotation || []).filter((flow) => flow.connector_id === selected.id);
    const panel = $('properties');
    if (!panel) return;

    const presentation = document.createElement('div');
    presentation.className = 'item-flow-properties';
    presentation.innerHTML = `<div class="property-heading">Presentation Labels</div>
      <div class="muted">These controls hide diagram text without deleting Connector or Item Flow semantics.</div>
      <label class="check-row"><input id="show-connector-name" type="checkbox"${labelVisible(diagram.id, selected.id, 'connector-name') ? ' checked' : ''}> Show Connector name</label>
      <label class="check-row"><input id="show-item-flow-labels" type="checkbox"${labelVisible(diagram.id, selected.id, 'item-flow-labels') ? ' checked' : ''}> Show Item Flow conveyed-item labels</label>`;
    panel.appendChild(presentation);

    $('show-connector-name').onchange = (event) => {
      setLabelVisible(diagram.id, selected.id, 'connector-name', event.target.checked);
      render();
    };
    $('show-item-flow-labels').onchange = (event) => {
      setLabelVisible(diagram.id, selected.id, 'item-flow-labels', event.target.checked);
      render();
    };

    const section = document.createElement('div');
    section.className = 'item-flow-properties';
    section.innerHTML = `<div class="property-heading">Item Flows</div>${
      flows.length
        ? flows.map((flow) => `<div class="item-flow-property-row">${escapeHtml((flow.conveyed_item_names || []).join(', ') || 'conveyed item')}</div>`).join('')
        : '<div class="muted">No Item Flow on this Connector.</div>'
    }`;
    panel.appendChild(section);
  };

  baseRequireInvokeItemFlow()('ibd_item_flow_notation')
    .then((flows) => {
      mergeItemFlows(flows);
      if (selectedIbd()) render();
    })
    .catch((error) => console.error('Unable to initialize Item Flow notation', error));
})();