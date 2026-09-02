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

  async function runModelScript() {
    try {
      const file = await chooseScript();
      if (!file) return;
      const source = await file.text();
      notify(`Dry-running ${file.name}…`);
      const preview = await invoke('preview_model_script', { scriptName: file.name, source });
      if (!await inspect(preview)) return;
      notify(`Applying ${file.name} atomically…`);
      const applied = await invoke('apply_model_script', { scriptName: file.name, source });
      if (!applied?.applied) throw new Error(previewText(applied));
      if (typeof refresh === 'function') await refresh();
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
