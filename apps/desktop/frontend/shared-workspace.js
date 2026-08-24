/* Permanent renderer host for every current and future Rust-registered diagram family. */
(() => {
  'use strict';
  const invoke = window.__TAURI__?.core?.invoke;
  const canvas = document.getElementById('canvas');
  if (!canvas) return;
  const clamp = (value, min, max) => Math.max(min, Math.min(max, value));
  const renderers = new Map();
  const commands = new Map();
  const state = { context: null, viewport: null, frame: null, frameElement: null, surface: null, spacer: null, panning: null, frameDrag: null, space: false, suppressPanClick:false }; const validFrame = (value) => value && ['x','y','width','height'].every((key) => Number.isFinite(value[key]));

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
    await persistViewport(); clearTimeout(framePersistTimer);
    const activated = await invoke('activate_diagram', {
      diagramId: input.diagramId, familyId: input.familyId, name: input.name, modelElementName:input.modelElementName||input.name,
      semanticContextId: input.semanticContextId || '',
    });
    state.context = activated.context; interaction = activated.interaction; applyCommands(activated.commands);
    state.viewport = await invoke('get_viewport_preference', { diagramId: input.diagramId });
    const storedFrame=await invoke('get_diagram_frame_preference',{diagramId:input.diagramId}); Object.assign(state,{frame:validFrame(storedFrame)?storedFrame:null});
    updateHeader();
    queueMicrotask(mountSurface);
    return state.context;
  }

  function updateHeader() {
    const context=state.context;
    document.getElementById('workspace-diagram-title').textContent = context ? '' : 'No diagram selected';
    const contextLabel=document.getElementById('workspace-diagram-context');contextLabel.textContent=context?.family.displayName||'Select a diagram from the repository';contextLabel.title=context?.semanticContextId?`Semantic context: ${context.semanticContextId}`:'';
    canvas.setAttribute('aria-label',context?.family.accessibilityName||'Diagram canvas'); canvas.dataset.family=context?.family.id||''; document.getElementById('workspace-header').dataset.family=context?.family.id||'';
  }
  function mountSurface() {
    const root = [...canvas.children].find((node) => !node.classList.contains('workspace-transform-spacer'));
    if (!root) return; if(root===state.surface){if(!state.frameElement?.isConnected||state.frameElement.dataset.diagramId!==state.context?.diagramId)mountDiagramFrame();applyViewport();return;}
    const spacer=document.createElement('div'); spacer.className='workspace-transform-spacer'; canvas.insertBefore(spacer,root); spacer.appendChild(root);
    state.surface = root; state.spacer = spacer;
    root.classList.add('workspace-renderer-surface'); root.dataset.renderer=state.context?.family.rendererId||'';
    mountDiagramFrame(); applyViewport();
  }

  function automaticFrame() { const bounds=contentBounds(), padding=42; return { x:Math.max(0,bounds.x-padding), y:Math.max(0,bounds.y-padding), width:Math.max(320,bounds.width+padding*2), height:Math.max(240,bounds.height+padding*2), manuallySized:false }; }

  function mountDiagramFrame() { if(!state.spacer||!state.context)return; state.frameElement?.remove(); const frame=document.createElement('section'); frame.className='sysml-diagram-frame'; frame.dataset.diagramId=state.context.diagramId; frame.setAttribute('aria-label',state.context.frameLabel); frame.innerHTML=`<header class="sysml-frame-label" tabindex="0" title="Double-click or press Enter to edit diagram names"><span></span></header><button type="button" class="sysml-frame-resize" aria-label="Resize diagram frame" title="Drag to resize diagram frame"></button>`; const label=frame.querySelector('.sysml-frame-label');label.querySelector('span').textContent=state.context.frameLabel;label.ondblclick=()=>void editFrameHeader();label.onkeydown=(event)=>{if(event.key==='Enter'||event.key==='F2'){event.preventDefault();void editFrameHeader();}};state.spacer.insertBefore(frame,state.surface); Object.assign(state,{frameElement:frame,frame:validFrame(state.frame)?state.frame:automaticFrame()}); applyDiagramFrame(); }
  async function editFrameHeader(){if(!invoke||!state.context)return;const result=await window.smpDialogs?.edit({title:'Edit SysML diagram header',description:'The diagram kind and model-element type are controlled by the SysML family.',fields:[{id:'modelElementName',label:`${state.context.family.frameModelElementType} name`,value:state.context.modelElementName,required:true},{id:'diagramName',label:'Diagram name',value:state.context.name,required:true}],confirmLabel:'Apply'});if(!result)return;const context=await invoke('rename_active_diagram_header',{diagramId:state.context.diagramId,modelElementName:result.values.modelElementName,diagramName:result.values.diagramName});Object.assign(state,{context});mountDiagramFrame();await renderer()?.refresh?.();notify('Diagram header updated.','info');}
  function applyDiagramFrame() { const frame=state.frameElement,geometry=state.frame; if(frame&&geometry)Object.assign(frame.style,{left:`${geometry.x}px`,top:`${geometry.y}px`,width:`${geometry.width}px`,height:`${geometry.height}px`}); }
  let framePersistTimer; function persistDiagramFrame(diagramId=state.context?.diagramId,preference=state.frame) { if(!invoke||!diagramId||!validFrame(preference))return Promise.resolve(); clearTimeout(framePersistTimer); return invoke('set_diagram_frame_preference',{diagramId,preference}).catch((error)=>notify(String(error),'error')); }
  function scheduleFramePersistence() { clearTimeout(framePersistTimer); const diagramId=state.context?.diagramId,preference=state.frame?{...state.frame}:null; framePersistTimer=setTimeout(()=>persistDiagramFrame(diagramId,preference),120); }

  function contentBounds() {
    const supplied=renderer()?.contentBounds?.();
    if (supplied && Number.isFinite(supplied.width) && Number.isFinite(supplied.height)) return supplied;
    const root=state.surface;if(root?.getBBox){try{const box=root.getBBox();if(box.width>0&&box.height>0)return{x:box.x,y:box.y,width:box.width,height:box.height};}catch{}} return root?{x:0,y:0,width:Math.max(root.scrollWidth,root.offsetWidth,320),height:Math.max(root.scrollHeight,root.offsetHeight,240)}:{x:0,y:0,width:320,height:240};
  }

  function applyViewport() {
    if(!state.viewport)return; mountSurfaceIfNeeded(); const root=state.surface,spacer=state.spacer;
    if (!root || !spacer) return;
    const bounds=contentBounds(),view=state.viewport;
    if (state.frame&&!state.frame.manuallySized) { Object.assign(state,{frame:automaticFrame()}); applyDiagramFrame(); }
    const transform=`scale(${view.zoom})`; root.style.transform=transform;if(state.frameElement){state.frameElement.style.transform=transform;state.frameElement.style.transformOrigin='0 0';}
    const frameRight=state.frame?state.frame.x+state.frame.width:bounds.x+bounds.width, frameBottom=state.frame?state.frame.y+state.frame.height:bounds.y+bounds.height;
    spacer.style.width = `${Math.ceil(Math.max(bounds.x + bounds.width, frameRight) * view.zoom + 56)}px`;
    spacer.style.height = `${Math.ceil(Math.max(bounds.y + bounds.height, frameBottom) * view.zoom + 56)}px`;
    if(!state.panning)canvas.scrollTo(view.panX,view.panY);
    canvas.classList.toggle('grid-hidden',!view.gridVisible); canvas.dataset.zoom=String(view.zoom);
  }

  function mountSurfaceIfNeeded() {
    if (!state.surface?.isConnected) {
      state.surface = null; state.spacer = null;
      const root = [...canvas.children].find((node) => !node.classList.contains('workspace-transform-spacer'));
      if (root) {
        const spacer = document.createElement('div'); spacer.className = 'workspace-transform-spacer';
        canvas.insertBefore(spacer, root); spacer.appendChild(root); state.surface = root; state.spacer = spacer;
        root.classList.add('workspace-renderer-surface');
        mountDiagramFrame();
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

  let zoomRequest = Promise.resolve();
  function setZoom(next, clientX, clientY, relative = false) {
    if (!state.viewport || !invoke) return Promise.resolve();
    const rect = canvas.getBoundingClientRect();
    const x = (clientX ?? rect.left + canvas.clientWidth / 2) - rect.left;
    const y = (clientY ?? rect.top + canvas.clientHeight / 2) - rect.top;
    zoomRequest = zoomRequest.then(async () => {
      const requestedZoom = relative ? state.viewport.zoom * next : next;
      state.viewport = await invoke('zoom_diagram_viewport', {
        current: state.viewport, requestedZoom, pointerX: x, pointerY: y,
      });
      applyViewport(); scheduleViewportPersistence();
    }).catch((error) => notify(String(error), 'error'));
    return zoomRequest;
  }

  async function fitDiagram() {
    if (!state.viewport || !invoke) return;
    state.viewport = await invoke('fit_diagram_viewport', {
      bounds: contentBounds(), viewportWidth: canvas.clientWidth, viewportHeight: canvas.clientHeight,
      padding: 28, current: state.viewport,
    });
    applyViewport(); scheduleViewportPersistence();
  }
  function interactionPayload() {
    const adapter = renderer();
    const selections = (adapter?.selection?.() || []).map((selection, index) => {
      if (selection && typeof selection === 'object') {
        return { kind: String(selection.type || selection.kind || `selection-${index}`), id: String(selection.id || selection.semantic?.id || '') };
      }
      return { kind: `selection-${index}`, id: String(selection || '') };
    }).filter((selection) => selection.id);
    const tool = adapter?.activeTool?.();
    const activeTool = tool && typeof tool === 'object' ? String(tool.id || tool.kind || tool.relationship_kind || tool.element_kind || tool.type || 'pending-tool') : (tool == null ? null : String(tool));
    return { selections, activeTool };
  }
  let interactionRequest = Promise.resolve(), interaction = null;
  function queueInteraction(command, args) {
    interactionRequest = interactionRequest.then(async () => {
      const request = () => invoke(command, { diagramId:state.context.diagramId, ...args, expectedRevision:interaction?.revision ?? null });
      try { interaction = await request(); }
      catch (error) { interaction = await invoke('workspace_interaction_snapshot'); if (!String(error).includes('revision conflict')) throw error; interaction = await request(); }
      return interaction;
    }).catch((error) => { notify(`Unable to synchronize workspace interaction: ${error}`, 'error'); return interaction; });
    return interactionRequest;
  }
  function publishInteraction() {
    if (!invoke || !state.context) return Promise.resolve(null);
    const payload = interactionPayload();
    return queueInteraction('set_workspace_interaction', { selections:payload.selections, activeTool:payload.activeTool });
  }
  async function clearRendererSelections(cancelTools = false) {
    for (const adapter of renderers.values()) { adapter.clearSelection(); if (cancelTools) adapter.cancelInteraction(); }
    canvas.dispatchEvent(new CustomEvent('smp:selection-changed')); await renderer()?.refresh?.();
  }
  async function clearSelection() {
    await clearRendererSelections();
    if (invoke && state.context) await queueInteraction('clear_workspace_interaction', { cancelTool:false });
  }
  async function cancelEverything() {
    window.smpDialogs?.cancelActive?.();
    state.panning = null; canvas.classList.remove('pan-active', 'is-panning');
    await clearRendererSelections(true);
    if (invoke && state.context) await queueInteraction('clear_workspace_interaction', { cancelTool:true });
  }
  const transientHandlers = {
    select: () => canvas.focus(), clearSelection, zoomIn: () => setZoom(1.15, undefined, undefined, true),
    zoomOut: () => setZoom(1 / 1.15, undefined, undefined, true), actualSize: () => setZoom(1), fitDiagram,
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
    try {
      await publishInteraction();
      const committedArgs=(id==='route'||id==='cleanLayout')?{framePreference:state.frame,...args}:args;
      await invoke(command.rustAdapter, { diagramId: state.context.diagramId, ...committedArgs });
      // Use the same authoritative refresh path for every family. The Behavior
      // refresh wrapper hydrates STM/Sequence state before render, while the
      // common path also refreshes command bindings, frame bounds, and pointer
      // interactions after Route or Clean Layout changes node geometry.
      await renderer()?.refresh?.();
      if (id === 'route' || id === 'cleanLayout') notify(`${command.label} completed.`, 'info');
      return true;
    } catch (error) {
      notify(`${command.label} failed: ${String(error)}`, 'error');
      return false;
    }
  }

  async function loadCommands() {
    const manifest = await invoke('active_diagram_command_manifest');
    applyCommands(manifest);
  }
  function applyCommands(manifest) {
    commands.clear();
    manifest.forEach((command) => commands.set(command.id, command));
    document.dispatchEvent(new CustomEvent('smp:commands-ready', { detail: manifest }));
  }

  async function loadContracts() {
    if (!invoke) return;
    const [, , stylesheet] = await Promise.all([
      loadCommands(), invoke('diagram_family_registry'), invoke('semantic_presentation_stylesheet'),
    ]);
    let style = document.getElementById('rust-semantic-presentation');
    if (!style) { style = document.createElement('style'); style.id = 'rust-semantic-presentation'; document.head.appendChild(style); }
    style.textContent = stylesheet;
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
      if (diagram) await activate({ diagramId:structuralId, familyId:ibd ? 'ibd' : (bdd?.family || 'bdd'), name:diagram.name, semanticContextId:ibd?.context_block_id || bdd?.owner_id || '' });
    }
  }

  function startPan(event) {
    if (!state.viewport || !(event.button === 1 || (event.button === 0 && (state.space || event.ctrlKey || event.metaKey || canvas.classList.contains('pan-active'))))) return false;
    event.preventDefault(); event.stopImmediatePropagation(); canvas.setPointerCapture(event.pointerId); canvas.classList.add('is-panning');
    state.panning = { pointerId:event.pointerId, x:event.clientX, y:event.clientY, left:canvas.scrollLeft, top:canvas.scrollTop, moved:false };
    return true;
  }
  canvas.addEventListener('pointerdown', (event) => {
    const frameControl=event.target.closest?.('.sysml-frame-label,.sysml-frame-resize'); if(frameControl&&state.frame){event.preventDefault();event.stopPropagation();const resizing=frameControl.classList.contains('sysml-frame-resize');Object.assign(state,{frameDrag:{pointerId:event.pointerId,x:event.clientX,y:event.clientY,start:{...state.frame},resizing}});frameControl.setPointerCapture(event.pointerId);state.frameElement.classList.add(resizing?'is-resizing':'is-moving');return;}
    if (startPan(event)) return;
    if (event.target === canvas || event.target === state.spacer) clearSelection();
  }, true);
  canvas.addEventListener('pointermove', (event) => {
    if(state.frameDrag?.pointerId===event.pointerId){const dx=(event.clientX-state.frameDrag.x)/(state.viewport?.zoom||1),dy=(event.clientY-state.frameDrag.y)/(state.viewport?.zoom||1);Object.assign(state,{frame:state.frameDrag.resizing?{...state.frameDrag.start,width:Math.max(320,state.frameDrag.start.width+dx),height:Math.max(240,state.frameDrag.start.height+dy),manuallySized:true}:{...state.frameDrag.start,x:Math.max(0,state.frameDrag.start.x+dx),y:Math.max(0,state.frameDrag.start.y+dy),manuallySized:true}});applyDiagramFrame();return;}
    if (!state.panning || state.panning.pointerId !== event.pointerId) return;
    const dx=event.clientX-state.panning.x,dy=event.clientY-state.panning.y;canvas.scrollLeft=state.panning.left-dx;canvas.scrollTop=state.panning.top-dy;state.panning.moved||=Math.hypot(dx,dy)>3;
  });
  function finishPan(event) { if(state.frameDrag?.pointerId===event?.pointerId){Object.assign(state,{frameDrag:null});state.frameElement?.classList.remove('is-moving','is-resizing');scheduleFramePersistence();applyViewport();void renderer()?.refresh?.();return;} if(!state.panning||(event?.pointerId!==undefined&&state.panning.pointerId!==event.pointerId))return;Object.assign(state.viewport,{panX:canvas.scrollLeft,panY:canvas.scrollTop});Object.assign(state,{suppressPanClick:state.panning.moved,panning:null});canvas.classList.remove('is-panning');scheduleViewportPersistence(); }
  canvas.addEventListener('pointerup', finishPan); canvas.addEventListener('pointercancel', finishPan);canvas.addEventListener('lostpointercapture',finishPan);window.addEventListener('blur',()=>{state.space=false;canvas.classList.remove('space-pan');finishPan();});
  canvas.addEventListener('click',(event)=>{if(!state.suppressPanClick)return;Object.assign(state,{suppressPanClick:false});event.preventDefault();event.stopImmediatePropagation();},true);
  canvas.addEventListener('wheel', (event) => { if (!event.ctrlKey) return; event.preventDefault(); void setZoom(event.deltaY < 0 ? 1.1 : 1 / 1.1, event.clientX, event.clientY, true); }, { passive:false });
  window.addEventListener('keydown', (event) => {
    const editable = event.target.closest?.('input,textarea,select,[contenteditable="true"],[role="dialog"]');
    if (event.code === 'Space' && !editable) { state.space = true; canvas.classList.add('space-pan'); event.preventDefault(); }
    if (event.key === 'Escape') { event.preventDefault(); event.stopImmediatePropagation(); void cancelEverything(); }
    if (editable) return;
    if (window.smpRepositoryEditing?.handleDelete?.(event)) return;
    const shortcuts = { Delete:'delete', Backspace:'delete' };
    if (shortcuts[event.key]) { event.preventDefault(); void execute(shortcuts[event.key]); }
    if (event.ctrlKey && event.key === '0') { event.preventDefault(); void execute('actualSize'); }
    if (event.ctrlKey && event.key === '9') { event.preventDefault(); void execute('fitDiagram'); }
    if (event.ctrlKey && event.key.toLowerCase() === 'c') { event.preventDefault(); void execute('copy'); }
    if (event.ctrlKey && event.key.toLowerCase() === 'v') { event.preventDefault(); void execute('paste'); }
    if (event.ctrlKey && event.key.toLowerCase() === 'd') { event.preventDefault(); void execute('duplicate'); }
  }, true);
  document.addEventListener('keyup', (event) => { if (event.code === 'Space') { state.space = false; canvas.classList.remove('space-pan'); } });
  let interactionSyncTimer;
  new MutationObserver(() => {
    queueMicrotask(mountSurface);
    clearTimeout(interactionSyncTimer);
    interactionSyncTimer = setTimeout(() => { void publishInteraction(); }, 0);
  }).observe(canvas, { childList:true, subtree:true });

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

  window.smpRendererHost = Object.freeze({ registerRenderer, activate, execute, clearSelection, cancelEverything, publishInteraction, togglePanel, context:() => state.context, contentBounds, frameGeometry:() => state.frame&&{...state.frame} });
  const selectionAdapter = (selectionKeys, toolKeys) => ({
    selection: () => {
      const standard = window.smpStandardEditing?.selections?.() || window.smpStandardSelections;
      if (Array.isArray(standard) && standard.length) return standard;
      return selectionKeys.map((key) => window.smpState?.[key]).filter(Boolean);
    },
    clearSelection: () => {
      if (window.smpStandardEditing?.setSelections) window.smpStandardEditing.setSelections([]);
      else window.smpStandardSelections = [];
      for (const key of selectionKeys) if (window.smpState) window.smpState[key] = null;
    },
    activeTool: () => toolKeys.map((key) => window.smpState?.[key]).find(Boolean) || null,
    cancelInteraction: () => { for (const key of toolKeys) if (window.smpState) window.smpState[key] = null; },
    refresh: async () => { if (typeof window.refresh === 'function') await window.refresh(); },
  });
  registerRenderer('bdd', selectionAdapter(['selectedElementId','selectedRelationshipId'], ['paletteTool','pendingRelationship']));
  registerRenderer('ibd', selectionAdapter(['selectedElementId','selectedRelationshipId'], ['paletteTool','pendingRelationship']));
  registerRenderer('requirement', selectionAdapter(['selectedElementId','selectedRelationshipId'], ['paletteTool','pendingRelationship']));
  registerRenderer('state-machine', selectionAdapter(['selectedBehaviorItem'], ['behaviorTool','behaviorPending','behaviorTargetRegionId']));
  registerRenderer('sequence', selectionAdapter(['selectedBehaviorItem'], ['behaviorTool','behaviorPending']));
  registerRenderer('activity', selectionAdapter(['selectedActivityNodeId','selectedActivityEdgeId'], ['activityTool','activityPendingFlow']));
  Promise.all([loadContracts(), initializePanels()]).then(activateCurrentDiagram).catch((error) => notify(`Workspace initialization failed: ${error}`, 'error'));
})();
