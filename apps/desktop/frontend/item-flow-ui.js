(() => {
  const baseRefreshItemFlow = refresh;
  refresh = async function refreshWithItemFlowNotation() {
    await baseRefreshItemFlow();
    try {
      state.itemFlowNotation = await requireInvoke()('ibd_item_flow_notation');
    } catch (error) {
      console.error('Unable to load Item Flow notation data', error);
      state.itemFlowNotation = [];
    }
    if (selectedIbd()) render();
  };

  const baseRenderIbdConnectorLayer = renderIbdConnectorLayer;
  renderIbdConnectorLayer = function renderIbdConnectorLayerWithItemFlows(frame, diagram, project) {
    baseRenderIbdConnectorLayer(frame, diagram, project);
    const svg = frame.querySelector('svg.relationship-layer');
    if (!svg) return;

    const flowsByConnector = new Map();
    for (const flow of state.itemFlowNotation || []) {
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
        const arrowLength = 12;
        const arrowHalfWidth = 6;
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

        const label = document.createElementNS(SVG_NS, 'text');
        label.classList.add('item-flow-label');
        label.setAttribute('x', tipX + px * 12 + 4);
        label.setAttribute('y', tipY + py * 12 - 4);
        label.textContent = (flow.conveyed_item_names || []).join(', ') || 'item';
        svg.appendChild(label);
      });
    }
  };

  // Load notation immediately for an already-open project/IBD.
  requireInvoke()('ibd_item_flow_notation')
    .then((flows) => {
      state.itemFlowNotation = flows;
      if (selectedIbd()) render();
    })
    .catch((error) => console.error('Unable to initialize Item Flow notation', error));
})();
