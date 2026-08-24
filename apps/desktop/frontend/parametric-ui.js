(() => {
  'use strict';

  const PARAMETRIC_NODE_KINDS = new Set(['ConstraintProperty', 'ValueProperty']);
  const VALUE_TYPE_KINDS = new Set(['ValueType', 'DataType', 'PrimitiveType', 'Enumeration']);

  function selectedParametricDiagram() {
    return (state.snapshot?.diagrams || []).find(
      (diagram) => diagram.id === state.selectedDiagramId && diagram.family === 'parametric',
    );
  }

  function elementMap() {
    return new Map((state.snapshot?.project?.elements || []).map((element) => [element.id, element]));
  }

  function optionsFor(kinds, selectedId, includeNone = false) {
    const prefix = includeNone ? '<option value="">None</option>' : '';
    return prefix + (state.snapshot?.project?.elements || [])
      .filter((element) => kinds.has(element.kind))
      .sort((left, right) => left.name.localeCompare(right.name))
      .map((element) => `<option value="${escapeAttr(element.id)}"${element.id === selectedId ? ' selected' : ''}>${escapeHtml(element.name)} (${escapeHtml(element.kind)})</option>`)
      .join('');
  }

  function elementIdForExternal(externalId) {
    return state.snapshot?.project?.elements?.find((element) => element.external_id === externalId)?.id || '';
  }

  function endpointOptions(diagram, selectedPresentationId) {
    const elements = elementMap();
    const choices = [];
    for (const node of diagram.nodes || []) {
      const role = elements.get(node.element_id);
      if (role?.kind === 'ValueProperty') {
        choices.push({ id: node.id, label: `${role.name} (ValueProperty)` });
      }
      for (const parameter of node.parameter_presentations || []) {
        const definition = elements.get(parameter.parameter_id);
        choices.push({ id: parameter.id, label: `${role?.name || 'constraint'}.${definition?.name || 'parameter'}` });
      }
    }
    return choices.map((choice) => `<option value="${escapeAttr(choice.id)}"${choice.id === selectedPresentationId ? ' selected' : ''}>${escapeHtml(choice.label)}</option>`).join('');
  }

  const baseLoadPalette = loadPalette;
  loadPalette = async function loadParametricPalette() {
    if (!selectedParametricDiagram()) return baseLoadPalette();
    state.paletteItems = await requireInvoke()('diagram_palette', { diagramType: 'Parametric' });
  };

  async function completeBinding(diagram, presentationId, semanticId) {
    if (!state.pendingRelationship?.sourcePresentationId) {
      state.pendingRelationship.sourcePresentationId = presentationId;
      state.pendingRelationship.sourceElementId = semanticId;
      state.selectedElementId = semanticId;
      render();
      return;
    }
    if (state.pendingRelationship.sourcePresentationId === presentationId) return;
    const sourcePresentationId = state.pendingRelationship.sourcePresentationId;
    Object.assign(state, { pendingRelationship: null, paletteTool: null });
    const relationshipId = await runCommand('Creating Binding Connector…', () => requireInvoke()('create_binding_connector', {
      diagramId: diagram.id,
      sourcePresentationId,
      targetPresentationId: presentationId,
    }));
    Object.assign(state, { selectedElementId: null, selectedRelationshipId: relationshipId });
    await refresh();
  }

  function bindParameterMove(button, diagram, node, parameter) {
    button.addEventListener('pointerdown', (event) => {
      if (event.button !== 0 || state.pendingRelationship || state.paletteTool) return;
      event.preventDefault();
      event.stopPropagation();
      const startX = event.clientX;
      const startY = event.clientY;
      const originalX = parameter.offset_x;
      const originalY = parameter.offset_y;
      let nextX = originalX;
      let nextY = originalY;
      button.setPointerCapture?.(event.pointerId);
      const move = (pointer) => {
        nextX = originalX + pointer.clientX - startX;
        nextY = originalY + pointer.clientY - startY;
        button.style.left = `${nextX}px`;
        button.style.top = `${nextY}px`;
      };
      const up = async () => {
        button.removeEventListener('pointermove', move);
        button.removeEventListener('pointerup', up);
        button.removeEventListener('pointercancel', up);
        await runCommand('Moving constraint parameter…', () => requireInvoke()('update_constraint_parameter_presentation', {
          diagramId: diagram.id,
          presentationId: parameter.id,
          offsetX: nextX,
          offsetY: nextY,
        }));
        await refresh();
      };
      button.addEventListener('pointermove', move);
      button.addEventListener('pointerup', up);
      button.addEventListener('pointercancel', up);
    }, true);
  }

  function constraintMarkup(property, block) {
    return `<span class="parametric-stereotype">«constraint»</span>
      <span class="parametric-type">${escapeHtml(property.name)} : ${escapeHtml(block?.name || 'Unresolved ConstraintBlock')}</span>
      ${block?.constraint_expression ? `<div class="parametric-expression">${escapeHtml(block.constraint_expression)}</div>` : ''}`;
  }

  function valueMarkup(property, type, elements) {
    const unitExternal = property.unit_external_id || type?.unit_external_id;
    const unit = [...elements.values()].find((element) => element.external_id === unitExternal);
    const flags = `${property.is_derived ? '/' : ''}${property.is_read_only ? '{readOnly} ' : ''}`;
    const value = property.default_value ? ` = ${property.default_value}${unit?.unit_symbol ? ` ${unit.unit_symbol}` : ''}` : '';
    return `<span class="parametric-type">${escapeHtml(flags)}${escapeHtml(property.name)} : ${escapeHtml(type?.name || 'UnresolvedType')}</span>
      <span class="parametric-value">${escapeHtml(property.multiplicity || '1')}${escapeHtml(value)}</span>`;
  }

  const baseRenderCanvas = renderCanvas;
  renderCanvas = function renderParametricCanvas() {
    const diagram = selectedParametricDiagram();
    if (!diagram) return baseRenderCanvas();
    const canvas = $('canvas');
    const project = state.snapshot.project;
    const elements = elementMap();
    const context = elements.get(diagram.semantic_context_id);
    canvas.innerHTML = '';
    const frame = document.createElement('div');
    frame.className = 'diagram-frame parametric-diagram';
    frame.innerHTML = `<div class="diagram-header">par [${escapeHtml(context?.kind || 'Block')}] ${escapeHtml(context?.name || diagram.name)} [${escapeHtml(diagram.name)}]</div>`;
    canvas.appendChild(frame);
    createRelationshipLayer(frame, diagram, project);

    frame.ondragover = (event) => {
      if (![...event.dataTransfer.types].some((type) => [
        'application/x-smp-palette-id',
        'application/x-smp-element-id',
        'application/x-smp-repository-element-id',
      ].includes(type))) return;
      event.preventDefault();
      frame.classList.add('palette-target');
    };
    frame.ondragleave = () => frame.classList.remove('palette-target');
    frame.ondrop = async (event) => {
      event.preventDefault();
      event.stopPropagation();
      frame.classList.remove('palette-target');
      const point = diagramCoordinates(frame, event);
      const paletteId = event.dataTransfer.getData('application/x-smp-palette-id');
      const elementId = event.dataTransfer.getData('application/x-smp-element-id')
        || event.dataTransfer.getData('application/x-smp-repository-element-id');
      if (paletteId) {
        const item = state.paletteItems.find((candidate) => candidate.id === paletteId);
        if (item) await createPaletteElementAt(item, point.x, point.y);
      } else if (elementId) {
        await placeExistingElementAt(elementId, point.x, point.y);
      }
    };

    for (const node of diagram.nodes || []) {
      const property = elements.get(node.element_id);
      if (!property || !PARAMETRIC_NODE_KINDS.has(property.kind)) continue;
      const presentation = document.createElement('button');
      presentation.type = 'button';
      presentation.className = `bdd-block parametric-presentation ${property.kind === 'ConstraintProperty' ? 'constraint-property' : 'value-property'}`;
      presentation.dataset.semanticKind = property.kind;
      presentation.dataset.presentationId = node.id;
      if (state.selectedElementId === property.id) presentation.classList.add('selected');
      if (state.pendingRelationship?.sourcePresentationId === node.id) presentation.classList.add('relationship-source');
      Object.assign(presentation.style, {
        left: `${node.x}px`, top: `${node.y}px`, width: `${node.width}px`, height: `${node.height}px`,
      });
      const type = elements.get(property.type_id);
      presentation.innerHTML = property.kind === 'ConstraintProperty'
        ? constraintMarkup(property, type)
        : valueMarkup(property, type, elements);
      presentation.onclick = async (event) => {
        event.stopPropagation();
        if (state.pendingRelationship?.kind === 'BindingConnector') {
          if (property.kind !== 'ValueProperty') return;
          return completeBinding(diagram, node.id, property.id);
        }
        Object.assign(state, {
          selectedElementId: property.id,
          selectedRelationshipId: null,
          paletteTool: null,
        });
        render();
      };

      for (const parameter of node.parameter_presentations || []) {
        const definition = elements.get(parameter.parameter_id);
        const endpoint = document.createElement('button');
        endpoint.type = 'button';
        endpoint.className = 'constraint-parameter';
        endpoint.dataset.presentationId = parameter.id;
        endpoint.dataset.parameterId = parameter.parameter_id;
        endpoint.title = `${definition?.name || 'parameter'} : ${elements.get(definition?.type_id)?.name || 'UnresolvedType'}`;
        if (state.selectedElementId === parameter.parameter_id) endpoint.classList.add('selected');
        if (state.pendingRelationship?.sourcePresentationId === parameter.id) endpoint.classList.add('relationship-source');
        Object.assign(endpoint.style, {
          left: `${parameter.offset_x}px`, top: `${parameter.offset_y}px`,
          width: `${parameter.size}px`, height: `${parameter.size}px`,
        });
        endpoint.innerHTML = `<span class="constraint-parameter-label">${escapeHtml(definition?.name || 'parameter')}</span>`;
        endpoint.onclick = async (event) => {
          event.stopPropagation();
          if (state.pendingRelationship?.kind === 'BindingConnector') {
            return completeBinding(diagram, parameter.id, parameter.parameter_id);
          }
          Object.assign(state, {
            selectedElementId: parameter.parameter_id,
            selectedRelationshipId: null,
            paletteTool: null,
          });
          render();
        };
        bindParameterMove(endpoint, diagram, node, parameter);
        presentation.appendChild(endpoint);
      }
      frame.appendChild(presentation);
    }

    frame.onclick = async (event) => {
      if (state.paletteTool?.category === 'element') {
        const point = diagramCoordinates(frame, event);
        await createPaletteElementAt(state.paletteTool, point.x, point.y);
        return;
      }
      Object.assign(state, {
        selectedElementId: null,
        selectedRelationshipId: null,
        pendingRelationship: null,
        paletteTool: null,
      });
      render();
    };
  };

  const baseCreatePaletteElementAt = createPaletteElementAt;
  createPaletteElementAt = async function createParametricPaletteElement(item, x, y) {
    const diagram = selectedParametricDiagram();
    if (!diagram) return baseCreatePaletteElementAt(item, x, y);
    if (item.semantic_kind === 'ConstraintProperty') {
      const candidates = state.snapshot.project.elements
        .filter((element) => element.kind === 'ConstraintBlock')
        .map((element) => ({ id: element.id, label: element.name }));
      const definition = await window.smpDialogs?.choose({
        title: 'Create Constraint Property',
        description: 'Select the reusable ConstraintBlock definition for this usage.',
        fields: [{ id: 'name', label: 'Property name', value: 'constraint', required: true }],
        candidates,
        confirmLabel: 'Create',
      });
      if (!definition?.selectedId) return;
      await runCommand('Creating Constraint Property…', () => requireInvoke()('create_parametric_constraint_property', {
        diagramId: diagram.id,
        name: definition.values.name,
        constraintBlockId: definition.selectedId,
        x, y,
      }));
    } else if (item.semantic_kind === 'ValueProperty') {
      const candidates = state.snapshot.project.elements
        .filter((element) => VALUE_TYPE_KINDS.has(element.kind))
        .map((element) => ({ id: element.id, label: `${element.name} (${element.kind})` }));
      const definition = await window.smpDialogs?.choose({
        title: 'Create Value Property',
        description: 'Select the semantic value type. Values are not copied into the diagram.',
        fields: [
          { id: 'name', label: 'Property name', value: 'value', required: true },
          { id: 'value', label: 'Initial value (optional)', value: '' },
          { id: 'multiplicity', label: 'Multiplicity', value: '1', required: true },
        ],
        candidates,
        confirmLabel: 'Create',
      });
      if (!definition?.selectedId) return;
      await runCommand('Creating Value Property…', () => requireInvoke()('create_parametric_value_property', {
        diagramId: diagram.id,
        name: definition.values.name,
        valueTypeId: definition.selectedId,
        value: definition.values.value || null,
        multiplicity: definition.values.multiplicity,
        isDerived: false,
        isReadOnly: false,
        x, y,
      }));
    }
    Object.assign(state, { paletteTool: null, pendingRelationship: null });
    await refresh();
  };

  const basePlaceExistingElementAt = placeExistingElementAt;
  placeExistingElementAt = async function placeExistingParametricElement(elementId, x, y) {
    const diagram = selectedParametricDiagram();
    if (!diagram) return basePlaceExistingElementAt(elementId, x, y);
    await runCommand('Placing existing Parametric property…', () => requireInvoke()('place_on_parametric_diagram', {
      diagramId: diagram.id, elementId, x, y,
    }));
    Object.assign(state, { selectedElementId: elementId, selectedRelationshipId: null });
    await refresh();
  };

  function renderBindingProperties(panel, project, diagram, relationship) {
    const edge = (diagram.edges || []).find((candidate) => candidate.relationship_id === relationship.id);
    panel.innerHTML = `<div class="property-heading">Binding Connector</div>
      <div class="muted">Equality binding between compatible value and constraint endpoints.</div>
      <label>Source<select id="par-binding-source">${endpointOptions(diagram, edge?.source_node_id)}</select></label>
      <label>Target<select id="par-binding-target">${endpointOptions(diagram, edge?.target_node_id)}</select></label>
      <label>Stable ID<input value="${escapeAttr(relationship.external_id)}" disabled></label>
      <button id="apply-par-binding" class="primary">Reconnect endpoints</button>
      <button id="delete-par-binding">Delete from model</button>`;
    $('apply-par-binding').onclick = async () => {
      if ($('par-binding-source').value !== edge.source_node_id) {
        await runCommand('Reconnecting Binding Connector…', () => requireInvoke()('reconnect_binding_connector', {
          diagramId: diagram.id,
          relationshipId: relationship.id,
          side: 'source',
          presentationId: $('par-binding-source').value,
        }));
      }
      if ($('par-binding-target').value !== edge.target_node_id) {
        await runCommand('Reconnecting Binding Connector…', () => requireInvoke()('reconnect_binding_connector', {
          diagramId: diagram.id,
          relationshipId: relationship.id,
          side: 'target',
          presentationId: $('par-binding-target').value,
        }));
      }
      await refresh();
    };
    $('delete-par-binding').onclick = async () => {
      await runCommand('Deleting Binding Connector…', () => requireInvoke()('delete_binding_connector', {
        relationshipId: relationship.id,
      }));
      state.selectedRelationshipId = null;
      await refresh();
    };
  }

  function renderDiagramProperties(panel, project, diagram) {
    const context = project.elements.find((element) => element.id === diagram.semantic_context_id);
    panel.innerHTML = `<div class="property-heading">Parametric Diagram</div>
      <label>Name<input value="${escapeAttr(diagram.name)}" disabled></label>
      <label>Semantic context<input value="${escapeAttr(`${context?.name || 'Unresolved'} (${context?.kind || '?'})`)}" disabled></label>
      <div class="parametric-evaluation-summary">Evaluation is explicit. Opening the diagram never changes engineering values.</div>
      <button id="evaluate-parametrics" class="primary">Evaluate Parametrics</button>`;
    $('evaluate-parametrics').onclick = async () => {
      const result = await runCommand('Evaluating Parametrics…', () => requireInvoke()('evaluate_parametric_diagram', {
        diagramId: diagram.id,
      }));
      await refresh();
      renderStatus(`Evaluated ${result.evaluated_constraints} constraint(s); ${result.changed_values} value(s) changed.`);
    };
  }

  const baseRenderProperties = renderProperties;
  renderProperties = function renderParametricProperties() {
    const diagram = selectedParametricDiagram();
    if (!diagram) return baseRenderProperties();
    const panel = $('properties');
    const project = state.snapshot.project;
    const relationship = project.relationships.find((candidate) => candidate.id === state.selectedRelationshipId);
    if (relationship?.kind === 'BindingConnector') {
      return renderBindingProperties(panel, project, diagram, relationship);
    }
    const element = project.elements.find((candidate) => candidate.id === state.selectedElementId);
    if (!element) return renderDiagramProperties(panel, project, diagram);

    if (element.kind === 'ConstraintBlock') {
      panel.innerHTML = `<div class="property-heading">ConstraintBlock</div>
        <label>Name<input id="par-name" value="${escapeAttr(element.name)}"></label>
        <label>Constraint expression<textarea id="par-expression" rows="5" placeholder="energy = 0.5 * mass * velocity^2">${escapeHtml(element.constraint_expression || '')}</textarea></label>
        <label>Documentation<textarea id="par-documentation" rows="5">${escapeHtml(element.documentation || '')}</textarea></label>
        <button id="apply-constraint-block" class="primary">Apply ConstraintBlock</button>
        <button id="add-constraint-parameter">Add constraint parameter</button>`;
      $('apply-constraint-block').onclick = async () => {
        await runCommand('Updating ConstraintBlock…', () => requireInvoke()('update_constraint_block_details', {
          elementId: element.id,
          name: $('par-name').value,
          documentation: $('par-documentation').value,
          expression: $('par-expression').value,
        }));
        await refresh();
      };
      $('add-constraint-parameter').onclick = async () => {
        const candidates = project.elements.filter((candidate) => VALUE_TYPE_KINDS.has(candidate.kind))
          .map((candidate) => ({ id: candidate.id, label: `${candidate.name} (${candidate.kind})` }));
        const definition = await window.smpDialogs?.choose({
          title: 'Create constraint parameter',
          fields: [
            { id: 'name', label: 'Parameter name', value: 'parameter', required: true },
            { id: 'multiplicity', label: 'Multiplicity', value: '1', required: true },
          ],
          candidates,
          confirmLabel: 'Create',
        });
        if (!definition?.selectedId) return;
        await runCommand('Creating constraint parameter…', () => requireInvoke()('create_constraint_parameter', {
          constraintBlockId: element.id,
          name: definition.values.name,
          typeId: definition.selectedId,
          multiplicity: definition.values.multiplicity,
        }));
        await refresh();
      };
      return;
    }

    if (element.kind === 'ConstraintParameter') {
      panel.innerHTML = `<div class="property-heading">Constraint Parameter</div>
        <label>Name<input id="par-name" value="${escapeAttr(element.name)}"></label>
        <label>Type<select id="par-type">${optionsFor(VALUE_TYPE_KINDS, element.type_id)}</select></label>
        <label>Multiplicity<input id="par-multiplicity" value="${escapeAttr(element.multiplicity || '1')}"></label>
        <label>Stable ID<input value="${escapeAttr(element.external_id)}" disabled></label>
        <button id="apply-constraint-parameter" class="primary">Apply parameter</button>`;
      $('apply-constraint-parameter').onclick = async () => {
        await runCommand('Updating constraint parameter…', () => requireInvoke()('update_constraint_parameter', {
          elementId: element.id,
          name: $('par-name').value,
          typeId: $('par-type').value,
          multiplicity: $('par-multiplicity').value,
        }));
        await refresh();
      };
      return;
    }

    if (element.kind === 'ValueProperty') {
      panel.innerHTML = `<div class="property-heading">Value Property</div>
        <label>Name<input id="par-name" value="${escapeAttr(element.name)}"></label>
        <label>Type<select id="par-type">${optionsFor(VALUE_TYPE_KINDS, element.type_id)}</select></label>
        <label>Value<input id="par-value" value="${escapeAttr(element.default_value || '')}"></label>
        <label>Multiplicity<input id="par-multiplicity" value="${escapeAttr(element.multiplicity || '1')}"></label>
        <label>QuantityKind<select id="par-quantity">${optionsFor(new Set(['QuantityKind']), elementIdForExternal(element.quantity_kind_external_id), true)}</select></label>
        <label>Unit<select id="par-unit">${optionsFor(new Set(['Unit']), elementIdForExternal(element.unit_external_id), true)}</select></label>
        <label><input id="par-derived" type="checkbox"${element.is_derived ? ' checked' : ''}> Derived</label>
        <label><input id="par-read-only" type="checkbox"${element.is_read_only ? ' checked' : ''}> Read-only</label>
        <button id="apply-value-property" class="primary">Apply Value Property</button>`;
      $('apply-value-property').onclick = async () => {
        await runCommand('Updating Value Property…', () => requireInvoke()('update_parametric_value_property', {
          elementId: element.id,
          name: $('par-name').value,
          typeId: $('par-type').value,
          value: $('par-value').value || null,
          multiplicity: $('par-multiplicity').value,
          isDerived: $('par-derived').checked,
          isReadOnly: $('par-read-only').checked,
          quantityKindId: $('par-quantity').value || null,
          unitId: $('par-unit').value || null,
        }));
        await refresh();
      };
      return;
    }

    if (element.kind === 'ConstraintProperty') {
      const block = project.elements.find((candidate) => candidate.id === element.type_id);
      const parameters = project.elements.filter((candidate) => candidate.owner_id === block?.id && candidate.kind === 'ConstraintParameter');
      panel.innerHTML = `<div class="property-heading">Constraint Property</div>
        <label>Name<input id="par-name" value="${escapeAttr(element.name)}"></label>
        <label>Definition<select id="par-type">${optionsFor(new Set(['ConstraintBlock']), element.type_id)}</select></label>
        <button id="apply-constraint-property" class="primary">Apply Constraint Property</button>
        <div class="property-heading">Parameters</div>
        ${parameters.map((parameter) => `<div>${escapeHtml(parameter.name)} : ${escapeHtml(project.elements.find((candidate) => candidate.id === parameter.type_id)?.name || 'UnresolvedType')}</div>`).join('') || '<div class="muted">No parameters defined.</div>'}
        <label>Stable ID<input value="${escapeAttr(element.external_id)}" disabled></label>`;
      $('apply-constraint-property').onclick = async () => {
        await runCommand('Updating Constraint Property…', () => requireInvoke()('update_parametric_constraint_property', {
          elementId: element.id,
          name: $('par-name').value,
          constraintBlockId: $('par-type').value,
        }));
        await refresh();
      };
      return;
    }

    if (element.kind === 'ValueType') {
      panel.innerHTML = `<div class="property-heading">ValueType</div>
        <label>Name<input id="par-name" value="${escapeAttr(element.name)}"></label>
        <label>Documentation<textarea id="par-documentation" rows="5">${escapeHtml(element.documentation || '')}</textarea></label>
        <label>QuantityKind<select id="par-quantity">${optionsFor(new Set(['QuantityKind']), elementIdForExternal(element.quantity_kind_external_id), true)}</select></label>
        <label>Unit<select id="par-unit">${optionsFor(new Set(['Unit']), elementIdForExternal(element.unit_external_id), true)}</select></label>
        <button id="apply-value-type" class="primary">Apply ValueType</button>`;
      $('apply-value-type').onclick = async () => {
        await runCommand('Updating ValueType…', () => requireInvoke()('update_value_type_details', {
          elementId: element.id,
          name: $('par-name').value,
          documentation: $('par-documentation').value,
          quantityKindId: $('par-quantity').value || null,
          unitId: $('par-unit').value || null,
        }));
        await refresh();
      };
      return;
    }

    if (element.kind === 'QuantityKind') {
      panel.innerHTML = `<div class="property-heading">QuantityKind</div>
        <label>Name<input id="par-name" value="${escapeAttr(element.name)}"></label>
        <label>Dimension<input id="par-dimension" value="${escapeAttr(element.quantity_dimension || '')}" placeholder="M*L^2*T^-2"></label>
        <label>Documentation<textarea id="par-documentation" rows="5">${escapeHtml(element.documentation || '')}</textarea></label>
        <button id="apply-quantity-kind" class="primary">Apply QuantityKind</button>`;
      $('apply-quantity-kind').onclick = async () => {
        await runCommand('Updating QuantityKind…', () => requireInvoke()('update_quantity_kind_details', {
          elementId: element.id,
          name: $('par-name').value,
          documentation: $('par-documentation').value,
          dimension: $('par-dimension').value,
        }));
        await refresh();
      };
      return;
    }

    if (element.kind === 'Unit') {
      panel.innerHTML = `<div class="property-heading">Unit</div>
        <label>Name<input id="par-name" value="${escapeAttr(element.name)}"></label>
        <label>Symbol<input id="par-symbol" value="${escapeAttr(element.unit_symbol || '')}"></label>
        <label>Scale to base unit<input id="par-scale" type="number" step="any" min="0" value="${element.unit_scale_to_base || 1}"></label>
        <label>QuantityKind<select id="par-quantity">${optionsFor(new Set(['QuantityKind']), elementIdForExternal(element.quantity_kind_external_id))}</select></label>
        <label>Documentation<textarea id="par-documentation" rows="5">${escapeHtml(element.documentation || '')}</textarea></label>
        <button id="apply-unit" class="primary">Apply Unit</button>`;
      $('apply-unit').onclick = async () => {
        await runCommand('Updating Unit…', () => requireInvoke()('update_unit_details', {
          elementId: element.id,
          name: $('par-name').value,
          documentation: $('par-documentation').value,
          symbol: $('par-symbol').value,
          scaleToBase: Number($('par-scale').value),
          quantityKindId: $('par-quantity').value,
        }));
        await refresh();
      };
      return;
    }

    baseRenderProperties();
  };

  const baseRenderContext = renderContext;
  renderContext = function renderParametricContext() {
    const diagram = selectedParametricDiagram();
    if (!diagram) return baseRenderContext();
    $('active-diagram-summary').textContent = `${diagram.name} · Parametric Diagram`;
    $('palette-title').textContent = 'Elements (Parametric)';
  };

  const baseRenderStatus = renderStatus;
  renderStatus = function renderParametricStatus(message) {
    const diagram = selectedParametricDiagram();
    if (!diagram) return baseRenderStatus(message);
    if (message) $('status').textContent = message;
    else if (state.pendingRelationship?.kind === 'BindingConnector') {
      $('status').textContent = state.pendingRelationship.sourcePresentationId
        ? 'Binding Connector: source selected. Click a compatible value or constraint parameter.'
        : 'Binding Connector: click a value or constraint parameter, then a compatible target.';
    } else {
      $('status').textContent = `${state.snapshot.project.name} · Parametric: ${diagram.name}`;
    }
    $('model-counts').textContent = `Elements: ${state.snapshot.project.elements.length}   Relationships: ${state.snapshot.project.relationships.length}   Diagram: ${diagram.name} (PAR)`;
  };

  async function createParametricDiagram() {
    if (!state.snapshot?.project) return window.smpDialogs?.notify?.('Create a project first.', 'warning');
    const ownerId = state.selectedPackageId || state.snapshot.project.root_id;
    const candidates = state.snapshot.project.elements.filter((element) => [
      'Block', 'AssociationBlock', 'ConstraintBlock',
    ].includes(element.kind)).map((element) => ({ id: element.id, label: `${element.name} (${element.kind})` }));
    const definition = await window.smpDialogs?.choose({
      title: 'Create Parametric Diagram',
      description: 'Select the semantic Block or ConstraintBlock context. Diagram ownership remains separate.',
      fields: [{ id: 'name', label: 'Diagram name', value: 'System Parametrics', required: true }],
      candidates,
      confirmLabel: 'Create',
    });
    if (!definition?.selectedId) return;
    const selectedDiagramId = await runCommand('Creating Parametric Diagram…', () => requireInvoke()('create_parametric_diagram', {
      ownerId,
      name: definition.values.name,
      semanticContextId: definition.selectedId,
    }));
    Object.assign(state, {
      selectedDiagramId,
      selectedElementId: null,
      selectedRelationshipId: null,
      pendingRelationship: null,
      paletteTool: null,
    });
    await refresh();
    await selectDiagram(selectedDiagramId);
  }

  window.smpCreateParametricDiagram = createParametricDiagram;
})();
