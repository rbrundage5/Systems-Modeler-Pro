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
      state.selectedBehaviorDiagramId = diagramId;
      state.selectedDiagramId = null;
      state.selectedElementId = null;
      state.selectedRelationshipId = null;
      await refresh();
      $('status').textContent = `${label} created for ${context.name}`;
    } catch (error) {
      console.error(`Unable to create ${label}`, error);
    }
  }

  const stateMachine = $('new-state-machine');
  const sequence = $('new-sequence');
  if (stateMachine) stateMachine.onclick = () => createBehaviorDiagram('StateMachine');
  if (sequence) sequence.onclick = () => createBehaviorDiagram('Sequence');
})();
