(() => {
  const FEATURE_KINDS = new Set([
    'PartProperty', 'ReferenceProperty', 'ValueProperty', 'FlowProperty',
    'ConstraintProperty', 'ConstraintParameter', 'ProxyPort', 'FullPort', 'Parameter',
  ]);

  const renderStructuralProperties = renderProperties;
  renderProperties = function renderPropertiesWithFeatureSemantics() {
    const panel = $('properties');
    const project = state.snapshot?.project;
    if (!project) {
      panel.innerHTML = '<div class="muted">Create or open a project to inspect properties.</div>';
      return;
    }
    const relationship = project.relationships?.find((item) => item.id === state.selectedRelationshipId);
    if (relationship) return renderRelationshipProperties(panel, project, relationship);
    const element = project.elements.find((item) => item.id === state.selectedElementId);
    if (element?.kind === 'Requirement' || element?.kind === 'TestCase') return renderStructuralProperties();
    if (!element) {
      panel.innerHTML = '<div class="muted">Select an element or relationship.</div>';
      return;
    }

    const supportsMultiplicity = FEATURE_KINDS.has(element.kind);
    const supportsAggregation = element.kind === 'PartProperty' || element.kind === 'ReferenceProperty';
    const isPort = element.kind === 'ProxyPort' || element.kind === 'FullPort';
    const isParameter = element.kind === 'Parameter';
    const isFlowProperty = element.kind === 'FlowProperty';
    const isReception = element.kind === 'Reception';
    const typeFieldId = isReception ? 'property-reception-signal' : 'property-type';
    const supportsType = supportsMultiplicity || isReception || element.kind === 'InstanceSpecification';
    const supportsDefault = supportsMultiplicity || element.default_value != null;
    const quantityKinds = project.elements.filter((candidate) => candidate.kind === 'QuantityKind');
    const units = project.elements.filter((candidate) => candidate.kind === 'Unit');

    panel.innerHTML = `<div class="property-heading">${escapeHtml(element.kind)}</div>
      <label>Name<input id="property-name" value="${escapeAttr(element.name)}"></label>
      <label>Documentation<textarea id="property-documentation" rows="5">${escapeHtml(element.documentation || '')}</textarea></label>
      <label>Stable ID<input value="${escapeAttr(element.external_id)}" disabled></label>
      ${supportsType ? `<label>Find type<input id="property-type-search" type="search" placeholder="Filter by name or stable ID" disabled></label><label>${isReception ? 'Accepted Signal' : 'Type'}<select id="${typeFieldId}" disabled><option value="${escapeAttr(element.type_id || '')}">Loading compatible types…</option></select></label>` : ''}
      ${supportsMultiplicity ? `<label>Multiplicity<input id="property-multiplicity" value="${escapeAttr(element.multiplicity || '1')}"></label>` : ''}
      ${supportsAggregation ? `<label>Structural usage and aggregation<select id="property-aggregation"><option value="composite">Composite part</option><option value="none">Reference (noncomposite)</option><option value="shared">Shared reference</option></select></label><p class="property-help">Apply changes this property definition in every usage. Its stable ID, owner and Block type are retained.</p>` : ''}
      ${element.kind === 'ValueType' ? `<label>Quantity Kind ID<input id="property-quantity-kind" list="quantity-kind-ids" value="${escapeAttr(element.quantity_kind_external_id || '')}"></label><datalist id="quantity-kind-ids">${quantityKinds.map((item) => `<option value="${escapeAttr(item.external_id)}">${escapeHtml(item.name)}</option>`).join('')}</datalist><label>Unit ID<input id="property-unit" list="unit-ids" value="${escapeAttr(element.unit_external_id || '')}"></label><datalist id="unit-ids">${units.map((item) => `<option value="${escapeAttr(item.external_id)}">${escapeHtml(item.name)}</option>`).join('')}</datalist>` : ''}
      ${supportsDefault ? `<label>Default Value<input id="property-default" value="${escapeAttr(element.default_value || '')}"></label>` : ''}
      ${supportsMultiplicity ? `<label class="property-check"><input id="property-derived" type="checkbox" ${element.is_derived ? 'checked' : ''}> Derived</label><label class="property-check"><input id="property-read-only" type="checkbox" ${element.is_read_only ? 'checked' : ''}> Read only</label>` : ''}
      ${element.kind === 'ProxyPort' ? `<label class="property-check"><input id="property-conjugated" type="checkbox" ${element.is_conjugated ? 'checked' : ''}> Conjugated</label>` : ''}
      ${isParameter ? `<label>Direction<select id="property-direction"><option value="in">in</option><option value="out">out</option><option value="inout">inout</option><option value="return">return</option></select></label>` : ''}
      ${isFlowProperty ? `<label>Flow Direction<select id="property-flow-direction"><option value="in">in</option><option value="out">out</option><option value="inout">inout</option></select></label>` : ''}
      <div id="property-error" role="alert" tabindex="-1"></div>
      <button id="apply-element" type="button" class="primary">Apply</button>`;

    const compartmentDisplay=window.smpCompartmentDisplay?.(element.id);
    if(compartmentDisplay?.labels?.length&&!panel.querySelector('.bdd-compartment-controls')){
      const section=document.createElement('section'); section.className='bdd-compartment-controls';
      section.innerHTML='<div class="property-heading">Presentation Display</div><div class="muted">Choose which compartments are visible on this diagram presentation.</div>';
      for(const label of compartmentDisplay.labels){const row=document.createElement('label');row.className='compartment-visibility-toggle';const checkbox=document.createElement('input');checkbox.type='checkbox';checkbox.checked=compartmentDisplay.shown(label);checkbox.onchange=()=>compartmentDisplay.set(label,checkbox.checked);const text=document.createElement('span');text.textContent='Show '+label;row.append(checkbox,text);section.appendChild(row);}
      panel.insertBefore(section,$('apply-element'));
    }
    if (supportsAggregation) $('property-aggregation').value = element.aggregation || 'none';
    if (isParameter) $('property-direction').value = element.parameter_direction || 'in';
    if (isFlowProperty) $('property-flow-direction').value = element.flow_direction || 'inout';

    const apply = $('apply-element');
    const errorBox = $('property-error');
    const typeSelect = $(typeFieldId);
    const typeSearch = $('property-type-search');
    let applying = false;
    if (supportsType) {
      apply.disabled = true;
      requireInvoke()('element_type_choices', { elementId: element.id }).then((choices) => {
        if ($('apply-element') !== apply || state.selectedElementId !== element.id) return;
        const showChoices = () => {
          const selected = typeSelect.value || element.type_id || '';
          const query = typeSearch.value.trim().toLowerCase();
          const visible = choices.filter((choice) => choice.id === selected
            || `${choice.qualifiedName} ${choice.externalId}`.toLowerCase().includes(query));
          typeSelect.innerHTML = `<option value="" ${element.type_id ? 'disabled' : ''}>${element.type_id ? 'Choose a type' : 'Unspecified'}</option>`
            + visible.map((choice) => `<option value="${escapeAttr(choice.id)}">${escapeHtml(choice.qualifiedName)} (${escapeHtml(choice.kind)}) [${escapeHtml(choice.externalId)}]</option>`).join('');
          typeSelect.value = selected;
        };
        typeSearch.oninput = showChoices;
        showChoices();
        typeSelect.disabled = false;
        typeSearch.disabled = false;
        apply.disabled = false;
      }).catch((error) => {
        if ($('apply-element') !== apply) return;
        errorBox.textContent = `Could not load compatible types: ${error?.message || String(error)}. Reselect this element to retry.`;
      });
    }

    apply.onclick = async () => {
      if (applying || apply.disabled) return;
      applying = true;
      apply.disabled = true;
      errorBox.textContent = '';
      try {
        await runCommand('Applying element specification…', () => requireInvoke()('update_element_specification', {
          elementId: element.id,
          edit: {
            name: $('property-name').value,
            documentation: $('property-documentation').value,
            typeId: supportsType ? (typeSelect.value || null) : null,
            defaultValue: $('property-default')?.value ?? null,
            quantityKindExternalId: $('property-quantity-kind')?.value ?? null,
            unitExternalId: $('property-unit')?.value ?? null,
            multiplicity: supportsMultiplicity ? $('property-multiplicity').value : null,
            aggregation: supportsAggregation ? $('property-aggregation').value : null,
            isDerived: supportsMultiplicity ? $('property-derived').checked : null,
            isReadOnly: supportsMultiplicity ? $('property-read-only').checked : null,
            isConjugated: isPort ? ($('property-conjugated')?.checked ?? false) : null,
            parameterDirection: isParameter ? $('property-direction').value : null,
            flowDirection: isFlowProperty ? $('property-flow-direction').value : null,
          },
        }));
        await refresh();
      } catch (error) {
        // Keep the form and every draft value intact on Rust validation failure.
        errorBox.textContent = error?.message || String(error);
        errorBox.focus();
      } finally {
        applying = false;
        apply.disabled = false;
      }
    };
  };
})();
