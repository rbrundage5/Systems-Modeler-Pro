(() => {
  'use strict';

  const editing = window.smpRepositoryEditing;
  if (!editing) return;

  const NAMESPACE_KINDS = new Set(['Model', 'Package', 'ModelLibrary']);
  const CLASSIFIER_KINDS = new Set([
    'Block',
    'AssociationBlock',
    'InterfaceBlock',
    'ConstraintBlock',
    'ValueType',
    'DataType',
    'PrimitiveType',
    'Enumeration',
    'Signal',
    'Requirement',
    'TestCase',
    'Actor',
    'UseCase',
  ]);

  function ownerKindAccepted(elementKind, ownerKind) {
    switch (elementKind) {
      case 'PartProperty':
      case 'ReferenceProperty':
        return ['Block', 'AssociationBlock'].includes(ownerKind);
      case 'ValueProperty':
        return ['Block', 'AssociationBlock', 'DataType', 'ValueType'].includes(ownerKind);
      case 'FlowProperty':
        return ['Block', 'InterfaceBlock'].includes(ownerKind);
      case 'ConstraintProperty':
        return ['Block', 'AssociationBlock', 'ConstraintBlock'].includes(ownerKind);
      case 'ConstraintParameter':
        return ownerKind === 'ConstraintBlock';
      case 'ProxyPort':
      case 'FullPort':
        return ['Block', 'AssociationBlock', 'InterfaceBlock'].includes(ownerKind);
      case 'Operation':
      case 'Reception':
        return CLASSIFIER_KINDS.has(ownerKind);
      case 'Parameter':
        return ownerKind === 'Operation';
      case 'EnumerationLiteral':
        return ownerKind === 'Enumeration';
      case 'Slot':
        return ownerKind === 'InstanceSpecification';
      case 'Comment':
        return NAMESPACE_KINDS.has(ownerKind) || CLASSIFIER_KINDS.has(ownerKind);
      default:
        return NAMESPACE_KINDS.has(ownerKind);
    }
  }

  function isWithinSubtree(owner, excludedId, byId) {
    if (!excludedId) return false;
    let current = owner;
    const visited = new Set();
    while (current && !visited.has(String(current.id))) {
      if (String(current.id) === String(excludedId)) return true;
      visited.add(String(current.id));
      current = byId.get(String(current.owner_id || ''));
    }
    return false;
  }

  function rebuildElementOwnerOptions() {
    const select = document.getElementById('repository-element-owner');
    const project = state.snapshot?.project;
    if (!select || !project) return;

    const selectedId = state.selectedElementId || state.selectedPackageId;
    const element = (project.elements || []).find(
      (candidate) => String(candidate.id) === String(selectedId || ''),
    );
    if (!element) return;

    const byId = new Map((project.elements || []).map((candidate) => [String(candidate.id), candidate]));
    const candidates = (project.elements || [])
      .filter((owner) => String(owner.id) !== String(element.id))
      .filter((owner) => !isWithinSubtree(owner, element.id, byId))
      .filter((owner) => ownerKindAccepted(element.kind, owner.kind))
      .sort((left, right) => String(left.name).localeCompare(String(right.name)));

    select.replaceChildren();
    for (const owner of candidates) {
      const option = document.createElement('option');
      option.value = String(owner.id);
      option.textContent = `${owner.name} (${owner.kind})`;
      option.selected = String(owner.id) === String(element.owner_id || '');
      select.appendChild(option);
    }

    // Never let the browser silently display the first namespace when the
    // semantic owner is a classifier/feature owner omitted by a stale UI rule.
    if (element.owner_id && ![...select.options].some((option) => option.selected)) {
      const owner = byId.get(String(element.owner_id));
      if (owner) {
        const option = document.createElement('option');
        option.value = String(owner.id);
        option.textContent = `${owner.name} (${owner.kind})`;
        option.selected = true;
        select.insertBefore(option, select.firstChild);
      }
    }
  }

  function renderProperties() {
    editing.renderProperties();
    rebuildElementOwnerOptions();
  }

  window.smpRepositoryEditing = Object.freeze({
    renderProperties,
    handleDelete: editing.handleDelete,
  });
})();
