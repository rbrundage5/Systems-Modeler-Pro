(() => {
  let historyBusy = false;
  let restoringHistory = false;

  function isSessionResetCommand(label) {
    const text = String(label || '').toLowerCase();
    return text.startsWith('creating project') || text.startsWith('opening project');
  }

  function isNonMutatingCommand(label) {
    const text = String(label || '').toLowerCase();
    return text.startsWith('saving ')
      || text.startsWith('loading ')
      || text.startsWith('reading ')
      || text.startsWith('exporting ')
      || isSessionResetCommand(label);
  }

  function isRustCheckpointedCommand(label) {
    const text = String(label || '').toLowerCase();
    return text.startsWith('updating diagram presentation');
  }

  async function checkpointIfNeeded(label) {
    if (restoringHistory || isNonMutatingCommand(label) || isRustCheckpointedCommand(label)) return;
    await requireInvoke()('history_checkpoint');
  }

  const baseRunCommand = runCommand;
  runCommand = async function runCommandWithHistory(label, operation) {
    await checkpointIfNeeded(label);
    const result = await baseRunCommand(label, operation);
    if (isSessionResetCommand(label)) await requireInvoke()('history_reset');
    return result;
  };

  async function performHistory(direction) {
    if (historyBusy) return;
    historyBusy = true;
    restoringHistory = true;
    try {
      const command = direction === 'undo' ? 'history_undo' : 'history_redo';
      const changed = await requireInvoke()(command);
      if (!changed) {
        renderStatus(direction === 'undo' ? 'Nothing to undo' : 'Nothing to redo');
        return;
      }

      state.selectedRelationshipId = null;
      state.selectedActivityEdgeId = null;
      state.pendingRelationship = null;
      state.paletteTool = null;
      state.behaviorPending = null;
      state.behaviorTool = null;

      // Reuse the already-qualified structural, Activity, and Behavior refresh chain.
      await refresh();
      renderStatus(direction === 'undo' ? 'Undo complete' : 'Redo complete');
    } catch (error) {
      const message = error?.message || String(error);
      console.error(`${direction} failed`, error);
      renderStatus(`${direction === 'undo' ? 'Undo' : 'Redo'} failed: ${message}`);
    } finally {
      restoringHistory = false;
      historyBusy = false;
    }
  }

  // ui-shell owns the visible ribbon and recreates it whenever the user changes
  // ribbon tabs. Export stable actions for that owner instead of inserting DOM here.
  window.smpUndo = () => performHistory('undo');
  window.smpRedo = () => performHistory('redo');

  document.addEventListener('keydown', (event) => {
    const tag = document.activeElement?.tagName?.toLowerCase();
    if (['input', 'textarea', 'select'].includes(tag)) return;
    const modifier = event.ctrlKey || event.metaKey;
    if (!modifier) return;
    const key = event.key.toLowerCase();
    if (key === 'z' && !event.shiftKey) {
      event.preventDefault();
      void performHistory('undo');
    } else if (key === 'y' || (key === 'z' && event.shiftKey)) {
      event.preventDefault();
      void performHistory('redo');
    }
  }, true);
})();
