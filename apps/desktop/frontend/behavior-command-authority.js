(() => {
  const previousRequireInvoke = requireInvoke;

  requireInvoke = function requireInvokeWithBehaviorAuthority() {
    const invoke = previousRequireInvoke();
    return (command, args = {}) => {
      if (command === 'create_state_machine_diagram') {
        return invoke('create_state_machine_diagram_staged', args);
      }
      if (command === 'create_sequence_diagram') {
        return invoke('create_sequence_diagram_staged', args);
      }
      if (command === 'add_state_transition') {
        return invoke('add_state_transition_complete', {
          diagramId: args.diagramId,
          sourceVertexId: args.sourceVertexId,
          targetVertexId: args.targetVertexId,
          kind: args.kind,
          eventKind: args.eventKind ?? null,
          eventReferenceId: args.eventReferenceId ?? null,
          eventExpression: args.eventExpression ?? null,
          guard: args.guard ?? null,
          effect: args.effect ?? null,
        });
      }
      return invoke(command, args);
    };
  };

  // app.js historically reloads only the structural workspace snapshot after
  // Open. Because behavior metadata is restored by Rust in the same project-open
  // command, hydrate the Rust behavior snapshot immediately after that handler
  // completes so saved STM/SEQ diagrams appear without creating another diagram.
  const openProjectButton = $('open-project');
  const previousOpenProject = openProjectButton?.onclick;
  if (openProjectButton && previousOpenProject) {
    openProjectButton.onclick = async function openProjectWithBehaviorHydration(event) {
      const result = await previousOpenProject.call(this, event);
      await window.smpLoadBehaviorSnapshot?.();
      state.selectedBehaviorDiagramId = null;
      state.selectedBehaviorItem = null;
      state.behaviorTool = null;
      state.behaviorPending = null;
      render();
      return result;
    };
  }
})();
