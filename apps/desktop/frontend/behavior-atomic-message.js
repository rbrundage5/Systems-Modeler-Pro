(() => {
  function activeSequence() {
    const diagram = state.behaviorSnapshot?.diagrams?.find(
      (item) => item.id === state.selectedBehaviorDiagramId,
    );
    return diagram?.kind === 'Sequence' ? diagram : null;
  }

  function bindAtomicMessageEditor() {
    const diagram = activeSequence();
    const selected = state.selectedBehaviorItem;
    const button = $('behavior-message-apply');
    if (!diagram || selected?.type !== 'Message' || !button) return;
    const message = selected.semantic;
    button.onclick = async () => {
      const argumentsValue = $('behavior-message-arguments').value
        .split(',')
        .map((value) => value.trim())
        .filter(Boolean);
      await runCommand('Updating Message…', () => requireInvoke()(
        'update_sequence_message_complete',
        {
          diagramId: diagram.id,
          messageIdValue: message.id,
          sort: message.sort,
          name: $('behavior-message-name').value,
          signatureId: $('behavior-message-signature')?.value || null,
          arguments: argumentsValue,
          order: Number($('behavior-message-order').value),
          sourceLifelineId: $('behavior-message-source').value || null,
          targetLifelineId: $('behavior-message-target').value || null,
        },
      ));
      await refresh();
    };
  }

  const previousRender = render;
  render = function renderWithAtomicMessageEditor() {
    previousRender();
    bindAtomicMessageEditor();
  };

  bindAtomicMessageEditor();
})();
