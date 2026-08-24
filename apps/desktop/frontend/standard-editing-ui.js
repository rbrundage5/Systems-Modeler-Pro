(() => {
  'use strict';

  const COMMANDS = Object.freeze({
    copy: 'copy_selection',
    paste: 'paste_selection',
    duplicate: 'duplicate_selection',
    delete: 'delete_active_selection',
  });
  const selectionState = { diagramId: null, items: [] };
  window.smpStandardSelections = [];

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

  const application = () => window.smpState || null;
  const hostContext = () => window.smpRendererHost?.context?.() || null;
  const activeDiagramId = () => hostContext()?.diagramId
    || application()?.selectedBehaviorDiagramId
    || application()?.selectedActivityDiagramId
    || application()?.selectedDiagramId
    || null;
  const activeFamilyId = () => hostContext()?.family?.id || hostContext()?.familyId || null;
  const normalizedSelection = (kind, id) => ({ kind: String(kind || 'Presentation'), id: String(id) });
  const selectionKey = (item) => `${item.kind}:${item.id}`;

  function resetForDiagram() {
    const diagramId = activeDiagramId();
    if (selectionState.diagramId === diagramId) return;
    selectionState.diagramId = diagramId;
    selectionState.items = [];
    window.smpStandardSelections = [];
  }

  function legacySelections() {
    const app = application();
    if (!app) return [];
    const family = activeFamilyId();
    if (family === 'state-machine' || family === 'sequence') {
      const item = app.selectedBehaviorItem;
      return item?.id ? [normalizedSelection(item.type || 'BehaviorItem', item.id)] : [];
    }
    if (family === 'activity') {
      if (app.selectedActivityEdgeId) return [normalizedSelection('ActivityEdge', app.selectedActivityEdgeId)];
      if (app.selectedActivityNodeId) return [normalizedSelection('ActivityNode', app.selectedActivityNodeId)];
      return [];
    }
    if (app.selectedRelationshipId) return [normalizedSelection('Relationship', app.selectedRelationshipId)];
    if (app.selectedElementId) return [normalizedSelection('Element', app.selectedElementId)];
    return [];
  }

  function selections() {
    resetForDiagram();
    return (selectionState.items.length ? selectionState.items : legacySelections())
      .map((item) => ({ ...item }));
  }

  function applySelectionClasses() {
    const wanted = new Set(selections().map(selectionKey));
    document.querySelectorAll('#canvas [data-smp-presentation-id]').forEach((node) => {
      const item = normalizedSelection(node.dataset.smpPresentationKind, node.dataset.smpPresentationId);
      node.classList.toggle('smp-standard-selected', wanted.has(selectionKey(item)));
    });
  }

  function setSelections(items) {
    resetForDiagram();
    const unique = new Map();
    for (const value of items || []) {
      if (!value?.id) continue;
      const item = normalizedSelection(value.kind, value.id);
      unique.set(selectionKey(item), item);
    }
    selectionState.items = [...unique.values()];
    window.smpStandardSelections = selectionState.items.map((item) => ({ ...item }));
    applySelectionClasses();
    renderPropertiesActions();
    void window.smpRendererHost?.publishInteraction?.();
  }

  function toggleSelection(item, additive) {
    if (!additive) return setSelections([item]);
    const next = new Map(selections().map((value) => [selectionKey(value), value]));
    const key = selectionKey(item);
    if (next.has(key)) next.delete(key);
    else next.set(key, item);
    setSelections([...next.values()]);
  }

  function presentationFromTarget(target) {
    const node = target?.closest?.('[data-smp-presentation-id]');
    if (!node || !document.getElementById('canvas')?.contains(node)) return null;
    return normalizedSelection(node.dataset.smpPresentationKind, node.dataset.smpPresentationId);
  }

  function bddDiagram() {
    const app = application();
    return app?.snapshot?.diagrams?.find((item) => String(item.id) === String(activeDiagramId())) || null;
  }

  function ibdDiagram() {
    const app = application();
    return app?.snapshot?.ibd_diagrams?.find((item) => String(item.id) === String(activeDiagramId())) || null;
  }

  function activityDiagram() {
    const app = application();
    return app?.activitySnapshot?.diagrams?.find((item) => String(item.id) === String(activeDiagramId())) || null;
  }

  function behaviorDiagram() {
    const app = application();
    return app?.behaviorSnapshot?.diagrams?.find((item) => String(item.id) === String(activeDiagramId())) || null;
  }

  function projectElement(id) {
    return application()?.snapshot?.project?.elements?.find((item) => String(item.id) === String(id)) || null;
  }

  function projectRelationship(id) {
    return application()?.snapshot?.project?.relationships?.find((item) => String(item.id) === String(id)) || null;
  }

  function mark(node, kind, id) {
    if (!node || !id) return;
    node.dataset.smpPresentationKind = kind;
    node.dataset.smpPresentationId = String(id);
  }

  function decorateStructural() {
    const diagram = bddDiagram();
    if (!diagram) return;
    if (diagram.family === 'use-case' && diagram.subject_boundary) {
      const boundary = document.querySelector(
        `#canvas .use-case-subject-boundary[data-subject-boundary-id="${CSS.escape(String(diagram.subject_boundary.id))}"]`,
      );
      mark(boundary, 'UseCaseSubjectBoundary', diagram.subject_boundary.id);
    }
    [...document.querySelectorAll('#canvas .bdd-block')].forEach((node, index) => {
      const presentation = diagram.nodes?.find((item) => String(item.id) === String(node.dataset.presentationId))
        || diagram.nodes?.[index];
      if (presentation) mark(node, 'BddNode', presentation.id);
    });
    [...document.querySelectorAll('#canvas .bdd-relationship')].forEach((node, index) => {
      const relationshipId = node.dataset.relationshipId;
      const presentation = relationshipId
        ? diagram.edges?.find((item) => String(item.relationship_id) === String(relationshipId))
        : diagram.edges?.[index];
      if (presentation) mark(node, 'BddEdge', presentation.id);
    });
  }

  function decorateIbd() {
    const diagram = ibdDiagram();
    if (!diagram) return;
    [...document.querySelectorAll('#canvas .ibd-property')].forEach((node, index) => {
      const presentation = diagram.properties?.find((item) => String(item.id) === String(node.dataset.presentationId))
        || diagram.properties?.[index];
      if (presentation) mark(node, 'IbdProperty', presentation.id);
    });
    const ports = [
      ...(diagram.properties || []).flatMap((property) => property.ports || []),
      ...(diagram.boundary_ports || []),
    ];
    [...document.querySelectorAll('#canvas .ibd-port')].forEach((node, index) => {
      const presentation = ports.find((item) => String(item.id) === String(node.dataset.presentationId)) || ports[index];
      if (presentation) mark(node, 'IbdPort', presentation.id);
    });
    [...document.querySelectorAll('#canvas .ibd-connector')].forEach((node, index) => {
      const relationshipId = node.dataset.relationshipId;
      const presentation = relationshipId
        ? diagram.connectors?.find((item) => String(item.relationship_id) === String(relationshipId))
        : diagram.connectors?.[index];
      if (presentation) mark(node, 'IbdConnector', presentation.id);
    });
  }

  function decorateActivity() {
    const diagram = activityDiagram();
    if (!diagram) return;
    document.querySelectorAll('#canvas .activity-node').forEach((node) => {
      const presentation = diagram.nodes?.find(
        (item) => String(item.activity_node_id) === String(node.dataset.activityNodeId),
      );
      if (presentation) mark(node, 'ActivityNode', presentation.id);
    });
    document.querySelectorAll('#canvas [data-activity-edge-id], #canvas .activity-edge').forEach((node) => {
      const presentation = diagram.edges?.find(
        (item) => String(item.activity_edge_id) === String(node.dataset.activityEdgeId),
      );
      if (presentation) mark(node, 'ActivityEdge', presentation.id);
    });
  }

  const behaviorSelector = Object.freeze({
    Vertex: '[data-vertex-id]', Transition: '[data-transition-id]', Lifeline: '[data-lifeline-id]',
    Message: '[data-message-id]', Execution: '[data-execution-id]', Fragment: '[data-fragment-id]',
    Invariant: '[data-invariant-id]',
  });

  function behaviorIdentity(node) {
    for (const [kind, selector] of Object.entries(behaviorSelector)) {
      if (!node.matches?.(selector)) continue;
      const key = `${kind.charAt(0).toLowerCase()}${kind.slice(1)}Id`;
      return { kind, id: node.dataset[key] };
    }
    return null;
  }

  function behaviorNode(kind, semanticId) {
    const selector = behaviorSelector[kind];
    if (!selector) return null;
    return [...document.querySelectorAll(`#canvas ${selector}`)].find((node) => {
      if (node.classList.contains('smp-presentation-copy')) return false;
      return String(behaviorIdentity(node)?.id) === String(semanticId);
    }) || null;
  }

  function decorateBehavior() {
    const diagram = behaviorDiagram();
    if (!diagram) return;
    const hidden = new Set((diagram.hidden_semantic_ids || []).map(String));
    for (const selector of Object.values(behaviorSelector)) {
      document.querySelectorAll(`#canvas ${selector}`).forEach((node) => {
        if (node.classList.contains('smp-presentation-copy')) return;
        const identity = behaviorIdentity(node);
        if (!identity?.id) return;
        mark(node, identity.kind, identity.id);
        node.style.display = hidden.has(String(identity.id)) ? 'none' : '';
      });
    }

    const desired = new Map((diagram.presentation_copies || []).map((copy) => [String(copy.id), copy]));
    document.querySelectorAll('#canvas .smp-presentation-copy').forEach((node) => {
      if (!desired.has(String(node.dataset.standardCopyId))) node.remove();
    });
    for (const copy of desired.values()) {
      if (document.querySelector(`#canvas .smp-presentation-copy[data-standard-copy-id="${CSS.escape(String(copy.id))}"]`)) continue;
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
      } else {
        const left = parseFloat(source.style.left || '0');
        const top = parseFloat(source.style.top || '0');
        if (Number.isFinite(left)) clone.style.left = `${left + Number(copy.offset_x || 0)}px`;
        if (Number.isFinite(top)) clone.style.top = `${top + Number(copy.offset_y || 0)}px`;
      }
      source.parentNode?.appendChild(clone);
    }
  }

  function decoratePresentations() {
    resetForDiagram();
    decorateStructural();
    decorateIbd();
    decorateActivity();
    decorateBehavior();
    applySelectionClasses();
    renderPropertiesActions();
    ensureRibbonEditingGroup();
  }

  async function refreshAll() {
    if (typeof refresh === 'function') await refresh();
    if (typeof window.smpRefreshBehavior === 'function') await window.smpRefreshBehavior();
    const app = application();
    if (app?.selectedActivityDiagramId && typeof requireInvoke === 'function') {
      app.activitySnapshot = await requireInvoke()('activity_snapshot').catch(() => app.activitySnapshot);
    }
    decoratePresentations();
  }

  async function runStandard(commandId) {
    const adapter = COMMANDS[commandId];
    if (!adapter || !activeDiagramId()) return false;
    if (commandId !== 'paste' && selections().length === 0) return false;
    try {
      await window.smpRendererHost?.publishInteraction?.();
      const result = await requireInvoke()(adapter, { diagramId: activeDiagramId() });
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

  async function deleteRelationshipFromModel() {
    await window.smpRendererHost?.publishInteraction?.();
    return requireInvoke()('delete_active_selection', {
      diagramId: activeDiagramId(),
      fromModel: true,
    });
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
    const bdd = bddDiagram();
    const ibd = ibdDiagram();
    const bddNode = bdd?.nodes?.find((node) => node.id === item.id || node.element_id === item.id);
    const bddEdge = bdd?.edges?.find((edge) => edge.id === item.id || edge.relationship_id === item.id);
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
      if (bddNode || ibdProperty || ibdPort || projectElement(item.id)) {
        await requireInvoke()('delete_model_element', {
          elementId: bddNode?.element_id || ibdProperty?.element_id || ibdPort?.element_id || item.id,
        });
      } else if (bddEdge || ibdEdge || projectRelationship(item.id)) {
        await deleteRelationshipFromModel();
      } else if (family === 'state-machine' || family === 'sequence') {
        const copy = item.kind === 'BehaviorCopy'
          ? behaviorDiagram()?.presentation_copies?.find((value) => String(value.id) === String(item.id))
          : null;
        await runCommand('Deleting Behavior item from model…', () => requireInvoke()('delete_behavior_item', {
          diagramId,
          itemType: copy?.kind || item.kind,
          itemId: copy?.semantic_id || item.id,
        }));
      } else if (family === 'activity') {
        const activity = activityDiagram();
        const node = activity?.nodes?.find((value) => value.id === item.id || value.activity_node_id === item.id);
        const edge = activity?.edges?.find((value) => value.id === item.id || value.activity_edge_id === item.id);
        if (!node && !edge) {
          throw new Error('The selected Activity presentation does not resolve to a semantic node or edge.');
        }
        await runCommand('Deleting Activity item from model…', () => requireInvoke()('delete_activity_item', {
          diagramId,
          itemKind: edge ? 'edge' : 'node',
          itemId: edge?.activity_edge_id || node?.activity_node_id,
        }));
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

  const closeMenu = () => menu.classList.remove('open');
  function openMenu(x, y) {
    menu.style.left = `${Math.min(x, window.innerWidth - 210)}px`;
    menu.style.top = `${Math.min(y, window.innerHeight - 210)}px`;
    menu.classList.add('open');
  }

  menu.addEventListener('click', (event) => {
    const command = event.target.closest('[data-standard-command]')?.dataset.standardCommand;
    if (!command) return;
    closeMenu();
    if (command === 'delete-model') void deleteFromModel();
    else void runStandard(command);
  });
  document.addEventListener('pointerdown', (event) => {
    if (!menu.contains(event.target)) closeMenu();
  }, true);

  const canvas = document.getElementById('canvas');
  canvas?.addEventListener('click', (event) => {
    const item = presentationFromTarget(event.target);
    if (!item) {
      if (!event.ctrlKey && !event.metaKey && !event.shiftKey) setSelections([]);
      return;
    }
    toggleSelection(item, event.ctrlKey || event.metaKey || event.shiftKey);
  }, true);
  canvas?.addEventListener('contextmenu', (event) => {
    const item = presentationFromTarget(event.target);
    if (item && !selections().some((selected) => selectionKey(selected) === selectionKey(item))) setSelections([item]);
    if (!selections().length) return;
    if (selections().every((selected) => selected.kind === 'UseCaseSubjectBoundary')) return;
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
    if (current.every((selected) => selected.kind === 'UseCaseSubjectBoundary')) {
      section.innerHTML = '<div class="property-heading">Editing</div><div class="muted">The subject boundary is diagram context. Move or resize it directly; contained Use Cases move with it.</div>';
      panel.appendChild(section);
      return;
    }
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

  const scheduleDecorate = (() => {
    let pending = false;
    return () => {
      if (pending) return;
      pending = true;
      queueMicrotask(() => {
        pending = false;
        decoratePresentations();
      });
    };
  })();
  if (canvas) new MutationObserver(scheduleDecorate).observe(canvas, { childList: true, subtree: true });
  const ribbon = document.querySelector('.ribbon');
  if (ribbon) new MutationObserver(ensureRibbonEditingGroup).observe(ribbon, { childList: true });

  window.smpStandardEditing = Object.freeze({
    selections,
    setSelections,
    run: runStandard,
    deleteFromModel,
    decorate: decoratePresentations,
  });

  scheduleDecorate();
})();
