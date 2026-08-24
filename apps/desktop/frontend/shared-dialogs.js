(() => {
  'use strict';
  const host = document.createElement('div'); host.id = 'dialog-host'; document.body.appendChild(host);
  const notices = document.createElement('div'); notices.className = 'notification-host'; notices.setAttribute('aria-live','polite'); document.body.appendChild(notices);
  let active = null;
  function notify(message, level='info') { const item=document.createElement('div'); item.className=`notification ${level}`; item.textContent=message; notices.appendChild(item); setTimeout(()=>item.remove(),6000); }
  function cancelActive() { active?.cancel(); }
  function open({ title, description='', fields=[], candidates=[], searchable=false, confirmLabel='OK', destructive=false }) {
    cancelActive(); const invoking=document.activeElement;
    return new Promise((resolve) => {
      const overlay=document.createElement('div'); overlay.className='modal-overlay';
      overlay.innerHTML=`<section class="application-dialog" role="dialog" aria-modal="true" aria-labelledby="dialog-title"><header><h2 id="dialog-title"></h2></header><div class="dialog-body"><p class="dialog-description"></p><div class="dialog-fields"></div><div class="dialog-candidates"></div></div><footer><button type="button" data-cancel>Cancel</button><button type="button" data-confirm class="${destructive?'danger':'primary'}"></button></footer></section>`;
      overlay.querySelector('h2').textContent=title; overlay.querySelector('.dialog-description').textContent=description; overlay.querySelector('[data-confirm]').textContent=confirmLabel;
      const values={}; const fieldHost=overlay.querySelector('.dialog-fields');
      for(const field of fields){const label=document.createElement('label');label.textContent=field.label;const input=document.createElement(field.multiline?'textarea':'input');input.name=field.id;input.value=field.value||'';input.required=!!field.required;label.appendChild(input);fieldHost.appendChild(label);values[field.id]=input;}
      const candidateHost=overlay.querySelector('.dialog-candidates'); let selected=null;
      const render=(query='')=>{candidateHost.innerHTML='';for(const candidate of candidates.filter(c=>(c.label||'').toLowerCase().includes(query.toLowerCase()))){const button=document.createElement('button');button.type='button';button.className='dialog-candidate';button.textContent=candidate.label;button.onclick=()=>{selected=candidate.id;candidateHost.querySelectorAll('.selected').forEach(n=>n.classList.remove('selected'));button.classList.add('selected');};candidateHost.appendChild(button);}};
      if(searchable){const search=document.createElement('input');search.type='search';search.placeholder='Search';search.setAttribute('aria-label','Search candidates');search.oninput=()=>render(search.value);candidateHost.before(search);} render();
      const close=(result)=>{document.removeEventListener('keydown',key,true);overlay.remove();active=null;invoking?.focus?.();resolve(result);};
      const confirm=()=>{if([...Object.values(values)].some(input=>!input.reportValidity()))return;close({values:Object.fromEntries(Object.entries(values).map(([key,input])=>[key,input.value])),selectedId:selected});};
      const key=(event)=>{if(event.key==='Escape'){event.preventDefault();close(null);}if(event.key==='Enter'&&!event.shiftKey&&event.target.tagName!=='TEXTAREA'){event.preventDefault();confirm();}if(event.key==='Tab'){const focusable=[...overlay.querySelectorAll('button,input,textarea')];const first=focusable[0],last=focusable.at(-1);if(event.shiftKey&&document.activeElement===first){event.preventDefault();last.focus();}else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first.focus();}}};
      overlay.querySelector('[data-cancel]').onclick=()=>close(null);overlay.querySelector('[data-confirm]').onclick=confirm;document.addEventListener('keydown',key,true);host.appendChild(overlay);(overlay.querySelector('input,textarea,.dialog-candidate,[data-confirm]'))?.focus();active={cancel:()=>close(null)};
    });
  }
  window.smpDialogs=Object.freeze({open,notify,cancelActive,choose:(options)=>open({...options,searchable:true}),edit:(options)=>open(options),confirm:async(options)=>!!(await open({...options,destructive:options.destructive,confirmLabel:options.confirmLabel||'Confirm'}))});
})();

// Parametric binding error presentation adapter. Rust remains the sole authority
// for BindingConnector compatibility; this only makes Rust rejections actionable.
(() => {
  'use strict';
  const INCOMPATIBLE_BINDING = /binding endpoint types are incompatible:\s*([0-9a-f-]{36})\s+vs\s+([0-9a-f-]{36})/i;
  const originalAlert = window.alert.bind(window);
  const recentEndpoints = [];

  function projectElements() {
    return window.smpState?.snapshot?.project?.elements || [];
  }

  function elementsById() {
    return new Map(projectElements().map((element) => [element.id, element]));
  }

  function elementsByExternalId() {
    return new Map(projectElements().map((element) => [element.external_id, element]));
  }

  function activeParametricDiagram() {
    const state = window.smpState;
    return state?.snapshot?.diagrams?.find(
      (diagram) => diagram.id === state.selectedDiagramId && diagram.family === 'parametric',
    ) || null;
  }

  function typeDetails(feature, byId, byExternal) {
    const type = byId.get(feature?.type_id);
    const quantityExternal = feature?.quantity_kind_external_id || type?.quantity_kind_external_id;
    const unitExternal = feature?.unit_external_id || type?.unit_external_id;
    const quantity = byExternal.get(quantityExternal);
    const unit = byExternal.get(unitExternal);
    return {
      typeName: type?.name || 'UnresolvedType',
      typeKind: type?.kind || 'UnknownType',
      quantityName: quantity?.name || '',
      unitSymbol: unit?.unit_symbol || '',
    };
  }

  function describePresentation(presentationId) {
    const diagram = activeParametricDiagram();
    if (!diagram || !presentationId) return null;
    const byId = elementsById();
    const byExternal = elementsByExternalId();
    const node = (diagram.nodes || []).find((candidate) => candidate.id === presentationId);
    if (node) {
      const feature = byId.get(node.element_id);
      if (feature?.kind !== 'ValueProperty') return null;
      return {
        presentationId,
        roleId: feature.id,
        parameterId: null,
        label: feature.name,
        ...typeDetails(feature, byId, byExternal),
      };
    }
    for (const candidate of diagram.nodes || []) {
      const parameterPresentation = (candidate.parameter_presentations || []).find(
        (parameter) => parameter.id === presentationId,
      );
      if (!parameterPresentation) continue;
      const role = byId.get(candidate.element_id);
      const parameter = byId.get(parameterPresentation.parameter_id);
      if (!parameter) return null;
      return {
        presentationId,
        roleId: role?.id || candidate.element_id,
        parameterId: parameter.id,
        label: `${role?.name || 'constraint'}.${parameter.name}`,
        ...typeDetails(parameter, byId, byExternal),
      };
    }
    return null;
  }

  function describeRoleFallback(roleId) {
    const byId = elementsById();
    const byExternal = elementsByExternalId();
    const role = byId.get(roleId);
    if (!role) return null;
    if (role.kind === 'ValueProperty') {
      return { roleId, parameterId: null, label: role.name, ...typeDetails(role, byId, byExternal) };
    }
    return { roleId, parameterId: null, label: role.name, ...typeDetails(role, byId, byExternal) };
  }

  function endpointLine(endpoint) {
    if (!endpoint) return 'Unknown endpoint';
    const engineeringMetadata = [
      endpoint.quantityName ? `QuantityKind ${endpoint.quantityName}` : '',
      endpoint.unitSymbol ? `unit ${endpoint.unitSymbol}` : '',
    ].filter(Boolean);
    const suffix = engineeringMetadata.length ? `; ${engineeringMetadata.join(', ')}` : '';
    return `${endpoint.label} : ${endpoint.typeName} (${endpoint.typeKind}${suffix})`;
  }

  function formatBindingTypeError(message) {
    const match = String(message).match(INCOMPATIBLE_BINDING);
    if (!match) return String(message);
    const captured = recentEndpoints.slice(-2);
    const source = captured[0] || describeRoleFallback(match[1]);
    const target = captured[1] || describeRoleFallback(match[2]);
    return [
      'Binding Connector endpoints are incompatible.',
      '',
      endpointLine(source),
      '↔',
      endpointLine(target),
      '',
      'Set both endpoints to compatible types in Properties.',
      'Binding endpoints must use the same type, or compatible ValueTypes with matching QuantityKind and dimension.',
    ].join('\n');
  }

  function rememberEndpointFromClick(event) {
    const state = window.smpState;
    if (!activeParametricDiagram() || state?.pendingRelationship?.kind !== 'BindingConnector') return;
    const target = event.target.closest?.('.constraint-parameter, .parametric-presentation.value-property');
    const endpoint = describePresentation(target?.dataset?.presentationId);
    if (!endpoint) return;
    recentEndpoints.push(endpoint);
    while (recentEndpoints.length > 2) recentEndpoints.shift();
  }

  function exposeEndpointTypes(root = document) {
    const byId = elementsById();
    const byExternal = elementsByExternalId();
    for (const endpoint of root.querySelectorAll?.('.constraint-parameter') || []) {
      const parameter = byId.get(endpoint.dataset.parameterId);
      if (!parameter) continue;
      const details = typeDetails(parameter, byId, byExternal);
      const label = endpoint.querySelector('.constraint-parameter-label');
      if (label) label.textContent = `${parameter.name} : ${details.typeName}`;
      endpoint.title = endpointLine({ label: parameter.name, ...details });
    }
    for (const selectId of ['par-binding-source', 'par-binding-target']) {
      const select = document.getElementById(selectId);
      if (!select) continue;
      for (const option of select.options) {
        const endpoint = describePresentation(option.value);
        if (endpoint) option.textContent = endpointLine(endpoint);
      }
    }
  }

  document.addEventListener('click', rememberEndpointFromClick, true);
  window.alert = (message) => {
    const text = String(message);
    if (INCOMPATIBLE_BINDING.test(text)) return originalAlert(formatBindingTypeError(text));
    return originalAlert(message);
  };
  const observer = new MutationObserver(() => exposeEndpointTypes());
  observer.observe(document.body, { childList: true, subtree: true });
  exposeEndpointTypes();
  window.smpParametricBindingDiagnostics = Object.freeze({
    describePresentation,
    formatBindingTypeError,
    exposeEndpointTypes,
  });
})();
