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
    const kind = node?.kind?.Action?.kind;
    if (!kind || typeof kind === 'string') return null;
    return kind.CallBehavior?.activity_id || null;
  }

  document.addEventListener('dblclick', async (event) => {
    const group = event.target.closest?.('.activity-node');
    const nodeId = group?.dataset?.activityNodeId;
    if (!group || !nodeId || !state.selectedActivityDiagramId) return;
    const node = activeActivity()?.nodes?.find((item) => String(item.id) === String(nodeId));
    const activityId = calledActivityId(node);
    if (!activityId) return;
    const target = state.activitySnapshot?.diagrams?.find(
      (diagram) => String(diagram.activity_id) === String(activityId),
    );
    if (!target) {
      alert('The referenced Activity exists but has no Activity Diagram presentation yet.');
      return;
    }
    event.preventDefault();
    event.stopImmediatePropagation();
    if (typeof window.smpSelectActivityDiagram === 'function') {
      await window.smpSelectActivityDiagram(target.id);
    }
  }, true);
})();