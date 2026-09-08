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

  function hasEntries(value) {
    if (!value) return false;
    if (Array.isArray(value)) return value.length > 0;
    if (typeof value === 'object') return Object.keys(value).length > 0;
    return false;
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
      && (state.snapshot?.ibd_diagrams || []).length === 0;
  }

  function activityStatePresent(snapshot) {
    return hasEntries(snapshot?.repository?.activities)
      || hasEntries(snapshot?.repository?.external_ids)
      || (snapshot?.diagrams || []).length > 0;
  }

  function behaviorStatePresent(snapshot = state.snapshot) {
    const repository = snapshot?.behavior_repository;
    return hasEntries(repository?.state_machines)
      || hasEntries(repository?.interactions)
      || hasEntries(repository?.external_ids)
      || (snapshot?.behavior_diagrams || []).length > 0;
  }

  async function qualifyBlankProjectBaseline() {
    if (typeof refresh === 'function') await refresh();
    if (!projectIsSemanticallyBlank()) return false;

    // Blank-project model-script imports always start from one authoritative
    // clean Rust baseline. Do not infer whether specialized state is stale: a
    // semantically blank Project cannot intentionally own Activity or Behavior
    // semantics. Recreate the blank Project to clear Behavior state, then reset
    // the separately managed Activity workspace and executions before preview.
    const projectName = state.snapshot?.project?.name || 'Model Script Import';
    await invoke('new_project', { name: projectName });
    await invoke('reset_activity_workspace');
    await invoke('clear_activity_executions');
    if (typeof refresh === 'function') await refresh();
    const activitySnapshot = await invoke('activity_snapshot');
    Object.assign(state, {
      activitySnapshot,
      selectedActivityDiagramId: null,
      selectedActivityNodeId: null,
      selectedActivityEdgeId: null,
      selectedBehaviorDiagramId: null,
      selectedBehaviorItem: null,
    });

    if (!projectIsSemanticallyBlank() || activityStatePresent(activitySnapshot) || behaviorStatePresent()) {
      throw new Error('BLANK_PROJECT_BASELINE_INCOMPLETE: the Rust Project, Activity, and Behavior stores did not converge to one clean blank workspace');
    }
    notify('Qualified a clean Project, Activity, and Behavior baseline before model-script validation.');
    return true;
  }

  function normalizeDiagramFamily(value) {
    const family = String(value || '').trim().toLowerCase().replace(/_/g, '-');
    if (family === 'requirements') return 'requirement';
    if (family === 'usecase' || family === 'use case') return 'use-case';
    if (family === 'statemachine' || family === 'state machine') return 'state-machine';
    return family;
  }

  function requestedDiagrams(applied) {
    return (applied?.items || []).flatMap((item) => {
      const operation = String(item.operation || '');
      if (!operation.toLowerCase().startsWith('diagram::')) return [];
      return [{
        family: normalizeDiagramFamily(operation.slice('Diagram::'.length)),
        externalId: item.external_id || '',
        name: item.semantic_name || '',
      }];
    });
  }

  function committedDiagrams() {
    const ordinary = (state.snapshot?.diagrams || []).map((diagram) => ({
      id: diagram.id,
      name: diagram.name,
      family: normalizeDiagramFamily(diagram.family),
    }));
    const ibd = (state.snapshot?.ibd_diagrams || []).map((diagram) => ({
      id: diagram.id,
      name: diagram.name,
      family: 'ibd',
    }));
    const behavior = (state.snapshot?.behavior_diagrams || state.behaviorSnapshot?.diagrams || []).map((diagram) => ({
      id: diagram.id,
      name: diagram.name,
      family: diagram.kind === 'StateMachine' ? 'state-machine' : 'sequence',
    }));
    const activity = (state.activitySnapshot?.diagrams || []).map((diagram) => ({
      id: diagram.id,
      name: diagram.name,
      family: 'activity',
    }));
    return [...ordinary, ...ibd, ...behavior, ...activity];
  }

  async function qualifyCommittedDiagramSet(applied, freshImport) {
    if (typeof refresh === 'function') await refresh();
    const activitySnapshot = await invoke('activity_snapshot');
    Object.assign(state, { activitySnapshot });
    if (window.smpLoadBehaviorSnapshot) await window.smpLoadBehaviorSnapshot();

    const expected = requestedDiagrams(applied);
    const actual = committedDiagrams();
    const missing = [];
    const duplicated = [];
    for (const diagram of expected) {
      const matches = actual.filter((candidate) => candidate.family === diagram.family && candidate.name === diagram.name);
      if (matches.length === 0) missing.push(`${diagram.family}:${diagram.name || diagram.externalId}`);
      else if (matches.length > 1) duplicated.push(`${diagram.family}:${diagram.name || diagram.externalId}`);
    }
    if (missing.length || duplicated.length) {
      const details = [
        missing.length ? `missing ${missing.join(', ')}` : null,
        duplicated.length ? `duplicated ${duplicated.join(', ')}` : null,
      ].filter(Boolean).join('; ');
      throw new Error(`DIAGRAM_COMMIT_INCOMPLETE: ${details}`);
    }
    if (freshImport && actual.length !== expected.length) {
      throw new Error(`DIAGRAM_COMMIT_COUNT_MISMATCH: script declared ${expected.length} diagram(s), committed blank-project workspace contains ${actual.length}`);
    }

    if (typeof render === 'function') render();
    const tabs = [...document.querySelectorAll('#diagram-tabs .diagram-tab')].map((tab) => String(tab.textContent || ''));
    const missingTabs = expected.filter((diagram) => !tabs.some((label) => label.includes(diagram.name)));
    if (missingTabs.length) {
      throw new Error(`DIAGRAM_UI_REGISTRATION_INCOMPLETE: ${missingTabs.map((diagram) => `${diagram.family}:${diagram.name}`).join(', ')}`);
    }

    const requiredNine = new Set(['package', 'requirement', 'use-case', 'bdd', 'ibd', 'activity', 'state-machine', 'sequence', 'parametric']);
    const expectedFamilies = new Set(expected.map((diagram) => diagram.family));
    if (expected.length === 9 && requiredNine.size === expectedFamilies.size
      && [...requiredNine].every((family) => expectedFamilies.has(family))) {
      const committedFamilies = new Set(actual.map((diagram) => diagram.family));
      const absent = [...requiredNine].filter((family) => !committedFamilies.has(family));
      if (absent.length) throw new Error(`ALL_NINE_FAMILY_QUALIFICATION_FAILED: ${absent.join(', ')}`);
    }
  }

  async function runModelScript() {
    try {
      const file = await chooseScript();
      if (!file) return;
      const source = await file.text();
      const freshImport = await qualifyBlankProjectBaseline();
      notify(`Dry-running ${file.name}…`);
      const preview = await invoke('preview_model_script', { scriptName: file.name, source });
      if (!await inspect(preview)) return;
      notify(`Applying ${file.name} atomically…`);
      const applied = await invoke('apply_model_script', { scriptName: file.name, source });
      if (!applied?.applied) throw new Error(previewText(applied));
      await qualifyCommittedDiagramSet(applied, freshImport);
      notify(`Model script applied and diagram set qualified: ${actionSummary(applied)}`);
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