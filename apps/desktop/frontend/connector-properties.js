(() => {
  'use strict';
  let active = null;
  let generation = 0;

  function render(options) {
    const { container, projectId, diagram, relationship, invoke, refresh, isCurrent, route } = options;
    const key = `${projectId}:${diagram.id}:${relationship.id}`;
    if (active?.key !== key) active = {
      key, edit: { name: relationship.name || '', kind: relationship.connector.kind },
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
        <div id="connector-error" role="alert" tabindex="-1"></div>
        <button id="connector-apply" type="submit" class="primary" disabled>Apply</button>
        <button id="connector-reload" type="button">Reload Saved</button>
      </form><button id="route-ibd-selection" type="button">Route IBD</button>`;
    const field = (id) => container.querySelector(`#connector-${id}`);
    const name = field('name'), kind = field('kind'), source = field('source'), target = field('target');
    const apply = field('apply'), reload = field('reload'), error = field('error');
    name.value = draft.edit.name;
    kind.value = draft.edit.kind;
    const capture = () => {
      draft.edit.name = name.value;
      draft.edit.kind = kind.value;
      if (draft.ready) {
        draft.edit.source_presentation_id = source.value;
        draft.edit.target_presentation_id = target.value;
      }
    };
    for (const input of [name, kind, source, target]) input.oninput = input.onchange = capture;
    const status = () => {
      name.disabled = kind.disabled = draft.busy;
      source.disabled = target.disabled = draft.busy || !draft.ready;
      apply.disabled = draft.busy || !draft.ready;
      reload.disabled = draft.busy;
      error.textContent = draft.error || (draft.ready ? '' : 'Loading endpoint choices…');
    };
    const populate = (select, value, choices) => {
      select.replaceChildren();
      for (const choice of choices) {
        const option = document.createElement('option');
        option.value = choice.presentation_id;
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
      populate(source, draft.edit.source_presentation_id, specification.endpoints);
      populate(target, draft.edit.target_presentation_id, specification.endpoints);
      draft.ready = true;
      status();
    }).catch((failure) => {
      if (!current()) return;
      draft.error = failure?.message || String(failure);
      status();
    });
  }

  window.smpConnectorProperties = Object.freeze({ render, deactivate: () => { generation++; } });
})();
