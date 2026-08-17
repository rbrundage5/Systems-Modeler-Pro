(() => {
  const FEATURE_KINDS = new Set([
    'PartProperty', 'ReferenceProperty', 'ValueProperty', 'ConstraintProperty',
    'ProxyPort', 'FullPort', 'Parameter',
  ]);

  function parseMultiplicity(text) {
    const value = text.trim();
    if (value === '*') return { lower: 0, upper: null };
    if (/^\d+$/.test(value)) {
      const bound = Number(value);
      return { lower: bound, upper: bound };
    }
    const match = value.match(/^(\d+)\.\.(\d+|\*)$/);
    if (!match) throw new Error('Multiplicity must be 1, 0..1, 1..*, *, etc.');
    return {
      lower: Number(match[1]),
      upper: match[2] === '*' ? null : Number(match[2]),
    };
  }

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
    if (!element) {
      panel.innerHTML = '<div class="muted">Select an element or relationship.</div>';
      return;
    }

    const supportsMultiplicity = FEATURE_KINDS.has(element.kind);
    const supportsAggregation = element.kind === 'PartProperty' || element.kind === 'ReferenceProperty';
    const isPort = element.kind === 'ProxyPort' || element.kind === 'FullPort';
    const isParameter = element.kind === 'Parameter';

    panel.innerHTML = `<div class="property-heading">${escapeHtml(element.kind)}</div>
      <label>Name<input id="property-name" value="${escapeAttr(element.name)}"></label>
      <label>Documentation<textarea id="property-documentation" rows="5">${escapeHtml(element.documentation || '')}</textarea></label>
      <label>Stable ID<input value="${escapeAttr(element.external_id)}" disabled></label>
      ${element.type_id ? `<label>Type<input value="${escapeAttr(typeName(project, element))}" disabled></label>` : ''}
      ${supportsMultiplicity ? `<label>Multiplicity<input id="property-multiplicity" value="${escapeAttr(element.multiplicity || '1')}"></label>` : ''}
      ${supportsAggregation ? `<label>Aggregation<select id="property-aggregation"><option value="none">none</option><option value="shared">shared</option><option value="composite">composite</option></select></label>` : ''}
      ${element.kind === 'ValueType' ? `<label>Quantity Kind ID<input id="property-quantity-kind" value="${escapeAttr(element.quantity_kind_external_id || '')}"></label><label>Unit ID<input id="property-unit" value="${escapeAttr(element.unit_external_id || '')}"></label>` : ''}
      ${element.default_value !== null && element.default_value !== undefined ? `<label>Default Value<input id="property-default" value="${escapeAttr(element.default_value || '')}"></label>` : ''}
      ${supportsMultiplicity ? `<label class="property-check"><input id="property-derived" type="checkbox" ${element.is_derived ? 'checked' : ''}> Derived</label><label class="property-check"><input id="property-read-only" type="checkbox" ${element.is_read_only ? 'checked' : ''}> Read only</label>` : ''}
      ${isPort ? `<label class="property-check"><input id="property-conjugated" type="checkbox" ${element.is_conjugated ? 'checked' : ''}> Conjugated</label>` : ''}
      ${isParameter ? `<label>Direction<select id="property-direction"><option value="in">in</option><option value="out">out</option><option value="inout">inout</option><option value="return">return</option></select></label>` : ''}
      <button id="apply-element" class="primary">Apply</button>`;

    if (supportsAggregation) $('property-aggregation').value = element.aggregation || 'none';
    if (isParameter) $('property-direction').value = element.parameter_direction || 'in';

    $('apply-element').onclick = async () => {
      const name = $('property-name').value.trim();
      if (!name) return;
      if (name !== element.name) {
        await runCommand('Renaming element…', () => requireInvoke()('rename_element', {
          elementId: element.id,
          name,
        }));
      }

      await runCommand('Updating element details…', () => requireInvoke()('update_bdd_element_details', {
        elementId: element.id,
        documentation: $('property-documentation').value,
        defaultValue: $('property-default')?.value ?? null,
        quantityKindExternalId: $('property-quantity-kind')?.value ?? null,
        unitExternalId: $('property-unit')?.value ?? null,
      }));

      if (supportsMultiplicity) {
        const multiplicity = parseMultiplicity($('property-multiplicity').value);
        await runCommand('Updating feature semantics…', () => requireInvoke()('update_bdd_feature_semantics', {
          elementId: element.id,
          lower: multiplicity.lower,
          upper: multiplicity.upper,
          aggregation: supportsAggregation ? $('property-aggregation').value : null,
          isDerived: $('property-derived').checked,
          isReadOnly: $('property-read-only').checked,
          isConjugated: isPort ? $('property-conjugated').checked : null,
          parameterDirection: isParameter ? $('property-direction').value : null,
        }));
      }
      await refresh();
    };
  };
})();
