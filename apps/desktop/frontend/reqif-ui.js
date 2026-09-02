(() => {
  const invoke = window.__TAURI__?.core?.invoke;
  const ribbon = document.querySelector('.ribbon');

  function notify(message, type = 'info') {
    if (window.smpDialogs?.notify) window.smpDialogs.notify(message, type);
    else if (typeof renderStatus === 'function') renderStatus(message);
  }

  function chooseFile() {
    return new Promise((resolve) => {
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = '.reqif,.reqifz';
      input.onchange = () => resolve(input.files?.[0] || null);
      input.click();
    });
  }

  function projectState() {
    return typeof state !== 'undefined' ? state.snapshot?.project : null;
  }

  function targetScopeId() {
    const project = projectState();
    if (!project) throw new Error('Create or open a project before ReqIF import/export.');
    const selectedId = typeof state !== 'undefined'
      ? (state.selectedElementId || state.selectedPackageId)
      : null;
    const selected = project.elements?.find((element) => element.id === selectedId);
    return selected && ['Model', 'Package'].includes(selected.kind) ? selected.id : project.root_id;
  }

  function summarize(preview) {
    const totals = preview?.totals || {};
    return `CREATE ${totals.create || 0} · UPDATE ${totals.update || 0} · NO_CHANGE ${totals.no_change || 0} · REMOVE ${totals.remove || 0} · BLOCKED ${totals.blocked || 0}`;
  }

  function previewText(preview) {
    const rows = (preview?.items || []).slice(0, 250).map((item) =>
      `${item.action}: ${item.identifier} [${item.kind}]${item.detail ? ` — ${item.detail}` : ''}`
    );
    const diagnostics = (preview?.diagnostics || []).map((item) =>
      `${item.severity} ${item.code}${item.identifier ? ` ${item.identifier}` : ''}: ${item.reason}`
    );
    return [summarize(preview), ...rows, ...diagnostics.map((item) => `DIAGNOSTIC: ${item}`)].join('\n');
  }

  function validPreview(preview) {
    return (preview?.totals?.blocked || 0) === 0
      && !(preview?.diagnostics || []).some((item) => item.severity === 'ERROR');
  }

  async function inspectPreview(preview, confirmLabel) {
    return window.smpDialogs?.edit({
      title: 'ReqIF import preview',
      description: summarize(preview),
      fields: [{ id: 'preview', label: 'Actions and diagnostics', value: previewText(preview), multiline: true, readonly: true }],
      confirmLabel,
    });
  }

  function defaultNamespace(fileName) {
    const stem = fileName.replace(/\.(reqifz?|REQIFZ?)$/i, '').replace(/[^A-Za-z0-9._-]+/g, '-');
    return `reqif:${stem || 'supplier'}`;
  }

  async function configureImport(file) {
    const mapping = await window.smpDialogs?.edit({
      title: 'Configure ReqIF import',
      description: 'Source namespace is durable reimport identity. Keep the same value for later versions of this supplier exchange. Empty mapping objects use standards/name detection; add explicit type or attribute mappings when needed.',
      fields: [
        { id: 'sourceNamespace', label: 'Stable source namespace', value: defaultNamespace(file.name), required: true },
        { id: 'mappingJson', label: 'Mapping overrides (JSON)', value: '{\n  "object_type_mappings": {},\n  "relation_type_mappings": {},\n  "attribute_mappings": {}\n}', multiline: true },
      ],
      confirmLabel: 'Continue',
    });
    if (!mapping) return null;
    const policy = await window.smpDialogs?.choose({
      title: 'ReqIF synchronization policy',
      description: 'Additive mode never removes native content. Authoritative ReqIF scope removes only records proven to belong to this source namespace and missing from the new exchange.',
      candidates: [
        { id: 'additive', label: 'Additive / update-only' },
        { id: 'authoritative-reqif-scope', label: 'Authoritative ReqIF scope' },
      ],
      confirmLabel: 'Preview',
    });
    if (!policy) return null;
    const overrides = JSON.parse(mapping.values.mappingJson || '{}');
    return {
      source_namespace: mapping.values.sourceNamespace.trim(),
      target_scope: targetScopeId(),
      synchronization: policy.selectedId || 'additive',
      object_type_mappings: overrides.object_type_mappings || {},
      relation_type_mappings: overrides.relation_type_mappings || {},
      attribute_mappings: overrides.attribute_mappings || {},
    };
  }

  async function openImport() {
    let stagedPath = null;
    try {
      if (!invoke) throw new Error('ReqIF import is available in the desktop application.');
      const file = await chooseFile();
      if (!file) return;
      const configuration = await configureImport(file);
      if (!configuration) return;
      const bytes = [...new Uint8Array(await file.arrayBuffer())];
      stagedPath = await invoke('stage_reqif_upload', { fileName: file.name, bytes });
      const preview = await invoke('preview_reqif_import', { path: stagedPath, configuration });
      const inspected = await inspectPreview(preview, validPreview(preview) ? 'Apply' : 'Close');
      if (!inspected || !validPreview(preview)) return;
      const applied = await invoke('apply_reqif_import', { path: stagedPath, configuration });
      if (!applied.applied) throw new Error(previewText(applied));
      if (typeof refresh === 'function') await refresh();
      notify(`ReqIF applied atomically. ${summarize(applied)}`);
    } catch (error) {
      notify(String(error), 'error');
    } finally {
      if (stagedPath && invoke) {
        try { await invoke('discard_staged_reqif', { path: stagedPath }); } catch (_) { /* cleanup only */ }
      }
    }
  }

  async function openExport() {
    try {
      if (!invoke) throw new Error('ReqIF export is available in the desktop application.');
      const project = projectState();
      if (!project) throw new Error('Create or open a project before ReqIF export.');
      const result = await window.smpDialogs?.edit({
        title: 'Export ReqIF',
        description: 'Exports Requirement/TestCase content and supported traceability in the selected Model/Package scope. Use .reqifz for a compressed exchange container.',
        fields: [
          { id: 'path', label: 'Destination path (.reqif or .reqifz)', value: `${project.name || 'requirements'}.reqif`, required: true },
        ],
        confirmLabel: 'Export',
      });
      if (!result) return;
      const output = await invoke('export_reqif', { path: result.values.path, scopeId: targetScopeId() });
      notify(`ReqIF exported to ${output}.`);
    } catch (error) {
      notify(String(error), 'error');
    }
  }

  function installRibbonGroup() {
    if (!ribbon || ribbon.querySelector('[data-reqif-interchange]')) return;
    const fileActive = [...document.querySelectorAll('.workspace-tab')]
      .some((tab) => tab.classList.contains('active') && tab.textContent.trim() === 'File');
    if (!fileActive) return;
    const group = document.createElement('section');
    group.className = 'ribbon-group';
    group.dataset.reqifInterchange = 'true';
    group.innerHTML = '<div class="ribbon-actions ribbon-large-actions"><button class="ribbon-command" data-reqif-import><span class="command-icon">⇩</span><span>Import<br>ReqIF</span></button><button class="ribbon-command" data-reqif-export><span class="command-icon">⇧</span><span>Export<br>ReqIF</span></button></div><div class="ribbon-label">Requirements Interchange</div>';
    group.querySelector('[data-reqif-import]').addEventListener('click', openImport);
    group.querySelector('[data-reqif-export]').addEventListener('click', openExport);
    ribbon.appendChild(group);
  }

  if (ribbon) new MutationObserver(installRibbonGroup).observe(ribbon, { childList: true });
  document.querySelectorAll('.workspace-tab').forEach((tab) => {
    tab.addEventListener('click', () => queueMicrotask(installRibbonGroup));
  });
  installRibbonGroup();
  window.smpReqifInterchange = Object.freeze({ openImport, openExport });
})();
