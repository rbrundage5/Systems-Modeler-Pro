from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    if old not in text:
        raise SystemExit(f"PR39 patch anchor missing in {path}: {old[:120]!r}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


def replace_between(path: Path, start: str, end: str, replacement: str) -> None:
    text = path.read_text(encoding="utf-8")
    a = text.find(start)
    if a < 0:
        raise SystemExit(f"PR39 start anchor missing in {path}: {start!r}")
    b = text.find(end, a)
    if b < 0:
        raise SystemExit(f"PR39 end anchor missing in {path}: {end!r}")
    path.write_text(text[:a] + replacement + text[b:], encoding="utf-8")


bulk = ROOT / "apps/desktop/src-tauri/src/workspace/bulk_model.rs"
replace_once(
    bulk,
    "use systems_modeler_core::{DiagramFamilyId, supported_diagram_families};",
    "use systems_modeler_core::{DiagramFamilyId, FlowDirection, supported_diagram_families};",
)
replace_once(
    bulk,
    '''    UpdateElement {
        element: ElementReference,
        name: String,
    },
    CreateRelationship {
''',
    '''    UpdateElement {
        element: ElementReference,
        name: String,
    },
    /// Spreadsheet/interchange scalar and owned-feature update path.
    /// Mutation stays inside the PR36 candidate build so preview/apply remain atomic.
    UpdateElementFields {
        element: ElementReference,
        name: Option<String>,
        owner: Option<ElementReference>,
        type_ref: Option<ElementReference>,
        external_id: Option<String>,
        documentation: Option<String>,
        visibility: Option<VisibilityKind>,
        requirement_id: Option<String>,
        requirement_text: Option<String>,
        multiplicity: Option<Multiplicity>,
        default_value: Option<String>,
        flow_direction: Option<FlowDirection>,
    },
    CreateRelationship {
''',
)
replace_once(
    bulk,
    "fn external_key(namespace: &str, external_id: &str) -> String {",
    "pub(super) fn external_key(namespace: &str, external_id: &str) -> String {",
)
replace_once(
    bulk,
    '''        ModelBuildOperation::UpdateElement { name, .. } => {
            format!("UPDATE element name to {name}")
        }
        ModelBuildOperation::CreateRelationship { external_id, .. } => {
''',
    '''        ModelBuildOperation::UpdateElement { name, .. } => {
            format!("UPDATE element name to {name}")
        }
        ModelBuildOperation::UpdateElementFields { .. } => "UPDATE mapped element fields".into(),
        ModelBuildOperation::CreateRelationship { external_id, .. } => {
''',
)
replace_once(
    bulk,
    '''                ModelBuildOperation::UpdateElement { element, name } => {
                    let id = resolve_element(&project, &element_ids, namespace, element, index)?;
                    project.rename_element(id, name).map_err(|cause| {
                        error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                    })?;
                }
                ModelBuildOperation::CreateRelationship {
''',
    '''                ModelBuildOperation::UpdateElement { element, name } => {
                    let id = resolve_element(&project, &element_ids, namespace, element, index)?;
                    project.rename_element(id, name).map_err(|cause| {
                        error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                    })?;
                }
                ModelBuildOperation::UpdateElementFields {
                    element,
                    name,
                    owner,
                    type_ref,
                    external_id,
                    documentation,
                    visibility,
                    requirement_id,
                    requirement_text,
                    multiplicity,
                    default_value,
                    flow_direction,
                } => {
                    let id = resolve_element(&project, &element_ids, namespace, element, index)?;
                    if let Some(owner) = owner {
                        let owner_id = resolve_element(&project, &element_ids, namespace, owner, index)?;
                        project.move_element(id, owner_id).map_err(|cause| {
                            error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                        })?;
                    }
                    if let Some(name) = name {
                        project.rename_element(id, name.clone()).map_err(|cause| {
                            error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                        })?;
                    }
                    if let Some(type_ref) = type_ref {
                        let type_id = resolve_element(&project, &element_ids, namespace, type_ref, index)?;
                        project.set_element_type(id, type_id).map_err(|cause| {
                            error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                        })?;
                    }
                    if let Some(external_id) = external_id {
                        project.set_external_id(id, external_key(namespace, external_id)).map_err(|cause| {
                            error("DUPLICATE_EXTERNAL_ID", Some(index), cause.to_string())
                        })?;
                    }
                    if let Some(documentation) = documentation {
                        project.element_mut(id).map_err(|cause| {
                            error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                        })?.documentation = documentation.clone();
                    }
                    if let Some(visibility) = visibility {
                        project.element_mut(id).map_err(|cause| {
                            error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                        })?.visibility = *visibility;
                    }
                    if let Some(multiplicity) = multiplicity {
                        project.set_multiplicity(id, *multiplicity).map_err(|cause| {
                            error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                        })?;
                    }
                    if let Some(default_value) = default_value {
                        if project.element(id).map_err(|cause| {
                            error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                        })?.kind != ElementKind::ValueProperty {
                            return Err(error(
                                "SEMANTIC_VALIDATION",
                                Some(index),
                                "Default Value mapping is valid only for ValueProperty",
                            ));
                        }
                        project.element_mut(id).map_err(|cause| {
                            error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                        })?.default_value = (!default_value.trim().is_empty()).then(|| default_value.clone());
                    }
                    if let Some(flow_direction) = flow_direction {
                        if project.element(id).map_err(|cause| {
                            error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                        })?.kind != ElementKind::FlowProperty {
                            return Err(error(
                                "SEMANTIC_VALIDATION",
                                Some(index),
                                "Flow Direction mapping is valid only for FlowProperty",
                            ));
                        }
                        project.element_mut(id).map_err(|cause| {
                            error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                        })?.flow_direction = Some(*flow_direction);
                    }
                    if requirement_id.is_some() || requirement_text.is_some() {
                        let current = project.element(id).map_err(|cause| {
                            error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                        })?;
                        if current.kind != ElementKind::Requirement {
                            return Err(error(
                                "SEMANTIC_VALIDATION",
                                Some(index),
                                "Requirement ID/Text mappings are valid only for Requirement elements",
                            ));
                        }
                        let next_requirement_id = requirement_id
                            .clone()
                            .or_else(|| current.requirement_id.clone())
                            .ok_or_else(|| error(
                                "SEMANTIC_VALIDATION",
                                Some(index),
                                "Requirement ID is required when applying mapped Requirement fields",
                            ))?;
                        let next_requirement_text = requirement_text
                            .clone()
                            .or_else(|| current.requirement_text.clone())
                            .unwrap_or_default();
                        project.update_requirement(id, next_requirement_id, next_requirement_text).map_err(|cause| {
                            error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                        })?;
                    }
                    project.validate_element(id).map_err(|cause| {
                        error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                    })?;
                }
                ModelBuildOperation::CreateRelationship {
''',
)

feature_editing = ROOT / "apps/desktop/src-tauri/src/workspace/feature_editing.rs"
replace_once(
    feature_editing,
    "fn parse_flow_direction(value: &str) -> Result<systems_modeler_core::FlowDirection, String> {",
    "pub(super) fn parse_flow_direction(value: &str) -> Result<systems_modeler_core::FlowDirection, String> {",
)
parametrics = ROOT / "apps/desktop/src-tauri/src/workspace/parametrics.rs"
replace_once(
    parametrics,
    "fn parse_multiplicity(value: &str) -> Result<Multiplicity, String> {",
    "pub(super) fn parse_multiplicity(value: &str) -> Result<Multiplicity, String> {",
)

spreadsheet = ROOT / "apps/desktop/src-tauri/src/workspace/spreadsheet_import.rs"
replace_once(
    spreadsheet,
    "use systems_modeler_core::{Element, ElementId, ElementKind, Project, VisibilityKind};",
    "use systems_modeler_core::{Element, ElementId, ElementKind, FlowDirection, Multiplicity, Project, VisibilityKind};",
)
replace_once(
    spreadsheet,
    '''    Owner,
    ExternalId,
    Visibility,
''',
    '''    Owner,
    Type,
    Multiplicity,
    DefaultValue,
    FlowDirection,
    ExternalId,
    Visibility,
''',
)
replace_between(
    spreadsheet,
    "fn supported_kind(kind: &ElementKind) -> bool {",
    "fn diagnostic(",
    '''fn supported_kind(kind: &ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Package
            | ElementKind::ModelLibrary
            | ElementKind::Block
            | ElementKind::AssociationBlock
            | ElementKind::InterfaceBlock
            | ElementKind::ConstraintBlock
            | ElementKind::ValueType
            | ElementKind::DataType
            | ElementKind::PrimitiveType
            | ElementKind::Enumeration
            | ElementKind::Signal
            | ElementKind::Actor
            | ElementKind::UseCase
            | ElementKind::Requirement
            | ElementKind::TestCase
            | ElementKind::PartProperty
            | ElementKind::ReferenceProperty
            | ElementKind::ValueProperty
            | ElementKind::FlowProperty
            | ElementKind::ConstraintProperty
            | ElementKind::ConstraintParameter
    )
}

fn is_feature_kind(kind: &ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::PartProperty
            | ElementKind::ReferenceProperty
            | ElementKind::ValueProperty
            | ElementKind::FlowProperty
            | ElementKind::ConstraintProperty
            | ElementKind::ConstraintParameter
    )
}

''',
)
replace_once(
    spreadsheet,
    'format!("{:?} is outside the PR38 basic-element scope", map.element_kind),',
    'format!("{:?} is outside the PR39 package/basic-element/owned-feature scope", map.element_kind),',
)
replace_once(
    spreadsheet,
    '''    let target = project.element(map.target_scope).map_err(|_| {
''',
    '''    let has_property = |property| {
        map.column_mappings
            .iter()
            .any(|mapping| mapping.property == property)
    };
    if map.element_kind != ElementKind::Requirement
        && (has_property(SpreadsheetSemanticProperty::RequirementId)
            || has_property(SpreadsheetSemanticProperty::RequirementText))
    {
        return Err(diagnostic(
            Some(map), None, None, None, None,
            "SEMANTIC_PROPERTY_INVALID",
            "Requirement ID/Text columns can be mapped only for Requirement elements",
        ));
    }
    if is_feature_kind(&map.element_kind) && !has_property(SpreadsheetSemanticProperty::Type) {
        return Err(diagnostic(
            Some(map), None, None, Some(SpreadsheetSemanticProperty::Type), None,
            "FEATURE_TYPE_COLUMN_REQUIRED",
            format!("{:?} mappings require an explicit Type column", map.element_kind),
        ));
    }
    if !is_feature_kind(&map.element_kind)
        && (has_property(SpreadsheetSemanticProperty::Type)
            || has_property(SpreadsheetSemanticProperty::Multiplicity)
            || has_property(SpreadsheetSemanticProperty::DefaultValue)
            || has_property(SpreadsheetSemanticProperty::FlowDirection))
    {
        return Err(diagnostic(
            Some(map), None, None, None, None,
            "SEMANTIC_PROPERTY_INVALID",
            "Type/Multiplicity/Default Value/Flow Direction mappings are reserved for PR39 owned features",
        ));
    }
    if map.element_kind != ElementKind::ValueProperty
        && has_property(SpreadsheetSemanticProperty::DefaultValue)
    {
        return Err(diagnostic(
            Some(map), None, None, Some(SpreadsheetSemanticProperty::DefaultValue), None,
            "SEMANTIC_PROPERTY_INVALID",
            "Default Value can be mapped only for ValueProperty",
        ));
    }
    if map.element_kind == ElementKind::FlowProperty {
        if !has_property(SpreadsheetSemanticProperty::FlowDirection) {
            return Err(diagnostic(
                Some(map), None, None, Some(SpreadsheetSemanticProperty::FlowDirection), None,
                "FLOW_DIRECTION_COLUMN_REQUIRED",
                "FlowProperty mappings require an explicit Flow Direction column",
            ));
        }
    } else if has_property(SpreadsheetSemanticProperty::FlowDirection) {
        return Err(diagnostic(
            Some(map), None, None, Some(SpreadsheetSemanticProperty::FlowDirection), None,
            "SEMANTIC_PROPERTY_INVALID",
            "Flow Direction can be mapped only for FlowProperty",
        ));
    }

    let target = project.element(map.target_scope).map_err(|_| {
''',
)
replace_between(
    spreadsheet,
    "fn existing_owner_in_scope(",
    "fn find_by_external_id",
    '''fn reference_in_scope(
    project: &Project,
    element_id: ElementId,
    target: ElementId,
    search_scope: SpreadsheetSearchScope,
) -> bool {
    if element_id == target {
        return true;
    }
    match search_scope {
        SpreadsheetSearchScope::TargetOnly => project
            .element(element_id)
            .ok()
            .is_some_and(|element| element.owner_id == Some(target)),
        SpreadsheetSearchScope::TargetRecursive => {
            distance_from_target(project, element_id, target).is_some()
        }
    }
}

fn qname_aliases(canonical: &str, root_name: &str, target_qname: &str) -> Vec<String> {
    let mut aliases = vec![canonical.to_string()];
    if let Some(relative) = canonical.strip_prefix(&format!("{root_name}::")) {
        aliases.push(relative.to_string());
    }
    if canonical == target_qname {
        aliases.push(target_qname.rsplit("::").next().unwrap_or(target_qname).to_string());
    } else if let Some(relative) = canonical.strip_prefix(&format!("{target_qname}::")) {
        aliases.push(relative.to_string());
    }
    aliases.sort();
    aliases.dedup();
    aliases
}

fn resolve_semantic_reference(
    map: &SpreadsheetImportMap,
    project: &Project,
    planned: &[PlannedElement],
    requested: &str,
    property: SpreadsheetSemanticProperty,
    label: &str,
) -> Result<ResolvedOwner, SpreadsheetImportDiagnostic> {
    let target_qname = project.qualified_name(map.target_scope).map_err(|error| {
        diagnostic(Some(map), None, None, Some(property), None, "TARGET_SCOPE_UNRESOLVED", error.to_string())
    })?;
    let root_name = project.element(project.root_id).map(|root| root.name.as_str()).unwrap_or_default();
    let requested = requested.trim();

    let mut existing_external = project.elements.values()
        .filter(|element| reference_in_scope(project, element.id, map.target_scope, map.search_scope))
        .filter(|element| element.external_id == external_key(&map.source_namespace, requested))
        .collect::<Vec<_>>();
    existing_external.sort_by_key(|element| element.id.to_string());
    existing_external.dedup_by_key(|element| element.id);
    let mut pending_external = planned.iter()
        .filter(|element| match map.search_scope {
            SpreadsheetSearchScope::TargetOnly => element.depth_from_target == 1,
            SpreadsheetSearchScope::TargetRecursive => element.depth_from_target >= 1,
        })
        .filter(|element| element.external_id == requested)
        .collect::<Vec<_>>();
    pending_external.sort_by(|left, right| left.external_id.cmp(&right.external_id));
    pending_external.dedup_by(|left, right| left.external_id == right.external_id);
    match (existing_external.as_slice(), pending_external.as_slice()) {
        ([element], []) => {
            let qualified_name = project.qualified_name(element.id).unwrap_or_else(|_| element.name.clone());
            return Ok(ResolvedOwner {
                reference: BuildReference::Existing(element.id),
                qualified_name,
                kind: element.kind.clone(),
                depth_from_target: distance_from_target(project, element.id, map.target_scope).unwrap_or(0),
            });
        }
        ([], [element]) => return Ok(ResolvedOwner {
            reference: BuildReference::External(element.external_id.clone()),
            qualified_name: element.qualified_name.clone(),
            kind: element.kind.clone(),
            depth_from_target: element.depth_from_target,
        }),
        ([], []) => {}
        _ => return Err(diagnostic(
            Some(map), None, mapped_column_name(map, property), Some(property), Some(requested.into()),
            if property == SpreadsheetSemanticProperty::Owner { "OWNER_AMBIGUOUS" } else { "TYPE_AMBIGUOUS" },
            format!("{label} '{requested}' resolves to more than one element by source external identity"),
        )),
    }

    let mut existing = project.elements.values()
        .filter(|element| reference_in_scope(project, element.id, map.target_scope, map.search_scope))
        .filter_map(|element| {
            let canonical = project.qualified_name(element.id).ok()?;
            qname_aliases(&canonical, root_name, &target_qname).iter()
                .any(|alias| alias == requested)
                .then_some((element, canonical))
        })
        .collect::<Vec<_>>();
    existing.sort_by_key(|(element, _)| element.id.to_string());
    existing.dedup_by_key(|(element, _)| element.id);
    let mut pending = planned.iter()
        .filter(|element| match map.search_scope {
            SpreadsheetSearchScope::TargetOnly => element.depth_from_target == 1,
            SpreadsheetSearchScope::TargetRecursive => element.depth_from_target >= 1,
        })
        .filter(|element| qname_aliases(&element.qualified_name, root_name, &target_qname).iter()
            .any(|alias| alias == requested))
        .collect::<Vec<_>>();
    pending.sort_by(|left, right| left.external_id.cmp(&right.external_id));
    pending.dedup_by(|left, right| left.external_id == right.external_id);
    match (existing.as_slice(), pending.as_slice()) {
        ([(element, qualified_name)], []) => Ok(ResolvedOwner {
            reference: BuildReference::Existing(element.id),
            qualified_name: qualified_name.clone(),
            kind: element.kind.clone(),
            depth_from_target: distance_from_target(project, element.id, map.target_scope).unwrap_or(0),
        }),
        ([], [element]) => Ok(ResolvedOwner {
            reference: BuildReference::External(element.external_id.clone()),
            qualified_name: element.qualified_name.clone(),
            kind: element.kind.clone(),
            depth_from_target: element.depth_from_target,
        }),
        ([], []) => Err(diagnostic(
            Some(map), None, mapped_column_name(map, property), Some(property), Some(requested.into()),
            if property == SpreadsheetSemanticProperty::Owner { "OWNER_UNRESOLVED" } else { "TYPE_UNRESOLVED" },
            format!("{label} '{requested}' could not be resolved by namespaced External ID or exact qualified name within {:?} search scope", map.search_scope),
        )),
        _ => Err(diagnostic(
            Some(map), None, mapped_column_name(map, property), Some(property), Some(requested.into()),
            if property == SpreadsheetSemanticProperty::Owner { "OWNER_AMBIGUOUS" } else { "TYPE_AMBIGUOUS" },
            format!("{label} '{requested}' resolves to more than one semantic element"),
        )),
    }
}

fn resolve_owner(
    map: &SpreadsheetImportMap,
    project: &Project,
    planned: &[PlannedElement],
    value: Option<&str>,
) -> Result<ResolvedOwner, SpreadsheetImportDiagnostic> {
    let target = project.element(map.target_scope).expect("validated target");
    let target_qname = project.qualified_name(map.target_scope).map_err(|error| {
        diagnostic(Some(map), None, None, Some(SpreadsheetSemanticProperty::Owner), None, "TARGET_SCOPE_UNRESOLVED", error.to_string())
    })?;
    let Some(requested) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(ResolvedOwner {
            reference: BuildReference::Existing(map.target_scope),
            qualified_name: target_qname,
            kind: target.kind.clone(),
            depth_from_target: 0,
        });
    };
    resolve_semantic_reference(map, project, planned, requested, SpreadsheetSemanticProperty::Owner, "Owner")
}

fn resolve_type(
    map: &SpreadsheetImportMap,
    project: &Project,
    planned: &[PlannedElement],
    value: Option<&str>,
) -> Result<ResolvedOwner, SpreadsheetImportDiagnostic> {
    let requested = value.map(str::trim).filter(|value| !value.is_empty()).ok_or_else(|| {
        diagnostic(
            Some(map), None, mapped_column_name(map, SpreadsheetSemanticProperty::Type),
            Some(SpreadsheetSemanticProperty::Type), None, "TYPE_REQUIRED",
            format!("{:?} requires a non-empty Type", map.element_kind),
        )
    })?;
    resolve_semantic_reference(map, project, planned, requested, SpreadsheetSemanticProperty::Type, "Type")
}

''',
)
replace_between(
    spreadsheet,
    "fn mapped_field_changes(",
    "fn prepare_spreadsheet_import(",
    '''fn mapped_field_changes(
    map: &SpreadsheetImportMap,
    row: usize,
    element: &Element,
    owner: &ResolvedOwner,
    type_ref: Option<ElementReference>,
    multiplicity: Option<Multiplicity>,
    flow_direction: Option<FlowDirection>,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
) -> Result<(bool, ModelBuildOperation), SpreadsheetImportDiagnostic> {
    let name = values.get(&SpreadsheetSemanticProperty::Name).cloned();
    let documentation = values.get(&SpreadsheetSemanticProperty::Documentation).cloned();
    let external_id = values.get(&SpreadsheetSemanticProperty::ExternalId).cloned().filter(|value| !value.is_empty());
    let requirement_id = values.get(&SpreadsheetSemanticProperty::RequirementId).cloned();
    let requirement_text = values.get(&SpreadsheetSemanticProperty::RequirementText).cloned();
    let visibility = values.get(&SpreadsheetSemanticProperty::Visibility)
        .map(|value| parse_visibility(map, row, value)).transpose()?;
    let owner_mapped = values.contains_key(&SpreadsheetSemanticProperty::Owner);
    let default_value = values.get(&SpreadsheetSemanticProperty::DefaultValue).cloned();
    let owner_changed = owner_mapped && match owner.reference {
        BuildReference::Existing(id) => element.owner_id != Some(id),
        BuildReference::External(_) => true,
    };
    let type_changed = match &type_ref {
        Some(BuildReference::Existing(id)) => element.type_id != Some(*id),
        Some(BuildReference::External(_)) => true,
        None => false,
    };
    let default_changed = default_value.as_ref().is_some_and(|value| {
        let normalized = (!value.trim().is_empty()).then_some(value.as_str());
        element.default_value.as_deref() != normalized
    });
    let changed = name.as_ref().is_some_and(|value| element.name != *value)
        || documentation.as_ref().is_some_and(|value| element.documentation != *value)
        || external_id.as_ref().is_some_and(|value| element.external_id != external_key(&map.source_namespace, value))
        || visibility.is_some_and(|value| element.visibility != value)
        || requirement_id.as_ref().is_some_and(|value| element.requirement_id.as_deref() != Some(value.as_str()))
        || requirement_text.as_ref().is_some_and(|value| element.requirement_text.as_deref() != Some(value.as_str()))
        || multiplicity.is_some_and(|value| element.multiplicity != Some(value))
        || flow_direction.is_some_and(|value| element.flow_direction != Some(value))
        || default_changed || owner_changed || type_changed;
    Ok((changed, ModelBuildOperation::UpdateElementFields {
        element: BuildReference::Existing(element.id),
        name,
        owner: owner_mapped.then(|| owner.reference.clone()),
        type_ref,
        external_id,
        documentation,
        visibility,
        requirement_id,
        requirement_text,
        multiplicity,
        default_value,
        flow_direction,
    }))
}

''',
)
replace_once(
    spreadsheet,
    '''            if !is_namespace_kind(&owner.kind) {
                block_row(diagnostic(
                    Some(map),
                    Some(row.row_number),
                    mapped_column_name(map, SpreadsheetSemanticProperty::Owner),
                    Some(SpreadsheetSemanticProperty::Owner),
                    id_value.clone(),
                    "INVALID_OWNERSHIP",
                    format!("{:?} cannot be owned by {:?} in the PR38 packageable-element scope", map.element_kind, owner.kind),
                ));
                continue;
            }

            let existing = match find_existing(map, project, &values) {
''',
    '''            let type_resolution = if is_feature_kind(&map.element_kind) {
                match resolve_type(map, project, &planned, non_empty_value(&values, SpreadsheetSemanticProperty::Type)) {
                    Ok(value) => Some(value),
                    Err(error) => { block_row(error); continue; }
                }
            } else { None };
            let multiplicity = match non_empty_value(&values, SpreadsheetSemanticProperty::Multiplicity) {
                Some(value) => match super::parametrics::parse_multiplicity(value) {
                    Ok(value) => Some(value),
                    Err(reason) => {
                        block_row(diagnostic(
                            Some(map), Some(row.row_number),
                            mapped_column_name(map, SpreadsheetSemanticProperty::Multiplicity),
                            Some(SpreadsheetSemanticProperty::Multiplicity), id_value.clone(),
                            "MULTIPLICITY_INVALID",
                            format!("feature '{}' has invalid multiplicity '{}': {}", non_empty_value(&values, SpreadsheetSemanticProperty::Name).unwrap_or("<unnamed>"), value, reason),
                        ));
                        continue;
                    }
                },
                None => None,
            };
            let flow_direction = if map.element_kind == ElementKind::FlowProperty {
                let Some(value) = non_empty_value(&values, SpreadsheetSemanticProperty::FlowDirection) else {
                    block_row(diagnostic(
                        Some(map), Some(row.row_number),
                        mapped_column_name(map, SpreadsheetSemanticProperty::FlowDirection),
                        Some(SpreadsheetSemanticProperty::FlowDirection), id_value.clone(),
                        "FLOW_DIRECTION_INVALID",
                        "FlowProperty direction is blank; expected in, out, or inout",
                    ));
                    continue;
                };
                match super::feature_editing::parse_flow_direction(&value.trim().to_ascii_lowercase()) {
                    Ok(value) => Some(value),
                    Err(_) => {
                        block_row(diagnostic(
                            Some(map), Some(row.row_number),
                            mapped_column_name(map, SpreadsheetSemanticProperty::FlowDirection),
                            Some(SpreadsheetSemanticProperty::FlowDirection), id_value.clone(),
                            "FLOW_DIRECTION_INVALID",
                            format!("FlowProperty '{}' direction '{}' is invalid; expected in, out, or inout", non_empty_value(&values, SpreadsheetSemanticProperty::Name).unwrap_or("<unnamed>"), value),
                        ));
                        continue;
                    }
                }
            } else { None };

            let existing = match find_existing(map, project, &values) {
''',
)
replace_once(
    spreadsheet,
    "match mapped_field_changes(map, existing, &owner, &values) {",
    '''match mapped_field_changes(
                    map,
                    row.row_number,
                    existing,
                    &owner,
                    type_resolution.as_ref().map(|value| value.reference.clone()),
                    multiplicity,
                    flow_direction,
                    &values,
                ) {''',
)
replace_once(
    spreadsheet,
    '''                type_ref: None,
            });''',
    '''                type_ref: type_resolution.as_ref().map(|value| value.reference.clone()),
            });''',
)
replace_once(
    spreadsheet,
    '''            let requirement_text = values
                .get(&SpreadsheetSemanticProperty::RequirementText)
                .cloned();
            if documentation.is_some()
                || visibility.is_some()
                || requirement_id.is_some()
                || requirement_text.is_some()
            {
                operations.push(ModelBuildOperation::UpdateElementFields {
                    element: BuildReference::External(external_id.to_string()),
                    name: None,
                    owner: None,
                    external_id: None,
                    documentation,
                    visibility,
                    requirement_id,
                    requirement_text,
                });''',
    '''            let requirement_text = values
                .get(&SpreadsheetSemanticProperty::RequirementText)
                .cloned();
            let default_value = values.get(&SpreadsheetSemanticProperty::DefaultValue).cloned();
            if documentation.is_some()
                || visibility.is_some()
                || requirement_id.is_some()
                || requirement_text.is_some()
                || multiplicity.is_some()
                || default_value.is_some()
                || flow_direction.is_some()
            {
                operations.push(ModelBuildOperation::UpdateElementFields {
                    element: BuildReference::External(external_id.to_string()),
                    name: None,
                    owner: None,
                    type_ref: None,
                    external_id: None,
                    documentation,
                    visibility,
                    requirement_id,
                    requirement_text,
                    multiplicity,
                    default_value,
                    flow_direction,
                });''',
)

main = ROOT / "apps/desktop/src-tauri/src/main.rs"
replace_once(main, "    mod portable_interchange;\n    mod presentation_interaction;\n", "    mod portable_interchange;\n    mod spreadsheet_import;\n    mod presentation_interaction;\n")
replace_once(main, "    pub use portable_interchange::{export_portable_project_json, import_portable_project_json};\n", "    pub use portable_interchange::{export_portable_project_json, import_portable_project_json};\n    pub use spreadsheet_import::{apply_spreadsheet_import, preview_spreadsheet_import};\n")
replace_once(main, "    export_portable_project_json, fit_diagram_viewport, get_diagram_frame_preference,\n", "    apply_spreadsheet_import, export_portable_project_json, fit_diagram_viewport,\n    get_diagram_frame_preference,\n")
replace_once(main, "    history_reset, history_undo, ibd_item_flow_notation, import_portable_project_json,\n", "    history_reset, history_undo, ibd_item_flow_notation, import_portable_project_json,\n    preview_spreadsheet_import,\n")
replace_once(main, "            export_portable_project_json,\n            import_portable_project_json,\n", "            export_portable_project_json,\n            import_portable_project_json,\n            preview_spreadsheet_import,\n            apply_spreadsheet_import,\n")

cargo = ROOT / "apps/desktop/src-tauri/Cargo.toml"
replace_once(cargo, 'systems-modeler-persistence = { path = "../../../crates/persistence" }\n', 'systems-modeler-persistence = { path = "../../../crates/persistence" }\ncalamine = "0.36.1"\ncsv = "1.4.0"\n')

print("PR39 core patch applied")
