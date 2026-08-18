(() => {
  function activeSequenceDiagram() {
    return state.behaviorSnapshot?.diagrams?.find(
      (diagram) => String(diagram.id) === String(state.selectedBehaviorDiagramId)
        && diagram.kind === 'Sequence',
    ) || null;
  }

  document.addEventListener('click', async (event) => {
    const frame = event.target.closest?.('.sequence-frame');
    if (!frame || event.target !== frame || state.behaviorTool !== 'Lifeline') return;
    const diagram = activeSequenceDiagram();
    if (!diagram) return;

    event.preventDefault();
    event.stopImmediatePropagation();

    const candidates = await requireInvoke()('behavior_lifeline_candidates', {
      diagramId: diagram.id,
    });
    if (!candidates.length) {
      alert('This Block has no Part/Reference Properties to represent as Lifelines. Create structural properties first.');
      return;
    }

    const menu = candidates.map((item, index) => `${index + 1}. ${item.label}`).join('\n');
    const answer = prompt(`Choose represented property path:\n${menu}`, '1');
    const candidate = candidates[Number(answer) - 1];
    if (!candidate) return;

    const rect = frame.getBoundingClientRect();
    const x = Math.max(80, event.clientX - rect.left);
    await runCommand('Adding Lifeline…', () => requireInvoke()('add_sequence_lifeline', {
      diagramId: diagram.id,
      representedPath: candidate.property_path,
      x,
    }));
    state.behaviorTool = null;
    await refresh();
  }, true);
})();
