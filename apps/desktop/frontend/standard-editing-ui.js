(() => {
  'use strict';

  const STANDARD_COMMANDS = Object.freeze({
    copy: 'copy_selection',
    paste: 'paste_selection',
    duplicate: 'duplicate_selection',
    delete: 'delete_active_selection',
  });
  const selectionState = { diagramId: null, items: [] };
  const css = document.createElement('style');
  css.textContent = `
    .smp-standard-selected { outline: 2px solid #2f6fad !important; outline-offset: 2px; }
    svg .smp-standard-selected { stroke-width: 3 !important; }
    .smp-standard-context-menu { position: fixed; z-index: 100000; min-width: 190px; padding: 5px; border: 1px solid #9aa5af; border-radius: 4px; background: #fff; box-shadow: 0 8px 24px rgba(0,0,0,.18); display: none; }
    .smp-standard-context-menu.open { display: block; }
    .smp-standard-context-menu button { width: 100%; padding: 7px 10px; border: 0; background: transparent; text-align: left; cursor: pointer; }
    .smp-standard-context-menu button:hover { background: #edf3f8; }
    .smp-standard-context-menu .danger { color: #8f1d1d; }
    .standard-editing-properties { margin-top: 12px; padding-top: 10px; border-top: 1px solid #d4d9dd; }
    .standard-editing-actions { display: grid; grid-template-columns: 1fr 1fr; gap: 6px; margin-top: 7px; }
    .standard-editing-actions button { min-height: 30px; }
    .smp-presentation-copy { opacity: .96; }
  `;
  document.head.appendChild(css);

  const menu = document.createElement('div');
  menu.className = 'smp-standard-context-menu';
  menu.setAttribute('role', 'menu');
  menu.innerHTML = `
    <button type="button" data-standard-command="copy">Copy</button>
    <button type="button" data-standard-command="paste">Paste</button>
    <button type="button" data-standard-command="duplicate">Duplicate</button>
    <button type="button" data-standard-command="delete">Remove from Diagram</button>
    <button type="button" class="danger" data-standard-command="delete-model">Delete from Model…</button>`;
  document.body.appendChild(menu);

  function hostContext() {
    return window.smpRendererHost?.context?.() || null;
  }

  function activeDiagramId() {
    return hostContext()?.diagramId
      || state.selectedBehaviorDiagramId
      || state.selectedActivityDiagramId
      || state.selectedDiagramId
      || null;
  }

  function activeFamilyId() {
    return hostContext()?.familyId || null;
  }

  function projectElement(id) {
    return state.snapshot?.project?.elements?.find((item) => String(item.id) === String(id)) || null;
  }

  function projectRelationship(id) {
    return state.snapshot?.project?.relationships?.find((item) => String(item.id) === String(id)) || null;
  }

  function resetForDiagram() {
    const diagramId = activeDiagramId();
    if (selectionState.diagramId === diagramId) return;
    selectionState.diagramId = diagramId;
    selectionState.items = [];
  }

  function normalizedSelection(kind, id) {
    return { kind: String(kind || 'Presentation'), id: String(id) };
  }

  function selectionKey(item) {
    return `${item.kind}:${item.id}`;
  }

  function setSelections(items) {
    resetForDiagram();
    const unique = new Map();
    for (const item of items || []) {
      if (!item?.id) continue;
      const normalized = normalizedSelection(item.kind, item.id);
      unique.set(selectionKey(normalized), normalized);
    }
    selectionState.items = [...unique.values()];
    window.smpStandardSelections = selectionState.items.map((item) => ({ ...item }));
    applySelectionClasses();
    window.smpRendererHost?.publishInteraction?.().catch?.(() => {});
  }

  function selections() {
    resetForDiagram();
    if (selectionState.items.length) return selectionState.items.map((item) => ({ ...item }));
    const family = activeFamilyId();
    if (family === 'state-machine' || family === 'sequence') {
      const item = state.selectedBehaviorItem;
      return item?.id ? [normalizedSelection(item.type || 'BehaviorItem', item.id)] : [];
    }
    if (family === 'activity') {
      if (state.selectedActivityEdgeId) return [normalizedSelection('ActivityEdge', state.selectedActivityEdgeId)];
      if (state.selectedActivityNodeId) return [normalizedSelection('ActivityNode', state.selectedActivityNodeId)];
      return [];
    }
    if (state.selectedRelationshipId) return [normalizedSelection('Relationship', state.selectedRelationshipId)];
    if (state.selectedElementId) return [normalizedSelection('Element', state.selectedElementId)];
    return [];
  }

  function toggleSelection(item, additive) {
    resetForDiagram();
    const normalized = normalizedSelection(item.kind, item.id);
    if (!additive) {
      setSelections([normalized]);
      return;
    }
    const key = selectionKey(normalized);
    const next = new Map(selectionState.items.map((value) => [selectionKey(value), value]));
    if (next.has(key)) next.delete(key);
    else next.set(key, normalized);
    setSelections([...next.values()]);
  }

  function presentationFromTarget(target) {
    const node = target?.closest?.('[data-smp-presentation-id]');
    if (!node || !document.getElementById('canvas')?.contains(node)) return null;
    return normalizedSelection(node.dataset.smpPresentationKind, node.dataset.smpPresentationId);
  }

  function applySelectionClasses() {
    const wanted = new Set(selections().map(selectionKey));
    document.querySelectorAll('#canvas [data-smp-presentation-id]').forEach((node) => {
      const item = normalizedSelection(node.dataset.smpPresentationKind, node.dataset.smpPresentationId);
      node.classList.toggle('smp-standard-selected', wanted.has(selectionKey(item)));
    });
  }

  function bddDiagram() {
    const id = activeDiagramId();
    return state.snapshot?.diagrams?.find((item) => String(item.id) === String(id)) || null;
  }

  function ibdDiagram() {
    const id = activeDiagramId();
    return state.snapshot?.ibd_diagrams?.find((item) => String(item.id) === String(id)) || null;
  }

  function activityDiagram() {
    const id = activeDiagramId();
    return state.activitySnapshot?.diagrams?.find((item) => String(item.id) === String(id)) || null;
  }

  function behaviorDiagram() {
    const id = activeDiagramId();
    return state.behaviorSnapshot?.diagrams?.find((item) => String(item.id) === String(id)) || null;
  }

  function mark(node, kind, id) {
    if (!node || !id) return;
    node.dataset.smpPresentationKind = kind;
    node.dataset.smpPresentationId = String(id);
  }

  function decorateStructural() {
    const diagram = bddDiagram();
    if (!diagram) return;
    [...document.querySelectorAll('#canvas .bdd-block')].forEach((node, index) => {
      const presentation = diagram.nodes?.[index];
      if (presentation) mark(node, 'BddNode', presentation.id);
    });
    [...document.querySelectorAll('#canvas .bdd-relationship')].forEach((node, index) => {
      const presentation = diagram.edges?.[index];
      if (presentation) mark(node, 'BddEdge', presentation.id);
    });
  }

  function decorateIbd() {
    const diagram = ibdDiagram();
    if (!diagram) return;
    [...document.querySelectorAll('#canvas .ibd-property')].forEach((node, index) => {
      const presentation = diagram.properties?.[index];
      if (presentation) mark(node, 'IbdProperty', presentation.id);
    });
    const ports = [
      ...(diagram.properties || []).flatMap((property) => property.ports || []),
      ...(diagram.boundary_ports || []),
    ];
    [...document.querySelectorAll('#canvas .ibd-port')].forEach((node, index) => {
      const presentation = ports[index];
      if (presentation) mark(node, 'IbdPort', presentation.id);
    });
    [...document.querySelectorAll('#canvas .ibd-connector')].forEach((node, index) => {
      const presentation = diagram.connectors?.[index];
      if (presentation) mark(node, 'IbdConnector', presentation.id);
    });
  }

  function decorateActivity() {
    const diagram = activityDiagram();
    if (!diagram) return;
    document.querySelectorAll('#canvas .activity-node').forEach((node) => {
      const semantic = node.dataset.activityNodeId;
      const presentation = diagram.nodes?.find((item) => String(item.activity_node_id) === String(semantic));
      if (presentation) mark(node, 'ActivityNode', presentation.id);
    });
    document.querySelectorAll('#canvas [data-activity-edge-id], #canvas .activity-edge').forEach((node) => {
      const semantic = node.dataset.activityEdgeId;
      const presentation = semantic
        ? diagram.edges?.find((item) => String(item.activity_edge_id) === String(semantic))
        : null;
      if (presentation) mark(node, 'ActivityEdge', presentation.id);
    });
  }

  const behaviorSelector = Object.freeze({
    Vertex: '[data-vertex-id]',
    Transition: '[data-transition-id]',
    Lifeline: '[data-lifeline-id]',
    Message: '[data-message-id]',
    Execution: '[data-execution-id]',
    Fragment: '[data-fragment-id]',
    Invariant: '[data-invariant-id]',
  });

  function behaviorSemanticId(node) {
    return node.dataset.vertexId
      || node.dataset.transitionId
      || node.dataset.lifelineId
      || node.dataset.messageId
      || node.dataset.executionId
      || node.dataset.fragmentId
      || node.dataset.invariantId
      || null;
  }

  function behaviorKind(node) {
    if (node.dataset.vertexId) return 'Vertex';
    if (node.dataset.transitionId) return 'Transition';
    if (node.dataset.lifelineId) return 'Lifeline';
    if (node.dataset.messageId) return 'Message';
    if (node.dataset.executionId) return 'Execution';
    if (node.dataset.fragmentId) return 'Fragment';
    if (node.dataset.invariantId) return 'Invariant';
    return 'BehaviorItem';
  }

  function behaviorNode(kind, semanticId) {
    const selector = behaviorSelector[kind];
    if (!selector) return null;
    return [...document.querySelectorAll(`#canvas ${selector}`)]
      .find((node) => String(behaviorSemanticId(node)) === String(semanticId)) || null;
  }

  function applyBehaviorPresentationState() {
    const diagram = behaviorDiagram();
    if (!diagram) return;
    const hidden = new Set((diagram.hidden_semantic_ids || []).map(String));
    for (const selector of Object.values(behaviorSelector)) {
      document.querySelectorAll(`#canvas ${selector}`).forEach((node) => {
        const semanticId = behaviorSemanticId(node);
        if (!semanticId) return;
        const kind = behaviorKind(node);
        mark(node, kind, semanticId);
        if (hidden.has(String(semanticId))) node.style.display = 'none';
      });
    }

    document.querySelectorAll('#canvas .smp-presentation-copy').forEach((node) => node.remove());
    for (const copy of diagram.presentation_copies || []) {
      const source = behaviorNode(copy.kind, copy.semantic_id);
      if (!source) continue;
      const clone = source.cloneNode(true);
      clone.classList.add('smp-presentation-copy');
      clone.style.display = '';
      clone.dataset.standardCopyId = copy.id;
      mark(clone, 'BehaviorCopy', copy.id);
      if (clone.namespaceURI === 'http://www.w3.org/2000/svg') {
        const existing = clone.getAttribute('transform') || '';
        clone.setAttribute('transform', `${existing} translate(${copy.offset_x || 0} ${copy.offset_y || 0})`.trim());
        source.parentNode?.appendChild(clone);
      } else {
        const left = parseFloat(source.style.left || '0');
        const top = parseFloat(source.style.top || '0');
        if (Number.isFinite(left)) clone.style.left = `${left + Number(copy.offset_x || 0)}px`;
        if (Number.isFinite(top)) clone.style.top = `${top + Number(copy.offset_y || 0)}px`;
        source.parentNode?.appendChild(clone);
      }
    }
  }

  function decoratePresentations() {
    resetForDiagram();
    decorateStructural();
    decorateIbd();
    decorateActivity();
    applyBehaviorPresentationState();
    applySelectionClasses();
    renderPropertiesActions();
    ensureRibbonEditingGroup();
  }

  async function refreshAll() {
    if (typeof refresh === 'function') await refresh();
    if (typeof window.smpRefreshBehavior === 'function') await window.smpRefreshBehavior();
    if (state.selectedActivityDiagramId && typeof requireInvoke === 'function') {
      state.activitySnapshot = await requireInvoke()('activity_snapshot').catch(() => state.activitySnapshot);
    }
    decoratePresentations();
  }

  async function runStandard(commandId) {
    const adapter = STANDARD_COMMANDS[commandId];
    if (!adapter) return false;
    const diagramId = activeDiagramId();
    if (!diagramId) return false;
    const current = selections();
    if (commandId !== 'paste' && current.length === 0) return false;
    try {
      const result = await requireInvoke()(adapter, { diagramId, selections: current });
      if (Array.isArray(result?.selections)) setSelections(result.selections);
      else if (commandId === 'delete') setSelections([]);
      await refreshAll();
      renderStatus?.(`${result?.changed ?? 0} presentation${result?.changed === 1 ? '' : 's'} updated.`);
      return true;
    } catch (error) {
      const message = error?.message || String(error);
      window.smpDialogs?.notify?.(message, 'error');
      renderStatus?.(message);
      return false;
    }
  }

  async function deleteFromModel() {
    const current = selections();
    if (current.length !== 1) {
      window.smpDialogs?.notify?.('Delete from Model requires one selected semantic item at a time.', 'error');
      return;
    }
    const item = current[0];
    const diagramId = activeDiagramId();
    const family = activeFamilyId();
    const diagram = bddDiagram();
    const bddNode = diagram?.nodes?.find((node) => node.id === item.id || node.element_id === item.id);
    const bddEdge = diagram?.edges?.find((edge) => edge.id === item.id || edge.relationship_id === item.id);
    const ibd = ibdDiagram();
    const ibdProperty = ibd?.properties?.find((node) => node.id === item.id || node.element_id === item.id);
    const ibdPort = [
      ...(ibd?.properties || []).flatMap((property) => property.ports || []),
      ...(ibd?.boundary_ports || []),
    ].find((port) => port.id === item.id || port.element_id === item.id);
    const ibdEdge = ibd?.connectors?.find((edge) => edge.id === item.id || edge.relationship_id === item.id);
    const accepted = window.smpDialogs?.confirm
      ? await window.smpDialogs.confirm({
          title: 'Delete from Model',
          description: 'Delete the selected semantic item from the model? Rust reference validation will block unsafe deletion.',
          confirmLabel: 'Delete from Model',
          destructive: true,
        })
      : confirm('Delete the selected semantic item from the model?');
    if (!accepted) return;
    try {
      if (bddNode || ibdProperty || ibdPort) {
        const elementId = bddNode?.element_id || ibdProperty?.element_id || ibdPort?.element_id || item.id;
        await requireInvoke()('delete_model_element', { elementId });
      } else if (bddEdge) {
        await requireInvoke()('delete_bdd_relationship', { diagramId, relationshipId: bddEdge.relationship_id });
      } else if (family === 'state-machine' || family === 'sequence') {
        const semanticId = item.kind === 'BehaviorCopy'
          ? behaviorDiagram()?.presentation_copies?.find((copy) => String(copy.id) === String(item.id))?.semantic_id
          : item.id;
        const type = item.kind === 'BehaviorCopy'
          ? behaviorDiagram()?.presentation_copies?.find((copy) => String(copy.id) === String(item.id))?.kind
          : item.kind;
        await requireInvoke()('delete_behavior_item', { diagramId, itemType: type, itemId: semanticId });
      } else if (family === 'activity') {
        const activity = activityDiagram();
        const node = activity?.nodes?.find((value) => value.id === item.id || value.activity_node_id === item.id);
        const edge = activity?.edges?.find((value) => value.id === item.id || value.activity_edge_id === item.id);
        await requireInvoke()('delete_activity_item', {
          diagramId,
          itemType: edge ? 'Edge' : 'Node',
          itemId: edge?.activity_edge_id || node?.activity_node_id || item.id,
        });
      } else if (ibdEdge) {
        window.smpDialogs?.notify?.('Delete from Model for IBD connectors is awaiting the universal Rust reference-safe relationship deletion path.', 'error');
        return;
      } else if (projectRelationship(item.id)) {
        await requireInvoke()('delete_bdd_relationship', { diagramId, relationshipId: item.id });
      } else if (projectElement(item.id)) {
        await requireInvoke()('delete_model_element', { elementId: item.id });
      } else {
        throw new Error('The selected presentation does not resolve to a deletable semantic item.');
      }
      setSelections([]);
      await refreshAll();
    } catch (error) {
      const message = error?.message || String(error);
      window.smpDialogs?.notify?.(message, 'error');
      renderStatus?.(message);
    }
  }

  function closeMenu() {
    menu.classList.remove('open');
  }

  function openMenu(x, y) {
    menu.style.left = `${Math.min(x, window.innerWidth - 210)}px`;
    menu.style.top = `${Math.min(y, window.innerHeight - 210)}px`;
    menu.classList.add('open');
  }

  menu.addEventListener('click', (event) => {
    const button = event.target.closest('[data-standard-command]');
    if (!button) return;
    closeMenu();
    const command = button.dataset.standardCommand;
    if (command === 'delete-model') void deleteFromModel();
    else void runStandard(command);
  });

  document.addEventListener('pointerdown', (event) => {
    if (!menu.contains(event.target)) closeMenu();
  }, true);

  document.getElementById('canvas')?.addEventListener('click', (event) => {
    const item = presentationFromTarget(event.target);
    if (!item) {
      if (!event.ctrlKey && !event.metaKey && !event.shiftKey) setSelections([]);
      return;
    }
    toggleSelection(item, event.ctrlKey || event.metaKey || event.shiftKey);
  }, true);

  document.getElementById('canvas')?.addEventListener('contextmenu', (event) => {
    const item = presentationFromTarget(event.target);
    if (item && !selections().some((selected) => selectionKey(selected) === selectionKey(item))) {
      setSelections([item]);
    }
    if (!selections().length) return;
    event.preventDefault();
    openMenu(event.clientX, event.clientY);
  });

  function renderPropertiesActions() {
    const panel = document.getElementById('properties');
    if (!panel) return;
    panel.querySelector('.standard-editing-properties')?.remove();
    const current = selections();
    if (!current.length || !activeDiagramId()) return;
    const section = document.createElement('section');
    section.className = 'standard-editing-properties';
    section.innerHTML = `<div class="property-heading">Editing</div><div class="muted">${current.length} presentation${current.length === 1 ? '' : 's'} selected</div><div class="standard-editing-actions"><button type="button" data-action="copy">Copy</button><button type="button" data-action="duplicate">Duplicate</button><button type="button" data-action="delete">Remove from Diagram</button><button type="button" class="danger" data-action="delete-model">Delete from Model</button></div>`;
    section.addEventListener('click', (event) => {
      const action = event.target.closest('[data-action]')?.dataset.action;
      if (!action) return;
      if (action === 'delete-model') void deleteFromModel();
      else void runStandard(action);
    });
    panel.appendChild(section);
  }

  function ensureRibbonEditingGroup() {
    const ribbon = document.querySelector('.ribbon');
    if (!ribbon || ribbon.querySelector('.standard-editing-ribbon-group')) return;
    const group = document.createElement('section');
    group.className = 'ribbon-group standard-editing-ribbon-group';
    group.innerHTML = `<div class="ribbon-actions"><button class="ribbon-command" data-standard-ribbon="copy"><span class="command-icon">⧉</span><span>Copy</span></button><button class="ribbon-command" data-standard-ribbon="paste"><span class="command-icon">▣</span><span>Paste</span></button><button class="ribbon-command" data-standard-ribbon="duplicate"><span class="command-icon">⊕</span><span>Duplicate</span></button><button class="ribbon-command" data-standard-ribbon="delete"><span class="command-icon">×</span><span>Remove</span></button></div><div class="ribbon-label">Edit</div>`;
    group.addEventListener('click', (event) => {
      const command = event.target.closest('[data-standard-ribbon]')?.dataset.standardRibbon;
      if (command) void runStandard(command);
    });
    ribbon.insertBefore(group, ribbon.firstChild);
  }

  const ribbonObserver = new MutationObserver(() => ensureRibbonEditingGroup());
  const ribbon = document.querySelector('.ribbon');
  if (ribbon) ribbonObserver.observe(ribbon, { childList: true });

  const baseRender = window.render;
  if (typeof baseRender === 'function') {
    window.render = function renderWithStandardEditing() {
      baseRender();
      queueMicrotask(decoratePresentations);
    };
    try { render = window.render; } catch (_) { /* global lexical binding may already alias window.render */ }
  }

  window.smpStandardEditing = Object.freeze({
    selections,
    setSelections,
    run: runStandard,
    deleteFromModel,
    decorate: decoratePresentations,
  });

  queueMicrotask(decoratePresentations);
})();
