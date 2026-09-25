(() => {
  let busy = false;
  window.smpAuthorComposition = async (diagramId, ownerId, typeId) => {
    if (busy) return;
    busy = true;
    try {
      const project = state.snapshot?.project;
      const owner = project?.elements.find(element => element.id === ownerId);
      const type = project?.elements.find(element => element.id === typeId);
      if (!owner || !type) throw new Error('The selected Block definition is no longer available.');
      const parts = project.elements.filter(element => element.kind === 'PartProperty' && element.owner_id === ownerId && element.type_id === typeId);
      let choice;
      while (!choice) {
        const selected = await window.smpDialogs.choose({ title: 'Composition part property',
          description: `${owner.name} owns the part. Its existing Block type is ${type.name}. Choose a usage explicitly.`,
          candidates: [{ id: '__new__', label: 'Create a new named part property' },
            ...parts.map(part => ({ id: part.id, label: `Reuse ${part.name}: ${type.name} [${part.multiplicity || '1'}]` }))],
          confirmLabel: 'Continue' });
        if (!selected) return;
        choice = selected.selectedId;
        if (!choice) window.smpDialogs.notify('Select an existing usage or Create a new named part property.', 'warning');
      }
      let draft = { name: '', multiplicity: '1' };
      let errorText = '';
      while (true) {
        const request = { ownerId, typeId, propertyId: choice === '__new__' ? null : choice, name: null, multiplicity: null };
        if (choice === '__new__') {
          const answer = await window.smpDialogs.edit({ title: 'Create composite part',
            description: `${owner.name} owns this property; ${type.name} remains a reusable Block definition. Aggregation: composite.${errorText ? ` ${errorText}` : ''}`,
            fields: [{ id: 'name', label: 'Part property name', value: draft.name, required: true },
              { id: 'multiplicity', label: 'Multiplicity per containing instance', value: draft.multiplicity, required: true }], confirmLabel: 'Create composition' });
          if (!answer) return;
          draft = { ...answer.values };
          Object.assign(request, draft);
        }
        try {
          const id = await runCommand('Creating BDD part composition…', () => requireInvoke()('author_part_composition', { diagramId, request }));
          state.selectedRelationshipId = id;
          return id;
        } catch (error) {
          errorText = error?.message || String(error);
          if (choice !== '__new__') throw error;
          window.smpDialogs.notify(errorText, 'error');
        }
      }
    } finally { busy = false; }
  };
})();
