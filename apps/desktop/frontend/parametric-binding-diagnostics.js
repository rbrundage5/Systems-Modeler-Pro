(() => {
  'use strict';

  // Rust remains the sole authority for BindingConnector compatibility. This
  // adapter only turns an already-rejected Rust error into an engineer-readable
  // diagnostic and exposes semantic endpoint types in the existing UI.
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
      return {
        roleId,
        parameterId: null,
        label: role.name,
        ...typeDetails(role, byId, byExternal),
      };
    }
    if (role.kind === 'ConstraintProperty') {
      return {
        roleId,
        parameterId: null,
        label: role.name,
        typeName: byId.get(role.type_id)?.name || 'ConstraintBlock',
        typeKind: 'ConstraintProperty',
        quantityName: '',
        unitSymbol: '',
      };
    }
    return {
      roleId,
      parameterId: null,
      label: role.name,
      ...typeDetails(role, byId, byExternal),
    };
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
    if (!activeParametricDiagram()) return;
    if (state?.pendingRelationship?.kind !== 'BindingConnector') return;
    const target = event.target.closest?.('.constraint-parameter, .parametric-presentation.value-property');
    const presentationId = target?.dataset?.presentationId;
    const endpoint = describePresentation(presentationId);
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
    if (INCOMPATIBLE_BINDING.test(text)) {
      return originalAlert(formatBindingTypeError(text));
    }
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
