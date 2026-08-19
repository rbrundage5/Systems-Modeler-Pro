(() => {
  state.collapsedRepositoryNodes = state.collapsedRepositoryNodes
    || state.collapsedRepositoryPackages
    || new Set();
  state.collapsedRepositoryPackages = state.collapsedRepositoryNodes;

  const baseRenderRepository = renderRepository;

  function projectIndex(project) {
    const byId = new Map((project.elements || []).map((element) => [String(element.id), element]));
    const byOwner = new Map();
    for (const element of project.elements || []) {
      const ownerId = String(element.owner_id || '');
      if (!ownerId) continue;
      if (!byOwner.has(ownerId)) byOwner.set(ownerId, []);
      byOwner.get(ownerId).push(element);
    }
    for (const children of byOwner.values()) {
      children.sort((a, b) => String(a.name).localeCompare(String(b.name)));
    }
    return { byId, byOwner };
  }

  function matchesFilter(element) {
    const filter = String(state.repositoryFilter || '').trim().toLowerCase();
    if (!filter) return true;
    return String(element.name || '').toLowerCase().includes(filter)
      || String(element.kind || '').toLowerCase().includes(filter);
  }

  function flattenVisible(project, byOwner) {
    const output = [];
    const filterActive = Boolean(String(state.repositoryFilter || '').trim());

    function branchHasMatch(element) {
      if (!filterActive || matchesFilter(element)) return true;
      return (byOwner.get(String(element.id)) || []).some(branchHasMatch);
    }

    function visit(ownerId, depth) {
      for (const element of byOwner.get(String(ownerId)) || []) {
        if (!branchHasMatch(element)) continue;
        output.push({ element, depth });
        visit(element.id, depth + 1);
      }
    }

    visit(project.root_id, 0);
    return output;
  }

  function hasCollapsedAncestor(element, byId) {
    if (String(state.repositoryFilter || '').trim()) return false;
    let ownerId = element.owner_id;
    const seen = new Set();
    while (ownerId && !seen.has(String(ownerId))) {
      const key = String(ownerId);
      seen.add(key);
      if (state.collapsedRepositoryNodes.has(key)) return true;
      const owner = byId.get(key);
      ownerId = owner?.owner_id || null;
    }
    return false;
  }

  function disclosure(elementId, expanded) {
    const control = document.createElement('span');
    control.className = 'repository-disclosure';
    control.setAttribute('role', 'button');
    control.setAttribute('tabindex', '-1');
    control.setAttribute('aria-label', expanded ? 'Collapse contained elements' : 'Expand contained elements');
    control.setAttribute('aria-expanded', String(expanded));
    control.textContent = expanded ? '▾' : '▸';
    control.onclick = (event) => {
      event.preventDefault();
      event.stopPropagation();
      const key = String(elementId);
      if (state.collapsedRepositoryNodes.has(key)) state.collapsedRepositoryNodes.delete(key);
      else state.collapsedRepositoryNodes.add(key);
      render();
    };
    return control;
  }

  renderRepository = function renderCollapsibleRepository() {
    baseRenderRepository();

    const host = $('repository');
    const project = state.snapshot?.project;
    if (!host || !project) return;

    const { byId, byOwner } = projectIndex(project);
    const flattened = flattenVisible(project, byOwner);
    const semanticRows = [...host.querySelectorAll('.tree-row:not(.diagram-row)')];

    flattened.forEach(({ element, depth }, index) => {
      const row = semanticRows[index];
      if (!row) return;

      row.dataset.elementId = String(element.id);
      row.dataset.treeDepth = String(depth);
      row.style.paddingLeft = `${8 + depth * 16}px`;
      row.hidden = hasCollapsedAncestor(element, byId);

      const hasChildren = (byOwner.get(String(element.id)) || []).length > 0;
      row.classList.toggle('containment-parent-row', hasChildren);
      if (!hasChildren) return;

      const collapsed = state.collapsedRepositoryNodes.has(String(element.id));
      row.classList.toggle('collapsed', collapsed);
      row.insertBefore(disclosure(element.id, !collapsed), row.firstChild);
    });
  };
})();
