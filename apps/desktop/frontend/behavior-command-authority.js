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
})();
