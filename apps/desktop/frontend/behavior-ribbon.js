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
      ? 'create_state_machine_diagram_staged'
      : 'create_sequence_diagram_staged';

    try {
      const diagramId = await runCommand(
        `Creating ${label} diagram…`,
        () => requireInvoke()(command, { contextId: context.id, name }),
      );

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

  // app.js loads structural state directly after Open instead of calling the
  // behavior-aware refresh wrapper. Rehydrate the Rust behavior repository and
  // diagram presentations before rendering the reopened project.
  const baseOpenProject = openProject;
  async function openProjectWithBehaviorHydration() {
    await baseOpenProject();
    if (typeof window.smpLoadBehaviorSnapshot === 'function') {
      await window.smpLoadBehaviorSnapshot();
    } else {
      state.behaviorSnapshot = await requireInvoke()('behavior_snapshot');
    }
    state.selectedBehaviorDiagramId = null;
    state.selectedBehaviorItem = null;
    state.behaviorTool = null;
    state.behaviorPending = null;
    state.behaviorTargetRegionId = null;
    if (!state.selectedDiagramId && state.behaviorSnapshot?.diagrams?.length) {
      state.selectedBehaviorDiagramId = state.behaviorSnapshot.diagrams[0].id;
    }
    render();
  }

  window.smpCreateStateMachineForSelectedBlock = () => createBehaviorDiagram('StateMachine');
  window.smpCreateSequenceForSelectedBlock = () => createBehaviorDiagram('Sequence');

  const stateMachine = $('new-state-machine');
  const sequence = $('new-sequence');
  const open = $('open-project');
  if (stateMachine) stateMachine.onclick = window.smpCreateStateMachineForSelectedBlock;
  if (sequence) sequence.onclick = window.smpCreateSequenceForSelectedBlock;
  if (open) open.onclick = openProjectWithBehaviorHydration;
})();