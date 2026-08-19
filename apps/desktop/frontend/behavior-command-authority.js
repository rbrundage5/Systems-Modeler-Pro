(() => {
  const previousRequireInvoke = requireInvoke;

  requireInvoke = function requireInvokeWithBehaviorAuthority() {
    const invoke = previousRequireInvoke();
    return async (command, args = {}) => {
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

      const result = await invoke(command, args);

      // Rust restores BDD, IBD, State Machine and Sequence metadata atomically
      // inside open_project_file. Hydrate the behavior snapshot before the
      // command promise resolves so every frontend Open path renders from the
      // fully restored Rust workspace on its first render.
      if (command === 'open_project_file') {
        if (typeof window.smpLoadBehaviorSnapshot !== 'function') {
          throw new Error('Behavior snapshot loader is unavailable after project open.');
        }
        await window.smpLoadBehaviorSnapshot();
        state.selectedBehaviorDiagramId = null;
        state.selectedBehaviorItem = null;
        state.behaviorTool = null;
        state.behaviorPending = null;
      }

      return result;
    };
  };
})();
