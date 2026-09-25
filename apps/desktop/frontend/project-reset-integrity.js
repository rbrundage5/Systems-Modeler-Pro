(() => {
  const newProjectButton = $('new-project');
  if (!newProjectButton) return;

  // Native New publishes Project, Activity, Behavior, path and history together.
  // Only local view state and the separately managed execution registry remain.
  const previousNewProject = newProjectButton.onclick;
  newProjectButton.onclick = async (...args) => {
    const result = await previousNewProject?.apply(newProjectButton, args);
    if (result?.outcome !== 'committed') return result;

    await requireInvoke()('clear_activity_executions');
    state.activitySnapshot = { repository: { activities: {} }, diagrams: [] };
    state.selectedActivityDiagramId = null;
    state.selectedActivityNodeId = null;
    state.activityTool = null;
    state.activityPendingFlow = null;
    Object.assign(state, {
      activityExecutionSnapshot: null,
      activityExecutionRunning: false,
    });
    render();
    return result;
  };
})();
