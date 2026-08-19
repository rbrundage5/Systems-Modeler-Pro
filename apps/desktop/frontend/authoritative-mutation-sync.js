(() => {
  let synchronizing = false;

  function isNonMutatingLabel(label) {
    const text = String(label || '').toLowerCase();
    return text.startsWith('saving ')
      || text.startsWith('opening ')
      || text.startsWith('loading ')
      || text.startsWith('reading ')
      || text.startsWith('exporting ');
  }

  async function synchronizeAuthoritativeState() {
    if (synchronizing) return;
    synchronizing = true;
    try {
      const invoke = requireInvoke();
      const workspace = await invoke('workspace_snapshot_complete');
      state.snapshot = workspace;
      state.behaviorSnapshot = {
        repository: workspace.behavior_repository,
        diagrams: workspace.behavior_diagrams,
      };
      state.activitySnapshot = await invoke('activity_snapshot');

      if (state.selectedBehaviorDiagramId && !workspace.behavior_diagrams?.some(
        (diagram) => String(diagram.id) === String(state.selectedBehaviorDiagramId),
      )) {
        state.selectedBehaviorDiagramId = null;
        state.selectedBehaviorItem = null;
      }
      if (state.selectedActivityDiagramId && !state.activitySnapshot?.diagrams?.some(
        (diagram) => String(diagram.id) === String(state.selectedActivityDiagramId),
      )) {
        state.selectedActivityDiagramId = null;
        state.selectedActivityNodeId = null;
        state.selectedActivityEdgeId = null;
      }
      if (state.selectedDiagramId && !workspace.diagrams?.some(
        (diagram) => String(diagram.id) === String(state.selectedDiagramId),
      ) && !workspace.ibd_diagrams?.some(
        (diagram) => String(diagram.id) === String(state.selectedDiagramId),
      )) {
        state.selectedDiagramId = null;
      }

      render();
    } finally {
      synchronizing = false;
    }
  }

  window.smpSynchronizeAuthoritativeState = synchronizeAuthoritativeState;

  const baseRunCommand = runCommand;
  runCommand = async function runCommandWithAuthoritativeSync(label, operation) {
    const result = await baseRunCommand(label, operation);
    if (!isNonMutatingLabel(label)) {
      await synchronizeAuthoritativeState();
    }
    return result;
  };
})();
