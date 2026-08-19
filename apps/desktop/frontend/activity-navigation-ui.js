(() => {
  function activeActivity() {
    const diagram = state.activitySnapshot?.diagrams?.find(
      (item) => String(item.id) === String(state.selectedActivityDiagramId),
    );
    return diagram
      ? state.activitySnapshot?.repository?.activities?.[String(diagram.activity_id)] || null
      : null;
  }

  function calledActivityId(node) {
    const actionKind = node?.kind?.Action?.kind;
    if (!actionKind || typeof actionKind === 'string') return null;
    const callBehavior = actionKind.CallBehavior;
    if (!callBehavior) return null;
    if (typeof callBehavior === 'string') return callBehavior;
    return callBehavior.activity_id || callBehavior.activityId || null;
  }

  async function openCalledActivity(nodeId) {
    const node = activeActivity()?.nodes?.find(
      (item) => String(item.id) === String(nodeId),
    );
    const activityId = calledActivityId(node);
    if (!activityId) return false;

    const target = state.activitySnapshot?.diagrams?.find(
      (diagram) => String(diagram.activity_id) === String(activityId),
    );
    if (!target) {
      alert('The referenced Activity exists but has no Activity Diagram presentation yet.');
      return true;
    }

    if (typeof window.smpSelectActivityDiagram !== 'function') {
      alert('Activity diagram navigation is unavailable.');
      return true;
    }

    await window.smpSelectActivityDiagram(target.id);
    return true;
  }

  function bindCallBehaviorDrillDown() {
    document.querySelectorAll('.activity-node[data-activity-node-id]').forEach((group) => {
      const nodeId = group.dataset.activityNodeId;
      const node = activeActivity()?.nodes?.find(
        (item) => String(item.id) === String(nodeId),
      );
      if (!calledActivityId(node)) {
        group.ondblclick = null;
        return;
      }
      group.classList.add('activity-call-behavior-link');
      group.setAttribute('aria-label', `${node?.name || 'Call Behavior'}; double-click to open called Activity`);
      group.ondblclick = async (event) => {
        event.preventDefault();
        event.stopPropagation();
        try {
          await openCalledActivity(nodeId);
        } catch (error) {
          console.error('Unable to open called Activity', error);
          alert(error?.message || String(error));
        }
      };
    });
  }

  const baseRenderCanvas = renderCanvas;
  renderCanvas = function renderActivityNavigationCanvas() {
    baseRenderCanvas();
    if (!state.selectedActivityDiagramId) return;
    bindCallBehaviorDrillDown();
  };

  document.addEventListener('dblclick', async (event) => {
    const group = event.target.closest?.('.activity-node[data-activity-node-id]');
    if (!group || !state.selectedActivityDiagramId) return;
    if (!calledActivityId(activeActivity()?.nodes?.find(
      (item) => String(item.id) === String(group.dataset.activityNodeId),
    ))) return;
    if (group.ondblclick) return;
    event.preventDefault();
    event.stopPropagation();
    await openCalledActivity(group.dataset.activityNodeId);
  }, true);
})();
