(() => {
  async function deleteSelectedBehaviorItem() {
    const diagram = state.behaviorSnapshot?.diagrams?.find(
      (item) => item.id === state.selectedBehaviorDiagramId,
    );
    const selected = state.selectedBehaviorItem;
    if (!diagram || !selected?.id || !selected?.type) return false;
    const allowed = new Set(['Vertex', 'Transition', 'Lifeline', 'Message', 'Execution', 'Fragment', 'Invariant']);
    if (!allowed.has(selected.type)) return false;
    const label = selected.type === 'Vertex' ? 'State Machine vertex' : selected.type;
    if (!confirm(`Delete selected ${label}?`)) return true;
    await runCommand(`Deleting ${label}…`, () => requireInvoke()('delete_behavior_item', {
      diagramId: diagram.id,
      itemType: selected.type,
      itemId: selected.id,
    }));
    state.selectedBehaviorItem = null;
    state.behaviorPending = null;
    state.behaviorTool = null;
    await refresh();
    return true;
  }

  document.addEventListener('keydown', async (event) => {
    if (!['Delete', 'Backspace'].includes(event.key)) return;
    const tag = document.activeElement?.tagName?.toLowerCase();
    if (['input', 'textarea', 'select'].includes(tag)) return;
    if (await deleteSelectedBehaviorItem()) {
      event.preventDefault();
      event.stopPropagation();
    }
  }, true);

  const previousRenderProperties = renderProperties;
  renderProperties = function renderPropertiesWithBehaviorDelete() {
    previousRenderProperties();
    const selected = state.selectedBehaviorItem;
    if (!selected?.id) return;
    const allowed = new Set(['Vertex', 'Transition', 'Lifeline', 'Message', 'Execution', 'Fragment', 'Invariant']);
    if (!allowed.has(selected.type)) return;
    const button = document.createElement('button');
    button.className = 'danger';
    button.textContent = 'Delete';
    button.onclick = deleteSelectedBehaviorItem;
    $('properties')?.appendChild(button);
  };
})();
