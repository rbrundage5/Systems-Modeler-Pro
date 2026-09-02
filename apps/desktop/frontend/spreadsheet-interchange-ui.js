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
      input.onchange = () => resolve(input.files?.[0]?.path || input.files?.[0]?.name || null);
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
      const path = await chooseFile('.xlsx,.csv');
      if (!path) return;
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
