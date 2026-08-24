(() => {
  const ribbon = document.querySelector('.ribbon');
  const tabs = [...document.querySelectorAll('.workspace-tab')];
  const commandStates = new Map();
  if (!ribbon || !tabs.length) return;

  const historyGroup = `
      <section class="ribbon-group"><div class="ribbon-actions">
        <button class="ribbon-command" data-command="undo"><span class="command-icon">↶</span><span>Undo</span></button>
        <button class="ribbon-command" data-command="redo"><span class="command-icon">↷</span><span>Redo</span></button>
      </div><div class="ribbon-label">History</div></section>`;

  const panels = {
    File: `
      <section class="ribbon-group"><div class="ribbon-actions ribbon-large-actions">
        <button class="ribbon-command" data-forward="new-project"><span class="command-icon">＋</span><span>New<br>Project</span></button>
        <button class="ribbon-command" data-forward="open-project"><span class="command-icon">▱</span><span>Open</span></button>
        <button class="ribbon-command" data-forward="save-project"><span class="command-icon">▣</span><span>Save</span></button>
        <button class="ribbon-command" data-forward="save-project-as"><span class="command-icon">▣</span><span>Save As</span></button>
      </div><div class="ribbon-label">Project</div></section>${historyGroup}`,
    Home: `
      <section class="ribbon-group"><div class="ribbon-actions">
        <button class="ribbon-command" data-forward="new-package"><span class="command-icon">□</span><span>Package</span></button>
        <button class="ribbon-command" data-action="new-package-diagram"><span class="command-icon">PKG</span><span>Package<br>Diagram</span></button>
        <button class="ribbon-command" data-forward="new-bdd"><span class="command-icon">▤</span><span>BDD</span></button>
        <button class="ribbon-command" data-action="new-requirement"><span class="command-icon">R</span><span>Requirement</span></button>
        <button class="ribbon-command" data-action="new-use-case"><span class="command-icon">UC</span><span>Use Case</span></button>
        <button class="ribbon-command" data-action="new-parametric"><span class="command-icon">PAR</span><span>Parametric</span></button>
        <button class="ribbon-command" data-action="new-ibd"><span class="command-icon">▥</span><span>IBD</span></button>
        <button class="ribbon-command" data-action="new-state-machine"><span class="command-icon">◉</span><span>State Machine</span></button>
        <button class="ribbon-command" data-action="new-sequence"><span class="command-icon">⇥</span><span>Sequence</span></button>
        <button class="ribbon-command" data-action="new-activity"><span class="command-icon">▶</span><span>Activity</span></button>
      </div><div class="ribbon-label">Create</div></section>${historyGroup}
      <section class="ribbon-group ribbon-context"><div class="context-title">Active Diagram</div><div id="active-diagram-summary" class="context-value">No diagram selected</div><div class="context-subtitle">Elements and properties follow the active diagram</div><div class="ribbon-label">Context</div></section>`,
    Diagram: `
      <section class="ribbon-group"><div class="ribbon-actions">
        <button class="ribbon-command" data-action="new-package-diagram"><span class="command-icon">PKG</span><span>New Package</span></button>
        <button class="ribbon-command" data-forward="new-bdd"><span class="command-icon">▤</span><span>New BDD</span></button>
        <button class="ribbon-command" data-action="new-requirement"><span class="command-icon">R</span><span>New Requirement</span></button>
        <button class="ribbon-command" data-action="new-use-case"><span class="command-icon">UC</span><span>New Use Case</span></button>
        <button class="ribbon-command" data-action="new-parametric"><span class="command-icon">PAR</span><span>New Parametric</span></button>
        <button class="ribbon-command" data-action="new-ibd"><span class="command-icon">▥</span><span>New IBD</span></button>
        <button class="ribbon-command" data-action="new-state-machine"><span class="command-icon">◉</span><span>New State Machine</span></button>
        <button class="ribbon-command" data-action="new-sequence"><span class="command-icon">⇥</span><span>New Sequence</span></button>
        <button class="ribbon-command" data-action="new-activity"><span class="command-icon">▶</span><span>New Activity</span></button>
        <button class="ribbon-command" data-command="route"><span class="command-icon">⌁</span><span>Route</span></button>
      </div><div class="ribbon-label">Diagram</div></section>${historyGroup}
      <section class="ribbon-group ribbon-context"><div class="context-title">Active Diagram</div><div id="active-diagram-summary" class="context-value">No diagram selected</div><div class="context-subtitle">Rust-owned diagram commands</div><div class="ribbon-label">Context</div></section>`,
    Arrange: `<section class="ribbon-group"><div class="ribbon-actions"><button class="ribbon-command" data-command="route"><span class="command-icon">⌁</span><span>Route</span></button><button class="ribbon-command" data-command="cleanLayout"><span class="command-icon">⌁</span><span>Clean Layout</span></button><button class="ribbon-command" data-command="evaluateParametrics"><span class="command-icon">=</span><span>Evaluate Parametrics</span></button></div><div class="ribbon-label">Routing, Layout, and Analysis</div></section>${historyGroup}<section class="ribbon-group ribbon-context"><div class="context-title">Shared geometry and analysis</div><div class="context-value">Rust-owned routing, layout, and evaluation</div><div class="context-subtitle">Availability follows the active diagram capabilities.</div><div class="ribbon-label">Diagram</div></section>`,
    View: `
      <section class="ribbon-group"><div class="ribbon-actions compact-actions">
        <button class="ribbon-command panel-toggle" data-panel="repository-panel" data-command="showRepository"><span class="command-icon">▥</span><span>Repository</span></button>
        <button class="ribbon-command panel-toggle" data-panel="palette-panel" data-command="showElements"><span class="command-icon">▦</span><span>Elements</span></button>
        <button class="ribbon-command panel-toggle" data-panel="properties-panel" data-command="showProperties"><span class="command-icon">▤</span><span>Properties</span></button>
      </div><div class="ribbon-label">Panels</div></section>${historyGroup}`,
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
    ribbon.querySelectorAll('[data-action="new-package-diagram"]').forEach((button) => {
      button.addEventListener('click', () => window.smpCreatePackageDiagram?.());
    });
    ribbon.querySelectorAll('[data-action="new-ibd"]').forEach((button) => {
      button.addEventListener('click', () => window.smpCreateIbdForSelectedBlock?.());
    });
    ribbon.querySelectorAll('[data-action="new-requirement"]').forEach((button) => button.addEventListener('click', () => window.smpCreateRequirementDiagram?.()));
    ribbon.querySelectorAll('[data-action="new-use-case"]').forEach((button) => button.addEventListener('click', () => window.smpCreateUseCaseDiagram?.()));
    ribbon.querySelectorAll('[data-action="new-parametric"]').forEach((button) => button.addEventListener('click', () => window.smpCreateParametricDiagram?.()));
    ribbon.querySelectorAll('[data-action="new-state-machine"]').forEach((button) => {
      button.addEventListener('click', () => window.smpCreateStateMachineForSelectedBlock?.());
    });
    ribbon.querySelectorAll('[data-action="new-sequence"]').forEach((button) => {
      button.addEventListener('click', () => window.smpCreateSequenceForSelectedBlock?.());
    });
    ribbon.querySelectorAll('[data-action="new-activity"]').forEach((button) => {
      button.addEventListener('click', () => window.smpCreateActivityForSelection?.());
    });
    ribbon.querySelectorAll('[data-command]').forEach((button) => {
      button.addEventListener('click', async () => { await window.smpRendererHost?.execute(button.dataset.command); syncPanelToggles(); });
    });
    syncPanelToggles();
    syncCommandStates();
  }

  function syncCommandStates() {
    ribbon.querySelectorAll('[data-command]').forEach((button) => {
      const command = commandStates.get(button.dataset.command);
      if (!command) return;
      button.disabled = !command.enabled;
      button.title = command.enabled ? [command.label, command.shortcut].filter(Boolean).join(' · ') : command.disabledReason;
      button.setAttribute('aria-disabled', String(!command.enabled));
    });
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

  document.addEventListener('smp:commands-ready', (event) => {
    commandStates.clear();
    for (const command of event.detail || []) commandStates.set(command.id, command);
    syncCommandStates();
  });

  activate('Home');
})();