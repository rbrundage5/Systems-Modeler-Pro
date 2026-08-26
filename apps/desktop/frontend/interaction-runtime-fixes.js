(() => {
  function behaviorDiagram() {
    return state.behaviorSnapshot?.diagrams?.find(
      (diagram) => String(diagram.id) === String(state.selectedBehaviorDiagramId),
    ) || null;
  }

  function flattenTransitions(regions, output = []) {
    for (const region of regions || []) {
      for (const transition of region.transitions || []) output.push(transition);
      for (const vertex of region.vertices || []) {
        const children = vertex.kind?.State?.regions || [];
        if (children.length) flattenTransitions(children, output);
      }
    }
    return output;
  }

  function pointForState(presentation) {
    return {
      x: presentation.x + presentation.width / 2,
      y: presentation.y + presentation.height / 2,
    };
  }

  function boundaryPoint(rect, toward) {
    const center = pointForState(rect);
    const dx = toward.x - center.x;
    const dy = toward.y - center.y;
    if (Math.abs(dx) < 0.001 && Math.abs(dy) < 0.001) return center;
    const scaleX = Math.abs(dx) > 0.001
      ? Math.max(1, rect.width / 2) / Math.abs(dx)
      : Number.POSITIVE_INFINITY;
    const scaleY = Math.abs(dy) > 0.001
      ? Math.max(1, rect.height / 2) / Math.abs(dy)
      : Number.POSITIVE_INFINITY;
    const scale = Math.min(scaleX, scaleY);
    return { x: center.x + dx * scale, y: center.y + dy * scale };
  }

  function previewRoute(diagram, transition, source, target, movedVertexId, next) {
    const stored = (diagram.edge_routes || []).find(
      (route) => String(route.semantic_id) === String(transition.id),
    );
    const points = (stored?.points?.length >= 2
      ? stored.points
      : [pointForState(source), pointForState(target)])
      .map((point) => ({ x: point.x, y: point.y }));
    const movedSource = String(transition.source_id) === String(movedVertexId);
    const movedTarget = String(transition.target_id) === String(movedVertexId);
    if (movedSource && movedTarget) {
      const original = diagram.state_nodes?.find(
        (item) => String(item.vertex_id) === String(movedVertexId),
      );
      const dx = next.x - (original?.x ?? next.x);
      const dy = next.y - (original?.y ?? next.y);
      return points.map((point) => ({ x: point.x + dx, y: point.y + dy }));
    }
    if (movedSource) {
      points[0] = boundaryPoint(source, points[1] || pointForState(target));
    }
    if (movedTarget) {
      const last = points.length - 1;
      points[last] = boundaryPoint(target, points[last - 1] || pointForState(source));
    }
    return points;
  }

  function updateStateTransitionGeometry(diagram, movedVertexId, next) {
    const machine = state.behaviorSnapshot?.repository?.state_machines?.[String(diagram.semantic_id)];
    if (!machine) return;
    const positions = new Map((diagram.state_nodes || []).map((item) => [String(item.vertex_id), item]));
    positions.set(String(movedVertexId), next);
    for (const transition of flattenTransitions(machine.regions || [])) {
      if (String(transition.source_id) !== String(movedVertexId)
          && String(transition.target_id) !== String(movedVertexId)) continue;
      const source = positions.get(String(transition.source_id));
      const target = positions.get(String(transition.target_id));
      if (!source || !target) continue;
      const line = document.querySelector(
        `#canvas .state-transition[data-transition-id="${CSS.escape(String(transition.id))}"]`,
      );
      if (!line) continue;
      const points = previewRoute(diagram, transition, source, target, movedVertexId, next);
      line.setAttribute('points', points.map((point) => `${point.x},${point.y}`).join(' '));
    }
  }

  window.smpPreviewStateTransitionGeometry = updateStateTransitionGeometry;

  function updateSequenceMessages(interaction, lifelineId, x) {
    for (const message of interaction?.messages || []) {
      const line = document.querySelector(
        `#canvas .sequence-message[data-message-id="${CSS.escape(String(message.id))}"]`,
      );
      if (!line) continue;
      if (String(message.send_event?.lifeline_id || '') === String(lifelineId)) line.setAttribute('x1', x);
      if (String(message.receive_event?.lifeline_id || '') === String(lifelineId)) line.setAttribute('x2', x);
    }
  }

  function bindSequenceConnectedDrag() {
    const diagram = behaviorDiagram();
    if (!diagram || diagram.kind !== 'Sequence') return;
    const interaction = state.behaviorSnapshot?.repository?.interactions?.[String(diagram.semantic_id)];
    if (!interaction) return;
    document.querySelectorAll('#canvas .sequence-lifeline').forEach((node) => {
      const lifelineId = node.dataset.lifelineId;
      const presentation = diagram.lifelines?.find(
        (item) => String(item.lifeline_id) === String(lifelineId),
      );
      if (!presentation) return;
      node.onpointerdown = (event) => {
        if (event.button !== 0 || event.target.closest?.('.lifeline-resize-handle')) return;
        if (state.behaviorPending || state.behaviorTool) return;
        event.preventDefault();
        event.stopPropagation();
        const startX = event.clientX;
        const originalX = presentation.x;
        let nextX = originalX;
        node.classList.add('smp-dragging');
        node.setPointerCapture?.(event.pointerId);
        node.onpointermove = (move) => {
          nextX = Math.max(70, originalX + move.clientX - startX);
          node.style.left = `${nextX - 65}px`;
          updateSequenceMessages(interaction, lifelineId, nextX);
        };
        node.onpointerup = async () => {
          node.onpointermove = null;
          node.onpointerup = null;
          node.classList.remove('smp-dragging');
          await runCommand('Moving Lifeline…', () => requireInvoke()('move_sequence_lifeline', {
            diagramId: diagram.id,
            lifelineIdValue: String(lifelineId),
            x: nextX,
          }));
          await refresh();
        };
      };
    });
  }

  function installRuntimeInteractionFixes() {
    bindSequenceConnectedDrag();
    void window.smpSynchronizeSelectedDiagramContext?.();
  }

  const baseRender = render;
  render = function renderWithRuntimeInteractionFixes() {
    baseRender();
    installRuntimeInteractionFixes();
  };
  queueMicrotask(installRuntimeInteractionFixes);
})();

(() => {
  const canvas = document.getElementById('canvas');
  if (!canvas) return;

  const DOUBLE_CLICK_WINDOW_MS = 500;
  let lastActivation = null;

  function packagePresentation(target) {
    if (!(target instanceof Element)) return null;
    return target.closest('.package-diagram [data-presentation-id]');
  }

  function semanticElementFor(presentation) {
    const application = window.smpState;
    const diagram = application?.snapshot?.diagrams?.find(
      (candidate) => String(candidate.id) === String(application.selectedDiagramId)
        && candidate.family === 'package',
    );
    const node = diagram?.nodes?.find(
      (candidate) => String(candidate.id) === String(presentation.dataset.presentationId),
    );
    return application?.snapshot?.project?.elements?.find(
      (candidate) => String(candidate.id) === String(node?.element_id),
    ) || null;
  }

  function hasExistingDrillDownTarget(element) {
    if (!element) return false;
    const application = window.smpState;
    const currentDiagramId = String(application?.selectedDiagramId || '');
    const elementId = String(element.id);

    if ((application?.snapshot?.diagrams || []).some((diagram) =>
      String(diagram.id) !== currentDiagramId
      && (String(diagram.owner_id || '') === elementId
        || String(diagram.semantic_context_id || '') === elementId))) return true;

    if ((application?.snapshot?.ibd_diagrams || []).some(
      (diagram) => String(diagram.context_block_id || '') === elementId,
    )) return true;

    if ((application?.behaviorSnapshot?.diagrams || []).some(
      (diagram) => String(diagram.context_id || '') === elementId,
    )) return true;

    return (application?.activitySnapshot?.diagrams || []).some((diagram) =>
      String(diagram.context_id || diagram.owner_id || '') === elementId);
  }

  // Package selection re-renders the presentation on the first click. Native
  // dblclick therefore cannot reliably target the same DOM node twice. Key the
  // activation by stable presentation id and delegate to the existing handler.
  canvas.addEventListener('click', (event) => {
    const presentation = packagePresentation(event.target);
    if (!presentation
      || window.smpState?.pendingRelationship
      || window.smpState?.paletteTool) {
      lastActivation = null;
      return;
    }

    const key = String(presentation.dataset.presentationId || '');
    if (!key) return;
    const now = performance.now();
    const isDoubleActivation = lastActivation
      && lastActivation.key === key
      && now - lastActivation.at <= DOUBLE_CLICK_WINDOW_MS;
    lastActivation = { key, at: now };
    if (!isDoubleActivation) return;

    lastActivation = null;
    event.preventDefault();
    event.stopImmediatePropagation();

    const element = semanticElementFor(presentation);
    if (!hasExistingDrillDownTarget(element)) {
      window.smpDialogs?.notify?.(
        `No existing diagram is owned by or context-bound to ${element?.name || 'this element'}.`,
        'info',
      );
      return;
    }

    if (typeof presentation.ondblclick !== 'function') {
      window.smpDialogs?.notify?.('Drill-down is unavailable for this presentation.', 'warning');
      return;
    }

    Promise.resolve(presentation.ondblclick(event)).catch((error) => {
      window.smpDialogs?.notify?.(error?.message || String(error), 'error');
    });
  }, true);
})();

// Keep the shared Rust workspace context aligned with the diagram the application
// actually has selected. Several creation paths historically updated local UI
// selection before activating the shared host, leaving Copy/Paste, frames and
// interaction revisions pointed at the previous diagram.
(() => {
  let activationSerial = 0;

  function projectElement(id) {
    return window.smpState?.snapshot?.project?.elements?.find(
      (element) => String(element.id) === String(id),
    ) || null;
  }

  function selectedDiagramContext() {
    const application = window.smpState;
    if (!application) return null;

    if (application.selectedActivityDiagramId) {
      const diagram = application.activitySnapshot?.diagrams?.find(
        (candidate) => String(candidate.id) === String(application.selectedActivityDiagramId),
      );
      const activity = diagram
        ? application.activitySnapshot?.repository?.activities?.[String(diagram.activity_id)]
        : null;
      if (!diagram) return null;
      return {
        diagramId: diagram.id,
        familyId: 'activity',
        name: diagram.name,
        modelElementName: activity?.name || diagram.name,
        semanticContextId: diagram.activity_id || '',
      };
    }

    if (application.selectedBehaviorDiagramId) {
      const diagram = application.behaviorSnapshot?.diagrams?.find(
        (candidate) => String(candidate.id) === String(application.selectedBehaviorDiagramId),
      );
      if (!diagram) return null;
      const semantic = diagram.kind === 'Sequence'
        ? application.behaviorSnapshot?.repository?.interactions?.[String(diagram.semantic_id)]
        : application.behaviorSnapshot?.repository?.state_machines?.[String(diagram.semantic_id)];
      return {
        diagramId: diagram.id,
        familyId: diagram.kind === 'Sequence' ? 'sequence' : 'state-machine',
        name: diagram.name,
        modelElementName: semantic?.name || diagram.name,
        semanticContextId: diagram.context_id || '',
      };
    }

    if (!application.selectedDiagramId) return null;
    const ibd = application.snapshot?.ibd_diagrams?.find(
      (candidate) => String(candidate.id) === String(application.selectedDiagramId),
    );
    if (ibd) {
      return {
        diagramId: ibd.id,
        familyId: 'ibd',
        name: ibd.name,
        modelElementName: projectElement(ibd.context_block_id)?.name || ibd.name,
        semanticContextId: ibd.context_block_id || '',
      };
    }

    const diagram = application.snapshot?.diagrams?.find(
      (candidate) => String(candidate.id) === String(application.selectedDiagramId),
    );
    if (!diagram) return null;
    const contextId = diagram.semantic_context_id || diagram.owner_id || '';
    return {
      diagramId: diagram.id,
      familyId: diagram.family || 'bdd',
      name: diagram.name,
      modelElementName: projectElement(contextId)?.name
        || application.snapshot?.project?.name
        || diagram.name,
      semanticContextId: contextId,
    };
  }

  function sameContext(current, desired) {
    return String(current?.diagramId || current?.diagram_id || '') === String(desired.diagramId)
      && String(current?.family?.id || current?.familyId || '') === String(desired.familyId)
      && String(current?.name || '') === String(desired.name)
      && String(current?.modelElementName || current?.model_element_name || '') === String(desired.modelElementName);
  }

  async function synchronizeSelectedDiagramContext() {
    const host = window.smpRendererHost;
    const desired = selectedDiagramContext();
    if (!host?.activate || !desired) return false;
    if (sameContext(host.context?.(), desired)) return false;
    const serial = ++activationSerial;
    try {
      await host.activate(desired);
      return serial === activationSerial;
    } catch (error) {
      window.smpDialogs?.notify?.(
        `Unable to activate ${desired.name}: ${error?.message || String(error)}`,
        'error',
      );
      return false;
    }
  }

  window.smpSynchronizeSelectedDiagramContext = synchronizeSelectedDiagramContext;

  // If an Edit command is clicked during the brief interval between local
  // selection and shared-host activation, synchronize first instead of issuing
  // the command against the stale diagram and producing a false error.
  document.addEventListener('click', (event) => {
    const control = event.target.closest?.('[data-standard-ribbon], [data-standard-command]');
    if (!control) return;
    const desired = selectedDiagramContext();
    const current = window.smpRendererHost?.context?.();
    if (!desired || sameContext(current, desired)) return;
    const command = control.dataset.standardRibbon || control.dataset.standardCommand;
    if (!command || !window.smpStandardEditing?.run) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    void synchronizeSelectedDiagramContext().then(() => window.smpStandardEditing.run(command));
  }, true);

  queueMicrotask(() => void synchronizeSelectedDiagramContext());
})();
