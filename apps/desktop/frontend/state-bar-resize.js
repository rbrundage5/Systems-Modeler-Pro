(() => {
  const STORED_HEIGHT_OFFSET = 12;
  const MIN_BAR_THICKNESS = 8;
  const MAX_BAR_THICKNESS = 24;

  function activeStateDiagram() {
    return state.behaviorSnapshot?.diagrams?.find(
      (diagram) => String(diagram.id) === String(state.selectedBehaviorDiagramId),
    ) || null;
  }

  function clampThickness(value) {
    return Math.max(MIN_BAR_THICKNESS, Math.min(MAX_BAR_THICKNESS, value));
  }

  function displayThickness(presentation) {
    // Legacy STM Fork/Join presentations were created as generic 24x24
    // pseudostates even though the visible notation was a 24x6 bar. Preserve
    // that appearance until the user explicitly resizes the bar.
    if (Number(presentation.width) === 24 && Number(presentation.height) === 24) return 6;

    // Resized Fork/Join bars encode their visible thickness into the existing
    // Rust-owned presentation height. The +12 offset keeps persisted geometry
    // above the generic State Machine 20px minimum while still allowing an
    // 8..24px visible synchronization bar.
    return clampThickness((Number(presentation.height) || 20) - STORED_HEIGHT_OFFSET);
  }

  function storedHeightForThickness(thickness) {
    return clampThickness(thickness) + STORED_HEIGHT_OFFSET;
  }

  function updateIncidentTransitions(diagram, vertexId, next) {
    if (typeof window.smpPreviewStateTransitionGeometry === 'function') {
      window.smpPreviewStateTransitionGeometry(diagram, vertexId, next);
      return;
    }
    const machine = state.behaviorSnapshot?.repository?.state_machines?.[String(diagram.semantic_id)];
    if (!machine) return;
    const transitions = [];
    const walk = (regions) => {
      for (const region of regions || []) {
        transitions.push(...(region.transitions || []));
        for (const vertex of region.vertices || []) walk(vertex.kind?.State?.regions || []);
      }
    };
    walk(machine.regions || []);
    const positions = new Map((diagram.state_nodes || []).map((item) => [String(item.vertex_id), item]));
    positions.set(String(vertexId), next);
    for (const transition of transitions) {
      if (String(transition.source_id) !== String(vertexId)
          && String(transition.target_id) !== String(vertexId)) continue;
      const source = positions.get(String(transition.source_id));
      const target = positions.get(String(transition.target_id));
      if (!source || !target) continue;
      const line = document.querySelector(
        `#canvas .state-transition[data-transition-id="${CSS.escape(String(transition.id))}"]`,
      );
      if (!line) continue;
      line.setAttribute('x1', source.x + source.width / 2);
      line.setAttribute('y1', source.y + source.height / 2);
      line.setAttribute('x2', target.x + target.width / 2);
      line.setAttribute('y2', target.y + target.height / 2);
    }
  }

  function bindStateBars() {
    const diagram = activeStateDiagram();
    if (!diagram || diagram.kind !== 'StateMachine') return;

    document.querySelectorAll('#canvas .state-fork, #canvas .state-join').forEach((node) => {
      const vertexId = node.dataset.vertexId;
      const presentation = diagram.state_nodes?.find(
        (item) => String(item.vertex_id) === String(vertexId),
      );
      const bar = node.querySelector('.fork-bar');
      const handle = node.querySelector('.smp-resize-handle');
      if (!presentation || !bar || !handle) return;

      const renderedThickness = displayThickness(presentation);
      bar.style.width = '100%';
      bar.style.height = `${renderedThickness}px`;

      handle.onpointerdown = (event) => {
        if (event.button !== 0 || state.behaviorPending || state.behaviorTool) return;
        event.preventDefault();
        event.stopPropagation();
        const startX = event.clientX;
        const startY = event.clientY;
        const original = { ...presentation };
        const originalThickness = displayThickness(original);
        let next = { ...original };
        let nextThickness = originalThickness;
        handle.setPointerCapture?.(event.pointerId);
        handle.onpointermove = (move) => {
          next.width = Math.max(24, original.width + move.clientX - startX);
          nextThickness = clampThickness(originalThickness + move.clientY - startY);
          next.height = storedHeightForThickness(nextThickness);
          node.style.width = `${next.width}px`;
          node.style.height = `${next.height}px`;
          bar.style.width = '100%';
          bar.style.height = `${nextThickness}px`;
          updateIncidentTransitions(diagram, vertexId, next);
        };
        handle.onpointerup = async () => {
          handle.onpointermove = null;
          handle.onpointerup = null;
          await window.smpCommitPresentationGeometry('update_state_presentation_geometry', {
            diagramId: diagram.id,
            stateVertexId: String(vertexId),
            x: next.x,
            y: next.y,
            width: next.width,
            height: next.height,
          });
        };
      };
    });
  }

  const baseRender = render;
  render = () => {
    baseRender();
    bindStateBars();
  };
  queueMicrotask(bindStateBars);
})();

(() => {
  // PR12 established Behavior as Rust-authoritative state. Keep STM/SEQ hydration
  // explicit because the structural "complete" snapshot does not carry Behavior.
  // This wrapper preserves the selected behavior diagram across general refreshes
  // and then replaces the cached Behavior snapshot from Rust before the final render.
  let behaviorRefreshInProgress = false;

  async function refreshBehaviorSnapshotPreservingSelection() {
    if (behaviorRefreshInProgress) return;
    behaviorRefreshInProgress = true;
    const selectedDiagramId = state.selectedBehaviorDiagramId;
    try {
      state.behaviorSnapshot = await requireInvoke()('behavior_snapshot');
      if (selectedDiagramId && state.behaviorSnapshot?.diagrams?.some(
        (diagram) => String(diagram.id) === String(selectedDiagramId),
      )) {
        state.selectedBehaviorDiagramId = selectedDiagramId;
      } else if (selectedDiagramId) {
        state.selectedBehaviorDiagramId = null;
        state.selectedBehaviorItem = null;
        state.behaviorTool = null;
        state.behaviorPending = null;
      }
    } finally {
      behaviorRefreshInProgress = false;
    }
  }

  window.smpRefreshBehaviorSnapshot = refreshBehaviorSnapshotPreservingSelection;

  const baseRefresh = refresh;
  refresh = async function refreshWithAuthoritativeBehavior() {
    const selectedDiagramId = state.selectedBehaviorDiagramId;
    await baseRefresh();
    // Earlier refresh wrappers may clear behavior selection because the structural
    // complete snapshot has no Behavior payload. Rehydrate from the dedicated Rust
    // Behavior command, then restore selection only if the diagram still exists.
    state.selectedBehaviorDiagramId = selectedDiagramId;
    await refreshBehaviorSnapshotPreservingSelection();
    render();
  };
})();
