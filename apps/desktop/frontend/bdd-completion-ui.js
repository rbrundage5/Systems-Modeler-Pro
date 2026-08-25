const BDD_CLASSIFIER_KINDS = new Set(['Block', 'InterfaceBlock', 'ValueType', 'DataType', 'Enumeration', 'ConstraintBlock']);
const BDD_TYPED_FEATURE_KINDS = new Set(['PartProperty', 'ReferenceProperty', 'ValueProperty', 'ConstraintProperty', 'ConstraintParameter', 'ProxyPort', 'FullPort', 'Parameter']);

function typeName(project, element) {
  if (!element?.type_id) return '';
  return project.elements.find((candidate) => candidate.id === element.type_id)?.name || element.type_id;
}

function featureNotation(project, element) {
  const prefix = element.is_derived ? '/' : '';
  let text = `${prefix}${element.name}`;
  const type = typeName(project, element);
  if (type) text += ` : ${type}`;
  if (element.multiplicity) text += ` [${element.multiplicity}]`;
  if (element.default_value) text += ` = ${element.default_value}`;
  return text;
}

function classifierStereotype(kind) {
  return {
    Block: 'block', InterfaceBlock: 'interfaceBlock', ValueType: 'valueType',
    ConstraintBlock: 'constraintBlock', Enumeration: 'enumeration', DataType: 'dataType',
  }[kind] || kind;
}

function classifierCompartments(project, element) {
  const children = project.elements.filter((candidate) => candidate.owner_id === element.id);
  const groups = [
    ['literals', ['EnumerationLiteral']], ['parts', ['PartProperty']], ['references', ['ReferenceProperty']],
    ['values', ['ValueProperty']], ['constraints', ['ConstraintProperty']], ['ports', ['ProxyPort', 'FullPort']],
    ['operations', ['Operation']], ['receptions', ['Reception']], ['parameters', ['Parameter', 'ConstraintParameter']],
  ];
  return groups.map(([label, kinds]) => {
    const items = children.filter((child) => kinds.includes(child.kind));
    if (!items.length) return '';
    return `<div class="compartment"><div class="compartment-title">${escapeHtml(label)}</div>${items.map((child) => `<div>${escapeHtml(featureNotation(project, child))}</div>`).join('')}</div>`;
  }).join('');
}

paletteSymbol = function paletteSymbolComplete(item) {
  const symbols = {
    Block: '▭', InterfaceBlock: '◫', ValueType: 'V', DataType: 'D', Enumeration: 'E', ConstraintBlock: 'C',
    PartProperty: '◆', ReferenceProperty: '◇', ValueProperty: 'v', ConstraintProperty: 'c', ProxyPort: '□', FullPort: '■',
    Operation: 'ƒ', Reception: '⇥', Parameter: 'p', ConstraintParameter: 'p=', EnumerationLiteral: '•', Association: '──', Aggregation: '◇─',
    Composition: '◆─', Generalization: '▷─', Dependency: '⇢', Realization: '⇢▷',
  };
  return symbols[item.semantic_kind] || symbols[item.relationship_kind] || '·';
};

renderPalette = function renderPaletteComplete() {
  const host = $('palette'); host.innerHTML = '';
  if (!state.selectedDiagramId) { host.innerHTML = '<div class="muted">Select a diagram to load its Rust-defined palette.</div>'; return; }
  const groups = [['element', 'Classifiers'], ['feature', 'Owned Features'], ['relationship', 'Relationships']];
  for (const [category, title] of groups) {
    const items = state.paletteItems.filter((item) => item.category === category); if (!items.length) continue;
    const section = document.createElement('section'); section.className = 'palette-section'; section.innerHTML = `<div class="palette-section-title">${title}</div>`;
    for (const item of items) {
      const button = document.createElement('button'); button.className = `palette-item ${category}`;
      const active = category === 'element' ? state.paletteTool?.id === item.id : category === 'relationship' && state.pendingRelationship?.kind === item.relationship_kind;
      if (active) button.classList.add('active');
      button.innerHTML = `<span class="palette-symbol">${escapeHtml(paletteSymbol(item))}</span><span>${escapeHtml(item.label)}</span>`;
      button.onclick = () => activatePaletteItem(item);
      if (item.draggable) { button.draggable = true; button.title = 'Click then click the diagram, or drag directly onto the diagram.'; }
      else if (category === 'feature') button.title = 'Select a compatible semantic owner, then click this feature.';
      section.appendChild(button);
    }
    host.appendChild(section);
  }
  const hint = document.createElement('div'); hint.className = 'palette-hint'; hint.textContent = 'Classifier and relationship eligibility comes from Rust. Owned features are created under the selected classifier or operation and rendered in compartments.'; host.appendChild(hint);
};

async function chooseTypeId(kind) {
  const project = state.snapshot?.project; if (!project) return null;
  const compatible = {
    PartProperty: ['Block', 'InterfaceBlock'], ReferenceProperty: ['Block', 'InterfaceBlock', 'DataType', 'ValueType', 'Enumeration'],
    ValueProperty: ['ValueType', 'DataType', 'Enumeration'], ConstraintProperty: ['ConstraintBlock'],
    ProxyPort: ['InterfaceBlock', 'Block', 'DataType'], FullPort: ['InterfaceBlock', 'Block', 'DataType'],
    Parameter: ['Block', 'InterfaceBlock', 'ValueType', 'DataType', 'Enumeration'],
    ConstraintParameter: ['ValueType', 'DataType', 'PrimitiveType', 'Enumeration'],
  }[kind] || [];
  const choices = project.elements.filter((element) => compatible.includes(element.kind));
  // The Rust ConstraintParameter command can atomically provision a reusable
  // PrimitiveType named Real for a fresh model. Other typed features still
  // require the engineer to select an existing compatible semantic type.
  if (!choices.length && kind === 'ConstraintParameter') return '__create_real__';
  if (!choices.length) throw new Error(`${kind} requires a compatible type, but none exists in the model.`);
  const menu = choices.map((element, index) => `${index + 1}. ${element.name} (${element.kind})`).join('\n');
  const answer = prompt(`Choose a type for ${kind}:\n${menu}`, '1'); if (!answer) return null;
  const index = Number(answer) - 1; if (!Number.isInteger(index) || !choices[index]) throw new Error('Invalid type selection.');
  return choices[index].id;
}

async function createFeatureFromPalette(item) {
  const project = state.snapshot?.project;
  const owner = project?.elements.find((element) => element.id === state.selectedElementId);
  if (!owner) throw new Error(`Select the semantic owner before creating ${item.label}.`);
  // A ConstraintBlock owns ConstraintParameters, never behavioral Parameters.
  // Preserve the ordinary Parameter tool for Operations while making the
  // existing BDD workflow do the semantically correct thing for ConstraintBlocks.
  const semanticKind = item.semantic_kind === 'Parameter' && owner.kind === 'ConstraintBlock'
    ? 'ConstraintParameter'
    : item.semantic_kind;
  const name = prompt(`${item.label} name`, `New ${item.label}`); if (!name) return;
  const typeId = BDD_TYPED_FEATURE_KINDS.has(semanticKind) ? await chooseTypeId(semanticKind) : null;
  if (BDD_TYPED_FEATURE_KINDS.has(semanticKind) && !typeId) return;
  let lower = null, upper = null;
  let notation = null;
  if (BDD_TYPED_FEATURE_KINDS.has(semanticKind)) {
    notation = prompt('Multiplicity', '1'); if (!notation) return;
    if (notation === '*') { lower = 0; upper = null; }
    else if (notation.includes('..')) { const [lo, hi] = notation.split('..'); lower = Number(lo); upper = hi === '*' ? null : Number(hi); }
    else { lower = Number(notation); upper = Number(notation); }
    if (!Number.isInteger(lower) || (upper !== null && !Number.isInteger(upper))) throw new Error('Multiplicity must be 1, 0..1, 1..*, *, etc.');
  }
  const featureId = semanticKind === 'ConstraintParameter'
    ? await runCommand('Creating Constraint Parameter…', () => requireInvoke()('create_constraint_parameter', {
        constraintBlockId: owner.id,
        name,
        typeId: typeId === '__create_real__' ? null : typeId,
        multiplicity: notation,
      }))
    : await runCommand(`Creating ${item.label}…`, () => requireInvoke()('create_bdd_feature', {
        kind: semanticKind, ownerId: owner.id, name, typeId, lower, upper, defaultValue: null,
      }));
  state.selectedElementId = featureId; await refresh();
}

activatePaletteItem = function activatePaletteItemComplete(item) {
  state.selectedRelationshipId = null;
  if (item.category === 'relationship') { state.paletteTool = null; state.pendingRelationship = { kind: item.relationship_kind, sourceElementId: null }; render(); }
  else if (item.category === 'feature') { state.pendingRelationship = null; state.paletteTool = null; createFeatureFromPalette(item).catch((error) => { console.error(error); alert(error?.message || String(error)); }); }
  else { state.pendingRelationship = null; state.paletteTool = item; render(); }
};

const createStructuralPaletteElementAt = createPaletteElementAt;
createPaletteElementAt = async function createPaletteElementAtComplete(item, x, y) {
  const active = state.snapshot?.diagrams?.find((diagram) => diagram.id === state.selectedDiagramId);
  if (active?.family === 'requirement') return createStructuralPaletteElementAt(item, x, y);
  if (!BDD_CLASSIFIER_KINDS.has(item.semantic_kind)) throw new Error(`${item.label} is not a top-level BDD classifier.`);
  const diagram = state.snapshot.diagrams.find((candidate) => candidate.id === state.selectedDiagramId); if (!diagram) throw new Error('Select a BDD first.');
  const name = prompt(`${item.label} name`, `New ${item.label}`); if (!name) return;
  const elementId = await runCommand(`Creating ${item.label}…`, () => requireInvoke()('create_bdd_element', { kind: item.semantic_kind, ownerId: diagram.owner_id, name }));
  await runCommand(`Placing ${item.label}…`, () => requireInvoke()('place_bdd_element', { diagramId: diagram.id, elementId, x, y }));
  state.selectedElementId = elementId; state.paletteTool = null; await refresh();
};

const placeExistingStructuralElementAt = placeExistingElementAt;
placeExistingElementAt = async function placeExistingElementAtComplete(elementId, x, y) {
  const active = state.snapshot?.diagrams?.find((diagram) => diagram.id === state.selectedDiagramId);
  if (active?.family === 'requirement') return placeExistingStructuralElementAt(elementId, x, y);
  await runCommand('Placing existing classifier…', () => requireInvoke()('place_bdd_element', { diagramId: state.selectedDiagramId, elementId, x, y }));
  state.selectedElementId = elementId; await refresh();
};

refresh = async function refreshComplete() {
  state.snapshot = await requireInvoke()('workspace_snapshot_complete');
  if (state.selectedRelationshipId && !state.snapshot?.project?.relationships?.some((relationship) => relationship.id === state.selectedRelationshipId)) state.selectedRelationshipId = null;
  const diagramExists = state.snapshot?.diagrams?.some((diagram) => diagram.id === state.selectedDiagramId)
    || state.snapshot?.ibd_diagrams?.some((diagram) => diagram.id === state.selectedDiagramId);
  if (state.selectedDiagramId && !diagramExists) state.selectedDiagramId = null;
  await loadPalette(); render();
};

renderRepository = function renderRepositoryComplete() {
  const host = $('repository'); host.innerHTML = ''; const project = state.snapshot?.project;
  if (!project) { host.innerHTML = '<div class="muted">No project open.</div>'; return; }
  const root = document.createElement('div'); root.className = 'tree-root'; root.textContent = project.name; host.appendChild(root);
  const byOwner = new Map();
  for (const element of project.elements) { if (!element.owner_id) continue; if (!byOwner.has(element.owner_id)) byOwner.set(element.owner_id, []); byOwner.get(element.owner_id).push(element); }
  function branchHasMatch(element) { if (repositoryMatches(element)) return true; return (byOwner.get(element.id) || []).some(branchHasMatch); }
  function appendChildren(ownerId, depth) {
    for (const element of (byOwner.get(ownerId) || []).sort((a, b) => a.name.localeCompare(b.name))) {
      if (!branchHasMatch(element)) continue;
      const row = document.createElement('button'); row.className = 'tree-row';
      if (state.selectedElementId === element.id || state.selectedPackageId === element.id) row.classList.add('selected');
      row.style.paddingLeft = `${12 + depth * 16}px`;
      const icon = element.kind === 'Package' ? '▣' : BDD_CLASSIFIER_KINDS.has(element.kind) ? paletteSymbol({ semantic_kind: element.kind }) : '·';
      row.innerHTML = `<span class="kind">${escapeHtml(icon)}</span><span>${escapeHtml(element.name)}</span><span class="type-tag">${escapeHtml(element.kind)}</span>`;
      if (BDD_CLASSIFIER_KINDS.has(element.kind)) { row.draggable = true; row.title = 'Drag onto a BDD to create another presentation of this existing semantic classifier.'; }
      row.onclick = () => { state.selectedRelationshipId = null; state.paletteTool = null; if (element.kind === 'Package') { state.selectedPackageId = element.id; state.selectedElementId = null; } else state.selectedElementId = element.id; render(); };
      row.ondblclick = async () => {
        if (!['Block', 'AssociationBlock'].includes(element.kind)) return;
        const child = (state.snapshot.ibd_diagrams || []).find((diagram) => diagram.context_block_id === element.id);
        if (child) await selectDiagram(child.id);
      };
      host.appendChild(row); appendChildren(element.id, depth + 1);
    }
  }
  appendChildren(project.root_id, 0);
  if (state.snapshot.diagrams.length) {
    const heading = document.createElement('div'); heading.className = 'tree-heading'; heading.textContent = 'Diagrams'; host.appendChild(heading);
    for (const diagram of state.snapshot.diagrams) {
      if (state.repositoryFilter && !diagram.name.toLowerCase().includes(state.repositoryFilter.toLowerCase())) continue;
      const row = document.createElement('button'); row.className = 'tree-row diagram-row'; if (state.selectedDiagramId === diagram.id) row.classList.add('selected');
      row.innerHTML = `<span class="kind">▤</span><span>${escapeHtml(diagram.name)}</span><span class="type-tag">${diagram.family === 'package' ? 'PKG' : diagram.family === 'requirement' ? 'REQ' : diagram.family === 'use-case' ? 'UC' : 'BDD'}</span>`; row.onclick = () => selectDiagram(diagram.id); host.appendChild(row);
    }
  }
};

const renderStructuralCanvas = renderCanvas; renderCanvas = function renderCanvasComplete() {
  const canvas = $('canvas'); canvas.innerHTML = ''; const project = state.snapshot?.project;
  if (!project) { canvas.innerHTML = '<div class="empty-state"><h1>Systems Modeler Pro</h1><div>Create or open a project to begin.</div></div>'; return; }
  const diagram = state.snapshot.diagrams.find((item) => item.id === state.selectedDiagramId); if (diagram?.family === 'requirement') return renderStructuralCanvas();
  if (!diagram) { canvas.innerHTML = '<div class="empty-state"><h1>Model ready</h1><div>Create or select a Block Definition Diagram.</div></div>'; return; }
  const frame = document.createElement('div'); frame.className = 'diagram-frame'; frame.innerHTML = `<div class="diagram-header">bdd [package] ${escapeHtml(diagram.name)}</div>`; canvas.appendChild(frame);
  createRelationshipLayer(frame, diagram, project);
  const elementsById = new Map(project.elements.map((element) => [element.id, element]));
  for (const node of diagram.nodes) {
    const element = elementsById.get(node.element_id); if (!element) continue;
    const box = document.createElement('button'); box.className = 'bdd-block'; box.dataset.semanticKind = element.kind;
    if (state.selectedElementId === element.id) box.classList.add('selected'); if (state.pendingRelationship?.sourceElementId === element.id) box.classList.add('relationship-source');
    box.style.left = `${node.x}px`; box.style.top = `${node.y}px`; box.style.width = `${node.width}px`; box.style.minHeight = `${node.height}px`; box.style.height = 'auto';
    box.innerHTML = `<div class="stereotype">«${escapeHtml(classifierStereotype(element.kind))}»</div><div class="block-name">${escapeHtml(element.name)}</div>${classifierCompartments(project, element)}`;
    box.onclick = async (event) => {
      event.stopPropagation();
      if (state.pendingRelationship) {
        if (!BDD_CLASSIFIER_KINDS.has(element.kind)) return;
        if (!state.pendingRelationship.sourceElementId) { state.pendingRelationship.sourceElementId = element.id; state.selectedElementId = element.id; render(); return; }
        if (state.pendingRelationship.sourceElementId !== element.id) {
          const pending = { ...state.pendingRelationship }; state.pendingRelationship = null;
          await runCommand(`Creating ${pending.kind}…`, () => requireInvoke()('create_bdd_relationship_complete', { diagramId: state.selectedDiagramId, kind: pending.kind, sourceElementId: pending.sourceElementId, targetElementId: element.id }));
          state.selectedElementId = element.id; await refresh(); return;
        }
      }
      state.selectedRelationshipId = null; state.paletteTool = null; state.selectedElementId = element.id; render();
    };
    box.ondblclick = async (event) => {
      event.stopPropagation();
      if (!['Block', 'AssociationBlock'].includes(element.kind)) return;
      const child = (state.snapshot.ibd_diagrams || []).find((candidate) => candidate.context_block_id === element.id);
      if (child) await selectDiagram(child.id);
    };
    frame.appendChild(box);
  }
  frame.onclick = async (event) => {
    if (state.paletteTool?.category === 'element') { const point = diagramCoordinates(frame, event); await createPaletteElementAt(state.paletteTool, point.x, point.y); return; }
    state.selectedElementId = null; state.selectedRelationshipId = null; state.pendingRelationship = null; state.paletteTool = null; render();
  };
};

const renderStructuralProperties = renderProperties; renderProperties = function renderPropertiesComplete() {
  const panel = $('properties'); const project = state.snapshot?.project;
  if (!project) { panel.innerHTML = '<div class="muted">Create or open a project to inspect properties.</div>'; return; }
  const relationship = project.relationships?.find((item) => item.id === state.selectedRelationshipId); if (relationship) return renderRelationshipProperties(panel, project, relationship);
  const element = project.elements.find((item) => item.id === state.selectedElementId); if (element?.kind === 'Requirement' || element?.kind === 'TestCase') return renderStructuralProperties();
  if (!element) { panel.innerHTML = '<div class="muted">Select an element or relationship.</div>'; return; }
  panel.innerHTML = `<div class="property-heading">${escapeHtml(element.kind)}</div><label>Name<input id="property-name" value="${escapeAttr(element.name)}"></label><label>Documentation<textarea id="property-documentation" rows="5">${escapeHtml(element.documentation || '')}</textarea></label><label>Stable ID<input value="${escapeAttr(element.external_id)}" disabled></label>${element.type_id ? `<label>Type<input value="${escapeAttr(typeName(project, element))}" disabled></label>` : ''}${element.multiplicity ? `<label>Multiplicity<input value="${escapeAttr(element.multiplicity)}" disabled></label>` : ''}${element.kind === 'ValueType' ? `<label>Quantity Kind ID<input id="property-quantity-kind" value="${escapeAttr(element.quantity_kind_external_id || '')}"></label><label>Unit ID<input id="property-unit" value="${escapeAttr(element.unit_external_id || '')}"></label>` : ''}${element.default_value !== null && element.default_value !== undefined ? `<label>Default Value<input id="property-default" value="${escapeAttr(element.default_value || '')}"></label>` : ''}<button id="apply-element" class="primary">Apply</button>`;
  $('apply-element').onclick = async () => {
    const name = $('property-name').value.trim(); if (!name) return;
    if (name !== element.name) await runCommand('Renaming element…', () => requireInvoke()('rename_element', { elementId: element.id, name }));
    await runCommand('Updating element details…', () => requireInvoke()('update_bdd_element_details', { elementId: element.id, documentation: $('property-documentation').value, defaultValue: $('property-default')?.value ?? null, quantityKindExternalId: $('property-quantity-kind')?.value ?? null, unitExternalId: $('property-unit')?.value ?? null }));
    await refresh();
  };
};

async function saveProjectAsComplete() {
  if (!state.snapshot?.project) return alert('Create or open a project first.');
  const suggested = state.snapshot.current_file || `${state.snapshot.project.name}.smproj`; const path = prompt('Save project as (.smproj)', suggested); if (!path) return;
  await runCommand('Saving project…', () => requireInvoke()('save_project_file_complete', { path })); await refresh();
}

async function saveProjectComplete() {
  if (!state.snapshot?.project) return alert('Create or open a project first.'); if (!state.snapshot.current_file) return saveProjectAsComplete();
  await runCommand('Saving project…', () => requireInvoke()('save_current_project_complete')); await refresh();
}

async function openProjectComplete() {
  const suggested = state.snapshot?.current_file || 'Vehicle Model.smproj'; const path = prompt('Project file path (.smproj)', suggested); if (!path) return;
  await runCommand('Opening project…', () => requireInvoke()('open_project_file_complete', { path }));
  Object.assign(state, { paletteItems: [], paletteTool: null, selectedElementId: null, selectedPackageId: null, selectedDiagramId: null, selectedRelationshipId: null, pendingRelationship: null });
  state.snapshot = await requireInvoke()('workspace_snapshot_complete');
  if (state.snapshot.diagrams.length) state.selectedDiagramId = state.snapshot.diagrams[0].id; else if (state.snapshot.ibd_diagrams?.length) state.selectedDiagramId = state.snapshot.ibd_diagrams[0].id;
  await loadPalette(); render();
}

$('open-project').onclick = openProjectComplete;
$('save-project').onclick = saveProjectComplete;
$('save-project-as').onclick = saveProjectAsComplete;

refresh().catch((error) => renderStatus(error.message));
