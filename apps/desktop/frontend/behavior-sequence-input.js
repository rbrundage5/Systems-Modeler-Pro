(() => {
  const SECONDARY_TOOLS = new Set([
    'Execution', 'Invariant', 'alt', 'opt', 'loop', 'break', 'par', 'critical',
    'neg', 'assert', 'strict', 'seq', 'ignore', 'consider',
  ]);

  function activeSequenceDiagram() {
    return state.behaviorSnapshot?.diagrams?.find(
      (diagram) => String(diagram.id) === String(state.selectedBehaviorDiagramId)
        && diagram.kind === 'Sequence',
    ) || null;
  }

  function interaction(diagram) {
    return state.behaviorSnapshot?.repository?.interactions?.[String(diagram.semantic_id)] || null;
  }

  async function createSecondary(tool) {
    const diagram = activeSequenceDiagram();
    const semantic = diagram ? interaction(diagram) : null;
    if (!diagram || !semantic || state.behaviorTool !== tool) return;

    if (tool === 'Execution' || tool === 'Invariant') {
      const selected = state.selectedBehaviorItem;
      if (!selected || selected.type !== 'Lifeline') {
        alert(`Select a Lifeline first, then choose ${tool === 'Execution' ? 'Execution Specification' : 'State Invariant'}.`);
        state.behaviorTool = null;
        render();
        return;
      }
      state.behaviorTool = null;
      if (tool === 'Execution') {
        await runCommand('Adding Execution Specification…', () => requireInvoke()(
          'add_execution_specification',
          {
            diagramId: diagram.id,
            lifelineIdValue: selected.id,
          },
        ));
      } else {
        const constraint = prompt('State invariant constraint', 'state = Ready');
        if (!constraint) {
          render();
          return;
        }
        await runCommand('Adding State Invariant…', () => requireInvoke()('add_state_invariant', {
          diagramId: diagram.id,
          lifelineIdValue: selected.id,
          constraint,
        }));
      }
      await refresh();
      return;
    }

    const coveredLifelineIds = (semantic.lifelines || []).map((lifeline) => lifeline.id);
    if (!coveredLifelineIds.length) {
      alert('Add Lifelines before creating a Combined Fragment.');
      state.behaviorTool = null;
      render();
      return;
    }
    const guard = prompt(`${tool} operand guard (optional)`, '') || '';
    state.behaviorTool = null;
    await runCommand(`Adding ${tool} Combined Fragment…`, () => requireInvoke()(
      'add_combined_fragment',
      {
        diagramId: diagram.id,
        operator: tool,
        coveredLifelineIds,
        guard,
      },
    ));
    await refresh();
  }

  // Lifeline placement is a pointer-to-Rust-command bridge. Candidate property
  // paths and semantic legality come from Rust.
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

  // These tools are command-like rather than placement-like. The Rust-defined
  // palette activates the tool; this adapter forwards that intent once.
  document.addEventListener('click', (event) => {
    const button = event.target.closest?.('[data-behavior-tool]');
    const tool = button?.dataset.behaviorTool;
    if (!SECONDARY_TOOLS.has(tool)) return;
    queueMicrotask(() => {
      void createSecondary(tool);
    });
  });
})();
