(() => {
  const invoke = window.__TAURI__?.core?.invoke;
  if (!invoke) return;

  function notify(message, type = 'info') {
    if (window.smpDialogs?.notify) window.smpDialogs.notify(message, type);
    else if (typeof renderStatus === 'function') renderStatus(message);
  }

  function chooseScript() {
    return new Promise((resolve) => {
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = '.groovy,.gvy,.smscript,.json,text/plain,application/json';
      input.onchange = () => resolve(input.files?.[0] || null);
      input.click();
    });
  }

  function actionSummary(preview) {
    const counts = { CREATE: 0, UPDATE: 0, NO_CHANGE: 0, BLOCKED: 0 };
    for (const item of preview?.items || []) counts[item.action] = (counts[item.action] || 0) + 1;
    counts.BLOCKED += (preview?.diagnostics || []).length;
    return `CREATE ${counts.CREATE} · UPDATE ${counts.UPDATE} · NO_CHANGE ${counts.NO_CHANGE} · BLOCKED ${counts.BLOCKED}`;
  }

  function previewText(preview) {
    const items = (preview?.items || []).slice(0, 300).map((item) => {
      const id = item.external_id ? ` ${item.external_id}` : '';
      const name = item.semantic_name ? ` (${item.semantic_name})` : '';
      return `${item.action} [${item.statement}] ${item.operation}${id}${name} — ${item.detail}`;
    });
    const diagnostics = (preview?.diagnostics || []).map((item) => {
      const location = [item.script, item.line ? `line ${item.line}` : null, item.statement ? `statement ${item.statement}` : null].filter(Boolean).join(' · ');
      return `BLOCKED ${item.code}${location ? ` [${location}]` : ''}: ${item.reason}`;
    });
    return [actionSummary(preview), ...items, ...diagnostics].join('\n');
  }

  async function inspect(preview) {
    const blocked = (preview?.diagnostics || []).length > 0 || (preview?.items || []).some((item) => item.action === 'BLOCKED');
    const result = await window.smpDialogs?.edit?.({
      title: 'Model Script dry run',
      description: `${actionSummary(preview)}${blocked ? ' · Apply is blocked until diagnostics are resolved.' : ''}`,
      fields: [{ id: 'preview', label: 'Operations and diagnostics', value: previewText(preview), multiline: true, readonly: true }],
      confirmLabel: blocked ? 'Close' : 'Apply',
    });
    return !blocked && !!result;
  }

  function projectIsSemanticallyBlank() {
    const project = state.snapshot?.project;
    if (!project) return false;
    const elements = project.elements || [];
    const relationships = project.relationships || [];
    return elements.length === 1
      && String(elements[0]?.id || '') === String(project.root_id || '')
      && relationships.length === 0
      && (state.snapshot?.diagrams || []).length === 0
      && (state.snapshot?.ibd_diagrams || []).length === 0
      && (state.snapshot?.behavior_diagrams || []).length === 0;
  }

  async function repairOrphanedActivityStateForBlankProject() {
    if (typeof refresh === 'function') await refresh();
    if (!projectIsSemanticallyBlank()) return;

    const activitySnapshot = await invoke('activity_snapshot');
    const activities = Object.values(activitySnapshot?.repository?.activities || {});
    if (!activities.length) return;

    const projectIds = new Set((state.snapshot?.project?.elements || []).map((element) => String(element.id)));
    const orphaned = activities.some((activity) => {
      const ownerMissing = !projectIds.has(String(activity.owner_id || ''));
      const contextMissing = activity.context_id != null && !projectIds.has(String(activity.context_id));
      return ownerMissing || contextMissing;
    });
    if (!orphaned) return;

    // A blank Project cannot legitimately own an Activity whose owner/context
    // points outside that Project. This is stale cross-store session state, not
    // authored content. Repair only that provably inconsistent case so dry-run
    // semantics remain non-destructive for valid projects.
    await invoke('reset_activity_workspace');
    await invoke('clear_activity_executions');
    state.activitySnapshot = { repository: { activities: {} }, diagrams: [] };
    state.selectedActivityDiagramId = null;
    state.selectedActivityNodeId = null;
    notify('Cleared orphaned Activity state from the blank project before model-script validation.');
  }

  function requestedSpecializedFamilies(applied) {
    const requested = new Set();
    for (const item of applied?.items || []) {
      const operation = String(item.operation || '').toLowerCase();
      if (!operation.startsWith('diagram::')) continue;
      const family = operation.slice('diagram::'.length).replace(/_/g, '-').trim();
      if (family === 'activity') requested.add('ACT');
      else if (family === 'sequence') requested.add('SEQ');
      else if (['state machine', 'state-machine', 'statemachine'].includes(family)) requested.add('STM');
    }
    return requested;
  }

  async function qualifySpecializedDiagramCommit(applied) {
    if (typeof refresh === 'function') await refresh();
    state.activitySnapshot = await invoke('activity_snapshot');
    if (window.smpLoadBehaviorSnapshot) await window.smpLoadBehaviorSnapshot();

    const requested = requestedSpecializedFamilies(applied);
    const missing = [];
    if (requested.has('ACT') && !(state.activitySnapshot?.diagrams || []).length) missing.push('ACT');
    const behaviorDiagrams = state.snapshot?.behavior_diagrams || state.behaviorSnapshot?.diagrams || [];
    if (requested.has('STM') && !behaviorDiagrams.some((diagram) => diagram.kind === 'StateMachine')) missing.push('STM');
    if (requested.has('SEQ') && !behaviorDiagrams.some((diagram) => diagram.kind === 'Sequence')) missing.push('SEQ');

    if (missing.length) {
      throw new Error(`SPECIALIZED_DIAGRAM_COMMIT_INCOMPLETE: imported model declared ${missing.join(', ')} diagram(s), but they are absent from the committed Rust workspace`);
    }
    if (typeof render === 'function') render();
  }

  async function runModelScript() {
    try {
      const file = await chooseScript();
      if (!file) return;
      const source = await file.text();
      await repairOrphanedActivityStateForBlankProject();
      notify(`Dry-running ${file.name}…`);
      const preview = await invoke('preview_model_script', { scriptName: file.name, source });
      if (!await inspect(preview)) return;
      notify(`Applying ${file.name} atomically…`);
      const applied = await invoke('apply_model_script', { scriptName: file.name, source });
      if (!applied?.applied) throw new Error(previewText(applied));
      await qualifySpecializedDiagramCommit(applied);
      notify(`Model script applied: ${actionSummary(applied)}`);
    } catch (error) {
      notify(`Model script failed: ${error?.message || error}`, 'error');
    }
  }

  function ensureFileRibbonCommand() {
    const ribbon = document.querySelector('.ribbon');
    const fileTab = [...document.querySelectorAll('.workspace-tab')].find((tab) => tab.textContent.trim() === 'File');
    if (!ribbon || !fileTab?.classList.contains('active') || ribbon.querySelector('[data-pr51-model-script]')) return;
    const group = document.createElement('section');
    group.className = 'ribbon-group';
    group.dataset.pr51ModelScript = 'true';
    group.innerHTML = '<div class="ribbon-actions ribbon-large-actions"><button class="ribbon-command" data-pr51-run-model-script><span class="command-icon">{ }</span><span>Run Model<br>Script</span></button></div><div class="ribbon-label">Automation</div>';
    const history = [...ribbon.querySelectorAll('.ribbon-group')].find((candidate) => candidate.textContent.includes('History'));
    if (history) ribbon.insertBefore(group, history); else ribbon.appendChild(group);
    group.querySelector('[data-pr51-run-model-script]')?.addEventListener('click', runModelScript);
  }

  const ribbon = document.querySelector('.ribbon');
  if (ribbon) new MutationObserver(ensureFileRibbonCommand).observe(ribbon, { childList: true, subtree: true });
  document.addEventListener('click', (event) => {
    if (event.target?.closest?.('.workspace-tab')?.textContent.trim() === 'File') setTimeout(ensureFileRibbonCommand, 0);
  });
  window.smpRunModelScript = runModelScript;
})();
