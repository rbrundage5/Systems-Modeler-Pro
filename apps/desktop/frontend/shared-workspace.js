/* Permanent renderer host for every current and future Rust-registered diagram family. */
(() => {
  'use strict';
  const invoke = window.__TAURI__?.core?.invoke;
  const canvas = document.getElementById('canvas');
  if (!canvas) return;
  const clamp = (value, min, max) => Math.max(min, Math.min(max, value));
  const renderers = new Map();
  const commands = new Map();
  const state = { context: null, viewport: null, surface: null, spacer: null, panning: null, space: false };

  function notify(message, level = 'info') {
    window.smpDialogs?.notify(message, level);
    const status = document.getElementById('status');
    if (status) { status.textContent = message; status.dataset.level = level; }
  }

  function registerRenderer(familyId, adapter) {
    if (!familyId || renderers.has(familyId)) throw new Error(`Renderer already registered: ${familyId}`);
    for (const method of ['selection', 'clearSelection', 'cancelInteraction']) {
      if (typeof adapter[method] !== 'function') throw new Error(`${familyId} renderer is missing ${method}`);
    }
    renderers.set(familyId, Object.freeze({ ...adapter, familyId }));
  }

  function renderer() { return state.context ? renderers.get(state.context.family.id) : null; }

  async function activate(input) {
    if (!invoke || !input?.diagramId) return null;
    await persistViewport();
    state.context = await invoke('activate_diagram', {
      diagramId: input.diagramId, familyId: input.familyId, name: input.name,
      semanticContextId: input.semanticContextId || '',
    });
    state.viewport = await invoke('get_viewport_preference', { diagramId: input.diagramId });
    await loadCommands();
    updateHeader();
    queueMicrotask(mountSurface);
    return state.context;
  }

  function updateHeader() {
    const context = state.context;
    document.getElementById('workspace-diagram-title').textContent = context?.name || 'No diagram selected';
    document.getElementById('workspace-diagram-context').textContent = context
      ? `${context.family.displayName} · ${context.semanticContextId || 'model context'}`
      : 'Select a diagram from the repository';
    canvas.setAttribute('aria-label', context?.family.accessibilityName || 'Diagram canvas');
    canvas.dataset.family = context?.family.id || '';
  }

  function mountSurface() {
    const root = [...canvas.children].find((node) => !node.classList.contains('workspace-transform-spacer'));
    if (!root || root === state.surface) { applyViewport(); return; }
    const spacer = document.createElement('div');
    spacer.className = 'workspace-transform-spacer';
    canvas.insertBefore(spacer, root);
    spacer.appendChild(root);
    state.surface = root; state.spacer = spacer;
    root.classList.add('workspace-renderer-surface');
    root.dataset.renderer = state.context?.family.rendererId || '';
    applyViewport();
  }

  function contentBounds() {
    const supplied = renderer()?.contentBounds?.();
    if (supplied && Number.isFinite(supplied.width) && Number.isFinite(supplied.height)) return supplied;
    const root = state.surface;
    if (!root) return { x: 0, y: 0, width: 720, height: 520 };
    return { x: 0, y: 0, width: Math.max(root.scrollWidth, root.offsetWidth, 720), height: Math.max(root.scrollHeight, root.offsetHeight, 520) };
  }

  function applyViewport() {
    if (!state.viewport) return;
    mountSurfaceIfNeeded();
    const root = state.surface, spacer = state.spacer;
    if (!root || !spacer) return;
    const bounds = contentBounds(), view = state.viewport;
    const left = Math.max(0, view.panX), top = Math.max(0, view.panY);
    root.style.transform = `translate(${left}px,${top}px) scale(${view.zoom})`;
    spacer.style.width = `${Math.ceil(left + (bounds.x + bounds.width) * view.zoom + 56)}px`;
    spacer.style.height = `${Math.ceil(top + (bounds.y + bounds.height) * view.zoom + 56)}px`;
    canvas.classList.toggle('grid-hidden', !view.gridVisible);
    canvas.dataset.zoom = String(view.zoom);
  }

  function mountSurfaceIfNeeded() {
    if (!state.surface?.isConnected) {
      state.surface = null; state.spacer = null;
      const root = [...canvas.children].find((node) => !node.classList.contains('workspace-transform-spacer'));
      if (root) {
        const spacer = document.createElement('div'); spacer.className = 'workspace-transform-spacer';
        canvas.insertBefore(spacer, root); spacer.appendChild(root); state.surface = root; state.spacer = spacer;
        root.classList.add('workspace-renderer-surface');
      }
    }
  }

  let persistTimer;
  function persistViewport() {
    if (!invoke || !state.context || !state.viewport) return Promise.resolve();
    clearTimeout(persistTimer);
    return invoke('set_viewport_preference', { diagramId: state.context.diagramId, preference: state.viewport })
      .catch((error) => notify(String(error), 'error'));
  }
  function scheduleViewportPersistence() { clearTimeout(persistTimer); persistTimer = setTimeout(persistViewport, 120); }

  function setZoom(next, clientX, clientY) {
    if (!state.viewport) return;
    const previous = state.viewport.zoom;
    next = clamp(next, .25, 4);
    const rect = canvas.getBoundingClientRect();
    const x = (clientX ?? rect.left + canvas.clientWidth / 2) - rect.left + canvas.scrollLeft;
    const y = (clientY ?? rect.top + canvas.clientHeight / 2) - rect.top + canvas.scrollTop;
    state.viewport.panX = x - (x - state.viewport.panX) * next / previous;
    state.viewport.panY = y - (y - state.viewport.panY) * next / previous;
    state.viewport.zoom = next;
    applyViewport(); scheduleViewportPersistence();
  }

  function fitDiagram() {
    if (!state.viewport) return;
    const bounds = contentBounds();
    state.viewport.zoom = clamp(Math.min((canvas.clientWidth - 56) / Math.max(bounds.width, 1), (canvas.clientHeight - 56) / Math.max(bounds.height, 1)), .25, 1);
    state.viewport.panX = 28 - bounds.x * state.viewport.zoom;
    state.viewport.panY = 28 - bounds.y * state.viewport.zoom;
    applyViewport(); scheduleViewportPersistence(); canvas.scrollTo(0, 0);
  }

  function clearSelection() { renderer()?.clearSelection(); canvas.dispatchEvent(new CustomEvent('smp:selection-changed')); }
  function cancelEverything() {
    window.smpDialogs?.cancelActive?.();
    renderer()?.cancelInteraction();
    state.panning = null; canvas.classList.remove('pan-active', 'is-panning');
    clearSelection();
  }

  const transientHandlers = {
    select: () => canvas.focus(), clearSelection, zoomIn: () => setZoom(state.viewport.zoom * 1.15),
    zoomOut: () => setZoom(state.viewport.zoom / 1.15), actualSize: () => setZoom(1), fitDiagram,
    pan: () => canvas.classList.toggle('pan-active'), toggleGrid: () => { state.viewport.gridVisible = !state.viewport.gridVisible; applyViewport(); scheduleViewportPersistence(); },
    snapGrid: () => { state.viewport.snapToGrid = !state.viewport.snapToGrid; scheduleViewportPersistence(); },
    undo: () => window.smpUndo?.(), redo: () => window.smpRedo?.(),
    showRepository: () => togglePanel('repository'), showElements: () => togglePanel('elements'), showProperties: () => togglePanel('properties'),
  };

  async function execute(id, args = {}) {
    const command = commands.get(id);
    if (!command) { notify(`Unknown command: ${id}`, 'error'); return false; }
    if (!command.enabled) { notify(command.disabledReason || `${command.label} is unavailable.`, 'warning'); return false; }
    const local = transientHandlers[id];
    if (local) { await local(args); return true; }
    if (!invoke || !command.rustAdapter) { notify(`${command.label} is unavailable in this context.`, 'warning'); return false; }
    await invoke(command.rustAdapter, { diagramId: state.context.diagramId, ...args });
    await renderer()?.refresh?.(); return true;
  }

  async function loadCommands() {
    const manifest = await invoke('active_diagram_command_manifest');
    commands.clear();
    manifest.forEach((command) => commands.set(command.id, command));
    document.dispatchEvent(new CustomEvent('smp:commands-ready', { detail: manifest }));
  }

  async function loadContracts() {
    if (!invoke) return;
    await Promise.all([loadCommands(), invoke('diagram_family_registry')]);
  }

  async function activateCurrentDiagram() {
    const application = window.smpState;
    if (!application || state.context) return;
    const structuralId = application.selectedDiagramId;
    const behaviorId = application.selectedBehaviorDiagramId;
    const activityId = application.selectedActivityDiagramId;
    if (activityId) {
      const diagram = application.activitySnapshot?.diagrams?.find((item) => String(item.id) === String(activityId));
      if (diagram) await activate({ diagramId:activityId, familyId:'activity', name:diagram.name, semanticContextId:diagram.activity_id || '' });
      return;
    }
    if (behaviorId) {
      const diagram = application.behaviorSnapshot?.diagrams?.find((item) => String(item.id) === String(behaviorId));
      if (diagram) await activate({ diagramId:behaviorId, familyId:diagram.kind === 'Sequence' ? 'sequence' : 'state-machine', name:diagram.name, semanticContextId:diagram.context_id || '' });
      return;
    }
    if (structuralId) {
      const ibd = application.snapshot?.ibd_diagrams?.find((item) => String(item.id) === String(structuralId));
      const bdd = application.snapshot?.diagrams?.find((item) => String(item.id) === String(structuralId));
      const diagram = ibd || bdd;
      if (diagram) await activate({ diagramId:structuralId, familyId:ibd ? 'ibd' : 'bdd', name:diagram.name, semanticContextId:ibd?.context_block_id || bdd?.owner_id || '' });
    }
  }

  function startPan(event) {
    if (!state.viewport || !(event.button === 1 || (event.button === 0 && (state.space || canvas.classList.contains('pan-active'))))) return false;
    event.preventDefault(); canvas.setPointerCapture(event.pointerId); canvas.classList.add('is-panning');
    state.panning = { pointerId:event.pointerId, x:event.clientX, y:event.clientY, panX:state.viewport.panX, panY:state.viewport.panY };
    return true;
  }
  canvas.addEventListener('pointerdown', (event) => {
    if (startPan(event)) return;
    if (event.target === canvas || event.target === state.spacer) clearSelection();
  });
  canvas.addEventListener('pointermove', (event) => {
    if (!state.panning || state.panning.pointerId !== event.pointerId) return;
    state.viewport.panX = state.panning.panX + event.clientX - state.panning.x;
    state.viewport.panY = state.panning.panY + event.clientY - state.panning.y;
    applyViewport();
  });
  function finishPan(event) { if (!state.panning || state.panning.pointerId !== event.pointerId) return; state.panning = null; canvas.classList.remove('is-panning'); scheduleViewportPersistence(); }
  canvas.addEventListener('pointerup', finishPan); canvas.addEventListener('pointercancel', finishPan);
  canvas.addEventListener('wheel', (event) => { if (!event.ctrlKey) return; event.preventDefault(); setZoom(state.viewport.zoom * (event.deltaY < 0 ? 1.1 : 1 / 1.1), event.clientX, event.clientY); }, { passive:false });
  document.addEventListener('keydown', (event) => {
    const editable = event.target.closest?.('input,textarea,select,[contenteditable="true"],[role="dialog"]');
    if (event.code === 'Space' && !editable) { state.space = true; canvas.classList.add('space-pan'); event.preventDefault(); }
    if (event.key === 'Escape') { event.preventDefault(); cancelEverything(); }
    if (editable) return;
    const shortcuts = { Delete:'delete', Backspace:'delete' };
    if (shortcuts[event.key]) { event.preventDefault(); void execute(shortcuts[event.key]); }
    if (event.ctrlKey && event.key === '0') { event.preventDefault(); void execute('actualSize'); }
    if (event.ctrlKey && event.key === '9') { event.preventDefault(); void execute('fitDiagram'); }
    if (event.ctrlKey && event.key.toLowerCase() === 'c') { event.preventDefault(); void execute('copy'); }
    if (event.ctrlKey && event.key.toLowerCase() === 'v') { event.preventDefault(); void execute('paste'); }
    if (event.ctrlKey && event.key.toLowerCase() === 'd') { event.preventDefault(); void execute('duplicate'); }
  }, true);
  document.addEventListener('keyup', (event) => { if (event.code === 'Space') { state.space = false; canvas.classList.remove('space-pan'); } });
  new MutationObserver(() => queueMicrotask(mountSurface)).observe(canvas, { childList:true });

  const panels = Object.freeze({
    repository: { selector: '.repository-panel', variable: '--repository-width', hiddenClass: 'hide-repository-panel', direction: 1 },
    elements: { selector: '.palette-panel', variable: '--elements-width', hiddenClass: 'hide-palette-panel', direction: 1 },
    properties: { selector: '.properties-panel', variable: '--properties-width', hiddenClass: 'hide-properties-panel', direction: -1 },
  });

  function setPanelVisibility(name, visible) {
    const descriptor = panels[name];
    const workspace = document.querySelector('.workspace');
    const panel = descriptor && document.querySelector(descriptor.selector);
    if (!descriptor || !workspace || !panel) return;
    panel.classList.toggle('shell-hidden', !visible);
    panel.setAttribute('aria-hidden', String(!visible));
    workspace.classList.toggle(descriptor.hiddenClass, !visible);
    document.dispatchEvent(new CustomEvent('smp:panel-visibility-changed', { detail: { name, visible } }));
  }

  async function togglePanel(name) {
    const descriptor = panels[name];
    const panel = descriptor && document.querySelector(descriptor.selector);
    if (!panel) return;
    setPanelVisibility(name, panel.classList.contains('shell-hidden'));
    await persistPanels();
  }
  function configuredWidth(workspace, name) {
    const descriptor = panels[name];
    const value = parseFloat(getComputedStyle(workspace).getPropertyValue(descriptor.variable));
    return Number.isFinite(value) ? value : document.querySelector(descriptor.selector)?.getBoundingClientRect().width || 0;
  }
  async function persistPanels() {
    if (!invoke) return;
    const workspace = document.querySelector('.workspace');
    await invoke('set_panel_preferences', { preference: {
      repositoryWidth:clamp(Math.round(configuredWidth(workspace, 'repository')),150,480), elementsWidth:clamp(Math.round(configuredWidth(workspace, 'elements')),150,480), propertiesWidth:clamp(Math.round(configuredWidth(workspace, 'properties')),150,480),
      repositoryVisible:!document.querySelector('.repository-panel')?.classList.contains('shell-hidden'), elementsVisible:!document.querySelector('.palette-panel')?.classList.contains('shell-hidden'), propertiesVisible:!document.querySelector('.properties-panel')?.classList.contains('shell-hidden'),
    }}).catch((error) => notify(String(error), 'error'));
  }

  async function initializePanels() {
    if (!invoke) return;
    const preference = await invoke('get_panel_preferences');
    const workspace = document.querySelector('.workspace');
    workspace.style.setProperty('--repository-width', `${preference.repositoryWidth}px`);
    workspace.style.setProperty('--elements-width', `${preference.elementsWidth}px`);
    workspace.style.setProperty('--properties-width', `${preference.propertiesWidth}px`);
    setPanelVisibility('repository', preference.repositoryVisible);
    setPanelVisibility('elements', preference.elementsVisible);
    setPanelVisibility('properties', preference.propertiesVisible);
    for (const [name, descriptor] of Object.entries(panels)) {
      const panel=document.querySelector(descriptor.selector); if(!panel || panel.querySelector('.panel-splitter')) continue;
      const splitter=document.createElement('button');
      splitter.type='button'; splitter.className=`panel-splitter ${descriptor.direction>0?'right':'left'}`;
      splitter.setAttribute('aria-label', `Resize ${name} panel`); splitter.title=`Drag to resize the ${name} panel`;
      panel.appendChild(splitter);
      const resizeTo = (width) => workspace.style.setProperty(descriptor.variable, `${clamp(Math.round(width),150,480)}px`);
      splitter.addEventListener('keydown', (event) => {
        if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
        event.preventDefault();
        const delta = (event.key === 'ArrowRight' ? 12 : -12) * descriptor.direction;
        resizeTo(configuredWidth(workspace, name) + delta); void persistPanels();
      });
      splitter.addEventListener('pointerdown',(down)=>{
        if (down.button !== 0) return;
        down.preventDefault(); splitter.setPointerCapture(down.pointerId); splitter.classList.add('dragging');
        const start=down.clientX,initial=configuredWidth(workspace, name);
        const move=(event)=>{ if (event.pointerId === down.pointerId) resizeTo(initial+(event.clientX-start)*descriptor.direction); };
        const up=(event)=>{ if (event.pointerId !== down.pointerId) return; splitter.classList.remove('dragging'); splitter.removeEventListener('pointermove',move); void persistPanels(); };
        splitter.addEventListener('pointermove',move); splitter.addEventListener('pointerup',up,{once:true}); splitter.addEventListener('pointercancel',up,{once:true});
      });
    }
  }

  window.smpRendererHost = Object.freeze({ registerRenderer, activate, execute, clearSelection, cancelEverything, togglePanel, context:() => state.context, contentBounds });
  const selectionAdapter = (selectionKeys, toolKeys) => ({
    selection: () => selectionKeys.map((key) => window.smpState?.[key]).filter(Boolean),
    clearSelection: () => { for (const key of selectionKeys) if (window.smpState) window.smpState[key] = null; },
    cancelInteraction: () => { for (const key of toolKeys) if (window.smpState) window.smpState[key] = null; },
    refresh: async () => { if (typeof window.refresh === 'function') await window.refresh(); },
  });
  registerRenderer('bdd', selectionAdapter(['selectedElementId','selectedRelationshipId'], ['paletteTool','pendingRelationship']));
  registerRenderer('ibd', selectionAdapter(['selectedElementId','selectedRelationshipId'], ['paletteTool','pendingRelationship']));
  registerRenderer('state-machine', selectionAdapter(['selectedBehaviorItem'], ['behaviorTool','behaviorPending','behaviorTargetRegionId']));
  registerRenderer('sequence', selectionAdapter(['selectedBehaviorItem'], ['behaviorTool','behaviorPending']));
  registerRenderer('activity', selectionAdapter(['selectedActivityNodeId','selectedActivityEdgeId'], ['activityTool','activityPendingFlow']));
  Promise.all([loadContracts(), initializePanels()]).then(activateCurrentDiagram).catch((error) => notify(`Workspace initialization failed: ${error}`, 'error'));
})();
