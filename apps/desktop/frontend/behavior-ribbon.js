(() => {
  function selectedClassifier() {
    const id = state.selectedElementId;
    const element = state.snapshot?.project?.elements?.find((candidate) => candidate.id === id);
    return element && ['Block', 'AssociationBlock', 'InterfaceBlock'].includes(element.kind) ? element : null;
  }

  async function createBehaviorDiagram(kind) {
    const context = selectedClassifier();
    const label = kind === 'StateMachine' ? 'State Machine' : 'Sequence';
    if (!context) {
      alert(`Select a Block (or other Block-like classifier) in the Model Repository first, then create the ${label} diagram.`);
      return;
    }

    const name = prompt(`${label} diagram name`, `${context.name} ${label}`);
    if (!name) return;

    const command = kind === 'StateMachine'
      ? 'create_state_machine_diagram'
      : 'create_sequence_diagram';

    try {
      const diagramId = await runCommand(
        `Creating ${label} diagram…`,
        () => requireInvoke()(command, { contextId: context.id, name }),
      );

      // Do not depend on the chained global refresh wrappers to discover a newly
      // created behavior diagram. Read the authoritative Rust behavior workspace
      // immediately and select the exact ID returned by the creation command.
      state.behaviorSnapshot = await requireInvoke()('behavior_snapshot');
      if (!state.behaviorSnapshot?.diagrams?.some((diagram) => diagram.id === diagramId)) {
        throw new Error(`${label} was created by Rust but was not returned by the behavior workspace snapshot.`);
      }

      state.selectedBehaviorDiagramId = diagramId;
      state.selectedDiagramId = null;
      state.selectedElementId = null;
      state.selectedRelationshipId = null;
      state.selectedBehaviorItem = null;
      state.behaviorTool = null;
      state.behaviorPending = null;
      render();
      $('status').textContent = `${label} created for ${context.name}`;
    } catch (error) {
      const message = error?.message || String(error);
      console.error(`Unable to create ${label}`, error);
      $('status').textContent = `${label} creation failed: ${message}`;
      alert(`${label} creation failed: ${message}`);
    }
  }

  window.smpCreateStateMachineForSelectedBlock = () => createBehaviorDiagram('StateMachine');
  window.smpCreateSequenceForSelectedBlock = () => createBehaviorDiagram('Sequence');

  const stateMachine = $('new-state-machine');
  const sequence = $('new-sequence');
  if (stateMachine) stateMachine.onclick = window.smpCreateStateMachineForSelectedBlock;
  if (sequence) sequence.onclick = window.smpCreateSequenceForSelectedBlock;
})();