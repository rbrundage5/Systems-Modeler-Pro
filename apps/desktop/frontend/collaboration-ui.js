(() => {
  const invoke = window.__TAURI__?.core?.invoke;
  const NS = 'http://www.w3.org/2000/svg';
  const BDD_KINDS = new Set([
    'Block', 'AssociationBlock', 'InterfaceBlock', 'ConstraintBlock', 'ValueType', 'DataType',
    'PrimitiveType', 'Enumeration', 'Signal', 'Unit', 'QuantityKind', 'InstanceSpecification',
    'Comment', 'Requirement', 'TestCase', 'Actor', 'UseCase',
  ]);
  const SHARED_RELATIONSHIP_KINDS = [
    'Dependency', 'Generalization', 'Realization', 'Allocate', 'DeriveRequirement',
    'Satisfy', 'Verify', 'Refine', 'Trace', 'Copy', 'Include', 'Extend',
  ];
  const SHARED_RELATIONSHIP_KIND_SET = new Set(SHARED_RELATIONSHIP_KINDS);
  const RELATIONSHIP_LABELS = {
    Dependency: '«dependency»', Allocate: '«allocate»', DeriveRequirement: '«deriveReqt»',
    Satisfy: '«satisfy»', Verify: '«verify»', Refine: '«refine»', Trace: '«trace»',
    Copy: '«copy»', Include: '«include»', Extend: '«extend»',
  };
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
    <section class="collaboration-semantic"><h3>Repository edits</h3><p>Elements and relationships use the same server-authoritative project revision as shared diagram edits.</p>
    <label>Element / owner<select data-element></select></label>
    <form data-edit><label>Operation<select name="operation"><option value="CreatePackage">Create Package</option><option value="CreateBlock">Create Block</option><option value="CreateTestCase">Create Test Case</option><option value="RenameElement">Rename element</option></select></label>
    <label>Name<input name="name" required maxlength="1024" autocomplete="off"></label><button data-submit>Save shared edit</button></form>
    <section class="collaboration-requirements"><h4>Requirements</h4>
    <div class="collaboration-actions"><label>Existing requirement<select data-requirement-target></select></label><button type="button" data-requirement-load>Load requirement</button></div>
    <pre data-requirement-current aria-label="Current saved requirement"></pre>
    <form data-requirement-edit><label>Owner for new requirement<select data-requirement-owner></select></label>
    <label>Name<input name="name" required maxlength="1024" autocomplete="off"></label>
    <label>Requirement ID<input name="requirementId" required autocomplete="off"></label>
    <label>Requirement text<textarea name="text" rows="5"></textarea></label>
    <p data-requirement-context></p><button data-requirement-submit>Save requirement</button>
    <button type="button" data-requirement-reset>Clear draft / new requirement</button>
    <button type="button" data-requirement-rebase>Keep draft after reviewing latest revision</button></form></section>
    <div class="collaboration-relationships"><h4>Semantic relationships</h4><p>Create or delete the relationship kinds whose complete payload is source, target, and Model/Package owner. Specialized Association, Connector, ItemFlow, BindingConnector, and import operations remain separate.</p>
    <form data-relationship-edit><div class="collaboration-actions"><label>Kind<select name="kind">${SHARED_RELATIONSHIP_KINDS.map(kind => `<option value="${kind}">${kind}</option>`).join('')}</select></label><label>Source<select data-relationship-source></select></label><label>Target<select data-relationship-target></select></label><label>Owner<select data-relationship-owner></select></label></div><button data-relationship-create>Create relationship</button></form>
    <div class="collaboration-actions"><label>Existing relationship<select data-relationship></select></label><button type="button" data-relationship-delete>Delete relationship</button></div></div></section>
    <section class="collaboration-bdd"><h3>Shared Block Definition Diagram</h3><p>BDD nodes and simple semantic relationship presentations are revisioned together. The server reroutes every presented relationship after endpoint movement and rejects edits when no obstacle-clear orthogonal route is available.</p>
    <div class="collaboration-actions"><label>Diagram owner<select data-bdd-owner></select></label><label>Diagram name<input data-bdd-name maxlength="1024" autocomplete="off"></label><button type="button" data-bdd-create>Create BDD</button></div>
    <div class="collaboration-actions"><label>BDD<select data-bdd-diagram aria-label="Shared BDD"></select></label><button type="button" data-bdd-rename>Rename BDD</button><button type="button" data-bdd-delete>Delete BDD</button></div>
    <div class="collaboration-actions"><label>Model element<select data-bdd-element></select></label><button type="button" data-bdd-place>Place on BDD</button><button type="button" data-bdd-remove>Remove selected node</button></div>
    <div class="collaboration-actions"><label>Semantic relationship<select data-bdd-relationship></select></label><button type="button" data-bdd-edge-place>Show on BDD</button><label>Presented relationship<select data-bdd-edge></select></label><button type="button" data-bdd-edge-remove>Remove edge</button><button type="button" data-bdd-route>Route edges</button></div>
    <div class="collaboration-canvas-shell"><svg data-bdd-canvas class="collaboration-bdd-canvas" viewBox="0 0 1200 760" role="img" aria-label="Shared BDD canvas" tabindex="0"></svg></div></section>
    <button data-retry hidden>Retry pending edit</button><div class="collaboration-elements" data-elements></div></section>
    <p data-message role="status" aria-live="polite"></p><button data-close>Close</button>`;
  document.body.append(dialog);
  const find = selector => dialog.querySelector(selector);
  const connect = find('[data-connect]');
  const edit = find('[data-edit]');
  const relationshipEdit = find('[data-relationship-edit]');
  const requirementEdit = find('[data-requirement-edit]');
  const message = find('[data-message]');
  const canvas = find('[data-bdd-canvas]');
  let view = null;
  let busy = false;
  let selectedDiagramId = '';
  let selectedNodeId = '';
  let selectedEdgeId = '';
  let drag = null;
  let bddNameDirty = false;
  let requirementDraft = null;
  let requirementDirty = false;
  let requirementRetry = false;
  find('[data-bdd-name]').addEventListener('input', () => { bddNameDirty = true; });

  function resetRequirementDraft() {
    requirementDraft = null;
    requirementDirty = false;
    requirementRetry = false;
    for (const field of ['name', 'requirementId', 'text']) requirementEdit.elements[field].value = '';
  }

  function captureRequirementDraft() {
    if (!view?.snapshot) return;
    if (!requirementDraft) requirementDraft = {
      project: view.snapshot.project.id, revision: view.snapshot.revision, element: null,
    };
    requirementDirty = true;
  }
  requirementEdit.addEventListener('input', captureRequirementDraft);
  find('[data-requirement-owner]').addEventListener('change', captureRequirementDraft);

  function renderRequirements(elements) {
    const owner = find('[data-requirement-owner]');
    const priorOwner = owner.value;
    const owners = elements.filter(element => ['Model', 'Package', 'ModelLibrary'].includes(element.kind));
    owner.replaceChildren(...owners.map(element => new Option(element.name, element.id)));
    if (owners.some(element => element.id === priorOwner)) owner.value = priorOwner;
    const target = find('[data-requirement-target]');
    const priorTarget = target.value;
    const requirements = elements.filter(element => element.kind === 'Requirement');
    target.replaceChildren(...requirements.map(element => new Option(`${element.requirement_id || ''} · ${element.name}`, element.id)));
    if (requirements.some(element => element.id === priorTarget)) target.value = priorTarget;
    const current = requirements.find(element => element.id === (requirementDraft?.element || target.value));
    find('[data-requirement-current]').textContent = current
      ? `Saved at revision ${view.snapshot.revision}: ${current.name}\nID: ${current.requirement_id || ''}\n${current.requirement_text || ''}`
      : 'Select a saved requirement to inspect it, or enter a new requirement below.';
    const available = !!view?.snapshot && !view.pending && !view.needs_refresh;
    const editable = available && canEdit();
    owner.disabled = !editable || !!requirementDraft?.element;
    for (const field of ['name', 'requirementId', 'text']) requirementEdit.elements[field].disabled = !editable;
    find('[data-requirement-submit]').disabled = !editable;
    find('[data-requirement-load]').disabled = !available || !target.value || requirementDirty;
    find('[data-requirement-reset]').disabled = !!view.pending;
    const stale = requirementDraft && requirementDraft.revision !== view.snapshot?.revision;
    find('[data-requirement-rebase]').hidden = !stale;
    find('[data-requirement-rebase]').disabled = !editable || (!!requirementDraft?.element && !current);
    find('[data-requirement-context]').textContent = requirementDraft
      ? `${requirementDraft.element ? 'Editing requirement' : 'New requirement'} from revision ${requirementDraft.revision}.${stale ? ' Review the saved values above before keeping this draft on the latest revision.' : ''}`
      : 'Enter a new requirement, or load an existing requirement to edit its ID and text.';
  }

  function snapshotElements() {
    return Object.values(view?.snapshot?.project?.elements || {}).sort((a, b) => a.name.localeCompare(b.name));
  }

  function snapshotRelationships() {
    return Object.values(view?.snapshot?.project?.relationships || {})
      .filter(relationship => SHARED_RELATIONSHIP_KIND_SET.has(relationship.kind))
      .sort((a, b) => a.kind.localeCompare(b.kind) || String(a.id).localeCompare(String(b.id)));
  }

  function relationshipDescription(relationship, byId) {
    const source = byId.get(String(relationship.source_id))?.name || relationship.source_id;
    const target = byId.get(String(relationship.target_id))?.name || relationship.target_id;
    return `${source} — ${relationship.kind} → ${target}`;
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
      selectedEdgeId = '';
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

    const byId = new Map(elements.map(element => [String(element.id), element]));
    const relationshipsById = new Map(snapshotRelationships().map(relationship => [String(relationship.id), relationship]));
    const diagram = currentDiagram();
    const nodeElements = new Set((diagram?.nodes || []).map(node => String(node.element)));
    const presentedRelationships = new Set((diagram?.edges || []).map(edge => String(edge.relationship)));
    const availableRelationships = [...relationshipsById.values()].filter(relationship =>
      nodeElements.has(String(relationship.source_id))
      && nodeElements.has(String(relationship.target_id))
      && !presentedRelationships.has(String(relationship.id)));
    const bddRelationship = find('[data-bdd-relationship]');
    const priorBddRelationship = bddRelationship.value;
    bddRelationship.replaceChildren(...availableRelationships.map(relationship =>
      new Option(relationshipDescription(relationship, byId), relationship.id)));
    if (availableRelationships.some(relationship => String(relationship.id) === String(priorBddRelationship))) {
      bddRelationship.value = priorBddRelationship;
    }
    const bddEdge = find('[data-bdd-edge]');
    const priorBddEdge = selectedEdgeId || bddEdge.value;
    bddEdge.replaceChildren(...(diagram?.edges || []).map(edge => {
      const relationship = relationshipsById.get(String(edge.relationship));
      return new Option(relationship ? relationshipDescription(relationship, byId) : String(edge.relationship), edge.id);
    }));
    if ((diagram?.edges || []).some(edge => String(edge.id) === String(priorBddEdge))) {
      selectedEdgeId = String(priorBddEdge);
      bddEdge.value = priorBddEdge;
    } else {
      selectedEdgeId = '';
    }
    const nameInput = find('[data-bdd-name]');
    if (!bddNameDirty && document.activeElement !== nameInput) nameInput.value = diagram?.name || '';
    const editable = canEdit() && !view?.pending && !view?.needs_refresh;
    find('[data-bdd-create]').disabled = !editable || !ownerSelect.value;
    find('[data-bdd-rename]').disabled = !editable || !diagram;
    find('[data-bdd-delete]').disabled = !editable || !diagram;
    find('[data-bdd-place]').disabled = !editable || !diagram || !bddElement.value;
    find('[data-bdd-remove]').disabled = !editable || !diagram || !selectedNodeId;
    find('[data-bdd-edge-place]').disabled = !editable || !diagram || !bddRelationship.value;
    find('[data-bdd-edge-remove]').disabled = !editable || !diagram || !selectedEdgeId;
    find('[data-bdd-route]').disabled = !editable || !diagram || !(diagram.edges || []).length;

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
    for (const edge of diagram.edges || []) {
      for (const point of edge.points || []) {
        left = Math.min(left, point.x - 40);
        top = Math.min(top, point.y - 40);
        right = Math.max(right, point.x + 40);
        bottom = Math.max(bottom, point.y + 40);
      }
      if (edge.label_anchor) {
        left = Math.min(left, edge.label_anchor.x - 80);
        top = Math.min(top, edge.label_anchor.y - 35);
        right = Math.max(right, edge.label_anchor.x + 80);
        bottom = Math.max(bottom, edge.label_anchor.y + 20);
      }
    }
    canvas.setAttribute('viewBox', `${left} ${top} ${right - left} ${bottom - top}`);
    if ((diagram.edges || []).length) {
      const defs = document.createElementNS(NS, 'defs');
      const arrow = document.createElementNS(NS, 'marker');
      arrow.setAttribute('id', 'collaboration-bdd-arrow');
      arrow.setAttribute('viewBox', '0 0 10 10');
      arrow.setAttribute('refX', '9');
      arrow.setAttribute('refY', '5');
      arrow.setAttribute('markerWidth', '8');
      arrow.setAttribute('markerHeight', '8');
      arrow.setAttribute('orient', 'auto-start-reverse');
      const arrowPath = document.createElementNS(NS, 'path');
      arrowPath.setAttribute('d', 'M 1 1 L 9 5 L 1 9');
      arrowPath.classList.add('collaboration-bdd-arrow');
      arrow.append(arrowPath);
      const triangle = document.createElementNS(NS, 'marker');
      triangle.setAttribute('id', 'collaboration-bdd-triangle');
      triangle.setAttribute('viewBox', '0 0 12 10');
      triangle.setAttribute('refX', '11');
      triangle.setAttribute('refY', '5');
      triangle.setAttribute('markerWidth', '10');
      triangle.setAttribute('markerHeight', '10');
      triangle.setAttribute('orient', 'auto-start-reverse');
      const trianglePath = document.createElementNS(NS, 'path');
      trianglePath.setAttribute('d', 'M 1 1 L 11 5 L 1 9 Z');
      trianglePath.classList.add('collaboration-bdd-triangle');
      triangle.append(trianglePath);
      defs.append(arrow, triangle);
      canvas.append(defs);
    }
    for (const edge of diagram.edges || []) {
      const relationship = relationshipsById.get(String(edge.relationship));
      const group = document.createElementNS(NS, 'g');
      group.classList.add('collaboration-bdd-edge');
      if (String(edge.id) === String(selectedEdgeId)) group.classList.add('selected');
      group.dataset.edgeId = edge.id;
      const line = document.createElementNS(NS, 'polyline');
      line.setAttribute('points', (edge.points || []).map(point => `${point.x},${point.y}`).join(' '));
      line.classList.add('collaboration-bdd-edge-line');
      if (relationship?.kind !== 'Generalization') line.classList.add('dashed');
      line.setAttribute('marker-end', relationship?.kind === 'Generalization' || relationship?.kind === 'Realization'
        ? 'url(#collaboration-bdd-triangle)'
        : 'url(#collaboration-bdd-arrow)');
      group.append(line);
      const labelValue = RELATIONSHIP_LABELS[relationship?.kind];
      if (labelValue && edge.label_anchor) {
        const label = document.createElementNS(NS, 'text');
        label.setAttribute('x', edge.label_anchor.x);
        label.setAttribute('y', edge.label_anchor.y);
        label.classList.add('collaboration-bdd-edge-label');
        label.textContent = labelValue;
        group.append(label);
      }
      group.addEventListener('pointerdown', event => {
        selectedEdgeId = String(edge.id);
        selectedNodeId = '';
        bddEdge.value = selectedEdgeId;
        renderBdd();
        event.preventDefault();
      });
      canvas.append(group);
    }
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
        selectedEdgeId = '';
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
    const byId = new Map(elements.map(element => [String(element.id), element]));
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
    const setElementChoices = (selector, choices = elements) => {
      const control = find(selector);
      const previous = control.value;
      control.replaceChildren(...choices.map(element => new Option(`${element.name} [${element.kind}]`, element.id)));
      if (choices.some(element => String(element.id) === String(previous))) control.value = previous;
      return control;
    };
    const source = setElementChoices('[data-relationship-source]');
    const target = setElementChoices('[data-relationship-target]');
    const owners = elements.filter(element => element.kind === 'Model' || element.kind === 'Package');
    const owner = setElementChoices('[data-relationship-owner]', owners);
    const relationshipSelect = find('[data-relationship]');
    const priorRelationship = relationshipSelect.value;
    const relationships = snapshotRelationships();
    relationshipSelect.replaceChildren(...relationships.map(relationship => {
      return new Option(relationshipDescription(relationship, byId), relationship.id);
    }));
    if (relationships.some(relationship => String(relationship.id) === String(priorRelationship))) relationshipSelect.value = priorRelationship;
    const relationshipEditable = editable && !view.pending && !view.needs_refresh;
    find('[data-relationship-create]').disabled = !relationshipEditable || !source.value || !target.value || !owner.value;
    find('[data-relationship-delete]').disabled = !relationshipEditable || !relationshipSelect.value;
    find('[data-retry]').hidden = !view.pending;
    find('[data-refresh]').disabled = !snapshot || view.pending;
    find('[data-open]').disabled = view.pending || !view.projects.length;
    renderRequirements(elements);
    renderBdd();
  }

  async function run(action, success) {
    if (busy) return false;
    busy = true;
    dialog.setAttribute('aria-busy', 'true');
    const controls = [...dialog.querySelectorAll('input, select, textarea, button')];
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
    if (requirementDirty) throw new Error('Save or clear the requirement draft before opening a project. Use Refresh to inspect the current project.');
    view = await invoke('collaboration_open', { project: find('[data-project]').value });
    resetRequirementDraft();
    edit.elements.name.value = '';
    bddNameDirty = false;
    selectedDiagramId = String(view.snapshot?.diagrams?.[0]?.id || '');
    selectedNodeId = '';
    selectedEdgeId = '';
  }, 'Shared project loaded.');
  find('[data-refresh]').onclick = () => run(async () => {
    view = await invoke('collaboration_open', { project: view.snapshot.project.id });
  }, 'Latest shared revision loaded. Review it before saving an edit.');
  find('[data-requirement-target]').onchange = () => renderRequirements(snapshotElements());
  find('[data-requirement-load]').onclick = () => {
    if (busy || requirementDirty || view?.pending || view?.needs_refresh) return;
    const element = snapshotElements().find(item => item.id === find('[data-requirement-target]').value && item.kind === 'Requirement');
    if (!element) return;
    requirementDraft = { project: view.snapshot.project.id, revision: view.snapshot.revision, element: element.id };
    requirementEdit.elements.name.value = element.name;
    requirementEdit.elements.requirementId.value = element.requirement_id || '';
    requirementEdit.elements.text.value = element.requirement_text || '';
    renderRequirements(snapshotElements());
  };
  find('[data-requirement-reset]').onclick = () => {
    if (busy || view?.pending) return;
    resetRequirementDraft();
    renderRequirements(snapshotElements());
  };
  find('[data-requirement-rebase]').onclick = () => {
    if (busy || !requirementDraft || !canEdit() || view.pending || view.needs_refresh) return;
    if (requirementDraft.project !== view.snapshot.project.id) return;
    requirementDraft.revision = view.snapshot.revision;
    renderRequirements(snapshotElements());
  };
  requirementEdit.onsubmit = async event => {
    event.preventDefault();
    if (busy || !canEdit() || view.pending || view.needs_refresh) return;
    captureRequirementDraft();
    if (!requirementDraft || requirementDraft.project !== view.snapshot.project.id) return;
    const payload = {
      name: requirementEdit.elements.name.value,
      requirement_id: requirementEdit.elements.requirementId.value,
      text: requirementEdit.elements.text.value,
    };
    const operation = requirementDraft.element ? 'UpdateRequirement' : 'CreateRequirement';
    if (requirementDraft.element) payload.element = requirementDraft.element;
    else payload.owner = find('[data-requirement-owner]').value;
    const saved = await submitSharedEdit({ [operation]: payload }, 'Shared requirement saved.', requirementDraft.revision);
    requirementRetry = !saved && !!view?.pending;
    if (saved) resetRequirementDraft();
    renderRequirements(snapshotElements());
  };
  edit.onsubmit = event => {
    event.preventDefault();
    if (!view?.snapshot) return;
    const operation = edit.elements.operation.value;
    const target = find('[data-element]').value;
    const name = edit.elements.name.value;
    const payload = operation === 'RenameElement' ? { element: target, name } : { owner: target, name };
    submitSharedEdit({ [operation]: payload }, 'Shared repository edit saved.').then(saved => { if (saved) edit.elements.name.value = ''; });
  };
  relationshipEdit.onsubmit = event => {
    event.preventDefault();
    if (!view?.snapshot) return;
    submitSharedEdit({
      CreateRelationship: {
        kind: relationshipEdit.elements.kind.value,
        source: find('[data-relationship-source]').value,
        target: find('[data-relationship-target]').value,
        owner: find('[data-relationship-owner]').value,
      },
    }, 'Shared semantic relationship created.');
  };
  find('[data-relationship-delete]').onclick = () => {
    const relationship = find('[data-relationship]').value;
    if (!relationship) return;
    submitSharedEdit({ DeleteRelationship: { relationship } }, 'Shared semantic relationship deleted.');
  };

  find('[data-bdd-diagram]').onchange = event => {
    bddNameDirty = false;
    selectedDiagramId = event.target.value;
    selectedNodeId = '';
    selectedEdgeId = '';
    renderBdd();
  };
  find('[data-bdd-edge]').onchange = event => {
    selectedEdgeId = event.target.value;
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
      selectedEdgeId = '';
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
      selectedEdgeId = '';
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
  find('[data-bdd-edge-place]').onclick = () => {
    const relationship = find('[data-bdd-relationship]').value;
    if (!selectedDiagramId || !relationship || !view?.snapshot) return;
    const before = new Set((currentDiagram()?.edges || []).map(edge => String(edge.id)));
    const expectedRevision = view.snapshot.revision;
    run(async () => {
      view = await invoke('collaboration_edit', {
        expectedRevision,
        edit: { PresentBddRelationship: { diagram: selectedDiagramId, edge: crypto.randomUUID(), relationship } },
      });
      const created = (currentDiagram()?.edges || []).find(edge => !before.has(String(edge.id)));
      selectedEdgeId = created ? String(created.id) : '';
      selectedNodeId = '';
    }, 'Relationship shown and routed on shared BDD.');
  };
  find('[data-bdd-edge-remove]').onclick = () => {
    if (!selectedDiagramId || !selectedEdgeId) return;
    const edge = selectedEdgeId;
    submitSharedEdit({ RemoveBddEdge: { diagram: selectedDiagramId, edge } }, 'Relationship presentation removed from shared BDD.').then(saved => {
      if (saved) { selectedEdgeId = ''; renderBdd(); }
    });
  };
  find('[data-bdd-route]').onclick = () => {
    if (!selectedDiagramId) return;
    submitSharedEdit({ RouteBddDiagram: { diagram: selectedDiagramId } }, 'Shared BDD relationships routed.');
  };
  find('[data-retry]').onclick = () => run(async () => {
    view = await invoke('collaboration_retry');
    edit.elements.name.value = '';
    if (requirementRetry) resetRequirementDraft();
  }, 'Pending edit confirmed.');
  find('[data-disconnect]').onclick = () => run(async () => {
    if (requirementDirty) throw new Error('Save or clear the requirement draft before disconnecting.');
    await invoke('collaboration_disconnect');
    resetRequirementDraft();
    view = null;
    selectedDiagramId = '';
    selectedNodeId = '';
    selectedEdgeId = '';
    edit.elements.name.value = '';
  }, 'Disconnected.');

  // Poll only while visible and no edit or pointer gesture is being composed.
  // Never silently rebase an unsent edit or retry an operation with a new identity.
  setInterval(() => {
    const bddNameFocused = document.activeElement === find('[data-bdd-name]');
    if (dialog.open && !busy && !drag && !bddNameDirty && !bddNameFocused && !requirementDirty && view?.snapshot && !view.pending && !view.needs_refresh && !edit.elements.name.value) {
      run(async () => { view = await invoke('collaboration_open', { project: view.snapshot.project.id }); });
    }
  }, 5000);
})();
