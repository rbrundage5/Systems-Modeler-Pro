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

  async function checkpointIfNeeded(label) {
    if (restoringHistory || isNonMutatingCommand(label)) return;
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

      // This is the already-qualified refresh chain. In particular,
      // behavior-refresh-authority rehydrates STM/SEQ from behavior_snapshot
      // and preserves the active Behavior diagram when it still exists.
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

  function addHistoryButtons() {
    if (document.getElementById('undo-command')) return;
    const ribbon = document.querySelector('.ribbon');
    if (!ribbon) return;
    const group = document.createElement('section');
    group.className = 'ribbon-group history-ribbon-group';
    group.innerHTML = `
      <div class="ribbon-actions">
        <button id="undo-command" class="ribbon-command" title="Undo (Ctrl+Z)">
          <span class="command-icon">↶</span><span>Undo</span>
        </button>
        <button id="redo-command" class="ribbon-command" title="Redo (Ctrl+Y / Ctrl+Shift+Z)">
          <span class="command-icon">↷</span><span>Redo</span>
        </button>
      </div>
      <div class="ribbon-label">History</div>`;
    const context = ribbon.querySelector('.ribbon-context');
    ribbon.insertBefore(group, context || null);
    document.getElementById('undo-command').onclick = () => void performHistory('undo');
    document.getElementById('redo-command').onclick = () => void performHistory('redo');
  }

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

  addHistoryButtons();
})();
