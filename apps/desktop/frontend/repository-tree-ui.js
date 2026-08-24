(() => {
  'use strict';
  const REPOSITORY_ELEMENT_MIME = 'application/x-smp-repository-element-id';
  state.collapsedRepositoryNodes = state.collapsedRepositoryNodes
    || state.collapsedRepositoryPackages
    || new Set();
  state.collapsedRepositoryPackages = state.collapsedRepositoryNodes;

  const baseRenderRepository = renderRepository;

  function allDiagrams() {
    return [
      ...(state.snapshot?.diagrams || []),
      ...(state.snapshot?.ibd_diagrams || []),
      ...(state.behaviorSnapshot?.diagrams || []),
      ...(state.activitySnapshot?.diagrams || []),
    ];
  }

  function visibleDiagrams() {
    const filter = String(state.repositoryFilter || '').trim().toLowerCase();
    return allDiagrams().filter((diagram) => !filter || String(diagram.name || '').toLowerCase().includes(filter));
  }

  function diagramTag(diagram) {
    if (diagram.family === 'requirement') return 'REQ';
    if (diagram.family === 'bdd') return 'BDD';
    if (diagram.family === 'use-case') return 'UC';
    if (diagram.context_block_id) return 'IBD';
    if (diagram.kind === 'StateMachine') return 'STM';
    if (diagram.kind === 'Sequence') return 'SEQ';
    if (diagram.activity_id) return 'ACT';
    return 'Diagram';
  }

  function repositoryNamespaces(project) {
    return (project.elements || [])
      .filter((element) => element.kind === 'Model' || element.kind === 'Package')
      .sort((left, right) => String(left.name).localeCompare(String(right.name)));
  }

  function ownerOptions(project, selectedId, excludedId = null) {
    const byId = new Map((project.elements || []).map((element) => [String(element.id), element]));
    const isWithinExcludedSubtree = (owner) => {
      if (!excludedId) return false;
      let current = owner;
      const visited = new Set();
      while (current && !visited.has(String(current.id))) {
        if (String(current.id) === String(excludedId)) return true;
        visited.add(String(current.id));
        current = byId.get(String(current.owner_id || ''));
      }
      return false;
    };
    return repositoryNamespaces(project)
      .filter((owner) => !isWithinExcludedSubtree(owner))
      .map((owner) => `<option value="${escapeAttr(owner.id)}"${String(owner.id) === String(selectedId) ? ' selected' : ''}>${escapeHtml(owner.name)} (${escapeHtml(owner.kind)})</option>`)
      .join('');
  }

  function clearRepositorySelection() {
    Object.assign(state, { selectedRepositoryDiagramId: null });
  }

  async function confirmDestructive(title, description, confirmLabel) {
    if (window.smpDialogs?.confirm) {
      return window.smpDialogs.confirm({
        title,
        description,
        confirmLabel,
        destructive: true,
      });
    }
    return confirm(`${title}\n\n${description}`);
  }

  async function runRepositoryCommand(progressMessage, action) {
    try {
      renderStatus(progressMessage);
      return await action();
    } catch (error) {
      const message = error?.message || String(error);
      renderStatus(message);
      window.smpDialogs?.notify?.(message, 'error');
      throw error;
    }
  }

  async function moveElement(elementId, newOwnerId) {
    await runRepositoryCommand('Moving repository element…', () => requireInvoke()('move_repository_element', {
      elementId,
      newOwnerId,
    }));
    Object.assign(state, { selectedElementId: elementId, selectedPackageId: null });
    clearRepositorySelection();
    await refresh();
  }

  async function deleteElement(element) {
    const accepted = await confirmDestructive(
      `Delete ${element.kind} from model`,
      `Delete “${element.name}” from the semantic model and remove all of its diagram presentations? The operation is blocked while it owns content or is referenced.`,
      'Delete from Model',
    );
    if (!accepted) return;
    await runRepositoryCommand('Deleting model element…', () => requireInvoke()('delete_model_element', {
      elementId: element.id,
    }));
    Object.assign(state, { selectedElementId: null, selectedPackageId: null });
    clearRepositorySelection();
    await refresh();
  }

  async function moveDiagram(diagramId, newOwnerId) {
    await runRepositoryCommand('Moving diagram…', () => requireInvoke()('move_repository_diagram', {
      diagramId,
      newOwnerId,
    }));
    Object.assign(state, { selectedRepositoryDiagramId: diagramId });
    await refresh();
  }

  async function deleteDiagram(diagram) {
    const accepted = await confirmDestructive(
      'Delete diagram',
      `Delete the ${diagramTag(diagram)} diagram “${diagram.name}”? Semantic model content is retained.`,
      'Delete Diagram',
    );
    if (!accepted) return;
    await runRepositoryCommand('Deleting diagram…', () => requireInvoke()('delete_repository_diagram', {
      diagramId: diagram.id,
    }));
    for (const key of ['selectedDiagramId', 'selectedBehaviorDiagramId', 'selectedActivityDiagramId']) {
      if (String(state[key] || '') === String(diagram.id)) state[key] = null;
    }
    Object.assign(state, { selectedRepositoryDiagramId: null, selectedRelationshipId: null });
    await refresh();
  }

  function installDropTarget(target, ownerId) {
    target.dataset.repositoryOwnerId = String(ownerId);
    target.addEventListener('dragover', (event) => {
      if (![...event.dataTransfer.types].includes(REPOSITORY_ELEMENT_MIME)) return;
      event.preventDefault();
      event.dataTransfer.dropEffect = 'move';
      target.classList.add('repository-drop-target');
    });
    target.addEventListener('dragleave', () => target.classList.remove('repository-drop-target'));
    target.addEventListener('drop', (event) => {
      const elementId = event.dataTransfer.getData(REPOSITORY_ELEMENT_MIME);
      target.classList.remove('repository-drop-target');
      if (!elementId) return;
      event.preventDefault();
      event.stopPropagation();
      void moveElement(elementId, ownerId).catch((error) => console.error(error));
    });
  }

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

    const root = host.querySelector('.tree-root');
    if (root) {
      root.setAttribute('role', 'button');
      root.setAttribute('tabindex', '0');
      root.title = 'Project model root. Drop an element here to move it to the top level.';
      root.onclick = () => {
        Object.assign(state, {
          selectedPackageId: project.root_id,
          selectedElementId: null,
          selectedRelationshipId: null,
        });
        clearRepositorySelection();
        render();
      };
      installDropTarget(root, project.root_id);
    }

    flattened.forEach(({ element, depth }, index) => {
      const row = semanticRows[index];
      if (!row) return;

      row.dataset.elementId = String(element.id);
      row.dataset.treeDepth = String(depth);
      row.style.paddingLeft = `${8 + depth * 16}px`;
      const collapsedByAncestor = hasCollapsedAncestor(element, byId);
      row.hidden = collapsedByAncestor;
      row.style.display = collapsedByAncestor ? 'none' : '';
      row.draggable = true;
      row.addEventListener('dragstart', (event) => {
        event.dataTransfer.effectAllowed = 'copyMove';
        event.dataTransfer.setData(REPOSITORY_ELEMENT_MIME, String(element.id));
      });
      row.addEventListener('click', () => clearRepositorySelection(), true);

      if (element.kind === 'Model' || element.kind === 'Package') {
        installDropTarget(row, element.id);
      }

      const hasChildren = (byOwner.get(String(element.id)) || []).length > 0;
      row.classList.toggle('containment-parent-row', hasChildren);
      if (!hasChildren) return;

      const collapsed = state.collapsedRepositoryNodes.has(String(element.id));
      row.classList.toggle('collapsed', collapsed);
      row.insertBefore(disclosure(element.id, !collapsed), row.firstChild);
    });

    const diagrams = visibleDiagrams();
    [...host.querySelectorAll('.diagram-row')].forEach((row, index) => {
      const diagram = diagrams[index];
      if (!diagram) return;
      row.dataset.diagramId = String(diagram.id);
      row.addEventListener('click', () => {
        Object.assign(state, {
          selectedRepositoryDiagramId: String(diagram.id),
          selectedElementId: null,
          selectedPackageId: null,
          selectedRelationshipId: null,
        });
      }, true);
    });
  };

  function renderRepositoryEditingProperties() {
    const panel = $('properties');
    const project = state.snapshot?.project;
    if (!panel || !project) return;

    const canvasSelection = state.selectedElementId
      || state.selectedRelationshipId
      || state.selectedBehaviorItem
      || state.selectedActivityNodeId
      || state.selectedActivityEdgeId;
    const diagram = canvasSelection ? null : allDiagrams().find((candidate) => String(candidate.id) === String(state.selectedRepositoryDiagramId || ''));
    if (diagram) {
      panel.innerHTML = `<div class="property-heading">${escapeHtml(diagramTag(diagram))} Diagram</div><label>Name<input value="${escapeAttr(diagram.name)}" disabled></label><label>Owner<select id="repository-diagram-owner">${ownerOptions(project, diagram.owner_id)}</select></label><label>Diagram ID<input value="${escapeAttr(diagram.id)}" disabled></label><button id="move-repository-diagram" class="primary">Move Diagram</button><button id="delete-repository-diagram" class="danger">Delete Diagram</button>`;
      $('move-repository-diagram').onclick = () => void moveDiagram(diagram.id, $('repository-diagram-owner').value).catch((error) => console.error(error));
      $('delete-repository-diagram').onclick = () => void deleteDiagram(diagram).catch((error) => console.error(error));
      return;
    }

    const selectedId = state.selectedElementId || state.selectedPackageId;
    const element = project.elements.find((candidate) => String(candidate.id) === String(selectedId || ''));
    if (!element) return;
    const root = String(element.id) === String(project.root_id);
    const section = document.createElement('section');
    section.className = 'repository-governance-properties';
    section.innerHTML = `<div class="property-heading">Repository</div><div class="muted">Semantic ownership and containment. Move is validated by the Rust model.</div>${root ? '<div class="repository-protected">The project model root cannot be moved or deleted.</div>' : `<label>Owner<select id="repository-element-owner">${ownerOptions(project, element.owner_id, element.id)}</select></label><button id="move-repository-element" class="primary">Move Element</button><button id="delete-model-element" class="danger">Delete from Model</button>`}`;
    panel.appendChild(section);
    if (root) return;
    $('move-repository-element').onclick = () => void moveElement(element.id, $('repository-element-owner').value).catch((error) => console.error(error));
    $('delete-model-element').onclick = () => void deleteElement(element).catch((error) => console.error(error));
  }

  function handleDelete(event) {
    if (!['Delete', 'Backspace'].includes(event.key)) return;
    const repository = $('repository');
    if (!repository?.contains(document.activeElement)) return;
    const diagram = allDiagrams().find((candidate) => String(candidate.id) === String(state.selectedRepositoryDiagramId || ''));
    const selectedId = state.selectedElementId || state.selectedPackageId;
    const element = state.snapshot?.project?.elements?.find((candidate) => String(candidate.id) === String(selectedId || ''));
    if (!diagram && !element) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    if (diagram) void deleteDiagram(diagram).catch((error) => console.error(error));
    else void deleteElement(element).catch((error) => console.error(error));
    return true;
  }

  $('diagram-tabs')?.addEventListener('click', () => clearRepositorySelection(), true);
  $('canvas')?.addEventListener('pointerdown', () => clearRepositorySelection(), true);
  window.smpRepositoryEditing = Object.freeze({ renderProperties: renderRepositoryEditingProperties, handleDelete });
})();
