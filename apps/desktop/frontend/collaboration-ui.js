(() => {
  const invoke = window.__TAURI__?.core?.invoke;
  const NS = 'http://www.w3.org/2000/svg';
  const BDD_KINDS = new Set([
    'Block', 'AssociationBlock', 'InterfaceBlock', 'ConstraintBlock', 'ValueType', 'DataType',
    'PrimitiveType', 'Enumeration', 'Signal', 'Unit', 'QuantityKind', 'InstanceSpecification',
    'Comment', 'Requirement', 'TestCase', 'Actor', 'UseCase',
  ]);
  const button = document.createElement('button');
  button.textContent = 'Shared Projects';
  button.type = 'button';
  document.querySelector('.statusbar')?.append(button);
  const dialog = document.createElement('dialog');
  dialog.className = 'collaboration-dialog';
  dialog.innerHTML = `<h2>Shared Projects</h2>
    <p>Connect to your team's server to view and edit server-authoritative shared model content.</p>
    <form data-connect><label>Server address<input name="server" type="url" required placeholder="https://models.example.com" autocomplete="off"></label>
    <label>Access token<input name="token" type="password" required autocomplete="off" spellcheck="false"></label><button>Connect</button></form>
    <section data-session hidden><div class="collaboration-actions"><select data-project aria-label="Shared project"></select><button data-open>Open project</button><button data-refresh>Refresh</button><button data-disconnect>Disconnect</button></div>
    <p data-revision></p>
    <section class="collaboration-semantic"><h3>Repository edits</h3><p>Create Package, create Block, and rename use the same project revision as shared diagram edits.</p>
    <label>Element / owner<select data-element></select></label>
    <form data-edit><label>Operation<select name="operation"><option value="CreatePackage">Create Package</option><option value="CreateBlock">Create Block</option><option value="RenameElement">Rename element</option></select></label>
    <label>Name<input name="name" required maxlength="1024" autocomplete="off"></label><button data-submit>Save shared edit</button></form></section>
    <section class="collaboration-bdd"><h3>Shared Block Definition Diagram</h3><p>This initial diagram-collaboration slice supports shared BDD creation, node placement, movement, rename, removal, and deletion. Relationships/routes and other diagram families remain outside this increment.</p>
    <div class="collaboration-actions"><label>Diagram owner<select data-bdd-owner></select></label><label>Diagram name<input data-bdd-name maxlength="1024" autocomplete="off"></label><button type="button" data-bdd-create>Create BDD</button></div>
    <div class="collaboration-actions"><label>BDD<select data-bdd-diagram aria-label="Shared BDD"></select></label><button type="button" data-bdd-rename>Rename BDD</button><button type="button" data-bdd-delete>Delete BDD</button></div>
    <div class="collaboration-actions"><label>Model element<select data-bdd-element></select></label><button type="button" data-bdd-place>Place on BDD</button><button type="button" data-bdd-remove>Remove selected node</button></div>
    <div class="collaboration-canvas-shell"><svg data-bdd-canvas class="collaboration-bdd-canvas" viewBox="0 0 1200 760" role="img" aria-label="Shared BDD canvas" tabindex="0"></svg></div></section>
    <button data-retry hidden>Retry pending edit</button><div class="collaboration-elements" data-elements></div></section>
    <p data-message role="status" aria-live="polite"></p><button data-close>Close</button>`;
  document.body.append(dialog);
  const find = selector => dialog.querySelector(selector);
  const connect = find('[data-connect]');
  const edit = find('[data-edit]');
  const message = find('[data-message]');
  const canvas = find('[data-bdd-canvas]');
  let view = null;
  let busy = false;
  let selectedDiagramId = '';
  let selectedNodeId = '';
  let drag = null;
  let bddNameDirty = false;
  find('[data-bdd-name]').addEventListener('input', () => { bddNameDirty = true; });

  function snapshotElements() {
    return Object.values(view?.snapshot?.project?.elements || {}).sort((a, b) => a.name.localeCompare(b.name));
  }

  function diagrams() {
    return view?.snapshot?.diagrams || [];
  }

  function currentDiagram() {
    return diagrams().find(item => String(item.id) === String(selectedDiagramId)) || null;
  }

  function canEdit() {
    const snapshot = view?.snapshot;
    return !!snapshot && view.projects.some(grant => grant.id === snapshot.project.id && grant.role === 'editor');
  }

  function svgPoint(event) {
    const point = canvas.createSVGPoint();
    point.x = event.clientX;
    point.y = event.clientY;
    const matrix = canvas.getScreenCTM();
    return matrix ? point.matrixTransform(matrix.inverse()) : point;
  }

  function renderBdd() {
    const allDiagrams = diagrams();
    if (!allDiagrams.some(item => String(item.id) === String(selectedDiagramId))) {
      selectedDiagramId = allDiagrams[0] ? String(allDiagrams[0].id) : '';
      selectedNodeId = '';
    }
    const diagramSelect = find('[data-bdd-diagram]');
    diagramSelect.replaceChildren(...allDiagrams.map(item => new Option(item.name, item.id)));
    if (selectedDiagramId) diagramSelect.value = selectedDiagramId;

    const elements = snapshotElements();
    const ownerSelect = find('[data-bdd-owner]');
    const priorOwner = ownerSelect.value;
    const owners = elements.filter(element => element.kind === 'Model' || element.kind === 'Package');
    ownerSelect.replaceChildren(...owners.map(element => new Option(`${element.name} [${element.kind}]`, element.id)));
    if (owners.some(element => element.id === priorOwner)) ownerSelect.value = priorOwner;

    const bddElement = find('[data-bdd-element]');
    const priorElement = bddElement.value;
    const presentable = elements.filter(element => BDD_KINDS.has(element.kind));
    bddElement.replaceChildren(...presentable.map(element => new Option(`${element.name} [${element.kind}]`, element.id)));
    if (presentable.some(element => element.id === priorElement)) bddElement.value = priorElement;

    const diagram = currentDiagram();
    const nameInput = find('[data-bdd-name]');
    if (!bddNameDirty && document.activeElement !== nameInput) nameInput.value = diagram?.name || '';
    const editable = canEdit() && !view?.pending && !view?.needs_refresh;
    find('[data-bdd-create]').disabled = !editable || !ownerSelect.value;
    find('[data-bdd-rename]').disabled = !editable || !diagram;
    find('[data-bdd-delete]').disabled = !editable || !diagram;
    find('[data-bdd-place]').disabled = !editable || !diagram || !bddElement.value;
    find('[data-bdd-remove]').disabled = !editable || !diagram || !selectedNodeId;

    canvas.replaceChildren();
    if (!diagram) {
      const empty = document.createElementNS(NS, 'text');
      empty.setAttribute('x', '40');
      empty.setAttribute('y', '70');
      empty.textContent = 'Create or select a shared BDD.';
      canvas.append(empty);
      return;
    }
    let right = 1200;
    let bottom = 760;
    let left = 0;
    let top = 0;
    for (const node of diagram.nodes || []) {
      left = Math.min(left, node.x - 40);
      top = Math.min(top, node.y - 40);
      right = Math.max(right, node.x + node.width + 40);
      bottom = Math.max(bottom, node.y + node.height + 40);
    }
    canvas.setAttribute('viewBox', `${left} ${top} ${right - left} ${bottom - top}`);
    const byId = new Map(elements.map(element => [String(element.id), element]));
    for (const node of diagram.nodes || []) {
      const semantic = byId.get(String(node.element));
      const group = document.createElementNS(NS, 'g');
      group.classList.add('collaboration-bdd-node');
      if (String(node.id) === String(selectedNodeId)) group.classList.add('selected');
      group.dataset.nodeId = node.id;
      group.setAttribute('transform', `translate(${node.x} ${node.y})`);
      const rect = document.createElementNS(NS, 'rect');
      rect.setAttribute('width', node.width);
      rect.setAttribute('height', node.height);
      const stereotype = document.createElementNS(NS, 'text');
      stereotype.setAttribute('x', '10');
      stereotype.setAttribute('y', '22');
      stereotype.classList.add('collaboration-bdd-kind');
      stereotype.textContent = `«${semantic?.kind || 'Element'}»`;
      const label = document.createElementNS(NS, 'text');
      label.setAttribute('x', '10');
      label.setAttribute('y', '47');
      label.textContent = semantic?.name || String(node.element);
      group.append(rect, stereotype, label);
      group.addEventListener('pointerdown', event => {
        if (busy) return;
        selectedNodeId = String(node.id);
        canvas.querySelectorAll('.collaboration-bdd-node').forEach(item => item.classList.toggle('selected', item === group));
        find('[data-bdd-remove]').disabled = !canEdit() || !!view?.pending || !!view?.needs_refresh;
        if (!canEdit() || view?.pending || view?.needs_refresh || event.button !== 0) return;
        const start = svgPoint(event);
        drag = {
          pointerId: event.pointerId,
          expectedRevision: view.snapshot.revision,
          diagramId: String(diagram.id),
          nodeId: String(node.id),
          group,
          start,
          node: { ...node },
          x: node.x,
          y: node.y,
        };
        canvas.setPointerCapture(event.pointerId);
        event.preventDefault();
      });
      canvas.append(group);
    }
  }

  function render() {
    find('[data-session]').hidden = !view;
    connect.hidden = !!view;
    if (!view) return;
    const projects = find('[data-project]');
    const chosen = projects.value;
    projects.replaceChildren(...view.projects.map(grant => new Option(`${grant.id} (${grant.role})`, grant.id)));
    if (view.projects.some(grant => grant.id === chosen)) projects.value = chosen;
    const snapshot = view.snapshot;
    find('[data-revision]').textContent = snapshot ? `${snapshot.project.name} · revision ${snapshot.revision}${view.needs_refresh ? ' · refresh required' : ''}` : 'Choose a project to open.';
    const elements = snapshotElements();
    const select = find('[data-element]');
    const selected = select.value;
    select.replaceChildren(...elements.map(element => new Option(`${element.name} [${element.kind}]`, element.id)));
    select.value = elements.some(element => element.id === selected) ? selected : (snapshot?.project.root_id || '');
    find('[data-elements]').replaceChildren(...elements.map(element => {
      const row = document.createElement('div');
      row.textContent = `${element.name} [${element.kind}]`;
      return row;
    }));
    const editable = canEdit();
    find('[data-submit]').disabled = !editable || view.pending || view.needs_refresh;
    find('[data-retry]').hidden = !view.pending;
    find('[data-refresh]').disabled = !snapshot || view.pending;
    find('[data-open]').disabled = view.pending || !view.projects.length;
    renderBdd();
  }

  async function run(action, success) {
    if (busy) return false;
    busy = true;
    dialog.setAttribute('aria-busy', 'true');
    const controls = [...dialog.querySelectorAll('input, select, button')];
    const disabled = controls.map(control => control.disabled);
    controls.forEach(control => { control.disabled = true; });
    try {
      if (!invoke) throw new Error('Shared projects require the desktop application.');
      await action();
      if (success) message.textContent = success;
      return true;
    } catch (error) {
      message.textContent = String(error);
      if (view) {
        try { view = await invoke('collaboration_status'); } catch (_) { /* Keep last visible snapshot. */ }
      }
      return false;
    } finally {
      controls.forEach((control, index) => { control.disabled = disabled[index]; });
      busy = false;
      dialog.removeAttribute('aria-busy');
      render();
    }
  }

  async function submitSharedEdit(editPayload, success, expectedRevision = view?.snapshot?.revision) {
    if (!view?.snapshot) return false;
    return run(async () => {
      view = await invoke('collaboration_edit', { expectedRevision, edit: editPayload });
    }, success);
  }

  canvas.addEventListener('pointermove', event => {
    if (!drag || drag.pointerId !== event.pointerId) return;
    const point = svgPoint(event);
    drag.x = Math.max(0, drag.node.x + point.x - drag.start.x);
    drag.y = Math.max(42, drag.node.y + point.y - drag.start.y);
    drag.group.setAttribute('transform', `translate(${drag.x} ${drag.y})`);
  });

  canvas.addEventListener('pointerup', event => {
    if (!drag || drag.pointerId !== event.pointerId) return;
    const completed = drag;
    drag = null;
    try { canvas.releasePointerCapture(event.pointerId); } catch (_) { /* Pointer may already be released. */ }
    if (completed.x === completed.node.x && completed.y === completed.node.y) return;
    submitSharedEdit({
      UpdateBddNodeGeometry: {
        diagram: completed.diagramId,
        node: completed.nodeId,
        x: completed.x,
        y: completed.y,
        width: completed.node.width,
        height: completed.node.height,
      },
    }, 'Shared BDD node moved.', completed.expectedRevision);
  });

  canvas.addEventListener('pointercancel', () => {
    drag = null;
    renderBdd();
  });

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
    bddNameDirty = false;
    selectedDiagramId = String(view.snapshot?.diagrams?.[0]?.id || '');
    selectedNodeId = '';
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
    submitSharedEdit({ [operation]: payload }, 'Shared repository edit saved.').then(saved => { if (saved) edit.elements.name.value = ''; });
  };

  find('[data-bdd-diagram]').onchange = event => {
    bddNameDirty = false;
    selectedDiagramId = event.target.value;
    selectedNodeId = '';
    renderBdd();
  };
  find('[data-bdd-create]').onclick = () => {
    const owner = find('[data-bdd-owner]').value;
    const name = find('[data-bdd-name]').value.trim();
    if (!owner || !name || !view?.snapshot) return;
    const before = new Set(diagrams().map(item => String(item.id)));
    const expectedRevision = view.snapshot.revision;
    run(async () => {
      view = await invoke('collaboration_edit', {
        expectedRevision,
        edit: { CreateBddDiagram: { diagram: crypto.randomUUID(), owner, name } },
      });
      const created = diagrams().find(item => !before.has(String(item.id)));
      if (created) selectedDiagramId = String(created.id);
      bddNameDirty = false;
      selectedNodeId = '';
    }, 'Shared BDD created.');
  };
  find('[data-bdd-rename]').onclick = () => {
    const name = find('[data-bdd-name]').value.trim();
    if (!selectedDiagramId || !name) return;
    submitSharedEdit({ RenameBddDiagram: { diagram: selectedDiagramId, name } }, 'Shared BDD renamed.').then(saved => {
      if (saved) { bddNameDirty = false; renderBdd(); }
    });
  };
  find('[data-bdd-delete]').onclick = () => {
    if (!selectedDiagramId) return;
    const deleting = selectedDiagramId;
    submitSharedEdit({ DeleteBddDiagram: { diagram: deleting } }, 'Shared BDD deleted.').then(saved => {
      if (!saved) return;
      if (selectedDiagramId === deleting) selectedDiagramId = '';
      selectedNodeId = '';
    });
  };
  find('[data-bdd-place]').onclick = () => {
    const element = find('[data-bdd-element]').value;
    if (!selectedDiagramId || !element) return;
    submitSharedEdit({ PlaceBddElement: { diagram: selectedDiagramId, node: crypto.randomUUID(), element } }, 'Element placed on shared BDD.');
  };
  find('[data-bdd-remove]').onclick = () => {
    if (!selectedDiagramId || !selectedNodeId) return;
    const node = selectedNodeId;
    submitSharedEdit({ RemoveBddNode: { diagram: selectedDiagramId, node } }, 'Shared BDD node removed.').then(saved => { if (saved) { selectedNodeId = ''; renderBdd(); } });
  };
  find('[data-retry]').onclick = () => run(async () => {
    view = await invoke('collaboration_retry');
    edit.elements.name.value = '';
  }, 'Pending edit confirmed.');
  find('[data-disconnect]').onclick = () => run(async () => {
    await invoke('collaboration_disconnect');
    view = null;
    selectedDiagramId = '';
    selectedNodeId = '';
    edit.elements.name.value = '';
  }, 'Disconnected.');

  // Poll only while visible and no edit or pointer gesture is being composed.
  // Never silently rebase an unsent edit or retry an operation with a new identity.
  setInterval(() => {
    const bddNameFocused = document.activeElement === find('[data-bdd-name]');
    if (dialog.open && !busy && !drag && !bddNameDirty && !bddNameFocused && view?.snapshot && !view.pending && !view.needs_refresh && !edit.elements.name.value) {
      run(async () => { view = await invoke('collaboration_open', { project: view.snapshot.project.id }); });
    }
  }, 5000);
})();
