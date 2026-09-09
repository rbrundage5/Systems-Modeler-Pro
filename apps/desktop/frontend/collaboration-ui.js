(() => {
  const invoke = window.__TAURI__?.core?.invoke;
  const button = document.createElement('button');
  button.textContent = 'Shared Projects';
  button.type = 'button';
  document.querySelector('.statusbar')?.append(button);
  const dialog = document.createElement('dialog');
  dialog.className = 'collaboration-dialog';
  dialog.innerHTML = `<h2>Shared Projects</h2>
    <p>Connect to your team's server to view and edit shared model elements.</p>
    <form data-connect><label>Server address<input name="server" type="url" required placeholder="https://models.example.com" autocomplete="off"></label>
    <label>Access token<input name="token" type="password" required autocomplete="off" spellcheck="false"></label><button>Connect</button></form>
    <section data-session hidden><div class="collaboration-actions"><select data-project aria-label="Shared project"></select><button data-open>Open project</button><button data-refresh>Refresh</button><button data-disconnect>Disconnect</button></div>
    <p data-revision></p><p>Available edits: create Package, create Block, and rename. Shared diagrams and collaborative undo are not available yet.</p>
    <label>Element / owner<select data-element></select></label>
    <form data-edit><label>Operation<select name="operation"><option value="CreatePackage">Create Package</option><option value="CreateBlock">Create Block</option><option value="RenameElement">Rename element</option></select></label>
    <label>Name<input name="name" required maxlength="1024" autocomplete="off"></label><button data-submit>Save shared edit</button></form>
    <button data-retry hidden>Retry pending edit</button><div class="collaboration-elements" data-elements></div></section>
    <p data-message role="status" aria-live="polite"></p><button data-close>Close</button>`;
  document.body.append(dialog);
  const find = selector => dialog.querySelector(selector);
  const connect = find('[data-connect]');
  const edit = find('[data-edit]');
  const message = find('[data-message]');
  let view = null;
  let busy = false;
  function render() {
    find('[data-session]').hidden = !view;
    connect.hidden = !!view;
    if (!view) return;
    const projects = find('[data-project]');
    const chosen = projects.value;
    projects.replaceChildren(...view.projects.map(g => new Option(`${g.id} (${g.role})`, g.id)));
    if (view.projects.some(g => g.id === chosen)) projects.value = chosen;
    const snapshot = view.snapshot;
    find('[data-revision]').textContent = snapshot ? `${snapshot.project.name} · revision ${snapshot.revision}${view.needs_refresh ? ' · refresh required' : ''}` : 'Choose a project to open.';
    const elements = Object.values(snapshot?.project.elements || {}).sort((a, b) => a.name.localeCompare(b.name));
    const select = find('[data-element]');
    const selected = select.value;
    select.replaceChildren(...elements.map(e => new Option(`${e.name} [${e.kind}]`, e.id)));
    select.value = elements.some(e => e.id === selected) ? selected : (snapshot?.project.root_id || '');
    find('[data-elements]').replaceChildren(...elements.map(e => {
      const row = document.createElement('div');
      row.textContent = `${e.name} [${e.kind}]`;
      return row;
    }));
    const canEdit = snapshot && view.projects.some(g => g.id === snapshot.project.id && g.role === 'editor');
    find('[data-submit]').disabled = !canEdit || view.pending || view.needs_refresh;
    find('[data-retry]').hidden = !view.pending;
    find('[data-refresh]').disabled = !snapshot || view.pending;
    find('[data-open]').disabled = view.pending || !view.projects.length;
  }
  async function run(action, success) {
    if (busy) return;
    busy = true;
    dialog.setAttribute('aria-busy', 'true');
    // Disable the forms during a request; Rust also serializes session commands.
    const controls = [...dialog.querySelectorAll('input, select, button')];
    const disabled = controls.map(control => control.disabled);
    controls.forEach(control => { control.disabled = true; });
    try {
      if (!invoke) throw new Error('Shared projects require the desktop application.');
      await action();
      if (success) message.textContent = success;
    } catch (error) {
      message.textContent = String(error);
      if (view) {
        try { view = await invoke('collaboration_status'); } catch (_) { /* Keep last visible snapshot. */ }
      }
    } finally {
      controls.forEach((control, i) => { control.disabled = disabled[i]; });
      busy = false;
      dialog.removeAttribute('aria-busy');
      render();
    }
  }
  button.onclick = () => { if (!dialog.open) dialog.showModal(); };
  find('[data-close]').onclick = () => dialog.close();
  connect.onsubmit = event => {
    event.preventDefault();
    const server = connect.elements.server.value.trim();
    const token = connect.elements.token.value.trim();
    connect.elements.token.value = '';
    run(async () => { view = await invoke('collaboration_connect', { server, token }); }, 'Connected. Select a shared project.');
  };
  find('[data-open]').onclick = () => run(async () => {
    view = await invoke('collaboration_open', { project: find('[data-project]').value });
    edit.elements.name.value = '';
  }, 'Shared project loaded.');
  find('[data-refresh]').onclick = () => run(async () => {
    view = await invoke('collaboration_open', { project: view.snapshot.project.id });
  }, 'Latest shared revision loaded. Review it before saving an edit.');
  edit.onsubmit = event => {
    event.preventDefault();
    if (!view?.snapshot) return;
    const operation = edit.elements.operation.value;
    const target = find('[data-element]').value;
    const name = edit.elements.name.value;
    const payload = operation === 'RenameElement' ? { element: target, name } : { owner: target, name };
    run(async () => {
      view = await invoke('collaboration_edit', { expectedRevision: view.snapshot.revision, edit: { [operation]: payload } });
      edit.elements.name.value = '';
    }, 'Shared edit saved.');
  };
  find('[data-retry]').onclick = () => run(async () => {
    view = await invoke('collaboration_retry');
    edit.elements.name.value = '';
  }, 'Pending edit confirmed.');
  find('[data-disconnect]').onclick = () => run(async () => {
    await invoke('collaboration_disconnect');
    view = null;
    edit.elements.name.value = '';
  }, 'Disconnected.');
  // Poll only while visible and no edit is being composed. Never silently rebase
  // an unsent edit or retry an operation with a new identity.
  setInterval(() => {
    if (dialog.open && !busy && view?.snapshot && !view.pending && !view.needs_refresh && !edit.elements.name.value) {
      run(async () => { view = await invoke('collaboration_open', { project: view.snapshot.project.id }); });
    }
  }, 5000);
})();
