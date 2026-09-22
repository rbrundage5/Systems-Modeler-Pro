(() => {
  'use strict';
  let active = null;
  let generation = 0;

  function render(options) {
    const { container, projectId, diagramId, connectorId, flows, invoke, refresh, isCurrent } = options;
    const key = `${projectId}:${diagramId}:${connectorId}`;
    const previousId = active?.key === key ? active.flowId : null;
    const flowId = options.flowId || previousId || flows[0]?.id;
    const flow = flows.find(candidate => candidate.id === flowId) || flows[0];
    if (!flow) {
      generation++;
      active = null;
      container.innerHTML = '<div class="property-heading">Item Flows</div><div class="muted">No Item Flow on this Connector. Use Item Flow in the IBD palette to add one.</div>';
      return;
    }
    const saved = JSON.stringify([flow, options.connector]);
    if (active?.key === key && active.flowId === flow.id && active.saved !== saved && !active.dirty && !active.busy) active = null;
    if (active?.key !== key || active.flowId !== flow.id) active = {
      key, flowId: flow.id, saved, dirty: false, edit: null, request: null, busy: false, error: '',
    };
    const draft = active;
    const version = ++generation;
    const current = () => active === draft && generation === version && isCurrent();
    container.innerHTML = `<div class="property-heading">Item Flows</div>
      <label>Flow<select id="flow-selection"></select></label>
      <form id="flow-specification-form">
        <label>Name<input id="flow-name" autocomplete="off" disabled></label>
        <div id="flow-endpoints" class="property-help"></div>
        <label>Direction<select id="flow-direction" disabled><option value="Forward">Source to target</option><option value="Reverse">Target to source</option></select></label>
        <label>Conveyed classifiers<select id="flow-classifiers" multiple size="6" disabled></select></label>
        <div class="property-help">Select one or more classifiers. Hold Ctrl or Command to select multiple entries.</div>
        <div id="flow-error" role="alert" tabindex="-1"></div>
        <button id="flow-apply" type="submit" class="primary" disabled>Apply Item Flow</button>
        <button id="flow-reload" type="button">Reload Saved</button>
      </form>`;
    const field = id => container.querySelector(`#flow-${id}`);
    const choice = (value, label) => {
      const option = document.createElement('option');
      option.value = value;
      option.textContent = label;
      return option;
    };
    const selection = field('selection'), name = field('name'), direction = field('direction');
    const classifiers = field('classifiers'), apply = field('apply'), reload = field('reload'), error = field('error');
    for (const candidate of flows) selection.appendChild(choice(candidate.id, `${candidate.name || 'Item Flow'} [${candidate.id}]`));
    selection.value = flow.id;
    const capture = () => {
      if (!draft.edit) return;
      draft.dirty = true;
      draft.edit = { name: name.value, direction: direction.value,
        conveyed_item_ids: [...classifiers.selectedOptions].map(option => option.value) };
    };
    for (const input of [name, direction, classifiers]) input.oninput = input.onchange = capture;
    const status = () => {
      selection.disabled = reload.disabled = draft.busy;
      name.disabled = direction.disabled = classifiers.disabled = apply.disabled = draft.busy || !draft.edit;
      error.textContent = draft.error || (draft.edit ? '' : 'Loading Item Flow specification…');
    };
    const fill = specification => {
      draft.edit ||= { name: specification.name, direction: specification.direction, conveyed_item_ids: [...specification.conveyed_item_ids] };
      name.value = draft.edit.name;
      direction.value = draft.edit.direction;
      field('endpoints').textContent = `Source: ${specification.source_label} → Target: ${specification.target_label}`;
      classifiers.replaceChildren();
      for (const classifier of specification.classifiers) {
        const option = choice(classifier.id, classifier.label);
        option.selected = draft.edit.conveyed_item_ids.includes(classifier.id);
        classifiers.appendChild(option);
      }
      status();
    };
    selection.onchange = () => { if (current() && !draft.busy) render({ ...options, flowId: selection.value }); };
    reload.onclick = () => {
      if (!current() || draft.busy) return;
      active = null;
      render({ ...options, flowId: flow.id });
    };
    field('specification-form').onsubmit = async event => {
      event.preventDefault();
      if (!current() || draft.busy || !draft.edit) return;
      capture(); draft.busy = true; draft.error = ''; status();
      try {
        await invoke('update_ibd_item_flow_specification', { relationshipId: flow.id,
          edit: { ...draft.edit, conveyed_item_ids: [...draft.edit.conveyed_item_ids] } });
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
    status();
    draft.request ||= Promise.resolve().then(() => invoke('ibd_item_flow_specification', { relationshipId: flow.id }));
    void draft.request.then(specification => { if (current()) fill(specification); }).catch(failure => {
      if (!current()) return;
      draft.error = failure?.message || String(failure); status();
    });
  }

  window.smpItemFlowProperties = Object.freeze({ render, deactivate: () => { generation++; } });
})();
