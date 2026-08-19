(() => {
  const previousRequireInvoke = requireInvoke;
  requireInvoke = function requireInvokeWithSafeBehaviorTransitions() {
    const invoke = previousRequireInvoke();
    return (command, args = {}) => {
      if (command !== 'add_state_transition') return invoke(command, args);
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
    };
  };
})();
