(() => {
  'use strict';
  let active = null;
  let generation = 0;

  function render(options) {
    const { container, projectId, diagram, relationship, invoke, refresh, isCurrent, route } = options;
    const key = `${projectId}:${diagram.id}:${relationship.id}`;
    const saved = JSON.stringify([relationship, diagram]);
    if (active?.key === key && active.saved !== saved && !active.dirty && !active.busy) active = null;
    if (active?.key !== key) active = {
      key, saved, dirty: false, edit: { name: relationship.name || '', kind: relationship.connector?.kind },
      ready: false, busy: false, error: '', request: null,
    };
    const draft = active;
    const version = ++generation;
    const current = () => active === draft && generation === version && isCurrent();
    container.innerHTML = `<div class="property-heading">Connector</div>
      <div class="property-help">Edit this connection. Replacement endpoints must be presented in every diagram that shows it.</div>
      <form id="connector-specification-form">
        <label>Name<input id="connector-name" autocomplete="off"></label>
        <label>Kind<select id="connector-kind"><option value="Assembly">Assembly</option><option value="Delegation">Delegation</option></select></label>
        <label>Source endpoint<select id="connector-source" disabled></select></label>
        <label>Target endpoint<select id="connector-target" disabled></select></label>
        <label>Association type<select id="connector-type" disabled></select></label>
        <div id="connector-defining-ends" class="property-help"></div>
        <label>Source connector-end multiplicity<input id="connector-source-multiplicity" disabled></label>
        <label>Target connector-end multiplicity<input id="connector-target-multiplicity" disabled></label>
        <div class="property-help">End order is explicit. These multiplicities constrain links; they do not change part counts.</div>
        <div id="connector-error" role="alert" tabindex="-1"></div>
        <button id="connector-apply" type="submit" class="primary" disabled>Apply</button>
        <button id="connector-reload" type="button">Reload Saved</button>
      </form><button id="route-ibd-selection" type="button">Route IBD</button>`;
    const field = (id) => container.querySelector(`#connector-${id}`);
    const name = field('name'), kind = field('kind'), source = field('source'), target = field('target');
    const type = field('type'), sourceMultiplicity = field('source-multiplicity'), targetMultiplicity = field('target-multiplicity');
    const apply = field('apply'), reload = field('reload'), error = field('error');
    name.value = draft.edit.name;
    kind.value = draft.edit.kind;
    const capture = () => {
      draft.dirty = true;
      draft.edit.name = name.value;
      draft.edit.kind = kind.value;
      if (draft.ready) {
        draft.edit.source_presentation_id = source.value;
        draft.edit.target_presentation_id = target.value;
        draft.edit.typing = { association_type_id: type.value || null,
          source_multiplicity: sourceMultiplicity.value, target_multiplicity: targetMultiplicity.value };
        showEnds();
      }
    };
    const showEnds = () => {
      const selected = draft.associations?.find(choice => choice.id === type.value);
      field('defining-ends').textContent = selected ? `Source → ${selected.ends[0]}; Target → ${selected.ends[1]}` : 'Untyped connector';
    };
    for (const input of [name, kind, source, target, type, sourceMultiplicity, targetMultiplicity]) input.oninput = input.onchange = capture;
    const status = () => {
      name.disabled = kind.disabled = draft.busy;
      source.disabled = target.disabled = draft.busy || !draft.ready;
      type.disabled = sourceMultiplicity.disabled = targetMultiplicity.disabled = draft.busy || !draft.ready;
      apply.disabled = draft.busy || !draft.ready;
      reload.disabled = draft.busy;
      error.textContent = draft.error || (draft.ready ? '' : 'Loading endpoint choices…');
    };
    const populate = (select, value, choices) => {
      select.replaceChildren();
      for (const choice of choices) {
        const option = document.createElement('option');
        option.value = choice.presentation_id ?? choice.id;
        option.textContent = choice.label;
        select.appendChild(option);
      }
      select.value = value;
    };
    container.querySelector('#connector-specification-form').onsubmit = async (event) => {
      event.preventDefault();
      if (!current() || draft.busy || !draft.ready) return;
      capture();
      draft.busy = true;
      draft.error = '';
      status();
      try {
        await invoke('update_ibd_connector_specification', {
          diagramId: diagram.id, relationshipId: relationship.id, edit: { ...draft.edit },
        });
        if (active === draft) active = null;
        await refresh();
      } catch (failure) {
        draft.error = failure?.message || String(failure);
        if (current()) { error.textContent = draft.error; error.focus(); }
      } finally {
        draft.busy = false;
        if (current()) status();
      }
    };
    reload.onclick = () => {
      if (!current() || draft.busy) return;
      active = null;
      render(options);
    };
    container.querySelector('#route-ibd-selection').onclick = route;
    status();
    draft.request ||= Promise.resolve().then(() => invoke('ibd_connector_specification', { diagramId: diagram.id, relationshipId: relationship.id }));
    void draft.request.then((specification) => {
      if (!current()) return;
      draft.edit.source_presentation_id ??= specification.source_presentation_id;
      draft.edit.target_presentation_id ??= specification.target_presentation_id;
      draft.edit.kind ??= specification.kind;
      kind.value = draft.edit.kind;
      draft.edit.typing ??= { association_type_id: specification.association_type_id || null,
        source_multiplicity: specification.source_multiplicity || '1', target_multiplicity: specification.target_multiplicity || '1' };
      draft.associations = specification.associations || [];
      populate(source, draft.edit.source_presentation_id, specification.endpoints);
      populate(target, draft.edit.target_presentation_id, specification.endpoints);
      populate(type, draft.edit.typing.association_type_id || '', [{ id: '', label: 'Untyped' }, ...draft.associations]);
      sourceMultiplicity.value = draft.edit.typing.source_multiplicity;
      targetMultiplicity.value = draft.edit.typing.target_multiplicity;
      showEnds();
      draft.ready = true;
      status();
    }).catch((failure) => {
      if (!current()) return;
      draft.error = failure?.message || String(failure);
      status();
    });
  }

  let creating = false;
  async function create({ diagramId, kind, sourcePresentationId, targetPresentationId, invoke, commit, isCurrent }) {
    if (creating) return null;
    creating = true;
    try {
      const choices = await invoke('ibd_connector_type_choices');
      if (!isCurrent()) return null;
      const selection = await window.smpDialogs.choose({ title: 'Connector Association type',
        description: 'Choose a reusable Association or Untyped. Source corresponds to its first end; target corresponds to its second end.',
        candidates: [{ id: '__untyped__', label: 'Untyped' }, ...choices.map(choice => ({ id: choice.id, label: `${choice.label}: ${choice.ends[0]} → ${choice.ends[1]}` }))], confirmLabel: 'Continue' });
      if (!selection?.selectedId || !isCurrent()) return null;
      let draft = { name: '', source_multiplicity: '1', target_multiplicity: '1' };
      let failure = '';
      while (isCurrent()) {
        const answer = await window.smpDialogs.edit({ title: 'Create connector',
          description: `This connector is owned by the active IBD's type. Multiplicities describe links, separately from part counts. ${failure}`,
          fields: [{ id: 'name', label: 'Connector name', value: draft.name },
            { id: 'source_multiplicity', label: 'Source connector-end multiplicity', value: draft.source_multiplicity, required: true },
            { id: 'target_multiplicity', label: 'Target connector-end multiplicity', value: draft.target_multiplicity, required: true }], confirmLabel: 'Create connector' });
        if (!answer || !isCurrent()) return null;
        draft = { ...answer.values };
        try {
          return await commit('create_ibd_connector_specification', { diagramId, edit: {
            name: draft.name, kind, source_presentation_id: sourcePresentationId, target_presentation_id: targetPresentationId,
            typing: { association_type_id: selection.selectedId === '__untyped__' ? null : selection.selectedId,
              source_multiplicity: draft.source_multiplicity, target_multiplicity: draft.target_multiplicity },
          } });
        } catch (error) {
          failure = error?.message || String(error);
          window.smpDialogs.notify(failure, 'error');
        }
      }
      return null;
    } finally { creating = false; }
  }

  window.smpConnectorProperties = Object.freeze({ render, create, deactivate: () => { generation++; } });
})();
