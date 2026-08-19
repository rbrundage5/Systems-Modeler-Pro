(() => {
  function activeBehaviorDiagram() {
    return state.behaviorSnapshot?.diagrams?.find(
      (diagram) => diagram.id === state.selectedBehaviorDiagramId,
    ) || null;
  }

  function clearBehaviorSelection() {
    state.selectedBehaviorDiagramId = null;
    state.selectedBehaviorItem = null;
    state.behaviorTool = null;
    state.behaviorPending = null;
    state.behaviorTargetRegionId = null;
  }

  function clearStructuralSelection() {
    state.selectedDiagramId = null;
    state.selectedElementId = null;
    state.selectedRelationshipId = null;
    state.selectedPackageId = null;
    state.paletteTool = null;
    state.pendingRelationship = null;
  }

  // Diagram modes are mutually exclusive. The old prototype already enforced this
  // invariant; keeping both IDs selected caused two tabs/repository rows to appear
  // active and prevented BDD from becoming authoritative again after opening STM/SEQ.
  document.addEventListener('click', (event) => {
    const tab = event.target.closest?.('.diagram-tab');
    if (tab && /·\s*BDD\s*$/.test(tab.textContent || '')) {
      clearBehaviorSelection();
      return;
    }
    const row = event.target.closest?.('.diagram-row');
    if (row && /BDD\s*$/.test(row.textContent || '')) clearBehaviorSelection();
  }, true);

  function flattenVertices(regions, output = []) {
    for (const region of regions || []) {
      for (const vertex of region.vertices || []) {
        output.push(vertex);
        if (vertex.kind?.State?.regions) flattenVertices(vertex.kind.State.regions, output);
      }
    }
    return output;
  }

  function kindName(vertex) {
    if (vertex.kind === 'FinalState' || vertex.kind?.FinalState != null) return 'FinalState';
    if (vertex.kind?.State != null || vertex.kind === 'State') return 'State';
    return vertex.kind?.Pseudostate || '';
  }

  function ensureStatePresentations(diagram) {
    const machine = state.behaviorSnapshot?.repository?.state_machines?.[diagram.semantic_id];
    const frame = document.querySelector('.state-machine-frame');
    if (!machine || !frame) return;
    const presentations = new Map((diagram.state_nodes || []).map((item) => [item.vertex_id, item]));
    const rendered = new Set(
      [...frame.querySelectorAll('.state-vertex[data-vertex-id]')]
        .map((node) => node.dataset.vertexId),
    );
    for (const vertex of flattenVertices(machine.regions)) {
      if (rendered.has(vertex.id)) continue;
      const p = presentations.get(vertex.id);
      if (!p) {
        console.error('Rust behavior presentation invariant violated: semantic State vertex has no presentation', vertex.id);
        $('status').textContent = 'State Machine presentation is incomplete. A semantic vertex has no Rust presentation record.';
        continue;
      }
      const kind = kindName(vertex);
      const node = document.createElement('button');
      node.className = `state-vertex state-${kind.toLowerCase()}`;
      node.dataset.vertexId = vertex.id;
      node.style.left = `${p.x}px`;
      node.style.top = `${p.y}px`;
      node.style.width = `${p.width}px`;
      node.style.height = `${p.height}px`;
      if (kind === 'State') {
        node.innerHTML = `<strong>${escapeHtml(vertex.name || 'State')}</strong>`;
      } else if (kind === 'Initial') node.innerHTML = '<span class="pseudo-dot"></span>';
      else if (kind === 'FinalState') node.innerHTML = '<span class="final-ring"><i></i></span>';
      else if (kind === 'Choice') node.innerHTML = '<span class="choice-diamond"></span>';
      else if (kind === 'Fork' || kind === 'Join') node.innerHTML = '<span class="fork-bar"></span>';
      else node.textContent = kind.replace(/([A-Z])/g, ' $1').trim();
      node.onclick = (click) => {
        click.stopPropagation();
        state.selectedBehaviorItem = { type: 'Vertex', id: vertex.id, semantic: vertex };
        render();
      };
      frame.appendChild(node);
    }
  }

  function enforceSingleMode() {
    if (state.selectedBehaviorDiagramId) {
      // A behavior diagram owns the workspace selection; semantic context may be
      // shown in Properties, but must not remain visually selected in Repository.
      state.selectedDiagramId = null;
      state.selectedElementId = null;
      state.selectedRelationshipId = null;
      state.selectedPackageId = null;
    }
  }

  const baseRender = render;
  render = function renderBehaviorRuntimeHardened() {
    enforceSingleMode();
    baseRender();
    const diagram = activeBehaviorDiagram();
    if (diagram?.kind === 'StateMachine') ensureStatePresentations(diagram);
  };

  // Expose one explicit mode-switch helper for later diagram-type PRs. New diagram
  // implementations should use this rather than layering independent selections.
  window.smpActivateDiagramMode = (mode, diagramId) => {
    if (mode === 'behavior') {
      clearStructuralSelection();
      state.selectedBehaviorDiagramId = diagramId;
    } else {
      clearBehaviorSelection();
      state.selectedDiagramId = diagramId;
    }
    render();
  };
})();