(() => {
  'use strict';
  let busy = false;

  async function edit(options) {
    if (busy) return;
    busy = true;
    try {
      const specification = await options.invoke('ibd_occurrence_specification', {
        diagramId: options.diagramId, presentationId: options.presentationId,
      });
      if (!options.isCurrent()) return;
      let groups = specification.groups || specification.suggested_groups;
      let failure = '';
      while (options.isCurrent()) {
        const answer = await window.smpDialogs.edit({ title: 'Separate part occurrences',
          description: `Definition multiplicity [${specification.coverage.constraint}]. Enter one display name = count per line. Names label occurrences of the existing property; they do not create new properties. ${failure}`,
          fields: [{ id: 'groups', label: 'Occurrence groups', value: groups, multiline: true, required: true }], confirmLabel: 'Apply occurrence view' });
        if (!answer || !options.isCurrent()) return;
        groups = answer.values.groups;
        try {
          await options.commit('set_ibd_occurrence_groups', { diagramId: options.diagramId, presentationId: options.presentationId, groups });
          await options.refresh(); return;
        } catch (error) {
          failure = error?.message || String(error);
          window.smpDialogs.notify(failure, 'error');
        }
      }
    } finally { busy = false; }
  }

  async function compact(options) {
    if (busy || !options.isCurrent()) return;
    busy = true;
    try {
      await options.commit('set_ibd_occurrence_groups', { diagramId: options.diagramId, presentationId: options.presentationId, groups: null });
      await options.refresh();
    } finally { busy = false; }
  }

  async function append(options, request) {
    if (busy) return;
    busy = true;
    try {
      let draft = { name: '', count: '1' }, failure = '';
      while (options.isCurrent()) {
        const answer = await window.smpDialogs.edit({ title: 'Add part occurrence',
          description: `Add a named subset of this existing property's population. An existing compact symbol becomes an occurrence view. ${failure}`,
          fields: [{ id: 'name', label: 'Occurrence display name', value: draft.name, required: true },
            { id: 'count', label: 'Count represented by this symbol', value: draft.count, required: true }], confirmLabel: 'Add occurrence' });
        if (!answer || !options.isCurrent()) return;
        draft = { ...answer.values };
        try {
          const id = await options.commit('append_ibd_occurrence', { diagramId: options.diagramId, request: { ...request, ...draft } });
          await options.refresh(); return id;
        } catch (error) {
          failure = error?.message || String(error);
          window.smpDialogs.notify(failure, 'error');
        }
      }
    } finally { busy = false; }
  }

  async function coverage(target, options) {
    try {
      const specification = await options.invoke('ibd_occurrence_specification', { diagramId: options.diagramId, presentationId: options.presentationId });
      if (!options.isCurrent()) return;
      const report = specification.coverage;
      target.textContent = `${report.status}${report.displayed === null ? '' : `: ${report.displayed} displayed`} · definition [${report.constraint}]. Counts apply per enclosing instance.`;
    } catch (error) { if (options.isCurrent()) target.textContent = error?.message || String(error); }
  }

  window.smpIbdOccurrences = Object.freeze({ edit, compact, append, coverage });
})();
