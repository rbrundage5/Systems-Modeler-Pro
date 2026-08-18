(() => {
  state.behaviorSnapshot = null;
  state.selectedBehaviorDiagramId = null;
  state.selectedBehaviorItem = null;
  state.behaviorTool = null;
  state.behaviorPending = null;

  function behaviorDiagram() {
    return state.behaviorSnapshot?.diagrams?.find((diagram) => diagram.id === state.selectedBehaviorDiagramId) || null;
  }

  function behaviorRepository() { return state.behaviorSnapshot?.repository || null; }
  function projectElement(id) { return state.snapshot?.project?.elements?.find((element) => element.id === id); }

  function stateMachine(diagram = behaviorDiagram()) {
    if (!diagram || diagram.kind !== 'StateMachine') return null;
    return behaviorRepository()?.state_machines?.[diagram.semantic_id] || null;
  }

  function interaction(diagram = behaviorDiagram()) {
    if (!diagram || diagram.kind !== 'Sequence') return null;
    return behaviorRepository()?.interactions?.[diagram.semantic_id] || null;
  }

  function allStateVertices(regions, output = []) {
    for (const region of regions || []) {
      for (const vertex of region.vertices || []) {
        output.push({ vertex, region });
        if (vertex.kind?.State?.regions) allStateVertices(vertex.kind.State.regions, output);
      }
    }
    return output;
  }

  function vertexKindName(vertex) {
    if (!vertex) return '';
    if (vertex.kind === 'FinalState' || vertex.kind?.FinalState != null) return 'FinalState';
    if (vertex.kind?.State != null || vertex.kind === 'State') return 'State';
    const pseudo = vertex.kind?.Pseudostate;
    return typeof pseudo === 'string' ? pseudo : pseudo || '';
  }

  function rootRegion(machine) { return machine?.regions?.[0] || null; }

  async function loadBehaviorSnapshot() {
    try { state.behaviorSnapshot = await requireInvoke()('behavior_snapshot'); }
    catch (error) { console.error('Unable to load behavior workspace', error); state.behaviorSnapshot = { repository: { state_machines: {}, interactions: {} }, diagrams: [] }; }
  }

  const baseRefreshBehavior = refresh;
  refresh = async function refreshBehavior() {
    const selected = state.selectedBehaviorDiagramId;
    await baseRefreshBehavior();
    await loadBehaviorSnapshot();
    if (selected && state.behaviorSnapshot.diagrams.some((diagram) => diagram.id === selected)) state.selectedBehaviorDiagramId = selected;
    render();
  };

  async function selectBehaviorDiagram(id) {
    state.selectedBehaviorDiagramId = id;
    state.selectedDiagramId = null;
    state.selectedElementId = null;
    state.selectedRelationshipId = null;
    state.selectedBehaviorItem = null;
    state.behaviorTool = null;
    state.behaviorPending = null;
    render();
  }

  function selectedClassifier() {
    const id = state.selectedElementId;
    const element = projectElement(id);
    if (element && ['Block', 'AssociationBlock', 'InterfaceBlock'].includes(element.kind)) return element;
    return null;
  }

  async function createBehaviorDiagram(kind) {
    const context = selectedClassifier();
    if (!context) {
      alert(`Select a Block (or other Block-like classifier) in the Model Repository first, then create the ${kind === 'StateMachine' ? 'State Machine' : 'Sequence'} diagram.`);
      return;
    }
    const defaultName = kind === 'StateMachine' ? `${context.name} State Machine` : `${context.name} Sequence`;
    const name = prompt(`${kind === 'StateMachine' ? 'State Machine' : 'Sequence'} diagram name`, defaultName);
    if (!name) return;
    const command = kind === 'StateMachine' ? 'create_state_machine_diagram' : 'create_sequence_diagram';
    const id = await runCommand(`Creating ${kind === 'StateMachine' ? 'State Machine' : 'Sequence'} diagram…`, () => requireInvoke()(command, { contextId: context.id, name }));
    await loadBehaviorSnapshot();
    await selectBehaviorDiagram(id);
    $('status').textContent = `${kind === 'StateMachine' ? 'State Machine' : 'Sequence'} created for ${context.name}`;
  }

  function installBehaviorRibbon() {
    const group = document.querySelector('.ribbon-group .ribbon-actions');
    if (!group || $('new-state-machine')) return;
    const stateButton = document.createElement('button');
    stateButton.id = 'new-state-machine';
    stateButton.className = 'ribbon-command';
    stateButton.innerHTML = '<span class="command-icon">◉</span><span>State Machine</span>';
    stateButton.onclick = () => createBehaviorDiagram('StateMachine');
    const sequenceButton = document.createElement('button');
    sequenceButton.id = 'new-sequence';
    sequenceButton.className = 'ribbon-command';
    sequenceButton.innerHTML = '<span class="command-icon">⇥</span><span>Sequence</span>';
    sequenceButton.onclick = () => createBehaviorDiagram('Sequence');
    group.append(stateButton, sequenceButton);
  }

  const STATE_PALETTE = [
    ['State', 'State'], ['Initial', 'Initial'], ['FinalState', 'Final State'], ['Choice', 'Choice'], ['Junction', 'Junction'],
    ['Fork', 'Fork'], ['Join', 'Join'], ['ShallowHistory', 'Shallow History'], ['DeepHistory', 'Deep History'],
    ['EntryPoint', 'Entry Point'], ['ExitPoint', 'Exit Point'], ['Terminate', 'Terminate'], ['Transition', 'Transition'],
  ];
  const SEQUENCE_PALETTE = [
    ['Lifeline', 'Lifeline'], ['SynchCall', 'Synchronous Call'], ['AsynchCall', 'Asynchronous Call'], ['AsynchSignal', 'Asynchronous Signal'],
    ['Reply', 'Reply'], ['Create', 'Create Message'], ['Delete', 'Delete Message'], ['Lost', 'Lost Message'], ['Found', 'Found Message'],
    ['Execution', 'Execution Specification'], ['alt', 'alt Fragment'], ['opt', 'opt Fragment'], ['loop', 'loop Fragment'], ['par', 'par Fragment'],
    ['break', 'break Fragment'], ['critical', 'critical Fragment'], ['neg', 'neg Fragment'], ['assert', 'assert Fragment'], ['strict', 'strict Fragment'],
    ['seq', 'seq Fragment'], ['ignore', 'ignore Fragment'], ['consider', 'consider Fragment'], ['Invariant', 'State Invariant'],
  ];

  const baseRenderDiagramTabsBehavior = renderDiagramTabs;
  renderDiagramTabs = function renderDiagramTabsBehavior() {
    baseRenderDiagramTabsBehavior();
    const host = $('diagram-tabs');
    for (const diagram of state.behaviorSnapshot?.diagrams || []) {
      const tab = document.createElement('button');
      tab.className = 'diagram-tab';
      if (diagram.id === state.selectedBehaviorDiagramId) tab.classList.add('active');
      tab.textContent = `${diagram.name} · ${diagram.kind === 'StateMachine' ? 'STM' : 'SEQ'}`;
      tab.onclick = () => selectBehaviorDiagram(diagram.id);
      host.appendChild(tab);
    }
  };

  const baseRenderRepositoryBehavior = renderRepository;
  renderRepository = function renderRepositoryBehavior() {
    baseRenderRepositoryBehavior();
    const host = $('repository');
    const diagrams = state.behaviorSnapshot?.diagrams || [];
    if (!diagrams.length) return;
    for (const diagram of diagrams) {
      if (state.repositoryFilter && !diagram.name.toLowerCase().includes(state.repositoryFilter.toLowerCase())) continue;
      const row = document.createElement('button');
      row.className = 'tree-row diagram-row';
      if (diagram.id === state.selectedBehaviorDiagramId) row.classList.add('selected');
      const tag = diagram.kind === 'StateMachine' ? 'STM' : 'SEQ';
      row.innerHTML = `<span class="kind">${diagram.kind === 'StateMachine' ? '◉' : '⇥'}</span><span>${escapeHtml(diagram.name)}</span><span class="type-tag">${tag}</span>`;
      row.onclick = () => selectBehaviorDiagram(diagram.id);
      host.appendChild(row);
    }
  };

  const baseRenderContextBehavior = renderContext;
  renderContext = function renderContextBehavior() {
    const diagram = behaviorDiagram();
    if (!diagram) return baseRenderContextBehavior();
    const label = diagram.kind === 'StateMachine' ? 'State Machine' : 'Sequence';
    $('active-diagram-summary').textContent = `${diagram.name} · ${label}`;
    $('palette-title').textContent = `${label} Palette`;
  };

  const baseRenderStatusBehavior = renderStatus;
  renderStatus = function renderStatusBehavior(message) {
    const diagram = behaviorDiagram();
    if (!diagram) return baseRenderStatusBehavior(message);
    if (message) { $('status').textContent = message; return; }
    if (state.behaviorPending?.kind === 'Transition') $('status').textContent = state.behaviorPending.source ? 'Transition: source selected; click target vertex.' : 'Transition: click source vertex, then target vertex.';
    else if (state.behaviorPending?.kind === 'Message') $('status').textContent = state.behaviorPending.source ? `${state.behaviorPending.sort}: source selected; click target Lifeline.` : `${state.behaviorPending.sort}: click source Lifeline, then target Lifeline.`;
    else if (state.behaviorTool) $('status').textContent = `${state.behaviorTool}: click the active ${diagram.kind === 'StateMachine' ? 'State Machine' : 'Sequence'} diagram.`;
    else $('status').textContent = `${state.snapshot.project.name} · ${diagram.kind === 'StateMachine' ? 'State Machine' : 'Sequence'}: ${diagram.name}`;
    $('model-counts').textContent = `Elements: ${state.snapshot.project.elements.length}   Relationships: ${state.snapshot.project.relationships.length}   Diagram: ${diagram.name}`;
  };

  function activateBehaviorTool(id) {
    state.selectedBehaviorItem = null;
    if (id === 'Transition') { state.behaviorTool = null; state.behaviorPending = { kind: 'Transition', source: null }; }
    else if (['SynchCall','AsynchCall','AsynchSignal','Reply','Create','Delete','Lost','Found'].includes(id)) { state.behaviorTool = null; state.behaviorPending = { kind: 'Message', sort: id, source: null }; }
    else { state.behaviorPending = null; state.behaviorTool = id; }
    render();
  }

  const baseRenderPaletteBehavior = renderPalette;
  renderPalette = function renderPaletteBehavior() {
    const diagram = behaviorDiagram();
    if (!diagram) return baseRenderPaletteBehavior();
    const host = $('palette'); host.innerHTML = '';
    const items = diagram.kind === 'StateMachine' ? STATE_PALETTE : SEQUENCE_PALETTE;
    const hint = document.createElement('div'); hint.className = 'palette-hint behavior-palette-hint';
    hint.textContent = diagram.kind === 'StateMachine'
      ? 'State semantics, Regions, Transition triggers/guards/effects, and validation are owned by Rust. Select Transition then click source and target.'
      : 'Lifelines represent real property paths. Message ordering/signatures and fragment semantics are owned by Rust.';
    host.appendChild(hint);
    const section = document.createElement('section'); section.className = 'palette-section';
    section.innerHTML = `<div class="palette-section-title">${diagram.kind === 'StateMachine' ? 'States & Transitions' : 'Interaction Elements'}</div>`;
    for (const [id, label] of items) {
      const button = document.createElement('button'); button.className = 'palette-item element';
      const active = state.behaviorTool === id || state.behaviorPending?.sort === id || (id === 'Transition' && state.behaviorPending?.kind === 'Transition');
      if (active) button.classList.add('active');
      button.innerHTML = `<span class="palette-symbol">${id === 'Transition' ? '→' : id === 'Lifeline' ? '┆' : id === 'State' ? '▢' : '•'}</span><span>${escapeHtml(label)}</span>`;
      button.onclick = () => activateBehaviorTool(id);
      section.appendChild(button);
    }
    host.appendChild(section);
  };

  function transitionLabel(transition) {
    let trigger = '';
    const event = transition.trigger?.event;
    if (event?.Signal) trigger = projectElement(event.Signal.signal_id)?.name || 'signal';
    else if (event?.Call) trigger = projectElement(event.Call.operation_id)?.name || 'operation';
    else if (event?.Time) trigger = `after(${event.Time.expression})`;
    else if (event?.Change) trigger = `when(${event.Change.expression})`;
    else if (event === 'AnyReceive' || event?.AnyReceive != null) trigger = 'all';
    const guard = transition.guard ? ` [${transition.guard}]` : '';
    const effect = transition.effect ? ` / ${transition.effect}` : '';
    return `${trigger}${guard}${effect}`.trim();
  }

  function stateNodePresentation(diagram, vertexId) { return diagram.state_nodes.find((node) => node.vertex_id === vertexId); }

  async function createStateVertexAt(diagram, x, y) {
    const kind = state.behaviorTool;
    if (!kind || kind === 'Transition') return;
    const name = kind === 'State' ? prompt('State name', 'State') : '';
    if (kind === 'State' && !name) return;
    await runCommand(`Creating ${kind}…`, () => requireInvoke()('add_state_vertex', { diagramId: diagram.id, regionIdValue: null, kind, name: name || '', x, y }));
    state.behaviorTool = null;
    await refresh();
  }

  async function commitTransition(diagram, targetId) {
    if (!state.behaviorPending?.source) { state.behaviorPending.source = targetId; render(); return; }
    const source = state.behaviorPending.source;
    state.behaviorPending = null;
    const eventKind = prompt('Trigger type: None, Signal, Call, Time, Change, AnyReceive', 'None');
    if (eventKind === null) { render(); return; }
    let eventReferenceId = null; let eventExpression = null;
    if (eventKind === 'Signal' || eventKind === 'Call') {
      const requiredKind = eventKind === 'Signal' ? 'Signal' : 'Operation';
      const candidates = state.snapshot.project.elements.filter((element) => element.kind === requiredKind);
      if (!candidates.length) { alert(`Create a ${requiredKind} first, then use it as the transition trigger.`); render(); return; }
      const menu = candidates.map((item, index) => `${index + 1}. ${item.name}`).join('\n');
      const answer = prompt(`Choose ${requiredKind}:\n${menu}`, '1');
      const selected = candidates[Number(answer) - 1]; if (!selected) { render(); return; } eventReferenceId = selected.id;
    } else if (eventKind === 'Time' || eventKind === 'Change') {
      eventExpression = prompt(eventKind === 'Time' ? 'Time expression' : 'Change expression', eventKind === 'Time' ? '5 s' : 'temperature > limit');
      if (!eventExpression) { render(); return; }
    }
    const guard = prompt('Guard condition (optional; do not include brackets)', '') ?? '';
    const effect = prompt('Effect behavior/expression (optional)', '') ?? '';
    try {
      await runCommand('Creating Transition…', () => requireInvoke()('add_state_transition', {
        diagramId: diagram.id, regionIdValue: null, sourceVertexId: source, targetVertexId: targetId, kind: 'External',
        eventKind, eventReferenceId, eventExpression, guard, effect,
      }));
      await refresh();
    } catch (_) { state.behaviorPending = { kind: 'Transition', source: null }; render(); }
  }

  function renderStateMachine(canvas, diagram) {
    const machine = stateMachine(diagram); if (!machine) return;
    const frame = document.createElement('div'); frame.className = 'diagram-frame behavior-frame state-machine-frame';
    const context = projectElement(diagram.context_id);
    frame.innerHTML = `<div class="diagram-header">stm [${escapeHtml(context?.name || 'classifier')}] ${escapeHtml(diagram.name)}</div>`;
    const svg = document.createElementNS(SVG_NS, 'svg'); svg.classList.add('behavior-relationship-layer'); svg.setAttribute('width','100%'); svg.setAttribute('height','100%');
    const defs = document.createElementNS(SVG_NS, 'defs');
    marker(defs, 'state-arrow', 'M 1 1 L 11 6 L 1 11', { fill: 'none', refX: '11' }); svg.appendChild(defs);
    for (const region of machine.regions || []) for (const transition of region.transitions || []) {
      const a = stateNodePresentation(diagram, transition.source_id); const b = stateNodePresentation(diagram, transition.target_id); if (!a || !b) continue;
      const x1=a.x+a.width/2,y1=a.y+a.height/2,x2=b.x+b.width/2,y2=b.y+b.height/2;
      const line=document.createElementNS(SVG_NS,'line'); line.setAttribute('x1',x1);line.setAttribute('y1',y1);line.setAttribute('x2',x2);line.setAttribute('y2',y2); line.setAttribute('marker-end','url(#state-arrow)'); line.classList.add('state-transition');
      if (state.selectedBehaviorItem?.type==='Transition'&&state.selectedBehaviorItem.id===transition.id) line.classList.add('selected');
      line.onclick=(event)=>{event.stopPropagation();state.selectedBehaviorItem={type:'Transition',id:transition.id,semantic:transition};render();}; svg.appendChild(line);
      const label=transitionLabel(transition); if(label){const text=document.createElementNS(SVG_NS,'text');text.classList.add('behavior-edge-label');text.setAttribute('x',(x1+x2)/2+5);text.setAttribute('y',(y1+y2)/2-7);text.textContent=label;svg.appendChild(text);}
    }
    frame.appendChild(svg);
    for (const {vertex} of allStateVertices(machine.regions)) {
      const p=stateNodePresentation(diagram,vertex.id); if(!p) continue; const kind=vertexKindName(vertex); const node=document.createElement('button'); node.className=`state-vertex state-${kind.toLowerCase()}`;
      if(state.selectedBehaviorItem?.type==='Vertex'&&state.selectedBehaviorItem.id===vertex.id)node.classList.add('selected'); if(state.behaviorPending?.source===vertex.id)node.classList.add('connector-source');
      node.style.left=`${p.x}px`;node.style.top=`${p.y}px`;node.style.width=`${p.width}px`;node.style.height=`${p.height}px`;
      if(kind==='State'){const s=vertex.kind.State||{};node.innerHTML=`<strong>${escapeHtml(vertex.name)}</strong>${s.entry?`<span>entry / ${escapeHtml(s.entry)}</span>`:''}${s.do_activity?`<span>do / ${escapeHtml(s.do_activity)}</span>`:''}${s.exit?`<span>exit / ${escapeHtml(s.exit)}</span>`:''}`;}
      else if(kind==='Initial')node.innerHTML='<span class="pseudo-dot"></span>'; else if(kind==='FinalState')node.innerHTML='<span class="final-ring"><i></i></span>'; else if(kind==='Choice')node.innerHTML='<span class="choice-diamond"></span>'; else if(kind==='Fork'||kind==='Join')node.innerHTML='<span class="fork-bar"></span>'; else node.textContent=kind.replace(/([A-Z])/g,' $1').trim();
      node.onclick=async(event)=>{event.stopPropagation();if(state.behaviorPending?.kind==='Transition'){await commitTransition(diagram,vertex.id);return;}state.selectedBehaviorItem={type:'Vertex',id:vertex.id,semantic:vertex};render();};
      node.onpointerdown=(event)=>{if(state.behaviorPending||state.behaviorTool)return;const startX=event.clientX,startY=event.clientY,ox=p.x,oy=p.y;node.setPointerCapture(event.pointerId);node.onpointermove=(move)=>{p.x=ox+move.clientX-startX;p.y=oy+move.clientY-startY;node.style.left=`${p.x}px`;node.style.top=`${p.y}px`;};node.onpointerup=async()=>{node.onpointermove=null;await requireInvoke()('move_state_vertex',{diagramId:diagram.id,stateVertexId:vertex.id,x:p.x,y:p.y});await refresh();};};
      frame.appendChild(node);
    }
    frame.onclick=async(event)=>{if(event.target!==frame)return;state.selectedBehaviorItem=null;if(state.behaviorTool)await createStateVertexAt(diagram,event.offsetX-80,event.offsetY-50);else render();}; canvas.appendChild(frame);
  }

  function lifelineX(diagram,id){return diagram.lifelines.find((item)=>item.lifeline_id===id)?.x??140;}
  function messageY(message,index){const order=message.send_event?.order??message.receive_event?.order??((index+1)*10);return 110+order*4;}

  async function addLifeline(diagram, x) {
    const candidates=await requireInvoke()('behavior_lifeline_candidates',{diagramId:diagram.id});
    if(!candidates.length){alert('This Block has no Part/Reference Properties to represent as Lifelines. Create structural properties on the Block first.');return;}
    const menu=candidates.map((item,index)=>`${index+1}. ${item.label}`).join('\n');const answer=prompt(`Choose represented property path:\n${menu}`,'1');const candidate=candidates[Number(answer)-1];if(!candidate)return;
    await runCommand('Adding Lifeline…',()=>requireInvoke()('add_sequence_lifeline',{diagramId:diagram.id,representedPath:candidate.property_path,x}));state.behaviorTool=null;await refresh();
  }

  async function messageSignature(sort) {
    if(!['SynchCall','AsynchCall','AsynchSignal'].includes(sort)) return null;
    const kind=sort==='AsynchSignal'?'Signal':'Operation';const candidates=state.snapshot.project.elements.filter((e)=>e.kind===kind);if(!candidates.length){alert(`Create a ${kind} first; ${sort} Messages require a real ${kind} signature.`);return undefined;}
    const menu=candidates.map((e,i)=>`${i+1}. ${e.name}`).join('\n');const answer=prompt(`Choose ${kind}:\n${menu}`,'1');return candidates[Number(answer)-1]?.id;
  }

  async function commitMessage(diagram,targetId){const pending=state.behaviorPending;if(!pending?.source){pending.source=targetId;render();return;}const source=pending.source,sort=pending.sort;state.behaviorPending=null;const signatureId=await messageSignature(sort);if(signatureId===undefined){render();return;}const name=prompt('Message name',sort==='Reply'?'reply':'message')||'';const argsText=['SynchCall','AsynchCall','AsynchSignal'].includes(sort)?prompt('Arguments, comma separated (optional)','')||'':'';await runCommand(`Creating ${sort} Message…`,()=>requireInvoke()('add_sequence_message',{diagramId:diagram.id,sourceLifelineId:source,targetLifelineId:targetId,sort,name,signatureId,arguments:argsText?argsText.split(',').map(v=>v.trim()).filter(Boolean):[]}));await refresh();}

  function renderSequence(canvas,diagram){const inter=interaction(diagram);if(!inter)return;const frame=document.createElement('div');frame.className='diagram-frame behavior-frame sequence-frame';const context=projectElement(diagram.context_id);frame.innerHTML=`<div class="diagram-header">seq [${escapeHtml(context?.name||'classifier')}] ${escapeHtml(diagram.name)}</div>`;
    const svg=document.createElementNS(SVG_NS,'svg');svg.classList.add('sequence-message-layer');svg.setAttribute('width','100%');svg.setAttribute('height','100%');const defs=document.createElementNS(SVG_NS,'defs');marker(defs,'seq-filled','M 1 1 L 11 6 L 1 11 Z',{fill:'#111',refX:'11'});marker(defs,'seq-open','M 1 1 L 11 6 L 1 11',{fill:'none',refX:'11'});svg.appendChild(defs);
    (inter.messages||[]).forEach((message,index)=>{const sx=message.send_event?lifelineX(diagram,message.send_event.lifeline_id):70;const tx=message.receive_event?lifelineX(diagram,message.receive_event.lifeline_id):frame.clientWidth-70;const y=messageY(message,index);const line=document.createElementNS(SVG_NS,'line');line.setAttribute('x1',sx);line.setAttribute('y1',y);line.setAttribute('x2',tx);line.setAttribute('y2',y);line.classList.add('sequence-message',`message-${message.sort.toLowerCase()}`);line.setAttribute('marker-end',message.sort==='Reply'?'url(#seq-open)':'url(#seq-filled)');if(message.sort==='Reply')line.setAttribute('stroke-dasharray','6 4');if(['AsynchCall','AsynchSignal'].includes(message.sort))line.setAttribute('marker-end','url(#seq-open)');line.onclick=(e)=>{e.stopPropagation();state.selectedBehaviorItem={type:'Message',id:message.id,semantic:message};render();};svg.appendChild(line);const text=document.createElementNS(SVG_NS,'text');text.classList.add('behavior-edge-label');text.setAttribute('x',Math.min(sx,tx)+Math.abs(tx-sx)/2);text.setAttribute('y',y-6);const sig=message.signature?.Operation?projectElement(message.signature.Operation)?.name:message.signature?.Signal?projectElement(message.signature.Signal)?.name:'';text.textContent=`${sig||message.name}${message.arguments?.length?`(${message.arguments.join(', ')})`:''}`;svg.appendChild(text);});frame.appendChild(svg);
    for(const lifeline of inter.lifelines||[]){const x=lifelineX(diagram,lifeline.id);const node=document.createElement('button');node.className='sequence-lifeline';if(state.selectedBehaviorItem?.type==='Lifeline'&&state.selectedBehaviorItem.id===lifeline.id)node.classList.add('selected');if(state.behaviorPending?.source===lifeline.id)node.classList.add('connector-source');node.style.left=`${x-65}px`;node.innerHTML=`<div class="lifeline-head">${escapeHtml(lifeline.name)}</div><div class="lifeline-line"></div>`;node.onclick=async(e)=>{e.stopPropagation();if(state.behaviorPending?.kind==='Message'){await commitMessage(diagram,lifeline.id);return;}state.selectedBehaviorItem={type:'Lifeline',id:lifeline.id,semantic:lifeline};render();};node.onpointerdown=(event)=>{if(state.behaviorPending||state.behaviorTool)return;const start=event.clientX,orig=x;node.setPointerCapture(event.pointerId);node.onpointermove=(m)=>{node.style.left=`${orig+m.clientX-start-65}px`;};node.onpointerup=async(m)=>{node.onpointermove=null;await requireInvoke()('move_sequence_lifeline',{diagramId:diagram.id,lifelineIdValue:lifeline.id,x:orig+m.clientX-start});await refresh();};};frame.appendChild(node);}
    for(const execution of inter.executions||[]){const x=lifelineX(diagram,execution.lifeline_id);const bar=document.createElement('div');bar.className='execution-spec';bar.style.left=`${x-7}px`;bar.style.top=`${110+execution.start.order*4}px`;bar.style.height=`${Math.max(30,(execution.finish.order-execution.start.order)*4)}px`;frame.appendChild(bar);}
    for(const fragment of inter.fragments||[]){const xs=fragment.covered_lifelines.map(id=>lifelineX(diagram,id));if(!xs.length)continue;const top=110+Math.min(...fragment.operands.map(o=>o.start_order))*4;const bottom=110+Math.max(...fragment.operands.map(o=>o.end_order))*4;const box=document.createElement('div');box.className='combined-fragment';box.style.left=`${Math.min(...xs)-85}px`;box.style.width=`${Math.max(180,Math.max(...xs)-Math.min(...xs)+170)}px`;box.style.top=`${top}px`;box.style.height=`${bottom-top}px`;box.innerHTML=`<div class="fragment-tag">${String(fragment.operator).toLowerCase()}</div>`;frame.appendChild(box);}
    frame.onclick=async(e)=>{if(e.target!==frame)return;if(state.behaviorTool==='Lifeline'){await addLifeline(diagram,e.offsetX);return;}state.selectedBehaviorItem=null;render();};canvas.appendChild(frame);
  }

  const baseRenderCanvasBehavior=renderCanvas;
  renderCanvas=function renderCanvasBehavior(){const diagram=behaviorDiagram();if(!diagram)return baseRenderCanvasBehavior();const canvas=$('canvas');canvas.innerHTML='';if(diagram.kind==='StateMachine')renderStateMachine(canvas,diagram);else renderSequence(canvas,diagram);};

  async function createSequenceSecondary(tool,diagram){const inter=interaction(diagram);if(!inter)return;if(tool==='Execution'){if(!state.selectedBehaviorItem||state.selectedBehaviorItem.type!=='Lifeline'){alert('Select a Lifeline first, then choose Execution Specification.');return;}await runCommand('Adding Execution Specification…',()=>requireInvoke()('add_execution_specification',{diagramId:diagram.id,lifelineIdValue:state.selectedBehaviorItem.id}));}
    else if(tool==='Invariant'){if(!state.selectedBehaviorItem||state.selectedBehaviorItem.type!=='Lifeline'){alert('Select a Lifeline first, then choose State Invariant.');return;}const constraint=prompt('State invariant constraint','state = Ready');if(!constraint)return;await runCommand('Adding State Invariant…',()=>requireInvoke()('add_state_invariant',{diagramId:diagram.id,lifelineIdValue:state.selectedBehaviorItem.id,constraint}));}
    else{const ids=(inter.lifelines||[]).map(l=>l.id);if(!ids.length){alert('Add Lifelines before creating a Combined Fragment.');return;}const guard=prompt(`${tool} operand guard (optional)`,'');await runCommand(`Adding ${tool} Combined Fragment…`,()=>requireInvoke()('add_combined_fragment',{diagramId:diagram.id,operator:tool,coveredLifelineIds:ids,guard}));}state.behaviorTool=null;await refresh();}

  const baseRenderPropertiesBehavior=renderProperties;
  renderProperties=function renderPropertiesBehavior(){const diagram=behaviorDiagram();if(!diagram)return baseRenderPropertiesBehavior();const panel=$('properties');const item=state.selectedBehaviorItem;if(!item){panel.innerHTML=`<div class="property-heading">${diagram.kind==='StateMachine'?'State Machine':'Interaction'}</div><label>Diagram<input value="${escapeAttr(diagram.name)}" disabled></label><label>Context<input value="${escapeAttr(projectElement(diagram.context_id)?.name||diagram.context_id)}" disabled></label><div class="muted">Select a state, transition, lifeline, or message to inspect its Rust semantic object.</div>`;return;}
    if(item.type==='Vertex'){const v=item.semantic;panel.innerHTML=`<div class="property-heading">${escapeHtml(vertexKindName(v))}</div><label>Name<input value="${escapeAttr(v.name||'')}" disabled></label>`;if(vertexKindName(v)==='State'){const s=v.kind.State||{};panel.innerHTML+=`<label>Entry<input id="behavior-entry" value="${escapeAttr(s.entry||'')}"></label><label>Do<input id="behavior-do" value="${escapeAttr(s.do_activity||'')}"></label><label>Exit<input id="behavior-exit" value="${escapeAttr(s.exit||'')}"></label><button id="behavior-apply-state" class="primary">Apply State Behaviors</button><button id="behavior-add-region">Add Region</button>`;$('behavior-apply-state').onclick=async()=>{await runCommand('Updating State behaviors…',()=>requireInvoke()('update_state_behaviors',{diagramId:diagram.id,stateVertexId:v.id,entry:$('behavior-entry').value,doActivity:$('behavior-do').value,exit:$('behavior-exit').value}));await refresh();};$('behavior-add-region').onclick=async()=>{const name=prompt('Region name','Region');if(!name)return;await runCommand('Adding Region…',()=>requireInvoke()('add_state_region',{diagramId:diagram.id,stateVertexId:v.id,name}));await refresh();};}return;}
    if(item.type==='Transition'){panel.innerHTML=`<div class="property-heading">Transition</div><label>Kind<input value="${escapeAttr(item.semantic.kind)}" disabled></label><label>Notation<input value="${escapeAttr(transitionLabel(item.semantic))}" disabled></label><div class="muted">Transition trigger, guard, and effect are stored as Rust semantics, not diagram text.</div>`;return;}
    if(item.type==='Lifeline'){panel.innerHTML=`<div class="property-heading">Lifeline</div><label>Represents<input value="${escapeAttr(item.semantic.name)}" disabled></label><button id="behavior-execution">Add Execution Specification</button><button id="behavior-invariant">Add State Invariant</button>`;$('behavior-execution').onclick=()=>{state.behaviorTool='Execution';createSequenceSecondary('Execution',diagram);};$('behavior-invariant').onclick=()=>{state.behaviorTool='Invariant';createSequenceSecondary('Invariant',diagram);};return;}
    if(item.type==='Message'){panel.innerHTML=`<div class="property-heading">Message</div><label>Sort<input value="${escapeAttr(item.semantic.sort)}" disabled></label><label>Name<input value="${escapeAttr(item.semantic.name||'')}" disabled></label><div class="muted">Occurrence ordering is semantic and independent of pixel position.</div>`;}
  };

  const baseActivatePaletteBehavior=activatePaletteItem;
  activatePaletteItem=function activatePaletteItemBehavior(item){if(behaviorDiagram()){activateBehaviorTool(item.id);return;}baseActivatePaletteBehavior(item);};

  const baseRenderBehavior=render;
  render=function renderBehavior(){baseRenderBehavior();const diagram=behaviorDiagram();if(diagram?.kind==='Sequence'&&state.behaviorTool&&['Execution','Invariant','alt','opt','loop','break','par','critical','neg','assert','strict','seq','ignore','consider'].includes(state.behaviorTool)){const tool=state.behaviorTool;state.behaviorTool=null;queueMicrotask(()=>createSequenceSecondary(tool,diagram));}};

  installBehaviorRibbon();
  loadBehaviorSnapshot().then(()=>render()).catch(console.error);
})();
