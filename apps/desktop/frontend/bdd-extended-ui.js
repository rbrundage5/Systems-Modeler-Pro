const BDD_RELATIONSHIP_CLASSIFIER_KINDS = new Set([
  'Block', 'AssociationBlock', 'InterfaceBlock', 'ConstraintBlock', 'ValueType',
  'DataType', 'PrimitiveType', 'Enumeration', 'Signal',
]);

for (const kind of [
  'AssociationBlock', 'PrimitiveType', 'Signal', 'Unit', 'QuantityKind',
  'InstanceSpecification', 'Comment',
]) BDD_CLASSIFIER_KINDS.add(kind);
BDD_TYPED_FEATURE_KINDS.add('FlowProperty');

classifierStereotype = function classifierStereotypeExtended(kind) {
  return {
    Block: 'block', AssociationBlock: 'block', InterfaceBlock: 'interfaceBlock',
    ConstraintBlock: 'constraint', ValueType: 'valueType', DataType: 'dataType',
    PrimitiveType: 'primitive', Enumeration: 'enumeration', Signal: 'signal',
    Unit: 'unit', QuantityKind: 'quantityKind', Requirement: 'requirement',
    TestCase: 'testCase',
  }[kind] || '';
};

classifierCompartments = function classifierCompartmentsExtended(project, element) {
  const children = project.elements.filter((candidate) => candidate.owner_id === element.id);
  const groups = [
    ['literals', ['EnumerationLiteral']], ['parts', ['PartProperty']],
    ['references', ['ReferenceProperty']], ['values', ['ValueProperty']],
    ['flow properties', ['FlowProperty']], ['constraints', ['ConstraintProperty']],
    ['ports', ['ProxyPort', 'FullPort']], ['operations', ['Operation']],
    ['receptions', ['Reception']], ['parameters', ['Parameter']], ['slots', ['Slot']],
  ];
  return groups.map(([label, kinds]) => {
    const items = children.filter((child) => kinds.includes(child.kind));
    if (!items.length) return '';
    return `<div class="compartment"><div class="compartment-title">${escapeHtml(label)}</div>${items.map((child) => {
      let notation = featureNotation(project, child);
      if (child.kind === 'FlowProperty' && child.flow_direction) notation = `${child.flow_direction} ${notation}`;
      return `<div>${escapeHtml(notation)}</div>`;
    }).join('')}</div>`;
  }).join('');
};

paletteSymbol = function paletteSymbolExtended(item) {
  const symbols = {
    Block: '▭', AssociationBlock: 'A', InterfaceBlock: '◫', ConstraintBlock: 'C',
    ValueType: 'V', DataType: 'D', PrimitiveType: 'P', Enumeration: 'E', Signal: 'S',
    Unit: 'U', QuantityKind: 'Q', InstanceSpecification: 'I', Comment: '◩',
    PartProperty: '◆', ReferenceProperty: '◇', ValueProperty: 'v', FlowProperty: '↔',
    ConstraintProperty: 'c', ProxyPort: '□', FullPort: '■', Operation: 'ƒ',
    Reception: '⇥', Parameter: 'p', EnumerationLiteral: '•', Slot: 's',
    Association: '──', Aggregation: '◇─', Composition: '◆─', Generalization: '▷─',
    Dependency: '⇢', Realization: '⇢▷',
  };
  return symbols[item.semantic_kind] || symbols[item.relationship_kind] || '·';
};

chooseTypeId = async function chooseTypeIdExtended(kind) {
  const project = state.snapshot?.project;
  if (!project) return null;
  const classifiers = ['Block', 'AssociationBlock', 'InterfaceBlock', 'ConstraintBlock', 'ValueType', 'DataType', 'PrimitiveType', 'Enumeration', 'Signal'];
  const compatible = {
    PartProperty: ['Block', 'AssociationBlock', 'InterfaceBlock'],
    ReferenceProperty: classifiers,
    ValueProperty: ['ValueType', 'DataType', 'PrimitiveType', 'Enumeration'],
    FlowProperty: classifiers,
    ConstraintProperty: ['ConstraintBlock'],
    ProxyPort: ['InterfaceBlock', 'Block', 'AssociationBlock', 'DataType'],
    FullPort: ['InterfaceBlock', 'Block', 'AssociationBlock', 'DataType'],
    Parameter: classifiers,
  }[kind] || [];
  const choices = project.elements.filter((element) => compatible.includes(element.kind));
  if (!choices.length) throw new Error(`${kind} requires a compatible type, but none exists in the model.`);
  const menu = choices.map((element, index) => `${index + 1}. ${element.name} (${element.kind})`).join('\n');
  const answer = prompt(`Choose a type for ${kind}:\n${menu}`, '1');
  if (!answer) return null;
  const index = Number(answer) - 1;
  if (!Number.isInteger(index) || !choices[index]) throw new Error('Invalid type selection.');
  return choices[index].id;
};

function extendedElementMarkup(project, element) {
  if (element.kind === 'Requirement') {
    return `<div class="classifier-header"><div class="stereotype">«requirement»</div><div class="block-name">${escapeHtml(element.name)}</div></div><div class="compartment"><div class="compartment-title">id</div><b>id</b> = ${escapeHtml(element.requirement_id || '')}</div><div class="compartment"><div class="compartment-title">text</div><b>text</b> = ${escapeHtml(element.requirement_text || '')}</div>${element.documentation ? `<div class="compartment"><div class="compartment-title">documentation</div>${escapeHtml(element.documentation)}</div>` : ''}`;
  }
  if (element.kind === 'Comment') return `<div class="comment-body">${escapeHtml(element.documentation || element.name)}</div>`;
  const stereotype = classifierStereotype(element.kind);
  const stereotypeMarkup = stereotype ? `<div class="stereotype">«${escapeHtml(stereotype)}»</div>` : '';
  const name = element.kind === 'InstanceSpecification' && element.type_id ? `${element.name} : ${typeName(project, element)}` : element.name;
  return `<div class="classifier-header">${stereotypeMarkup}<div class="block-name">${escapeHtml(name)}</div></div>${classifierCompartments(project, element)}`;
}

const baseRenderCanvasExtended = renderCanvas; renderCanvas = function renderCanvasExtended() {
  const canvas = $('canvas');
  canvas.innerHTML = '';
  const project = state.snapshot?.project;
  if (!project) {
    canvas.innerHTML = '<div class="empty-state"><h1>Systems Modeler Pro</h1><div>Create or open a project to begin.</div></div>';
    return;
  }
  const diagram = state.snapshot.diagrams.find((item) => item.id === state.selectedDiagramId);
  if (!diagram) {
    canvas.innerHTML = '<div class="empty-state"><h1>Model ready</h1><div>Create or select a Block Definition Diagram.</div></div>';
    return;
  }
  const frame = document.createElement('div');
  frame.className = 'diagram-frame';
  frame.innerHTML = `<div class="diagram-header">${diagram.family === 'requirement' ? 'req' : 'bdd'} [package] ${escapeHtml(diagram.name)}</div>`;
  canvas.appendChild(frame);
  createRelationshipLayer(frame, diagram, project);
  const elementsById = new Map(project.elements.map((element) => [element.id, element]));
  for (const node of diagram.nodes) {
    const element = elementsById.get(node.element_id);
    if (!element) continue;
    const box = document.createElement('button');
    box.className = `bdd-block kind-${element.kind.toLowerCase()}`;
    box.dataset.semanticKind = element.kind;
    box.dataset.presentationId = node.id;
    if (state.selectedElementId === element.id) box.classList.add('selected');
    if (state.pendingRelationship?.sourceElementId === element.id) box.classList.add('relationship-source');
    box.style.left = `${node.x}px`; box.style.top = `${node.y}px`; box.style.width = `${node.width}px`; box.style.minHeight = `${node.height}px`; box.style.height = 'auto';
    box.innerHTML = extendedElementMarkup(project, element);
    box.onclick = async (event) => {
      event.stopPropagation();
      if (state.pendingRelationship) {
        if (diagram.family !== 'requirement' && !BDD_RELATIONSHIP_CLASSIFIER_KINDS.has(element.kind)) { alert(`${element.kind} is not a classifier endpoint for this BDD relationship.`); return; }
        if (!state.pendingRelationship.sourceElementId) {
          Object.assign(state, { selectedElementId:element.id });
          state.pendingRelationship.sourceElementId = element.id; render(); return;
        }
        if (state.pendingRelationship.sourceElementId !== element.id) {
          const pending = { ...state.pendingRelationship }; state.pendingRelationship = null;
          const sourceNode = diagram.nodes.find((candidate) => candidate.element_id === pending.sourceElementId);
          const targetNode = diagram.nodes.find((candidate) => candidate.element_id === element.id);
          const command = diagram.family === 'requirement' ? 'create_traceability_relationship' : 'create_bdd_relationship_complete';
          const args = diagram.family === 'requirement'
            ? { diagramId:state.selectedDiagramId, relationshipKind:pending.kind, sourceNodeId:sourceNode?.id, targetNodeId:targetNode?.id }
            : { diagramId:state.selectedDiagramId, kind:pending.kind, sourceElementId:pending.sourceElementId, targetElementId:element.id };
          await runCommand(`Creating ${pending.kind}…`, () => requireInvoke()(command, args));
          state.selectedElementId = element.id;
          await refresh();
          return;
        }
      }
      state.selectedRelationshipId = null; state.paletteTool = null; state.selectedElementId = element.id; render();
    };
    frame.appendChild(box);
  }
  frame.onclick = async (event) => {
    if (state.paletteTool?.category === 'element') {
      const point = diagramCoordinates(frame, event);
      await createPaletteElementAt(state.paletteTool, point.x, point.y);
      return;
    }
    state.selectedElementId = null; state.selectedRelationshipId = null;
    state.pendingRelationship = null; state.paletteTool = null; render();
  };
};

const baseRenderPropertiesExtended = renderProperties;
renderProperties = function renderPropertiesExtended() {
  baseRenderPropertiesExtended();
  const project = state.snapshot?.project;
  const element = project?.elements.find((item) => item.id === state.selectedElementId);
  if (!element) return;
  const panel = $('properties');
  const apply = $('apply-element');
  if (!apply) return;
  const typedFeature = BDD_TYPED_FEATURE_KINDS.has(element.kind);
  if (typedFeature) {
    const semantic = document.createElement('div');
    semantic.className = 'property-semantic-group';
    semantic.innerHTML = `<div class="property-subheading">Feature semantics</div>
      <label>Derived<input id="property-derived" type="checkbox" ${element.is_derived ? 'checked' : ''}></label>
      <label>Read Only<input id="property-readonly" type="checkbox" ${element.is_read_only ? 'checked' : ''}></label>
      ${['ProxyPort', 'FullPort'].includes(element.kind) ? `<label>Conjugated<input id="property-conjugated" type="checkbox" ${element.is_conjugated ? 'checked' : ''}></label>` : ''}
      ${element.kind === 'Parameter' ? '<label>Direction<select id="property-parameter-direction"><option>in</option><option>out</option><option>inout</option><option>return</option></select></label>' : ''}
      ${element.kind === 'FlowProperty' ? '<label>Flow Direction<select id="property-flow-direction"><option>in</option><option>out</option><option>inout</option></select></label>' : ''}`;
    panel.insertBefore(semantic, apply);
    if ($('property-parameter-direction') && element.parameter_direction) $('property-parameter-direction').value = element.parameter_direction;
    if ($('property-flow-direction') && element.flow_direction) $('property-flow-direction').value = element.flow_direction;
  }
  const originalApply = apply.onclick;
  apply.onclick = async () => {
    if (originalApply) await originalApply();
    if (!typedFeature) return;
    await runCommand('Updating feature semantics…', () => requireInvoke()('update_bdd_feature_semantics', {
      elementId: element.id, lower: null, upper: null, aggregation: null,
      isDerived: $('property-derived')?.checked ?? null,
      isReadOnly: $('property-readonly')?.checked ?? null,
      isConjugated: $('property-conjugated')?.checked ?? null,
      parameterDirection: $('property-parameter-direction')?.value ?? null,
      flowDirection: $('property-flow-direction')?.value ?? null,
    }));
    await refresh();
  };
};
