(() => {
  state.activitySnapshot = state.activitySnapshot || { repository: { activities: {} }, diagrams: [] };
  state.selectedActivityDiagramId = state.selectedActivityDiagramId || null;
  state.selectedActivityNodeId = state.selectedActivityNodeId || null;
  state.activityTool = state.activityTool || null;
  state.activityPendingFlow = state.activityPendingFlow || null;
  state.activityPaletteItems = state.activityPaletteItems || [];

  function activeActivityDiagram() {
    return state.activitySnapshot?.diagrams?.find(
      (diagram) => String(diagram.id) === String(state.selectedActivityDiagramId),
    ) || null;
  }

  function activeActivity() {
    const diagram = activeActivityDiagram();
    if (!diagram) return null;
    return state.activitySnapshot?.repository?.activities?.[String(diagram.activity_id)] || null;
  }

  function semanticNode(id) {
    return activeActivity()?.nodes?.find((node) => String(node.id) === String(id)) || null;
  }

  function nodeKind(node) {
    const kind = node?.kind;
    if (typeof kind === 'string') return kind;
    if (!kind || typeof kind !== 'object') return 'ActivityNode';
    if (kind.Action) return 'OpaqueAction';
    if (kind.Object) {
      const objectKind = kind.Object.kind;
      if (objectKind === 'CentralBuffer') return 'CentralBufferNode';
      if (objectKind === 'DataStore') return 'DataStoreNode';
      return 'ObjectNode';
    }
    if (kind.Decision) return 'Decision';
    if (kind.Join) return 'Join';
    return Object.keys(kind)[0] || 'ActivityNode';
  }

  async function loadActivitySnapshot() {
    state.activitySnapshot = await requireInvoke()('activity_snapshot');
    return state.activitySnapshot;
  }

  async function loadActivityPalette() {
    if (!state.selectedActivityDiagramId) {
      state.activityPaletteItems = [];
      return;
    }
    state.activityPaletteItems = await requireInvoke()('diagram_palette', { diagramType: 'Activity' });
  }

  function clearActivityInteraction() {
    state.selectedActivityNodeId = null;
    state.activityTool = null;
    state.activityPendingFlow = null;
  }

  async function selectActivityDiagram(id) {
    state.selectedActivityDiagramId = id;
    state.selectedDiagramId = null;
    state.selectedBehaviorDiagramId = null;
    state.selectedElementId = null;
    state.selectedRelationshipId = null;
    state.paletteTool = null;
    state.pendingRelationship = null;
    clearActivityInteraction();
    await loadActivityPalette();
    render();
  }
  window.smpSelectActivityDiagram = selectActivityDiagram;

  async function createActivityForSelection() {
    if (!state.snapshot?.project) return alert('Create or open a project first.');
    const selected = state.snapshot.project.elements?.find((element) => element.id === state.selectedElementId);
    const contextId = selected?.kind === 'Block' ? selected.id : null;
    const ownerId = state.selectedPackageId
      || (selected?.owner_id && state.snapshot.project.elements?.find((element) => element.id === selected.owner_id)?.kind === 'Package' ? selected.owner_id : null)
      || state.snapshot.project.root_id;
    const name = prompt('Activity name', selected?.kind === 'Block' ? `${selected.name} Activity` : 'System Activity');
    if (!name) return;
    state.selectedActivityDiagramId = await runCommand('Creating Activity…', () => requireInvoke()('create_activity_diagram', {
      ownerId,
      contextId,
      name,
    }));
    state.selectedDiagramId = null;
    state.selectedBehaviorDiagramId = null;
    clearActivityInteraction();
    await loadActivitySnapshot();
    await loadActivityPalette();
    render();
  }
  window.smpCreateActivityForSelection = createActivityForSelection;

  const baseRefresh = refresh;
  refresh = async function refreshWithActivity() {
    const selected = state.selectedActivityDiagramId;
    await baseRefresh();
    await loadActivitySnapshot();
    if (selected && state.activitySnapshot.diagrams?.some((diagram) => String(diagram.id) === String(selected))) {
      state.selectedActivityDiagramId = selected;
    } else if (selected) {
      state.selectedActivityDiagramId = null;
      clearActivityInteraction();
    }
    if (state.selectedActivityDiagramId) await loadActivityPalette();
    render();
  };

  const baseRenderDiagramTabs = renderDiagramTabs;
  renderDiagramTabs = function renderActivityDiagramTabs() {
    baseRenderDiagramTabs();
    const host = $('diagram-tabs');
    for (const diagram of state.activitySnapshot?.diagrams || []) {
      const tab = document.createElement('button');
      tab.className = 'diagram-tab';
      if (String(diagram.id) === String(state.selectedActivityDiagramId)) tab.classList.add('active');
      tab.textContent = `${diagram.name} · ACT`;
      tab.onclick = () => void selectActivityDiagram(diagram.id);
      host.appendChild(tab);
    }
  };

  const baseRenderRepository = renderRepository;
  renderRepository = function renderActivityRepository() {
    baseRenderRepository();
    const host = $('repository');
    const filter = (state.repositoryFilter || '').trim().toLowerCase();
    for (const diagram of state.activitySnapshot?.diagrams || []) {
      if (filter && !String(diagram.name).toLowerCase().includes(filter)) continue;
      const row = document.createElement('button');
      row.className = 'tree-row diagram-row';
      if (String(diagram.id) === String(state.selectedActivityDiagramId)) row.classList.add('selected');
      row.innerHTML = `<span class="kind">▶</span><span>${escapeHtml(diagram.name)}</span><span class="type-tag">ACT</span>`;
      row.onclick = () => void selectActivityDiagram(diagram.id);
      host.appendChild(row);
    }
  };

  const baseRenderContext = renderContext;
  renderContext = function renderActivityContext() {
    const diagram = activeActivityDiagram();
    if (!diagram) return baseRenderContext();
    const summary = $('active-diagram-summary');
    if (summary) summary.textContent = `${diagram.name} · Activity`;
    const paletteTitle = $('palette-title');
    if (paletteTitle) paletteTitle.textContent = 'Activity Palette';
  };

  function paletteGlyph(item) {
    const kind = item.semantic_kind || item.relationship_kind;
    const glyphs = {
      Initial: '●', ActivityFinal: '◉', FlowFinal: '⊗', Decision: '◆', Merge: '◆',
      Fork: '━', Join: '━', OpaqueAction: '▭', ObjectNode: '□', CentralBufferNode: '□',
      DataStoreNode: '▣', ControlFlow: '→', ObjectFlow: '⇢',
    };
    return glyphs[kind] || '·';
  }

  const baseRenderPalette = renderPalette;
  renderPalette = function renderActivityPalette() {
    if (!activeActivityDiagram()) return baseRenderPalette();
    const host = $('palette');
    host.innerHTML = '';
    for (const [category, title] of [['element', 'Nodes'], ['relationship', 'Flows']]) {
      const items = state.activityPaletteItems.filter((item) => item.category === category);
      if (!items.length) continue;
      const section = document.createElement('section');
      section.className = 'palette-section';
      section.innerHTML = `<div class="palette-section-title">${title}</div>`;
      for (const item of items) {
        const button = document.createElement('button');
        button.className = `palette-item ${category}`;
        if (state.activityTool?.id === item.id || state.activityPendingFlow?.kind === item.relationship_kind) button.classList.add('active');
        button.innerHTML = `<span class="palette-symbol">${escapeHtml(paletteGlyph(item))}</span><span>${escapeHtml(item.label)}</span>`;
        button.onclick = () => {
          state.selectedActivityNodeId = null;
          if (category === 'relationship') {
            state.activityTool = null;
            state.activityPendingFlow = { kind: item.relationship_kind, source: null };
          } else {
            state.activityPendingFlow = null;
            state.activityTool = item;
          }
          render();
        };
        section.appendChild(button);
      }
      host.appendChild(section);
    }
    const hint = document.createElement('div');
    hint.className = 'palette-hint';
    hint.textContent = 'Activity tools are supplied by Rust. Select a node tool and click the canvas, or select a flow and choose source then target.';
    host.appendChild(hint);
  };

  function makeSvg(tag, attrs = {}) {
    const node = document.createElementNS(SVG_NS, tag);
    for (const [name, value] of Object.entries(attrs)) node.setAttribute(name, String(value));
    return node;
  }

  function drawActivityNode(svg, presentation, semantic) {
    const group = makeSvg('g', { class: `activity-node activity-${nodeKind(semantic).toLowerCase()}` });
    group.dataset.activityNodeId = semantic.id;
    if (String(semantic.id) === String(state.selectedActivityNodeId)) group.classList.add('selected');
    const kind = nodeKind(semantic);
    const x = presentation.x;
    const y = presentation.y;
    const w = presentation.width;
    const h = presentation.height;
    if (kind === 'Initial') {
      group.appendChild(makeSvg('circle', { cx: x + w / 2, cy: y + h / 2, r: Math.min(w, h) / 2 - 2 }));
    } else if (kind === 'ActivityFinal') {
      group.appendChild(makeSvg('circle', { class: 'activity-final-outer', cx: x + w / 2, cy: y + h / 2, r: Math.min(w, h) / 2 - 2 }));
      group.appendChild(makeSvg('circle', { class: 'activity-final-inner', cx: x + w / 2, cy: y + h / 2, r: Math.min(w, h) / 2 - 7 }));
    } else if (kind === 'FlowFinal') {
      group.appendChild(makeSvg('circle', { class: 'activity-flow-final', cx: x + w / 2, cy: y + h / 2, r: Math.min(w, h) / 2 - 2 }));
      group.appendChild(makeSvg('line', { x1: x + 6, y1: y + 6, x2: x + w - 6, y2: y + h - 6 }));
      group.appendChild(makeSvg('line', { x1: x + w - 6, y1: y + 6, x2: x + 6, y2: y + h - 6 }));
    } else if (kind === 'Decision' || kind === 'Merge') {
      group.appendChild(makeSvg('polygon', { points: `${x + w / 2},${y} ${x + w},${y + h / 2} ${x + w / 2},${y + h} ${x},${y + h / 2}` }));
    } else if (kind === 'Fork' || kind === 'Join') {
      group.appendChild(makeSvg('rect', { x, y: y + h / 2 - 4, width: w, height: 8, rx: 1 }));
    } else {
      group.appendChild(makeSvg('rect', { x, y, width: w, height: h, rx: kind === 'OpaqueAction' ? 12 : 2 }));
      const text = makeSvg('text', { x: x + w / 2, y: y + h / 2 + 4, 'text-anchor': 'middle' });
      text.textContent = semantic.name || kind;
      group.appendChild(text);
    }
    group.onclick = (event) => {
      event.stopPropagation();
      void handleActivityNodeClick(semantic);
    };
    svg.appendChild(group);
  }

  function drawActivityEdge(svg, presentation, semantic) {
    const points = presentation.points || [];
    if (points.length < 2) return;
    const path = makeSvg('polyline', {
      class: `activity-flow ${semantic?.kind === 'ObjectFlow' ? 'object-flow' : 'control-flow'}`,
      points: points.map((point) => `${point.x},${point.y}`).join(' '),
      'marker-end': 'url(#activity-arrow)',
    });
    svg.appendChild(path);
    const guard = semantic?.guard;
    if (guard) {
      const mid = points[Math.floor(points.length / 2)];
      const label = makeSvg('text', { class: 'activity-edge-label', x: mid.x + 6, y: mid.y - 6 });
      label.textContent = `[${guard}]`;
      svg.appendChild(label);
    }
  }

  const baseRenderCanvas = renderCanvas;
  renderCanvas = function renderActivityCanvas() {
    const diagram = activeActivityDiagram();
    if (!diagram) return baseRenderCanvas();
    const host = $('canvas');
    host.innerHTML = '';
    host.classList.add('activity-canvas');
    const svg = makeSvg('svg', { class: 'activity-svg', viewBox: '0 0 1800 1100' });
    const defs = makeSvg('defs');
    const arrow = makeSvg('marker', { id: 'activity-arrow', markerWidth: 10, markerHeight: 10, refX: 9, refY: 3, orient: 'auto', markerUnits: 'strokeWidth' });
    arrow.appendChild(makeSvg('path', { d: 'M0,0 L0,6 L9,3 z' }));
    defs.appendChild(arrow);
    svg.appendChild(defs);
    const activity = activeActivity();
    for (const edgePresentation of diagram.edges || []) {
      const semantic = activity?.edges?.find((edge) => String(edge.id) === String(edgePresentation.activity_edge_id));
      drawActivityEdge(svg, edgePresentation, semantic);
    }
    for (const nodePresentation of diagram.nodes || []) {
      const semantic = activity?.nodes?.find((node) => String(node.id) === String(nodePresentation.activity_node_id));
      if (semantic) drawActivityNode(svg, nodePresentation, semantic);
    }
    svg.onclick = async (event) => {
      if (!state.activityTool) return;
      const rect = svg.getBoundingClientRect();
      const x = ((event.clientX - rect.left) / rect.width) * 1800;
      const y = ((event.clientY - rect.top) / rect.height) * 1100;
      const kind = state.activityTool.semantic_kind;
      const name = kind === 'OpaqueAction' || /ObjectNode$/.test(kind) ? (prompt(`${state.activityTool.label} name`, state.activityTool.label) || '') : '';
      await runCommand(`Creating ${state.activityTool.label}…`, () => requireInvoke()('add_activity_node', {
        diagramId: diagram.id,
        kind,
        name,
        x,
        y,
      }));
      state.activityTool = null;
      await loadActivitySnapshot();
      render();
    };
    host.appendChild(svg);
  };

  async function handleActivityNodeClick(node) {
    const diagram = activeActivityDiagram();
    if (!diagram) return;
    if (!state.activityPendingFlow) {
      state.selectedActivityNodeId = node.id;
      render();
      return;
    }
    if (!state.activityPendingFlow.source) {
      state.activityPendingFlow.source = node.id;
      state.selectedActivityNodeId = node.id;
      render();
      return;
    }
    const source = state.activityPendingFlow.source;
    const kind = state.activityPendingFlow.kind;
    state.activityPendingFlow = null;
    const guard = kind === 'ControlFlow' ? (prompt('Guard (optional)', '') || null) : null;
    await runCommand(`Creating ${kind}…`, () => requireInvoke()('add_activity_edge', {
      diagramId: diagram.id,
      kind,
      sourceActivityNodeId: source,
      targetActivityNodeId: node.id,
      guard,
      weight: null,
    }));
    state.selectedActivityNodeId = null;
    await loadActivitySnapshot();
    render();
  }

  const baseRenderProperties = renderProperties;
  renderProperties = function renderActivityProperties() {
    const diagram = activeActivityDiagram();
    if (!diagram) return baseRenderProperties();
    const panel = $('properties');
    const activity = activeActivity();
    const node = semanticNode(state.selectedActivityNodeId);
    if (!node) {
      panel.innerHTML = `<div class="property-heading">Activity</div><label>Name<input value="${escapeAttr(activity?.name || diagram.name)}" disabled></label><label>Stable ID<input value="${escapeAttr(diagram.activity_id)}" disabled></label><div class="muted">Select an Activity node to inspect its Rust-owned semantics.</div>`;
      return;
    }
    panel.innerHTML = `<div class="property-heading">${escapeHtml(nodeKind(node))}</div><label>Name<input value="${escapeAttr(node.name || '')}" disabled></label><label>Semantic ID<input value="${escapeAttr(node.id)}" disabled></label><div class="muted">Node identity, kind, ownership, and flow connectivity are authoritative in Rust.</div>`;
  };

  const baseRenderStatus = renderStatus;
  renderStatus = function renderActivityStatus(message) {
    const diagram = activeActivityDiagram();
    if (!diagram) return baseRenderStatus(message);
    const status = $('status');
    if (message) status.textContent = message;
    else if (state.activityPendingFlow) status.textContent = state.activityPendingFlow.source ? `${state.activityPendingFlow.kind}: choose target node.` : `${state.activityPendingFlow.kind}: choose source node.`;
    else if (state.activityTool) status.textContent = `${state.activityTool.label}: click the Activity canvas to create it.`;
    else status.textContent = `${state.snapshot?.project?.name || 'Project'} · Activity: ${diagram.name}`;
    const counts = $('model-counts');
    if (counts) counts.textContent = `Activity nodes: ${activeActivity()?.nodes?.length || 0}   Flows: ${activeActivity()?.edges?.length || 0}`;
  };

  document.addEventListener('click', (event) => {
    const tab = event.target.closest?.('.diagram-tab, .diagram-row');
    if (!tab) return;
    const label = tab.textContent || '';
    if (!/\bACT\b/.test(label)) {
      state.selectedActivityDiagramId = null;
      clearActivityInteraction();
    }
  }, true);

  const originalNewProject = $('new-project')?.onclick;
  const originalOpenProject = $('open-project')?.onclick;
  const originalSaveProject = $('save-project')?.onclick;
  const originalSaveAs = $('save-project-as')?.onclick;

  if ($('new-project')) $('new-project').onclick = async () => {
    await originalNewProject?.();
    if (state.snapshot?.project) await requireInvoke()('reset_activity_workspace');
    state.activitySnapshot = { repository: { activities: {} }, diagrams: [] };
    state.selectedActivityDiagramId = null;
    render();
  };
  if ($('open-project')) $('open-project').onclick = async () => {
    await originalOpenProject?.();
    if (state.snapshot?.current_file) {
      await requireInvoke()('load_activity_workspace', { path: state.snapshot.current_file });
      await loadActivitySnapshot();
      render();
    }
  };
  async function saveActivityAfterBase() {
    if (state.snapshot?.current_file) {
      await requireInvoke()('save_activity_workspace', { path: state.snapshot.current_file });
      await loadActivitySnapshot();
      render();
    }
  }
  if ($('save-project')) $('save-project').onclick = async () => {
    await originalSaveProject?.();
    await saveActivityAfterBase();
  };
  if ($('save-project-as')) $('save-project-as').onclick = async () => {
    await originalSaveAs?.();
    await saveActivityAfterBase();
  };

  loadActivitySnapshot().then(render).catch((error) => console.error('Unable to initialize Activity workspace', error));
})();
