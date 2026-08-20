(() => {
  state.selectedActivityEdgeId = state.selectedActivityEdgeId || null;

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

  function endpointToken(endpoint) {
    if (!endpoint) return null;
    if (typeof endpoint === 'string') return endpoint;
    if (endpoint.Node) return String(endpoint.Node);
    if (endpoint.Pin) return `pin:${endpoint.Pin}`;
    return null;
  }

  function endpointCandidates() {
    const activity = activeActivity();
    const candidates = [];
    for (const node of activity?.nodes || []) {
      candidates.push({ token: String(node.id), label: node.name || String(node.id) });
      const pins = node.kind?.Action?.pins || [];
      for (const pin of pins) {
        candidates.push({
          token: `pin:${pin.id}`,
          label: `${node.name || 'Action'} :: ${pin.direction} ${pin.name}`,
        });
      }
    }
    return candidates;
  }

  function chooseEndpoint(title, current) {
    const candidates = endpointCandidates();
    const lines = candidates.map((candidate, index) =>
      `${index + 1}. ${candidate.label}${candidate.token === current ? '  [current]' : ''}`,
    ).join('\n');
    const answer = prompt(`${title}:\n${lines}`, '1');
    if (answer == null) return undefined;
    const candidate = candidates[Number(answer) - 1];
    return candidate?.token;
  }

  async function refreshActivity() {
    state.activitySnapshot = await requireInvoke()('activity_snapshot');
    render();
  }

  window.smpRouteActivityDiagram = async () => {
    const diagram = activeDiagram();
    if (!diagram) return false;
    await runCommand('Routing Activity…', () => requireInvoke()('route_activity_diagram', { diagramId: diagram.id }));
    await refreshActivity();
    return true;
  };

  async function deleteSelection() {
    const diagram = activeDiagram();
    if (!diagram) return false;
    if (state.selectedActivityEdgeId) {
      await runCommand('Deleting Activity flow…', () => requireInvoke()('delete_activity_item', {
        diagramId: diagram.id,
        itemKind: 'edge',
        itemId: state.selectedActivityEdgeId,
      }));
      state.selectedActivityEdgeId = null;
      await refreshActivity();
      return true;
    }
    if (state.selectedActivityNodeId) {
      await runCommand('Deleting Activity node and incident flows…', () => requireInvoke()('delete_activity_item', {
        diagramId: diagram.id,
        itemKind: 'node',
        itemId: state.selectedActivityNodeId,
      }));
      state.selectedActivityNodeId = null;
      await refreshActivity();
      return true;
    }
    return false;
  }

  document.addEventListener('keydown', (event) => {
    if (!activeDiagram() || !['Delete', 'Backspace'].includes(event.key)) return;
    const tag = document.activeElement?.tagName?.toLowerCase();
    if (['input', 'textarea', 'select'].includes(tag)) return;
    if (!state.selectedActivityEdgeId && !state.selectedActivityNodeId) return;
    event.preventDefault();
    void deleteSelection().catch((error) => console.error('Activity delete failed', error));
  }, true);

  const baseRenderCanvas = renderCanvas;
  renderCanvas = function renderActivityMutationCanvas() {
    baseRenderCanvas();
    const diagram = activeDiagram();
    if (!diagram) return;
    const flows = [...document.querySelectorAll('.activity-svg .activity-flow')];
    flows.forEach((flow, index) => {
      const presentation = diagram.edges?.[index];
      if (!presentation) return;
      flow.dataset.activityEdgeId = presentation.activity_edge_id;
      if (String(presentation.activity_edge_id) === String(state.selectedActivityEdgeId)) {
        flow.classList.add('selected');
      }
      flow.onclick = (event) => {
        event.stopPropagation();
        state.selectedActivityNodeId = null;
        state.selectedActivityEdgeId = presentation.activity_edge_id;
        render();
      };
    });
  };

  const baseRenderProperties = renderProperties;
  renderProperties = function renderActivityMutationProperties() {
    const diagram = activeDiagram();
    const activity = activeActivity();
    const edge = activity?.edges?.find(
      (candidate) => String(candidate.id) === String(state.selectedActivityEdgeId),
    );
    if (!diagram || !edge) {
      baseRenderProperties();
      if (diagram && !state.selectedActivityNodeId && !state.selectedActivityEdgeId) {
        const panel = $('properties');
        const routeButton = document.createElement('button');
        routeButton.className = 'primary';
        routeButton.textContent = 'Route Activity';
        routeButton.onclick = async () => {
          await runCommand('Routing Activity…', () => requireInvoke()('route_activity_diagram', {
            diagramId: diagram.id,
          }));
          await refreshActivity();
        };
        panel.appendChild(routeButton);
      }
      return;
    }

    const panel = $('properties');
    const source = endpointToken(edge.source);
    const target = endpointToken(edge.target);
    panel.innerHTML = `<div class="property-heading">${escapeHtml(edge.kind || 'Activity Flow')}</div>
      <label>Semantic ID<input value="${escapeAttr(edge.id)}" disabled></label>
      <label>Source<input value="${escapeAttr(source || '')}" disabled></label>
      <label>Target<input value="${escapeAttr(target || '')}" disabled></label>
      <label>Guard<input value="${escapeAttr(edge.guard || '')}" disabled></label>
      <div class="activity-mutation-actions">
        <button id="activity-reconnect-edge" class="primary">Reconnect Flow</button>
        <button id="activity-delete-edge">Delete Flow</button>
        <button id="activity-route-diagram">Route Activity</button>
      </div>`;

    $('activity-reconnect-edge').onclick = async () => {
      const nextSource = chooseEndpoint('Choose source endpoint', source);
      if (!nextSource) return;
      const nextTarget = chooseEndpoint('Choose target endpoint', target);
      if (!nextTarget) return;
      await runCommand('Reconnecting Activity flow…', () => requireInvoke()('reconnect_activity_edge', {
        diagramId: diagram.id,
        activityEdgeId: edge.id,
        sourceEndpoint: nextSource,
        targetEndpoint: nextTarget,
      }));
      await refreshActivity();
    };
    $('activity-delete-edge').onclick = () => void deleteSelection();
    $('activity-route-diagram').onclick = async () => {
      await runCommand('Routing Activity…', () => requireInvoke()('route_activity_diagram', {
        diagramId: diagram.id,
      }));
      await refreshActivity();
    };
  };
})();
