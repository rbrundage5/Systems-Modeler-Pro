(() => {
  // Thin behavior adapter. Rust owns behavior semantics, validation, ordering,
  // persistence and presentation records. This module only synchronizes the
  // Rust snapshot with the existing desktop shell and forwards user intent.
  state.behaviorSnapshot = state.behaviorSnapshot || null;
  state.selectedBehaviorDiagramId = state.selectedBehaviorDiagramId || null;
  state.selectedBehaviorItem = state.selectedBehaviorItem || null;
  state.behaviorTool = state.behaviorTool || null;
  state.behaviorPending = state.behaviorPending || null;

  const MESSAGE_SORTS = new Set([
    'SynchCall', 'AsynchCall', 'AsynchSignal', 'Reply', 'Create', 'Delete', 'Lost', 'Found',
  ]);

  function activeBehaviorDiagram() {
    return state.behaviorSnapshot?.diagrams?.find(
      (diagram) => String(diagram.id) === String(state.selectedBehaviorDiagramId),
    ) || null;
  }

  function projectElement(id) {
    return state.snapshot?.project?.elements?.find(
      (element) => String(element.id) === String(id),
    ) || null;
  }

  async function loadBehaviorSnapshot() {
    try {
      state.behaviorSnapshot = await requireInvoke()('behavior_snapshot');
    } catch (error) {
      console.error('Unable to load Rust behavior workspace', error);
      state.behaviorSnapshot = {
        repository: { state_machines: {}, interactions: {} },
        diagrams: [],
      };
      throw error;
    }
  }

  window.smpLoadBehaviorSnapshot = loadBehaviorSnapshot;

  const baseRefresh = refresh;
  refresh = async function refreshWithBehaviorSnapshot() {
    const selected = state.selectedBehaviorDiagramId;
    await baseRefresh();
    await loadBehaviorSnapshot();
    if (selected && state.behaviorSnapshot?.diagrams?.some(
      (diagram) => String(diagram.id) === String(selected),
    )) {
      state.selectedBehaviorDiagramId = selected;
    } else if (selected) {
      state.selectedBehaviorDiagramId = null;
      state.selectedBehaviorItem = null;
    }
    render();
  };

  function clearBehaviorInteractionState() {
    state.selectedBehaviorItem = null;
    state.behaviorTool = null;
    state.behaviorPending = null;
    state.behaviorTargetRegionId = null;
  }

  function selectBehaviorDiagram(id) {
    state.selectedBehaviorDiagramId = id;
    state.selectedDiagramId = null;
    state.selectedElementId = null;
    state.selectedRelationshipId = null;
    state.selectedPackageId = null;
    state.paletteTool = null;
    state.pendingRelationship = null;
    clearBehaviorInteractionState();
    render();
  }

  window.smpSelectBehaviorDiagram = selectBehaviorDiagram;

  // BDD/IBD and behavior diagram modes are mutually exclusive. The structural
  // renderer remains responsible for its own selection; this capture listener
  // only releases behavior mode before a structural diagram click is handled.
  document.addEventListener('click', (event) => {
    const target = event.target.closest?.('.diagram-tab, .diagram-row');
    if (!target) return;
    const label = target.textContent || '';
    if (/\bBDD\b/.test(label) || /\bIBD\b/.test(label)) {
      state.selectedBehaviorDiagramId = null;
      clearBehaviorInteractionState();
    }
  }, true);

  const baseRenderDiagramTabs = renderDiagramTabs;
  renderDiagramTabs = function renderBehaviorDiagramTabs() {
    baseRenderDiagramTabs();
    const host = $('diagram-tabs');
    if (!host) return;
    for (const diagram of state.behaviorSnapshot?.diagrams || []) {
      const tab = document.createElement('button');
      tab.className = 'diagram-tab';
      if (String(diagram.id) === String(state.selectedBehaviorDiagramId)) tab.classList.add('active');
      tab.textContent = `${diagram.name} · ${diagram.kind === 'StateMachine' ? 'STM' : 'SEQ'}`;
      tab.onclick = () => selectBehaviorDiagram(diagram.id);
      host.appendChild(tab);
    }
  };

  const baseRenderRepository = renderRepository;
  renderRepository = function renderBehaviorRepository() {
    baseRenderRepository();
    const host = $('repository');
    if (!host) return;
    const filter = (state.repositoryFilter || '').trim().toLowerCase();
    for (const diagram of state.behaviorSnapshot?.diagrams || []) {
      if (filter && !String(diagram.name).toLowerCase().includes(filter)) continue;
      const row = document.createElement('button');
      row.className = 'tree-row diagram-row';
      if (String(diagram.id) === String(state.selectedBehaviorDiagramId)) row.classList.add('selected');
      const tag = diagram.kind === 'StateMachine' ? 'STM' : 'SEQ';
      row.innerHTML = `<span class="kind">${diagram.kind === 'StateMachine' ? '◉' : '⇥'}</span><span>${escapeHtml(diagram.name)}</span><span class="type-tag">${tag}</span>`;
      row.onclick = () => selectBehaviorDiagram(diagram.id);
      host.appendChild(row);
    }
  };

  const baseRenderContext = renderContext;
  renderContext = function renderBehaviorContext() {
    const diagram = activeBehaviorDiagram();
    if (!diagram) return baseRenderContext();
    const label = diagram.kind === 'StateMachine' ? 'State Machine' : 'Sequence';
    const summary = $('active-diagram-summary');
    if (summary) summary.textContent = `${diagram.name} · ${label}`;
    const paletteTitle = $('palette-title');
    if (paletteTitle) paletteTitle.textContent = `${label} Palette`;
  };

  const baseRenderStatus = renderStatus;
  renderStatus = function renderBehaviorStatus(message) {
    const diagram = activeBehaviorDiagram();
    if (!diagram) return baseRenderStatus(message);
    const status = $('status');
    const counts = $('model-counts');
    if (status) {
      if (message) status.textContent = message;
      else if (state.behaviorPending?.kind?.includes('Transition')) {
        status.textContent = state.behaviorPending.source
          ? 'Transition source selected; choose the target vertex.'
          : 'Transition: choose the source vertex.';
      } else if (state.behaviorPending?.sort) {
        status.textContent = state.behaviorPending.source
          ? `${state.behaviorPending.sort}: source selected; choose the target Lifeline.`
          : `${state.behaviorPending.sort}: choose a Lifeline.`;
      } else if (state.behaviorTool) {
        status.textContent = `${state.behaviorTool}: place on the active ${diagram.kind === 'StateMachine' ? 'State Machine' : 'Sequence'} diagram.`;
      } else {
        status.textContent = `${state.snapshot?.project?.name || 'Project'} · ${diagram.kind === 'StateMachine' ? 'State Machine' : 'Sequence'}: ${diagram.name}`;
      }
    }
    if (counts && state.snapshot?.project) {
      counts.textContent = `Elements: ${state.snapshot.project.elements.length}   Relationships: ${state.snapshot.project.relationships.length}   Diagram: ${diagram.name}`;
    }
  };

  function stateKind(vertex) {
    if (vertex?.kind === 'FinalState' || vertex?.kind?.FinalState != null) return 'FinalState';
    if (vertex?.kind?.State != null || vertex?.kind === 'State') return 'State';
    return vertex?.kind?.Pseudostate || '';
  }

  const baseRenderProperties = renderProperties;
  renderProperties = function renderBehaviorBaseProperties() {
    const diagram = activeBehaviorDiagram();
    if (!diagram) return baseRenderProperties();
    const panel = $('properties');
    if (!panel) return;
    const item = state.selectedBehaviorItem;
    if (!item) {
      const context = projectElement(diagram.context_id);
      panel.innerHTML = `<div class="property-heading">${diagram.kind === 'StateMachine' ? 'State Machine' : 'Interaction'}</div><label>Diagram<input value="${escapeAttr(diagram.name)}" disabled></label><label>Context<input value="${escapeAttr(context?.name || diagram.context_id)}" disabled></label><div class="muted">Select a diagram object to edit its Rust-owned semantic properties.</div>`;
      return;
    }
    if (item.type === 'Vertex') {
      const vertex = item.semantic;
      const kind = stateKind(vertex);
      panel.innerHTML = `<div class="property-heading">${escapeHtml(kind || 'Vertex')}</div><label>Name<input value="${escapeAttr(vertex.name || '')}" disabled></label>`;
      if (kind === 'State') {
        const semantic = vertex.kind?.State || {};
        panel.innerHTML += `<label>Entry<input id="behavior-entry" value="${escapeAttr(semantic.entry || '')}"></label><label>Do<input id="behavior-do" value="${escapeAttr(semantic.do_activity || '')}"></label><label>Exit<input id="behavior-exit" value="${escapeAttr(semantic.exit || '')}"></label><button id="behavior-apply-state" class="primary">Apply State Behaviors</button><button id="behavior-add-region">Add Region</button>`;
        $('behavior-apply-state').onclick = async () => {
          await runCommand('Updating State behaviors…', () => requireInvoke()('update_state_behaviors', {
            diagramId: diagram.id,
            stateVertexId: vertex.id,
            entry: $('behavior-entry').value,
            doActivity: $('behavior-do').value,
            exit: $('behavior-exit').value,
          }));
          await refresh();
        };
        $('behavior-add-region').onclick = async () => {
          const name = prompt('Region name', 'Region');
          if (!name) return;
          await runCommand('Adding Region…', () => requireInvoke()('add_state_region', {
            diagramId: diagram.id,
            stateVertexId: vertex.id,
            name,
          }));
          await refresh();
        };
      }
      return;
    }
    if (item.type === 'Lifeline') {
      panel.innerHTML = `<div class="property-heading">Lifeline</div><label>Represents<input value="${escapeAttr(item.semantic.name || '')}" disabled></label><div class="muted">Lifeline identity and represented property path are Rust-owned.</div>`;
      return;
    }
    panel.innerHTML = `<div class="property-heading">${escapeHtml(item.type || 'Behavior element')}</div><div class="muted">Semantic editing for this object is provided by the Rust-backed behavior property editor.</div>`;
  };

  async function signatureForMessage(sort) {
    if (!['SynchCall', 'AsynchCall', 'AsynchSignal'].includes(sort)) return null;
    const requiredKind = sort === 'AsynchSignal' ? 'Signal' : 'Operation';
    const candidates = state.snapshot?.project?.elements?.filter(
      (element) => element.kind === requiredKind,
    ) || [];
    if (!candidates.length) {
      alert(`Create a ${requiredKind} first. ${sort} requires a real ${requiredKind} signature.`);
      return undefined;
    }
    const menu = candidates.map((item, index) => `${index + 1}. ${item.name}`).join('\n');
    const answer = prompt(`Choose ${requiredKind}:\n${menu}`, '1');
    return candidates[Number(answer) - 1]?.id;
  }

  async function createMessage(diagram, sourceId, targetId, sort) {
    const signatureId = await signatureForMessage(sort);
    if (signatureId === undefined) return;
    const name = prompt('Message name', sort === 'Reply' ? 'reply' : 'message') || '';
    const argumentsText = ['SynchCall', 'AsynchCall', 'AsynchSignal'].includes(sort)
      ? (prompt('Arguments, comma separated (optional)', '') || '')
      : '';
    const args = argumentsText.split(',').map((value) => value.trim()).filter(Boolean);
    await runCommand(`Creating ${sort} Message…`, () => requireInvoke()('add_sequence_message', {
      diagramId: diagram.id,
      sourceLifelineId: sourceId,
      targetLifelineId: targetId,
      sort,
      name,
      signatureId,
      arguments: args,
    }));
    await refresh();
  }

  window.smpBehaviorLifelineClick = async (diagram, lifeline) => {
    const pending = state.behaviorPending;
    if (!pending || !MESSAGE_SORTS.has(pending.sort) || pending.kind === 'SingleEndedMessage') {
      state.selectedBehaviorItem = { type: 'Lifeline', id: lifeline.id, semantic: lifeline };
      render();
      return true;
    }
    if (!pending.source) {
      pending.source = lifeline.id;
      render();
      return true;
    }
    const source = pending.source;
    const sort = pending.sort;
    state.behaviorPending = null;
    await createMessage(diagram, source, lifeline.id, sort);
    return true;
  };

  // The renderer stays presentation-only. Lifeline pointer intent is forwarded
  // here so message creation has one UI path and one Rust semantic command.
  document.addEventListener('click', (event) => {
    const node = event.target.closest?.('.sequence-lifeline');
    const diagram = activeBehaviorDiagram();
    if (!node || !diagram || diagram.kind !== 'Sequence') return;
    if (!state.behaviorPending || state.behaviorPending.kind === 'SingleEndedMessage') return;
    const interaction = state.behaviorSnapshot?.repository?.interactions?.[String(diagram.semantic_id)];
    const lifeline = interaction?.lifelines?.find(
      (candidate) => String(candidate.id) === String(node.dataset.lifelineId),
    );
    if (!lifeline) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    void window.smpBehaviorLifelineClick(diagram, lifeline);
  }, true);

  loadBehaviorSnapshot().then(() => render()).catch((error) => {
    const status = $('status');
    if (status) status.textContent = `Behavior workspace unavailable: ${error?.message || String(error)}`;
  });
})();
