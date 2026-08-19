(() => {
  let restoringHistory = false;
  let historyBusy = false;

  function isNonMutatingCommand(label) {
    const text = String(label || '').toLowerCase();
    return text.startsWith('saving ')
      || text.startsWith('opening ')
      || text.startsWith('loading ')
      || text.startsWith('reading ')
      || text.startsWith('exporting ');
  }

  async function checkpointIfNeeded(label) {
    if (restoringHistory || isNonMutatingCommand(label)) return;
    await requireInvoke()('history_checkpoint');
  }

  const baseRunCommand = runCommand;
  runCommand = async function runCommandWithHistory(label, operation) {
    await checkpointIfNeeded(label);
    return baseRunCommand(label, operation);
  };

  async function refreshAfterHistory() {
    if (typeof window.smpSynchronizeAuthoritativeState === 'function') {
      await window.smpSynchronizeAuthoritativeState();
      return;
    }
    await refresh();
  }

  async function performHistory(direction) {
    if (historyBusy) return;
    historyBusy = true;
    restoringHistory = true;
    try {
      const changed = await requireInvoke()(direction === 'undo' ? 'history_undo' : 'history_redo');
      if (changed) {
        state.selectedRelationshipId = null;
        state.selectedActivityEdgeId = null;
        state.selectedBehaviorItem = null;
        state.pendingRelationship = null;
        state.paletteTool = null;
        await refreshAfterHistory();
        renderStatus(direction === 'undo' ? 'Undo complete' : 'Redo complete');
      } else {
        renderStatus(direction === 'undo' ? 'Nothing to undo' : 'Nothing to redo');
      }
    } catch (error) {
      console.error(`${direction} failed`, error);
      renderStatus(`${direction === 'undo' ? 'Undo' : 'Redo'} failed: ${error}`);
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

  let lastProjectId = state.snapshot?.project?.id || null;
  const baseRender = render;
  render = function renderWithHistoryControls() {
    const nextProjectId = state.snapshot?.project?.id || null;
    if (lastProjectId && nextProjectId && nextProjectId !== lastProjectId) {
      void requireInvoke()('history_reset').catch((error) => console.error('History reset failed', error));
    }
    lastProjectId = nextProjectId;
    baseRender();
    addHistoryButtons();
  };

  addHistoryButtons();
})();
