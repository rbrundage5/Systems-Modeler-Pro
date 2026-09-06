(() => {
  'use strict';

  const invoke = window.__TAURI__?.core?.invoke;
  if (!invoke || typeof refresh !== 'function' || typeof renderRepository !== 'function' || typeof renderDiagramTabs !== 'function') return;

  function allDiagrams() {
    const byId = new Map();
    const add = (diagram, family) => {
      if (!diagram?.id) return;
      byId.set(String(diagram.id), { diagram, family });
    };
    for (const diagram of state.snapshot?.diagrams || []) add(diagram, diagram.family || 'bdd');
    for (const diagram of state.snapshot?.ibd_diagrams || []) add(diagram, 'ibd');
    for (const diagram of state.behaviorSnapshot?.diagrams || state.snapshot?.behavior_diagrams || []) {
      add(diagram, diagram.kind === 'Sequence' ? 'sequence' : 'state-machine');
    }
    for (const diagram of state.activitySnapshot?.diagrams || []) add(diagram, 'activity');
    return [...byId.values()];
  }

  function tag(family) {
    return ({
      package: 'PKG',
      requirement: 'REQ',
      'use-case': 'UC',
      bdd: 'BDD',
      ibd: 'IBD',
      activity: 'ACT',
      'state-machine': 'STM',
      sequence: 'SEQ',
      parametric: 'PAR',
    })[family] || String(family || 'Diagram').toUpperCase();
  }

  function selected(id, family) {
    if (family === 'activity') return String(state.selectedActivityDiagramId || '') === String(id);
    if (family === 'state-machine' || family === 'sequence') return String(state.selectedBehaviorDiagramId || '') === String(id);
    return String(state.selectedDiagramId || '') === String(id);
  }

  async function openDiagram(entry) {
    const { diagram, family } = entry;
    Object.assign(state, { selectedRepositoryDiagramId: String(diagram.id) });
    if (family === 'activity') {
      if (typeof window.smpSelectActivityDiagram !== 'function') throw new Error('Activity diagram navigation is unavailable.');
      await window.smpSelectActivityDiagram(diagram.id);
      return;
    }
    if (family === 'state-machine' || family === 'sequence') {
      if (typeof window.smpSelectBehaviorDiagram !== 'function') throw new Error('Behavior diagram navigation is unavailable.');
      await window.smpSelectBehaviorDiagram(diagram.id);
      return;
    }
    if (typeof selectDiagram !== 'function') throw new Error('Structural diagram navigation is unavailable.');
    await selectDiagram(diagram.id);
  }

  async function refreshSpecializedSnapshots() {
    const [behavior, activity] = await Promise.all([
      invoke('behavior_snapshot'),
      invoke('activity_snapshot'),
    ]);
    state.behaviorSnapshot = behavior;
    state.activitySnapshot = activity;
  }

  const baseRefresh = refresh;
  refresh = async function refreshWithImportedDiagramConvergence() {
    await baseRefresh();
    await refreshSpecializedSnapshots();
    render();
  };

  const baseRenderDiagramTabs = renderDiagramTabs;
  renderDiagramTabs = function renderUnifiedDiagramTabs() {
    baseRenderDiagramTabs();
    const host = document.getElementById('diagram-tabs');
    if (!host) return;
    host.innerHTML = '';
    for (const entry of allDiagrams()) {
      const tab = document.createElement('button');
      tab.className = 'diagram-tab';
      tab.dataset.diagramId = String(entry.diagram.id);
      tab.dataset.diagramFamily = entry.family;
      if (selected(entry.diagram.id, entry.family)) tab.classList.add('active');
      tab.textContent = `${entry.diagram.name} · ${tag(entry.family)}`;
      tab.onclick = () => void openDiagram(entry).catch((error) => {
        console.error(error);
        window.smpDialogs?.notify?.(error?.message || String(error), 'error');
      });
      host.appendChild(tab);
    }
  };

  const baseRenderRepository = renderRepository;
  renderRepository = function renderUnifiedDiagramRepository() {
    baseRenderRepository();
    const host = document.getElementById('repository');
    if (!host) return;

    for (const row of [...host.querySelectorAll('.diagram-row')]) row.remove();
    for (const heading of [...host.querySelectorAll('.tree-heading')]) {
      if (heading.textContent.trim() === 'Diagrams') heading.remove();
    }

    const filter = String(state.repositoryFilter || '').trim().toLowerCase();
    const entries = allDiagrams().filter(({ diagram, family }) => {
      if (!filter) return true;
      return String(diagram.name || '').toLowerCase().includes(filter)
        || tag(family).toLowerCase().includes(filter);
    });
    if (!entries.length) return;

    const heading = document.createElement('div');
    heading.className = 'tree-heading';
    heading.textContent = 'Diagrams';
    host.appendChild(heading);

    for (const entry of entries) {
      const row = document.createElement('button');
      row.className = 'tree-row diagram-row';
      row.dataset.diagramId = String(entry.diagram.id);
      row.dataset.diagramFamily = entry.family;
      if (selected(entry.diagram.id, entry.family)) row.classList.add('selected');
      const glyph = entry.family === 'activity' ? '▶'
        : entry.family === 'state-machine' ? '◉'
        : entry.family === 'sequence' ? '⇥'
        : entry.family === 'ibd' ? '▤'
        : '▤';
      row.innerHTML = `<span class="kind">${glyph}</span><span>${escapeHtml(entry.diagram.name)}</span><span class="type-tag">${tag(entry.family)}</span>`;
      row.onclick = () => void openDiagram(entry).catch((error) => {
        console.error(error);
        window.smpDialogs?.notify?.(error?.message || String(error), 'error');
      });
      host.appendChild(row);
    }
  };

  window.smpImportedDiagramCatalog = Object.freeze({
    all: allDiagrams,
    refreshSpecializedSnapshots,
  });
})();
