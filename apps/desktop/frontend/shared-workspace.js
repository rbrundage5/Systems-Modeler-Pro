/* Shared transient workspace controller. Semantic geometry and mutations remain Rust-owned. */
(() => {
  'use strict';
  const canvas = document.getElementById('canvas');
  if (!canvas) return;
  const STORAGE_KEY = 'smp.shared-workspace.v1';
  const DIAGRAM_TYPES = ['BDD', 'IBD', 'StateMachine', 'Sequence', 'Activity'];
  const clamp = (value, min, max) => Math.max(min, Math.min(max, value));
  const readStored = () => { try { return JSON.parse(localStorage.getItem(STORAGE_KEY)) || {}; } catch { return {}; } };
  const stored = readStored();
  stored.viewports ||= {}; stored.panels ||= {};

  function activeContext() {
    const summary = document.getElementById('active-diagram-summary')?.textContent || '';
    const id = window.state?.selectedActivityDiagramId || window.state?.selectedBehaviorDiagramId || window.state?.selectedIbdDiagramId || window.state?.selectedDiagramId || 'none';
    let type = DIAGRAM_TYPES.find((candidate) => summary.replaceAll(' ', '').includes(candidate)) || 'BDD';
    if (/State Machine/i.test(summary)) type = 'StateMachine';
    const name = summary.split('·')[0].trim() || 'No diagram selected';
    return { id: `${type}:${id}`, type, name };
  }
  const save = () => localStorage.setItem(STORAGE_KEY, JSON.stringify(stored));
  function viewport() { const key = activeContext().id; return stored.viewports[key] ||= { zoom: 1, panX: 0, panY: 0, grid: true, snap: true }; }
  function contentRoot() { return canvas.querySelector('.diagram-frame,.behavior-diagram,.activity-diagram,svg') || canvas.firstElementChild; }
  function applyViewport() {
    const view = viewport(), root = contentRoot();
    if (root) { root.style.transformOrigin = '0 0'; root.style.transform = `translate(${view.panX}px,${view.panY}px) scale(${view.zoom})`; }
    canvas.classList.toggle('grid-hidden', !view.grid);
    canvas.dataset.zoom = String(view.zoom);
  }
  function setZoom(next, clientX = canvas.clientWidth / 2, clientY = canvas.clientHeight / 2) {
    const view = viewport(), previous = view.zoom, rect = canvas.getBoundingClientRect();
    next = clamp(next, .25, 4); const x = clientX - rect.left + canvas.scrollLeft, y = clientY - rect.top + canvas.scrollTop;
    view.panX = x - (x - view.panX) * next / previous; view.panY = y - (y - view.panY) * next / previous; view.zoom = next;
    save(); applyViewport();
  }
  function fitDiagram() {
    const root = contentRoot(); if (!root) return;
    const bounds = root.getBoundingClientRect(); setZoom(clamp(Math.min((canvas.clientWidth - 56) / (bounds.width / viewport().zoom), (canvas.clientHeight - 56) / (bounds.height / viewport().zoom)), .25, 1));
  }
  function clearSelection() {
    document.querySelectorAll('.selected').forEach((node) => { if (canvas.contains(node)) node.classList.remove('selected'); });
    canvas.dispatchEvent(new CustomEvent('smp:clear-selection', { bubbles: true }));
  }

  const handlers = {
    select: () => canvas.focus(), clearSelection, zoomIn: () => setZoom(viewport().zoom * 1.15), zoomOut: () => setZoom(viewport().zoom / 1.15),
    actualSize: () => setZoom(1), fitDiagram, pan: () => canvas.classList.toggle('pan-active'),
    toggleGrid: () => { viewport().grid = !viewport().grid; save(); applyViewport(); }, snapGrid: () => { viewport().snap = !viewport().snap; save(); },
    undo: () => window.smpUndo?.(), redo: () => window.smpRedo?.(), route: () => {
      const { type } = activeContext();
      if (type === 'IBD') return window.smpRouteSelectedIbd?.();
      if (type === 'Activity') return window.smpRouteActivityDiagram?.();
      if (type === 'BDD') return window.smpRouteActiveBdd?.();
    },
    showRepository: () => togglePanel('repository-panel'), showElements: () => togglePanel('palette-panel'), showProperties: () => togglePanel('properties-panel'),
  };
  const registry = new Map();
  async function loadCommands() {
    let manifest = [];
    try { manifest = await window.__TAURI__?.core?.invoke('diagram_command_manifest') || []; } catch (error) { console.warn('Command manifest unavailable', error); }
    for (const command of manifest) registry.set(command.id, { ...command, execute: handlers[command.id] });
  }
  function execute(id) {
    const command = registry.get(id), type = activeContext().type;
    if (!command) return false;
    if (!command.supportedDiagrams.includes(type) || !command.execute) { window.smpDialogs?.notify(command.unavailableReason || `${command.label} is unavailable in this context.`, 'warning'); return false; }
    command.execute(); return true;
  }
  window.smpCommandRegistry = Object.freeze({ execute, get: (id) => registry.get(id), all: () => [...registry.values()] });

  canvas.addEventListener('wheel', (event) => { if (!event.ctrlKey) return; event.preventDefault(); setZoom(viewport().zoom * (event.deltaY < 0 ? 1.1 : 1 / 1.1), event.clientX, event.clientY); }, { passive:false });
  canvas.addEventListener('pointerdown', (event) => { if (event.target === canvas) clearSelection(); });
  document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape') execute('clearSelection');
    if (event.ctrlKey && event.key === '0') { event.preventDefault(); execute('actualSize'); }
    if (event.ctrlKey && event.key === '9') { event.preventDefault(); execute('fitDiagram'); }
  });
  new MutationObserver(() => { const context = activeContext(); document.getElementById('workspace-diagram-title').textContent = context.name; document.getElementById('workspace-diagram-context').textContent = `${context.type.replace('StateMachine','State Machine')} · Rust-owned model and presentation`; applyViewport(); }).observe(canvas, { childList:true, subtree:false });

  function togglePanel(className) { const panel = document.querySelector(`.${className}`); if (!panel) return; panel.classList.toggle('shell-hidden'); stored.panels[className] = { ...(stored.panels[className] || {}), visible: !panel.classList.contains('shell-hidden') }; save(); }
  for (const [selector, edge] of [['.repository-panel','right'],['.palette-panel','right'],['.properties-panel','left']]) {
    const panel = document.querySelector(selector); if (!panel) continue;
    const key = selector.slice(1), setting = stored.panels[key] || {}; if (setting.width) panel.style.width = `${setting.width}px`; if (setting.visible === false) panel.classList.add('shell-hidden');
    const splitter = document.createElement('div'); splitter.className = `panel-splitter ${edge}`; panel.appendChild(splitter);
    splitter.addEventListener('pointerdown', (down) => { down.preventDefault(); splitter.setPointerCapture(down.pointerId); splitter.classList.add('dragging'); const start = down.clientX, width = panel.getBoundingClientRect().width;
      const move = (event) => { const delta = (edge === 'right' ? 1 : -1) * (event.clientX - start); panel.style.width = `${clamp(width + delta, 150, 480)}px`; };
      const up = () => { splitter.classList.remove('dragging'); splitter.removeEventListener('pointermove', move); stored.panels[key] = { visible:true, width:Math.round(panel.getBoundingClientRect().width) }; save(); };
      splitter.addEventListener('pointermove', move); splitter.addEventListener('pointerup', up, { once:true });
    });
  }
  loadCommands(); applyViewport();
})();
