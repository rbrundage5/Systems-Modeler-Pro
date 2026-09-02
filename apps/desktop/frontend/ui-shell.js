(() => {
  const ribbon = document.querySelector('.ribbon');
  const tabs = [...document.querySelectorAll('.workspace-tab')];
  const commandStates = new Map();
  if (!ribbon || !tabs.length) return;

  const historyGroup = `
      <section class="ribbon-group"><div class="ribbon-actions">
        <button class="ribbon-command" data-command="undo"><span class="command-icon">↶</span><span>Undo</span></button>
        <button class="ribbon-command" data-command="redo"><span class="command-icon">↷</span><span>Redo</span></button>
      </div><div class="ribbon-label">History</div></section>`;

  const panels = {
    File: `
      <section class="ribbon-group"><div class="ribbon-actions ribbon-large-actions">
        <button class="ribbon-command" data-forward="new-project"><span class="command-icon">＋</span><span>New<br>Project</span></button>
        <button class="ribbon-command" data-forward="open-project"><span class="command-icon">▱</span><span>Open</span></button>
        <button class="ribbon-command" data-forward="save-project"><span class="command-icon">▣</span><span>Save</span></button>
        <button class="ribbon-command" data-forward="save-project-as"><span class="command-icon">▣</span><span>Save As</span></button>
      </div><div class="ribbon-label">Project</div></section>
      <section class="ribbon-group"><div class="ribbon-actions ribbon-large-actions">
        <button class="ribbon-command" data-action="import-spreadsheet"><span class="command-icon">⇩</span><span>Import<br>Spreadsheet</span></button>
        <button class="ribbon-command" data-action="export-spreadsheet"><span class="command-icon">⇧</span><span>Export<br>XLSX</span></button>
      </div><div class="ribbon-label">Spreadsheet Interchange</div></section>${historyGroup}`,
    Home: `
      <section class="ribbon-group"><div class="ribbon-actions">
        <button class="ribbon-command" data-forward="new-package"><span class="command-icon">□</span><span>Package</span></button>
        <button class="ribbon-command" data-forward="new-package-diagram"><span class="command-icon">pkg</span><span>Package<br>Diagram</span></button>
        <button class="ribbon-command" data-forward="new-bdd"><span class="command-icon">▤</span><span>BDD</span></button>
        <button class="ribbon-command" data-action="new-requirement"><span class="command-icon">R</span><span>Requirement</span></button>
        <button class="ribbon-command" data-action="new-use-case"><span class="command-icon">UC</span><span>Use Case</span></button>
        <button class="ribbon-command" data-action="new-parametric"><span class="command-icon">PAR</span><span>Parametric</span></button>
        <button class="ribbon-command" data-action="new-ibd"><span class="command-icon">▥</span><span>IBD</span></button>
        <button class="ribbon-command" data-action="new-state-machine"><span class="command-icon">◉</span><span>State Machine</span></button>
        <button class="ribbon-command" data-action="new-sequence"><span class="command-icon">⇥</span><span>Sequence</span></button>
        <button class="ribbon-command" data-action="new-activity"><span class="command-icon">▶</span><span>Activity</span></button>
      </div><div class="ribbon-label">Create</div></section>${historyGroup}
      <section class="ribbon-group ribbon-context"><div class="context-title">Active Diagram</div><div id="active-diagram-summary" class="context-value">No diagram selected</div><div class="context-subtitle">Elements and properties follow the active diagram</div><div class="ribbon-label">Context</div></section>`,
    Diagram: `
      <section class="ribbon-group"><div class="ribbon-actions">
        <button class="ribbon-command" data-forward="new-package-diagram"><span class="command-icon">pkg</span><span>New Package<br>Diagram</span></button>
        <button class="ribbon-command" data-forward="new-bdd"><span class="command-icon">▤</span><span>New BDD</span></button>
        <button class="ribbon-command" data-action="new-requirement"><span class="command-icon">R</span><span>New Requirement</span></button>
        <button class="ribbon-command" data-action="new-use-case"><span class="command-icon">UC</span><span>New Use Case</span></button>
        <button class="ribbon-command" data-action="new-parametric"><span class="command-icon">PAR</span><span>New Parametric</span></button>
        <button class="ribbon-command" data-action="new-ibd"><span class="command-icon">▥</span><span>New IBD</span></button>
        <button class="ribbon-command" data-action="new-state-machine"><span class="command-icon">◉</span><span>New State Machine</span></button>
        <button class="ribbon-command" data-action="new-sequence"><span class="command-icon">⇥</span><span>New Sequence</span></button>
        <button class="ribbon-command" data-action="new-activity"><span class="command-icon">▶</span><span>New Activity</span></button>
        <button class="ribbon-command" data-command="route"><span class="command-icon">⌁</span><span>Route</span></button>
      </div><div class="ribbon-label">Diagram</div></section>${historyGroup}
      <section class="ribbon-group ribbon-context"><div class="context-title">Active Diagram</div><div id="active-diagram-summary" class="context-value">No diagram selected</div><div class="context-subtitle">Rust-owned diagram commands</div><div class="ribbon-label">Context</div></section>`,
    Arrange: `<section class="ribbon-group"><div class="ribbon-actions"><button class="ribbon-command" data-command="route"><span class="command-icon">⌁</span><span>Route</span></button><button class="ribbon-command" data-command="cleanLayout"><span class="command-icon">⌁</span><span>Clean Layout</span></button><button class="ribbon-command" data-command="evaluateParametrics"><span class="command-icon">=</span><span>Evaluate Parametrics</span></button></div><div class="ribbon-label">Routing, Layout, and Analysis</div></section>${historyGroup}<section class="ribbon-group ribbon-context"><div class="context-title">Shared geometry and analysis</div><div class="context-value">Rust-owned routing, layout, and evaluation</div><div class="context-subtitle">Availability follows the active diagram capabilities.</div><div class="ribbon-label">Diagram</div></section>`,
    View: `
      <section class="ribbon-group"><div class="ribbon-actions compact-actions">
        <button class="ribbon-command panel-toggle" data-panel="repository-panel" data-command="showRepository"><span class="command-icon">▥</span><span>Repository</span></button>
        <button class="ribbon-command panel-toggle" data-panel="palette-panel" data-command="showElements"><span class="command-icon">▦</span><span>Elements</span></button>
        <button class="ribbon-command panel-toggle" data-panel="properties-panel" data-command="showProperties"><span class="command-icon">▤</span><span>Properties</span></button>
      </div><div class="ribbon-label">Panels</div></section>${historyGroup}`,
    Help: `<section class="ribbon-group ribbon-context"><div class="context-title">Systems Modeler Pro</div><div class="context-value">Native Rust migration</div><div class="context-subtitle">SysML engineering modeler desktop workspace</div><div class="ribbon-label">About</div></section>`,
  };

  const original = new Map();
  for (const id of ['new-project', 'open-project', 'save-project', 'save-project-as', 'new-package', 'new-package-diagram', 'new-bdd']) {
    const node = document.getElementById(id);
    if (node) original.set(id, node);
  }

  function syncPanelToggles() {
    document.querySelectorAll('.panel-toggle').forEach((button) => {
      const panel = document.querySelector(`.${button.dataset.panel}`);
      button.classList.toggle('active', !!panel && !panel.classList.contains('shell-hidden'));
    });
  }

  function bindRibbon() {
    ribbon.querySelectorAll('[data-forward]').forEach((button) => {
      button.addEventListener('click', () => original.get(button.dataset.forward)?.click());
    });
    ribbon.querySelectorAll('[data-action="new-ibd"]').forEach((button) => {
      button.addEventListener('click', () => window.smpCreateIbdForSelectedBlock?.());
    });
    ribbon.querySelectorAll('[data-action="new-requirement"]').forEach((button) => button.addEventListener('click', () => window.smpCreateRequirementDiagram?.()));
    ribbon.querySelectorAll('[data-action="new-use-case"]').forEach((button) => button.addEventListener('click', () => window.smpCreateUseCaseDiagram?.()));
    ribbon.querySelectorAll('[data-action="new-parametric"]').forEach((button) => button.addEventListener('click', () => window.smpCreateParametricDiagram?.()));
    ribbon.querySelectorAll('[data-action="new-state-machine"]').forEach((button) => {
      button.addEventListener('click', () => window.smpCreateStateMachineForSelectedBlock?.());
    });
    ribbon.querySelectorAll('[data-action="new-sequence"]').forEach((button) => {
      button.addEventListener('click', () => window.smpCreateSequenceForSelectedBlock?.());
    });
    ribbon.querySelectorAll('[data-action="new-activity"]').forEach((button) => {
      button.addEventListener('click', () => window.smpCreateActivityForSelection?.());
    });
    ribbon.querySelectorAll('[data-action="import-spreadsheet"]').forEach((button) => {
      button.addEventListener('click', () => window.smpSpreadsheetInterchange?.openImport());
    });
    ribbon.querySelectorAll('[data-action="export-spreadsheet"]').forEach((button) => {
      button.addEventListener('click', () => window.smpSpreadsheetInterchange?.openExport());
    });
    ribbon.querySelectorAll('[data-command]').forEach((button) => {
      button.addEventListener('click', async () => { await window.smpRendererHost?.execute(button.dataset.command); syncPanelToggles(); });
    });
    syncPanelToggles();
    syncCommandStates();
  }

  function syncCommandStates() {
    ribbon.querySelectorAll('[data-command]').forEach((button) => {
      const command = commandStates.get(button.dataset.command);
      if (!command) return;
      button.disabled = !command.enabled;
      button.title = command.enabled ? [command.label, command.shortcut].filter(Boolean).join(' · ') : command.disabledReason;
      button.setAttribute('aria-disabled', String(!command.enabled));
    });
  }

  function activate(name) {
    const currentContext = document.getElementById('active-diagram-summary')?.textContent || 'No diagram selected';
    tabs.forEach((tab) => tab.classList.toggle('active', tab.textContent.trim() === name));
    ribbon.innerHTML = panels[name] || panels.Home;
    bindRibbon();
    const context = document.getElementById('active-diagram-summary');
    if (context) context.textContent = currentContext;
    if (typeof renderContext === 'function') renderContext();
  }

  tabs.forEach((tab) => {
    tab.setAttribute('role', 'button');
    tab.tabIndex = 0;
    const activateTab = () => activate(tab.textContent.trim());
    tab.addEventListener('click', activateTab);
    tab.addEventListener('keydown', (event) => {
      if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); activateTab(); }
    });
  });

  document.addEventListener('smp:commands-ready', (event) => {
    commandStates.clear();
    for (const command of event.detail || []) commandStates.set(command.id, command);
    syncCommandStates();
  });

  activate('Home');
})();

// Spreadsheet interchange stays a thin desktop adapter. Semantic authority,
// validation, preview classification, and atomic application remain in Rust.
(() => {
  const invoke = window.__TAURI__?.core?.invoke;
  const STORAGE_KEY = 'smp.spreadsheet.map-groups.v1';

  function notify(message, type = 'info') {
    if (window.smpDialogs?.notify) window.smpDialogs.notify(message, type);
    else if (typeof renderStatus === 'function') renderStatus(message);
  }

  function savedGroups() {
    try { return JSON.parse(localStorage.getItem(STORAGE_KEY) || '{}'); }
    catch { return {}; }
  }

  function saveGroup(name, group) {
    const groups = savedGroups();
    groups[name] = group;
    localStorage.setItem(STORAGE_KEY, JSON.stringify(groups));
  }

  function chooseFile(accept) {
    return new Promise((resolve) => {
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = accept;
      input.onchange = () => resolve(input.files?.[0] || null);
      input.click();
    });
  }

  function summarize(preview) {
    const totals = preview?.totals;
    if (totals) return `CREATE ${totals.create} · UPDATE ${totals.update} · NO_CHANGE ${totals.no_change} · REMOVE ${totals.remove || 0} · BLOCKED ${totals.blocked}`;
    const counts = {};
    for (const item of preview?.items || []) counts[item.action] = (counts[item.action] || 0) + 1;
    return ['Create', 'Update', 'NoChange', 'Remove', 'Blocked'].map((key) => `${key.toUpperCase()} ${counts[key] || 0}`).join(' · ');
  }

  function previewText(preview) {
    const rows = (preview?.rows || preview?.items || []).slice(0, 200).map((row) => {
      const identity = row.identification_value || row.external_id || `row ${row.row || '?'}`;
      return `${row.action}: ${identity}${row.detail ? ` — ${row.detail}` : ''}`;
    });
    const diagnostics = (preview?.diagnostics || []).map((item) => typeof item === 'string' ? item : `${item.code}: ${item.reason}`);
    return [summarize(preview), ...rows, ...diagnostics.map((item) => `DIAGNOSTIC: ${item}`)].join('\n');
  }

  async function inspectAndConfirm(preview) {
    const result = await window.smpDialogs?.edit({
      title: 'Spreadsheet import preview',
      description: summarize(preview),
      fields: [{ id: 'preview', label: 'Actions and diagnostics', value: previewText(preview), multiline: true, readonly: true }],
      confirmLabel: 'Apply',
    });
    return !!result;
  }

  async function extendedImport(path) {
    const policyChoice = await window.smpDialogs?.choose({
      title: 'Synchronization policy',
      description: 'Missing workbook rows never remove content in additive mode. Authoritative mode previews proven removals.',
      candidates: [
        { id: 'additive', label: 'Additive / update-only' },
        { id: 'authoritative-mapped-scope', label: 'Authoritative mapped scope' },
      ],
      confirmLabel: 'Preview',
    });
    if (!policyChoice) return;
    const policy = policyChoice.selectedId || 'additive';
    const preview = await invoke('preview_spreadsheet_workbook_import', { path, policy });
    if (!await inspectAndConfirm(preview)) return;
    const applied = await invoke('apply_spreadsheet_workbook_import', { path, policy });
    if (!applied.applied) throw new Error(previewText(applied));
    if (window.smpState && typeof refresh === 'function') await refresh();
    notify('Spreadsheet workbook applied atomically.');
  }

  async function mappedImport(path) {
    const groups = savedGroups();
    const names = Object.keys(groups).sort();
    const result = await window.smpDialogs?.edit({
      title: 'Configure spreadsheet mapping',
      description: 'Paste a MapGroup JSON or reuse a saved map. Mapping order is preserved.',
      fields: [
        { id: 'savedName', label: `Saved map name${names.length ? ` (${names.join(', ')})` : ''}`, value: names[0] || '' },
        { id: 'groupJson', label: 'MapGroup JSON', value: names[0] ? JSON.stringify(groups[names[0]], null, 2) : '{\n  "mappings": []\n}', multiline: true },
      ],
      confirmLabel: 'Preview',
    });
    if (!result) return;
    const name = result.values.savedName.trim();
    const group = JSON.parse(result.values.groupJson);
    for (const map of group.mappings || []) map.source = path;
    if (name) saveGroup(name, group);
    const preview = await invoke('preview_spreadsheet_import', { group });
    if (!await inspectAndConfirm(preview)) return;
    const applied = await invoke('apply_spreadsheet_import', { group });
    if (!applied.applied) throw new Error(previewText(applied));
    if (typeof refresh === 'function') await refresh();
    notify('Mapped spreadsheet import applied atomically.');
  }

  async function openImport() {
    try {
      if (!invoke) throw new Error('Spreadsheet import is available in the desktop application.');
      const file = await chooseFile('.xlsx,.csv');
      if (!file) return;
      const bytes = [...new Uint8Array(await file.arrayBuffer())];
      const path = await invoke('stage_spreadsheet_upload', { fileName: file.name, bytes });
      try {
        const mode = await window.smpDialogs?.choose({
          title: 'Import Spreadsheet',
          description: 'Use full-fidelity mode for Systems-Modeler exports. Use mapped mode for CATIA-style XLSX or CSV.',
          candidates: [
            { id: 'extended', label: 'Systems-Modeler workbook' },
            { id: 'mapped', label: 'Mapped XLSX / CSV' },
          ],
          confirmLabel: 'Continue',
        });
        if (!mode) return;
        if (mode.selectedId === 'mapped') await mappedImport(path);
        else await extendedImport(path);
      } finally {
        await invoke('discard_staged_spreadsheet', { path });
      }
    } catch (error) { notify(String(error), 'error'); }
  }

  async function openExport() {
    try {
      if (!invoke) throw new Error('Spreadsheet export is available in the desktop application.');
      const result = await window.smpDialogs?.edit({
        title: 'Export XLSX',
        description: 'Systems-Modeler preserves all authored diagrams. CATIA-oriented exports semantic mapping sheets only.',
        fields: [
          { id: 'path', label: 'Destination path', value: 'systems-modeler-export.xlsx', required: true },
          { id: 'profile', label: 'Profile (systems-modeler or catia-semantic)', value: 'systems-modeler', required: true },
        ],
        confirmLabel: 'Export',
      });
      if (!result) return;
      await invoke('export_spreadsheet_workbook', { path: result.values.path, profile: result.values.profile });
      notify(`Spreadsheet exported to ${result.values.path}.`);
    } catch (error) { notify(String(error), 'error'); }
  }

  window.smpSpreadsheetInterchange = Object.freeze({ openImport, openExport, savedGroups });
})();
