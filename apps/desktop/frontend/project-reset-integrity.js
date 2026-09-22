(() => {
  const newProjectButton = $('new-project');
  if (!newProjectButton) return;

  // Activity semantics live in a separate Rust repository from Project. The
  // legacy Activity UI reset is conditional on the frontend snapshot already
  // containing a Project, which can leave old Activity owner/context ElementIds
  // alive after Project::new replaces the semantic Project. Those stale IDs then
  // block otherwise valid model-script imports during native Activity validation.
  const previousNewProject = newProjectButton.onclick;
  newProjectButton.onclick = async (...args) => {
    const result = await previousNewProject?.apply(newProjectButton, args);
    if (result?.outcome !== 'committed') return result;

    await requireInvoke()('reset_activity_workspace');
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
