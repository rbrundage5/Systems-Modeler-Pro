(() => {
  function activeSequenceDiagram() {
    return state.behaviorSnapshot?.diagrams?.find(
      (diagram) => diagram.id === state.selectedBehaviorDiagramId && diagram.kind === 'Sequence',
    ) || null;
  }

  function interaction(diagram) {
    return diagram
      ? state.behaviorSnapshot?.repository?.interactions?.[diagram.semantic_id] || null
      : null;
  }

  function occurrenceY(message) {
    const order = message.send_event?.order
      ?? Math.max(0, (message.receive_event?.order ?? 1) - 1);
    return 110 + order * 4;
  }

  function lifelineNodes(interactionValue) {
    const nodes = [...document.querySelectorAll('.sequence-frame .sequence-lifeline')];
    const byId = new Map();
    (interactionValue?.lifelines || []).forEach((lifeline, index) => {
      const node = nodes[index];
      if (!node) return;
      node.dataset.lifelineId = lifeline.id;
      byId.set(lifeline.id, node);
    });
    return byId;
  }

  function shortenLifelineAt(node, absoluteY) {
    if (!node) return;
    const line = node.querySelector('.lifeline-line');
    if (!line) return;
    const nodeTop = Number.parseFloat(node.style.top || '60') || 60;
    const lineTop = 42;
    line.style.bottom = 'auto';
    line.style.height = `${Math.max(0, absoluteY - nodeTop - lineTop)}px`;
  }

  function startLifelineAt(node, absoluteY) {
    if (!node) return;
    const headHeight = 42;
    node.classList.add('created-lifeline');
    node.style.top = `${Math.max(60, absoluteY - headHeight / 2)}px`;
  }

  function addDestructionMarker(frame, diagram, message, y) {
    const targetId = message.receive_event?.lifeline_id;
    const presentation = diagram.lifelines?.find((item) => item.lifeline_id === targetId);
    if (!presentation) return;
    const marker = document.createElement('button');
    marker.type = 'button';
    marker.className = 'sequence-destruction-marker';
    marker.style.left = `${presentation.x - 11}px`;
    marker.style.top = `${y - 11}px`;
    marker.setAttribute('aria-label', 'Destruction occurrence');
    marker.title = 'Destruction occurrence';
    marker.onclick = (event) => {
      event.stopPropagation();
      state.selectedBehaviorItem = { type: 'Message', id: message.id, semantic: message };
      render();
    };
    frame.appendChild(marker);
  }

  function decorateMessageLines(interactionValue) {
    const lines = [...document.querySelectorAll('.sequence-message-layer line.sequence-message')];
    (interactionValue?.messages || []).forEach((message, index) => {
      const line = lines[index];
      if (!line) return;
      line.dataset.messageId = message.id;
      line.dataset.messageSort = message.sort;
      if (message.sort === 'Reply') {
        line.setAttribute('stroke-dasharray', '6 4');
        line.setAttribute('marker-end', 'url(#seq-open)');
      }
    });
  }

  function decorateSequenceMessageNotation() {
    const diagram = activeSequenceDiagram();
    if (!diagram) return;
    const interactionValue = interaction(diagram);
    const frame = document.querySelector('.sequence-frame');
    if (!frame || !interactionValue) return;

    const lifelines = lifelineNodes(interactionValue);
    decorateMessageLines(interactionValue);

    for (const message of interactionValue.messages || []) {
      const y = occurrenceY(message);
      if (message.sort === 'Create' && message.receive_event) {
        startLifelineAt(lifelines.get(message.receive_event.lifeline_id), y);
      }
      if (message.sort === 'Delete' && message.receive_event) {
        const target = lifelines.get(message.receive_event.lifeline_id);
        shortenLifelineAt(target, y);
        addDestructionMarker(frame, diagram, message, y);
      }
    }
  }

  const baseRenderCanvas = renderCanvas;
  renderCanvas = function renderCanvasWithMessageNotation() {
    const result = baseRenderCanvas();
    decorateSequenceMessageNotation();
    return result;
  };
})();
