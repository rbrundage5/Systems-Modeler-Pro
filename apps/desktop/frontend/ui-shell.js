(() => {
  const ribbon = document.querySelector('.ribbon');
  const tabs = [...document.querySelectorAll('.workspace-tab')];
  if (!ribbon || !tabs.length) return;

  const panels = {
    File: `
      <section class="ribbon-group"><div class="ribbon-actions ribbon-large-actions">
        <button class="ribbon-command" data-forward="new-project"><span class="command-icon">＋</span><span>New<br>Project</span></button>
        <button class="ribbon-command" data-forward="open-project"><span class="command-icon">▱</span><span>Open</span></button>
        <button class="ribbon-command" data-forward="save-project"><span class="command-icon">▣</span><span>Save</span></button>
        <button class="ribbon-command" data-forward="save-project-as"><span class="command-icon">▣</span><span>Save As</span></button>
      </div><div class="ribbon-label">Project</div></section>`,
    Home: `
      <section class="ribbon-group"><div class="ribbon-actions">
        <button class="ribbon-command" data-forward="new-package"><span class="command-icon">□</span><span>Package</span></button>
        <button class="ribbon-command" data-forward="new-bdd"><span class="command-icon">▤</span><span>BDD</span></button>
        <button class="ribbon-command" data-action="new-ibd"><span class="command-icon">▥</span><span>IBD</span></button>
        <button class="ribbon-command" data-action="new-state-machine"><span class="command-icon">◉</span><span>State Machine</span></button>
        <button class="ribbon-command" data-action="new-sequence"><span class="command-icon">⇥</span><span>Sequence</span></button>
      </div><div class="ribbon-label">Create</div></section>
      <section class="ribbon-group ribbon-context"><div class="context-title">Active Diagram</div><div id="active-diagram-summary" class="context-value">No diagram selected</div><div class="context-subtitle">Elements and properties follow the active diagram</div><div class="ribbon-label">Context</div></section>`,
    Diagram: `
      <section class="ribbon-group"><div class="ribbon-actions">
        <button class="ribbon-command" data-forward="new-bdd"><span class="command-icon">▤</span><span>New BDD</span></button>
        <button class="ribbon-command" data-action="new-ibd"><span class="command-icon">▥</span><span>New IBD</span></button>
        <button class="ribbon-command" data-action="new-state-machine"><span class="command-icon">◉</span><span>New State Machine</span></button>
        <button class="ribbon-command" data-action="new-sequence"><span class="command-icon">⇥</span><span>New Sequence</span></button>
        <button class="ribbon-command" data-action="route-ibd"><span class="command-icon">⌁</span><span>Route</span></button>
      </div><div class="ribbon-label">Diagram</div></section>
      <section class="ribbon-group ribbon-context"><div class="context-title">Active Diagram</div><div id="active-diagram-summary" class="context-value">No diagram selected</div><div class="context-subtitle">Rust-owned diagram commands</div><div class="ribbon-label">Context</div></section>`,
    Arrange: `<section class="ribbon-group"><div class="ribbon-actions"><button class="ribbon-command" data-action="route-ibd"><span class="command-icon">⌁</span><span>Route IBD</span></button></div><div class="ribbon-label">Routing</div></section><section class="ribbon-group ribbon-context"><div class="context-title">Shared router</div><div class="context-value">Deterministic orthogonal routing</div><div class="context-subtitle">BDD and IBD use the same Rust obstacle-routing foundation.</div><div class="ribbon-label">Layout</div></section>`,
    View: `
      <section class="ribbon-group"><div class="ribbon-actions compact-actions">
        <button class="ribbon-command panel-toggle" data-panel="repository-panel"><span class="command-icon">▥</span><span>Repository</span></button>
        <button class="ribbon-command panel-toggle" data-panel="palette-panel"><span class="command-icon">▦</span><span>Elements</span></button>
        <button class="ribbon-command panel-toggle" data-panel="properties-panel"><span class="command-icon">▤</span><span>Properties</span></button>
      </div><div class="ribbon-label">Panels</div></section>`,
    Help: `<section class="ribbon-group ribbon-context"><div class="context-title">Systems Modeler Pro</div><div class="context-value">Native Rust migration</div><div class="context-subtitle">SysML engineering modeler desktop workspace</div><div class="ribbon-label">About</div></section>`,
  };

  const original = new Map();
  for (const id of ['new-project', 'open-project', 'save-project', 'save-project-as', 'new-package', 'new-bdd']) {
    const node = document.getElementById(id);
    if (node) original.set(id, node);
  }

  function syncPanelToggles() {
    document.querySelectorAll('.panel-toggle').forEach((button) => {
      const panel = document.querySelector(`.${button.dataset.panel}`);
      button.classList.toggle('active', !!panel && !panel.classList.contains('shell-hidden'));
    });
  }

  function bindRibbon() {
    ribbon.querySelectorAll('[data-forward]').forEach((button) => {
      button.addEventListener('click', () => original.get(button.dataset.forward)?.click());
    });
    ribbon.querySelectorAll('[data-action="new-ibd"]').forEach((button) => {
      button.addEventListener('click', () => window.smpCreateIbdForSelectedBlock?.());
    });
    ribbon.querySelectorAll('[data-action="new-state-machine"]').forEach((button) => {
      button.addEventListener('click', () => window.smpCreateStateMachineForSelectedBlock?.());
    });
    ribbon.querySelectorAll('[data-action="new-sequence"]').forEach((button) => {
      button.addEventListener('click', () => window.smpCreateSequenceForSelectedBlock?.());
    });
    ribbon.querySelectorAll('[data-action="route-ibd"]').forEach((button) => {
      button.addEventListener('click', () => window.smpRouteSelectedIbd?.());
    });
    ribbon.querySelectorAll('.panel-toggle').forEach((button) => {
      button.addEventListener('click', () => {
        document.querySelector(`.${button.dataset.panel}`)?.classList.toggle('shell-hidden');
        document.querySelector('.workspace')?.classList.toggle(`hide-${button.dataset.panel}`);
        syncPanelToggles();
      });
    });
    syncPanelToggles();
  }

  function activate(name) {
    const currentContext = document.getElementById('active-diagram-summary')?.textContent || 'No diagram selected';
    tabs.forEach((tab) => tab.classList.toggle('active', tab.textContent.trim() === name));
    ribbon.innerHTML = panels[name] || panels.Home;
    bindRibbon();
    const context = document.getElementById('active-diagram-summary');
    if (context) context.textContent = currentContext;
    if (typeof renderContext === 'function') renderContext();
  }

  tabs.forEach((tab) => {
    tab.setAttribute('role', 'button');
    tab.tabIndex = 0;
    const activateTab = () => activate(tab.textContent.trim());
    tab.addEventListener('click', activateTab);
    tab.addEventListener('keydown', (event) => {
      if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); activateTab(); }
    });
  });

  activate('Home');
})();