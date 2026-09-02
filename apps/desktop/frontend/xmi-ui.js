(function () {
  'use strict';

  const invoke = window.__TAURI__?.core?.invoke;
  const notify = (message) => window.modelerNotify ? window.modelerNotify(message) : console.info(message);
  const dialogs = () => window.smpDialogs;
  const activeProject = () => window.__MODEL_SNAPSHOT__?.project || window.workspaceSnapshot?.project;
  const targetScopeId = () => window.__MODEL_SELECTION__?.elementId || activeProject()?.root_id;

  async function pickXmi() {
    return new Promise((resolve) => {
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = '.xmi,.uml,.xml';
      input.addEventListener('change', () => resolve(input.files?.[0] || null), { once: true });
      input.click();
    });
  }

  function summary(preview) {
    return `CREATE ${preview.create_count || 0}, UPDATE ${preview.update_count || 0}, NO_CHANGE ${preview.no_change_count || 0}, REMOVE ${preview.remove_count || 0}, BLOCKED ${preview.blocked_count || 0}`;
  }

  function previewText(preview) {
    const namespaces = Object.entries(preview.namespaces || {}).map(([prefix, uri]) => `${prefix || '(default)'} = ${uri}`).join('\n');
    const diagnostics = (preview.diagnostics || []).map((item) => `${item.code}: ${item.reason}`).join('\n');
    const actions = (preview.items || []).slice(0, 30).map((item) => `${item.action} ${item.xmi_type} ${item.xmi_id}: ${item.detail}`).join('\n');
    return `${preview.producer ? `Producer: ${preview.producer}\n` : ''}${summary(preview)}\n\nNamespaces\n${namespaces || '(none)'}\n\nActions\n${actions || '(none)'}${diagnostics ? `\n\nDiagnostics\n${diagnostics}` : ''}`;
  }

  async function openImport() {
    if (!invoke) throw new Error('XMI import is available in the desktop application.');
    const project = activeProject();
    if (!project) throw new Error('Create or open a project before XMI import.');
    const file = await pickXmi();
    if (!file) return;
    const source = `xmi:${file.name.replace(/\.(xmi|uml|xml)$/i, '').replace(/[^A-Za-z0-9._-]+/g, '-') || 'exchange'}`;
    const mode = await dialogs().choose({
      title: 'Configure XMI import',
      description: 'Choose synchronization policy. Authoritative mode may remove only stale content previously imported from this source.',
      fields: [{ id: 'source', label: 'Stable source namespace', value: source, required: true }],
      candidates: [{ id: 'additive-update', label: 'Additive / update (recommended)' }, { id: 'authoritative-xmi-scope', label: 'Authoritative XMI source scope' }],
      confirmLabel: 'Preview'
    });
    if (!mode) return;
    const configuration = {
      source_namespace: mode.values.source,
      target_scope: targetScopeId() || project.root_id,
      synchronization: mode.selectedId || 'additive-update'
    };
    let staged;
    try {
      staged = await invoke('stage_xmi_upload', { fileName: file.name, bytes: Array.from(new Uint8Array(await file.arrayBuffer())) });
      const preview = await invoke('preview_xmi_import', { path: staged, configuration });
      const approved = await dialogs().confirm({ title: 'XMI import preview', description: previewText(preview), confirmLabel: 'Apply atomically' });
      if (!approved || (preview.blocked_count || 0) > 0) return;
      const applied = await invoke('apply_xmi_import', { path: staged, configuration });
      if (!applied.applied) throw new Error(previewText(applied));
      notify(`XMI applied atomically. ${summary(applied)}`);
      window.location.reload();
    } finally {
      if (staged) {
        try { await invoke('discard_staged_xmi', { path: staged }); } catch (_) { /* cleanup only */ }
      }
    }
  }

  async function openExport() {
    if (!invoke) throw new Error('XMI export is available in the desktop application.');
    const project = activeProject();
    if (!project) throw new Error('Create or open a project before XMI export.');
    const result = await dialogs().edit({ title: 'Export semantic XMI', description: 'Diagram geometry is intentionally excluded.', fields: [{ id: 'path', label: 'Destination path', value: `${project.name || 'model'}.xmi`, required: true }], confirmLabel: 'Export' });
    if (!result) return;
    const output = await invoke('export_xmi', { path: result.values.path });
    notify(`Semantic XMI exported to ${output}. Diagram geometry is intentionally excluded.`);
  }

  function install() {
    const ribbon = document.querySelector('.ribbon-content, .ribbon-groups');
    if (!ribbon || ribbon.querySelector('[data-xmi-interchange]')) return;
    const group = document.createElement('section');
    group.className = 'ribbon-group';
    group.dataset.xmiInterchange = 'true';
    group.innerHTML = '<div class="ribbon-actions ribbon-large-actions"><button class="ribbon-command" data-xmi-import><span class="command-icon">⇩</span><span>Import<br>XMI</span></button><button class="ribbon-command" data-xmi-export><span class="command-icon">⇧</span><span>Export<br>XMI</span></button></div><div class="ribbon-label">Semantic XMI</div>';
    group.querySelector('[data-xmi-import]').addEventListener('click', () => openImport().catch((error) => notify(`XMI import failed: ${error.message || error}`)));
    group.querySelector('[data-xmi-export]').addEventListener('click', () => openExport().catch((error) => notify(`XMI export failed: ${error.message || error}`)));
    ribbon.appendChild(group);
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', install, { once: true });
  else install();
})();
