(() => {
  // PR12 established Behavior as Rust-authoritative state. Keep STM/SEQ hydration
  // explicit because the structural "complete" snapshot does not carry Behavior.
  // This wrapper preserves the selected behavior diagram across general refreshes
  // and then replaces the cached Behavior snapshot from Rust before the final render.
  let behaviorRefreshInProgress = false;

  async function refreshBehaviorSnapshotPreservingSelection() {
    if (behaviorRefreshInProgress) return;
    behaviorRefreshInProgress = true;
    const selectedDiagramId = state.selectedBehaviorDiagramId;
    try {
      state.behaviorSnapshot = await requireInvoke()('behavior_snapshot');
      if (selectedDiagramId && state.behaviorSnapshot?.diagrams?.some(
        (diagram) => String(diagram.id) === String(selectedDiagramId),
      )) {
        state.selectedBehaviorDiagramId = selectedDiagramId;
      } else if (selectedDiagramId) {
        state.selectedBehaviorDiagramId = null;
        state.selectedBehaviorItem = null;
        state.behaviorTool = null;
        state.behaviorPending = null;
      }
    } finally {
      behaviorRefreshInProgress = false;
    }
  }

  window.smpRefreshBehaviorSnapshot = refreshBehaviorSnapshotPreservingSelection;

  const baseRefresh = refresh;
  refresh = async function refreshWithAuthoritativeBehavior() {
    const selectedDiagramId = state.selectedBehaviorDiagramId;
    await baseRefresh();
    // Earlier refresh wrappers may clear behavior selection because the structural
    // complete snapshot has no Behavior payload. Rehydrate from the dedicated Rust
    // Behavior command, then restore selection only if the diagram still exists.
    state.selectedBehaviorDiagramId = selectedDiagramId;
    await refreshBehaviorSnapshotPreservingSelection();
    render();
  };
})();
