(() => {
  function isAbsoluteProjectPath(path) {
    return /^[A-Za-z]:[\\/]/.test(path) || /^\\\\/.test(path) || path.startsWith('/');
  }

  function projectOpenCandidates(path) {
    if (isAbsoluteProjectPath(path)) return [path];
    const prefixes = ['', '..', '../..', '../../..', '../../../..'];
    return prefixes.map((prefix) => (prefix ? `${prefix}/${path}` : path));
  }

  async function openProjectCompat() {
    const suggested = state.snapshot?.current_file || 'Vehicle Model.smproj';
    const requestedPath = prompt('Project file path (.smproj)', suggested);
    if (!requestedPath) return { outcome: 'cancelled' };

    let openedPath = null;
    let missingError = null;
    for (const candidate of projectOpenCandidates(requestedPath.trim())) {
      try {
        openedPath = await requireInvoke()('open_project_file_complete', { path: candidate });
        break;
      } catch (error) {
        const message = error?.message || String(error);
        if (!message.includes('project file does not exist:')) throw error;
        missingError = error;
      }
    }

    if (!openedPath) throw missingError || new Error(`project file does not exist: ${requestedPath}`);

    Object.assign(state, {
      paletteItems: [],
      paletteTool: null,
      selectedElementId: null,
      selectedPackageId: null,
      selectedDiagramId: null,
      selectedRelationshipId: null,
      pendingRelationship: null,
      selectedBehaviorDiagramId: null,
      selectedActivityDiagramId: null,
      selectedActivityNodeId: null,
      activityTool: null,
      activityPendingFlow: null,
    });

    state.activitySnapshot = await requireInvoke()('activity_snapshot');
    await refresh();
    state.activitySnapshot = await requireInvoke()('activity_snapshot');

    if (!state.selectedDiagramId && state.snapshot?.diagrams?.length) {
      await selectDiagram(state.snapshot.diagrams[0].id);
    }
    render();
    return { outcome: 'committed' };
  }

  const openButton = $('open-project');
  if (openButton) {
    openButton.onclick = () => runCommand('Opening project…', openProjectCompat);
  }
})();
