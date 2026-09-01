from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED
from xml.sax.saxutils import escape


def replace_once(path, old, new):
    p = Path(path)
    text = p.read_text()
    if old not in text:
        raise SystemExit(f"missing anchor in {path}: {old[:120]!r}")
    p.write_text(text.replace(old, new, 1))

# -----------------------------------------------------------------------------
# PR36 candidate build: carry the already-native PR47 semantic fields through
# one atomic ModelBuildPlan rather than introducing importer mutation paths.
# -----------------------------------------------------------------------------
bulk = "apps/desktop/src-tauri/src/workspace/bulk_model.rs"
replace_once(
    bulk,
    "        is_conjugated: Option<bool>,\n    },\n    CreateRelationship {",
    "        is_conjugated: Option<bool>,\n        extension_points: Option<Vec<String>>,\n    },\n    CreateRelationship {",
)
replace_once(
    bulk,
    "        visibility: Option<VisibilityKind>,\n        source_end: Option<AssociationEndBuildFields>,\n        target_end: Option<AssociationEndBuildFields>,\n    },\n    CreateDiagram {",
    "        visibility: Option<VisibilityKind>,\n        source_end: Option<AssociationEndBuildFields>,\n        target_end: Option<AssociationEndBuildFields>,\n        alias: Option<Option<String>>,\n        extension_condition: Option<Option<String>>,\n        extension_location: Option<Option<String>>,\n    },\n    CreateDiagram {",
)
replace_once(
    bulk,
    "                    flow_direction,\n                    is_conjugated,\n                } => {",
    "                    flow_direction,\n                    is_conjugated,\n                    extension_points,\n                } => {",
)
replace_once(
    bulk,
    "                    if requirement_id.is_some() || requirement_text.is_some() {",
    "                    if let Some(extension_points) = extension_points {\n                        if project\n                            .element(id)\n                            .map_err(|cause| {\n                                error(\"SEMANTIC_VALIDATION\", Some(index), cause.to_string())\n                            })?\n                            .kind\n                            != ElementKind::UseCase\n                        {\n                            return Err(error(\n                                \"SEMANTIC_VALIDATION\",\n                                Some(index),\n                                \"Extension Points mapping is valid only for UseCase elements\",\n                            ));\n                        }\n                        project\n                            .element_mut(id)\n                            .map_err(|cause| {\n                                error(\"SEMANTIC_VALIDATION\", Some(index), cause.to_string())\n                            })?\n                            .extension_points = extension_points.clone();\n                    }\n                    if requirement_id.is_some() || requirement_text.is_some() {",
)
replace_once(
    bulk,
    "                    visibility,\n                    source_end,\n                    target_end,\n                } => {",
    "                    visibility,\n                    source_end,\n                    target_end,\n                    alias,\n                    extension_condition,\n                    extension_location,\n                } => {",
)
# Validate the next PR47-specific relationship fields on the same candidate clone.
replace_once(
    bulk,
    "                    let next_external_id = external_id\n                        .as_ref()\n                        .map(|value| external_key(namespace, value));",
    "                    if let Some(value) = alias {\n                        validation_project.relationships.get_mut(&id).map(|_| ());\n                        let candidate = validation_project\n                            .relationships\n                            .values_mut()\n                            .find(|relationship| {\n                                relationship.kind == kind\n                                    && relationship.source_id == next_source\n                                    && relationship.target_id == next_target\n                                    && relationship.owner_id == next_owner\n                            })\n                            .expect(\"validated replacement relationship\");\n                        candidate.alias = value.clone();\n                    }\n                    if let Some(value) = extension_condition {\n                        let candidate = validation_project\n                            .relationships\n                            .values_mut()\n                            .find(|relationship| {\n                                relationship.kind == kind\n                                    && relationship.source_id == next_source\n                                    && relationship.target_id == next_target\n                                    && relationship.owner_id == next_owner\n                            })\n                            .expect(\"validated replacement relationship\");\n                        candidate.extension_condition = value.clone();\n                    }\n                    if let Some(value) = extension_location {\n                        let candidate = validation_project\n                            .relationships\n                            .values_mut()\n                            .find(|relationship| {\n                                relationship.kind == kind\n                                    && relationship.source_id == next_source\n                                    && relationship.target_id == next_target\n                                    && relationship.owner_id == next_owner\n                            })\n                            .expect(\"validated replacement relationship\");\n                        candidate.extension_location = value.clone();\n                    }\n                    validation_project.validate().map_err(|cause| {\n                        error(\"SEMANTIC_VALIDATION\", Some(index), cause.to_string())\n                    })?;\n\n                    let next_external_id = external_id\n                        .as_ref()\n                        .map(|value| external_key(namespace, value));",
)
replace_once(
    bulk,
    "                    if let Some(ends) = next_association_ends.take() {\n                        relationship.association_ends = ends;\n                    }\n                    project.validate().map_err(|cause| {",
    "                    if let Some(ends) = next_association_ends.take() {\n                        relationship.association_ends = ends;\n                    }\n                    if let Some(value) = alias {\n                        relationship.alias = value.clone();\n                    }\n                    if let Some(value) = extension_condition {\n                        relationship.extension_condition = value.clone();\n                    }\n                    if let Some(value) = extension_location {\n                        relationship.extension_location = value.clone();\n                    }\n                    project.validate().map_err(|cause| {",
)

# -----------------------------------------------------------------------------
# Spreadsheet mapping layer.
# -----------------------------------------------------------------------------
spread = "apps/desktop/src-tauri/src/workspace/spreadsheet_import.rs"
replace_once(
    spread,
    "    RequirementText,\n    RelationshipKind,",
    "    RequirementText,\n    ExtensionPoints,\n    RelationshipKind,\n    Alias,\n    ExtensionCondition,\n    ExtensionLocation,",
)
replace_once(
    spread,
    "            | RelationshipKind::Connector\n            | RelationshipKind::ItemFlow\n    )",
    "            | RelationshipKind::Connector\n            | RelationshipKind::ItemFlow\n            | RelationshipKind::Include\n            | RelationshipKind::Extend\n            | RelationshipKind::PackageImport\n            | RelationshipKind::ElementImport\n            | RelationshipKind::PackageMerge\n    )",
)
replace_once(
    spread,
    "        \"connector\" => RelationshipKind::Connector,\n        \"itemflow\" | \"item flow\" => RelationshipKind::ItemFlow,",
    "        \"connector\" => RelationshipKind::Connector,\n        \"itemflow\" | \"item flow\" => RelationshipKind::ItemFlow,\n        \"include\" => RelationshipKind::Include,\n        \"extend\" => RelationshipKind::Extend,\n        \"packageimport\" | \"package import\" => RelationshipKind::PackageImport,\n        \"elementimport\" | \"element import\" => RelationshipKind::ElementImport,\n        \"packagemerge\" | \"package merge\" => RelationshipKind::PackageMerge,",
)
replace_once(
    spread,
    "relationship kind '{}' is unsupported; expected Association, Generalization, Dependency, Realization, Allocate, DeriveRequirement/deriveReqt, Satisfy, Verify, Refine, Trace, Copy, Connector, or ItemFlow",
    "relationship kind '{}' is unsupported; expected Association, Generalization, Dependency, Realization, Allocate, DeriveRequirement/deriveReqt, Satisfy, Verify, Refine, Trace, Copy, Connector, ItemFlow, Include, Extend, PackageImport, ElementImport, or PackageMerge",
)
replace_once(
    spread,
    "                    \"{:?} is outside the PR40/PR41/PR42 relationship scope\",",
    "                    \"{:?} is outside the PR40-PR47 relationship scope\",",
)
# Relationship-map validation: PR47 field ownership and no element-only ExtensionPoints.
replace_once(
    spread,
    "            SpreadsheetSemanticProperty::RequirementText,\n        ]",
    "            SpreadsheetSemanticProperty::RequirementText,\n            SpreadsheetSemanticProperty::ExtensionPoints,\n        ]",
)
anchor = "        let association_fields = [\n"
insert = "        let fixed_kind = map.relationship_kind.as_ref();\n        if has_property(SpreadsheetSemanticProperty::Alias)\n            && fixed_kind.is_some_and(|kind| *kind != RelationshipKind::ElementImport)\n        {\n            return Err(diagnostic(\n                Some(map), None, None, Some(SpreadsheetSemanticProperty::Alias), None,\n                \"ELEMENT_IMPORT_ALIAS_FIELD_INVALID\",\n                \"Alias can be mapped only for ElementImport rows\",\n            ));\n        }\n        if (has_property(SpreadsheetSemanticProperty::ExtensionCondition)\n            || has_property(SpreadsheetSemanticProperty::ExtensionLocation))\n            && fixed_kind.is_some_and(|kind| *kind != RelationshipKind::Extend)\n        {\n            return Err(diagnostic(\n                Some(map), None, None, None, None,\n                \"EXTEND_FIELD_INVALID\",\n                \"Extension Condition and Extension Location can be mapped only for Extend rows\",\n            ));\n        }\n"
replace_once(spread, anchor, insert + anchor)
# Element maps must reject relationship-only PR47 fields and constrain ExtensionPoints to UseCase.
replace_once(
    spread,
    "    if map.element_kind != ElementKind::Requirement\n        && (has_property(SpreadsheetSemanticProperty::RequirementId)",
    "    if [\n        SpreadsheetSemanticProperty::Alias,\n        SpreadsheetSemanticProperty::ExtensionCondition,\n        SpreadsheetSemanticProperty::ExtensionLocation,\n    ]\n    .into_iter()\n    .any(has_property)\n    {\n        return Err(diagnostic(\n            Some(map), None, None, None, None,\n            \"SEMANTIC_PROPERTY_INVALID\",\n            \"relationship-only PR47 fields cannot be used by element mappings\",\n        ));\n    }\n    if map.element_kind != ElementKind::UseCase\n        && has_property(SpreadsheetSemanticProperty::ExtensionPoints)\n    {\n        return Err(diagnostic(\n            Some(map), None, None, Some(SpreadsheetSemanticProperty::ExtensionPoints), None,\n            \"SEMANTIC_PROPERTY_INVALID\",\n            \"Extension Points can be mapped only for UseCase elements\",\n        ));\n    }\n    if map.element_kind != ElementKind::Requirement\n        && (has_property(SpreadsheetSemanticProperty::RequirementId)",
)
# Parse authored extension-point cells without an expression engine or auto-creation.
replace_once(
    spread,
    "fn parse_visibility(\n",
    "fn parse_extension_points(value: &str) -> Vec<String> {\n    value\n        .replace(\"\\r\\n\", \"\\n\")\n        .split(['\\n', ';'])\n        .map(str::trim)\n        .filter(|value| !value.is_empty())\n        .map(ToOwned::to_owned)\n        .collect()\n}\n\nfn normalized_optional_text(value: &str) -> Option<String> {\n    (!value.trim().is_empty()).then(|| value.trim().to_string())\n}\n\nfn valid_element_import_alias(value: &str) -> bool {\n    let value = value.trim();\n    if value.is_empty() {\n        return true;\n    }\n    let mut chars = value.chars();\n    let Some(first) = chars.next() else { return true; };\n    (first == '_' || first.is_alphabetic())\n        && chars.all(|character| character == '_' || character.is_alphanumeric())\n}\n\nfn parse_visibility(\n",
)
# Relationship field change plumbing.
replace_once(
    spread,
    "    let external_id_explicit =\n        non_empty_value(values, SpreadsheetSemanticProperty::ExternalId).is_some();",
    "    let alias = values\n        .get(&SpreadsheetSemanticProperty::Alias)\n        .map(|value| normalized_optional_text(value));\n    let extension_condition = values\n        .get(&SpreadsheetSemanticProperty::ExtensionCondition)\n        .map(|value| normalized_optional_text(value));\n    let extension_location = values\n        .get(&SpreadsheetSemanticProperty::ExtensionLocation)\n        .map(|value| normalized_optional_text(value));\n    let external_id_explicit =\n        non_empty_value(values, SpreadsheetSemanticProperty::ExternalId).is_some();",
)
replace_once(
    spread,
    "        || visibility.is_some_and(|value| relationship.visibility != value)\n        || external_changed;",
    "        || visibility.is_some_and(|value| relationship.visibility != value)\n        || alias.as_ref().is_some_and(|value| relationship.alias != *value)\n        || extension_condition\n            .as_ref()\n            .is_some_and(|value| relationship.extension_condition != *value)\n        || extension_location\n            .as_ref()\n            .is_some_and(|value| relationship.extension_location != *value)\n        || external_changed;",
)
replace_once(
    spread,
    "            source_end,\n            target_end,\n        },",
    "            source_end,\n            target_end,\n            alias,\n            extension_condition,\n            extension_location,\n        },",
)
# Row-level PR47 semantic field validation and native package owner semantics.
replace_once(
    spread,
    "    if kind == RelationshipKind::Allocate && source.reference == target.reference {",
    "    if matches!(kind, RelationshipKind::Include | RelationshipKind::Extend) {\n        if source.kind != ElementKind::UseCase || target.kind != ElementKind::UseCase {\n            return Err(diagnostic(\n                Some(map), Some(row), None, None,\n                non_empty_value(values, SpreadsheetSemanticProperty::ExternalId).map(ToOwned::to_owned),\n                \"USE_CASE_RELATIONSHIP_ENDPOINT_KIND_INVALID\",\n                format!(\"{:?} requires UseCase -> UseCase; resolved {:?} -> {:?}\", kind, source.kind, target.kind),\n            ));\n        }\n        if source.reference == target.reference {\n            return Err(diagnostic(\n                Some(map), Some(row), None, None,\n                non_empty_value(values, SpreadsheetSemanticProperty::ExternalId).map(ToOwned::to_owned),\n                \"USE_CASE_RELATIONSHIP_SELF_REFERENCE\",\n                format!(\"{:?} cannot connect a UseCase to itself\", kind),\n            ));\n        }\n    }\n    if kind == RelationshipKind::PackageImport\n        && (!is_namespace_kind(&source.kind) || !is_namespace_kind(&target.kind))\n    {\n        return Err(diagnostic(\n            Some(map), Some(row), None, None, None,\n            \"PACKAGE_IMPORT_ENDPOINT_KIND_INVALID\",\n            format!(\"PackageImport requires namespace -> namespace; resolved {:?} -> {:?}\", source.kind, target.kind),\n        ));\n    }\n    if kind == RelationshipKind::ElementImport && !is_namespace_kind(&source.kind) {\n        return Err(diagnostic(\n            Some(map), Some(row), None, None, None,\n            \"ELEMENT_IMPORT_SOURCE_KIND_INVALID\",\n            format!(\"ElementImport source must be a namespace; resolved {:?}\", source.kind),\n        ));\n    }\n    if kind == RelationshipKind::PackageMerge\n        && (!matches!(source.kind, ElementKind::Package | ElementKind::ModelLibrary)\n            || !matches!(target.kind, ElementKind::Package | ElementKind::ModelLibrary))\n    {\n        return Err(diagnostic(\n            Some(map), Some(row), None, None, None,\n            \"PACKAGE_MERGE_ENDPOINT_KIND_INVALID\",\n            format!(\"PackageMerge requires Package/ModelLibrary endpoints; resolved {:?} -> {:?}\", source.kind, target.kind),\n        ));\n    }\n    if let Some(alias) = values.get(&SpreadsheetSemanticProperty::Alias) {\n        if kind != RelationshipKind::ElementImport && !alias.trim().is_empty() {\n            return Err(diagnostic(\n                Some(map), Some(row), mapped_column_name(map, SpreadsheetSemanticProperty::Alias),\n                Some(SpreadsheetSemanticProperty::Alias), Some(alias.clone()),\n                \"ELEMENT_IMPORT_ALIAS_FIELD_INVALID\", \"Alias is valid only for ElementImport\",\n            ));\n        }\n        if kind == RelationshipKind::ElementImport && !valid_element_import_alias(alias) {\n            return Err(diagnostic(\n                Some(map), Some(row), mapped_column_name(map, SpreadsheetSemanticProperty::Alias),\n                Some(SpreadsheetSemanticProperty::Alias), Some(alias.clone()),\n                \"ELEMENT_IMPORT_ALIAS_INVALID\",\n                format!(\"ElementImport alias '{}' is not a valid identifier\", alias.trim()),\n            ));\n        }\n    }\n    if kind != RelationshipKind::Extend\n        && [SpreadsheetSemanticProperty::ExtensionCondition, SpreadsheetSemanticProperty::ExtensionLocation]\n            .into_iter()\n            .any(|property| non_empty_value(values, property).is_some())\n    {\n        return Err(diagnostic(\n            Some(map), Some(row), None, None, None,\n            \"EXTEND_FIELD_INVALID\",\n            \"Extension Condition/Location are valid only for Extend\",\n        ));\n    }\n    if kind == RelationshipKind::Allocate && source.reference == target.reference {",
)
old_owner = "    let owner = if let Some(owner_text) =\n        non_empty_value(values, SpreadsheetSemanticProperty::Owner)\n    {\n        resolve_owner(map, project, planned, Some(owner_text))?\n    } else if is_pr41_traceability_kind(&kind) || kind == RelationshipKind::Allocate {"
new_owner = "    let package_namespace_relationship = matches!(\n        kind,\n        RelationshipKind::PackageImport | RelationshipKind::ElementImport | RelationshipKind::PackageMerge\n    );\n    let owner = if package_namespace_relationship {\n        if let Some(owner_text) = non_empty_value(values, SpreadsheetSemanticProperty::Owner) {\n            let explicit = resolve_owner(map, project, planned, Some(owner_text))?;\n            if explicit.reference != source.reference {\n                return Err(diagnostic(\n                    Some(map), Some(row), mapped_column_name(map, SpreadsheetSemanticProperty::Owner),\n                    Some(SpreadsheetSemanticProperty::Owner), Some(owner_text.to_string()),\n                    \"NAMESPACE_RELATIONSHIP_OWNER_INVALID\",\n                    \"PackageImport, ElementImport, and PackageMerge are owned by their importing/receiving source namespace\",\n                ));\n            }\n        }\n        source.clone()\n    } else if let Some(owner_text) =\n        non_empty_value(values, SpreadsheetSemanticProperty::Owner)\n    {\n        resolve_owner(map, project, planned, Some(owner_text))?\n    } else if is_pr41_traceability_kind(&kind) || kind == RelationshipKind::Allocate {"
replace_once(spread, old_owner, new_owner)
# New relationship extra metadata update.
replace_once(
    spread,
    "    let visibility = values\n        .get(&SpreadsheetSemanticProperty::Visibility)\n        .map(|value| parse_visibility(map, row, value))\n        .transpose()?;\n    if name.is_some()\n        || documentation.is_some()\n        || visibility.is_some()\n        || source_end.is_some()\n        || target_end.is_some()\n    {",
    "    let visibility = values\n        .get(&SpreadsheetSemanticProperty::Visibility)\n        .map(|value| parse_visibility(map, row, value))\n        .transpose()?;\n    let alias = values\n        .get(&SpreadsheetSemanticProperty::Alias)\n        .map(|value| normalized_optional_text(value));\n    let extension_condition = values\n        .get(&SpreadsheetSemanticProperty::ExtensionCondition)\n        .map(|value| normalized_optional_text(value));\n    let extension_location = values\n        .get(&SpreadsheetSemanticProperty::ExtensionLocation)\n        .map(|value| normalized_optional_text(value));\n    if name.is_some()\n        || documentation.is_some()\n        || visibility.is_some()\n        || source_end.is_some()\n        || target_end.is_some()\n        || alias.is_some()\n        || extension_condition.is_some()\n        || extension_location.is_some()\n    {",
)
replace_once(
    spread,
    "            source_end,\n            target_end,\n        });\n    }\n    Ok(RelationshipRowPlan {",
    "            source_end,\n            target_end,\n            alias,\n            extension_condition,\n            extension_location,\n        });\n    }\n    Ok(RelationshipRowPlan {",
)
# Extension-point element update plumbing.
replace_once(
    spread,
    "    is_conjugated: Option<bool>,\n    values: &BTreeMap<SpreadsheetSemanticProperty, String>,",
    "    is_conjugated: Option<bool>,\n    extension_points: Option<Vec<String>>,\n    values: &BTreeMap<SpreadsheetSemanticProperty, String>,",
)
replace_once(
    spread,
    "        || is_conjugated.is_some_and(|value| element.is_conjugated != value)\n        || default_changed",
    "        || is_conjugated.is_some_and(|value| element.is_conjugated != value)\n        || extension_points\n            .as_ref()\n            .is_some_and(|value| element.extension_points != *value)\n        || default_changed",
)
# The first construction in mapped_field_changes.
replace_once(
    spread,
    "            flow_direction,\n            is_conjugated,\n        },\n    ))\n}",
    "            flow_direction,\n            is_conjugated,\n            extension_points,\n        },\n    ))\n}",
)
# Parse extension points before existing-match update path and pass into mapped_field_changes.
replace_once(
    spread,
    "            if let Some(existing) = existing {\n                match mapped_field_changes(",
    "            let extension_points = values\n                .get(&SpreadsheetSemanticProperty::ExtensionPoints)\n                .map(|value| parse_extension_points(value));\n\n            if let Some(existing) = existing {\n                match mapped_field_changes(",
)
replace_once(
    spread,
    "                    flow_direction,\n                    is_conjugated,\n                    &values,",
    "                    flow_direction,\n                    is_conjugated,\n                    extension_points.clone(),\n                    &values,",
)
# New element update list/construction.
replace_once(
    spread,
    "                || flow_direction.is_some()\n                || is_conjugated.is_some()\n            {",
    "                || flow_direction.is_some()\n                || is_conjugated.is_some()\n                || extension_points.is_some()\n            {",
)
replace_once(
    spread,
    "                    flow_direction,\n                    is_conjugated,\n                });\n                operation_contexts.push(context);",
    "                    flow_direction,\n                    is_conjugated,\n                    extension_points,\n                });\n                operation_contexts.push(context);",
)
# Register PR47 focused test module.
replace_once(
    spread,
    "#[cfg(test)]\nmod pr46_tests;",
    "#[cfg(test)]\nmod pr46_tests;\n#[cfg(test)]\nmod pr47_tests;",
)

# Portable test registration only; generic PR37 schema already carries these fields.
portable = "apps/desktop/src-tauri/src/workspace/portable_interchange.rs"
replace_once(
    portable,
    "#[cfg(test)]\nmod pr46_operation_parameter_reception_tests;",
    "#[cfg(test)]\nmod pr46_operation_parameter_reception_tests;\n#[cfg(test)]\nmod pr47_core_namespace_relationship_tests;",
)

# -----------------------------------------------------------------------------
# Focused desktop spreadsheet tests.
# -----------------------------------------------------------------------------
pr47_tests = r'''use super::*;
use std::fs;
use std::path::PathBuf;

const NS: &str = "catia:pr47";

fn workspace(name: &str) -> (WorkspaceState, ElementId) {
    let state = WorkspaceState::default();
    let project = Project::new(name);
    let root = project.root_id;
    *state.project.lock().unwrap() = Some(project);
    (state, root)
}

fn fixture_path() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pr47_core_namespace_relationships.xlsx")
        .to_string_lossy()
        .into_owned()
}

fn temp_csv(prefix: &str, body: &str) -> String {
    let path = std::env::temp_dir().join(format!("pr47-{prefix}-{}.csv", uuid::Uuid::new_v4()));
    fs::write(&path, body).unwrap();
    path.to_string_lossy().into_owned()
}

fn element_map(
    name: &str,
    source: String,
    sheet: Option<&str>,
    kind: ElementKind,
    root: ElementId,
    columns: &[(&str, SpreadsheetSemanticProperty)],
) -> SpreadsheetImportMap {
    SpreadsheetImportMap {
        name: name.into(), source, worksheet: sheet.map(ToOwned::to_owned), header_row: 1,
        element_kind: kind, relationship_kind: None,
        relationship_identity: SpreadsheetRelationshipIdentityPolicy::ExternalId,
        target_scope: root, identification_property: SpreadsheetIdentificationProperty::ExternalId,
        search_scope: SpreadsheetSearchScope::TargetRecursive, source_namespace: NS.into(),
        mapping_version: "1".into(),
        column_mappings: columns.iter().map(|(c,p)| SpreadsheetColumnMapping { source_column: (*c).into(), property: *p }).collect(),
    }
}

fn relationship_map(
    name: &str,
    source: String,
    sheet: Option<&str>,
    root: ElementId,
    kind: Option<RelationshipKind>,
    columns: &[(&str, SpreadsheetSemanticProperty)],
) -> SpreadsheetImportMap {
    SpreadsheetImportMap {
        name: name.into(), source, worksheet: sheet.map(ToOwned::to_owned), header_row: 1,
        element_kind: ElementKind::Package, relationship_kind: kind,
        relationship_identity: SpreadsheetRelationshipIdentityPolicy::ExternalId,
        target_scope: root, identification_property: SpreadsheetIdentificationProperty::ExternalId,
        search_scope: SpreadsheetSearchScope::TargetRecursive, source_namespace: NS.into(),
        mapping_version: "1".into(),
        column_mappings: columns.iter().map(|(c,p)| SpreadsheetColumnMapping { source_column: (*c).into(), property: *p }).collect(),
    }
}

fn xlsx_group(root: ElementId) -> SpreadsheetImportMapGroup {
    let file = fixture_path();
    let packages = element_map("Packages", file.clone(), Some("Packages"), ElementKind::Package, root, &[
        ("Package ID", SpreadsheetSemanticProperty::ExternalId),
        ("Package Name", SpreadsheetSemanticProperty::Name),
        ("Parent", SpreadsheetSemanticProperty::Owner),
    ]);
    let use_cases = element_map("Use Cases", file.clone(), Some("Use Cases"), ElementKind::UseCase, root, &[
        ("Use Case ID", SpreadsheetSemanticProperty::ExternalId),
        ("Use Case Name", SpreadsheetSemanticProperty::Name),
        ("Package", SpreadsheetSemanticProperty::Owner),
        ("Extension Points", SpreadsheetSemanticProperty::ExtensionPoints),
    ]);
    let signals = element_map("Signals", file.clone(), Some("Imported Elements"), ElementKind::Signal, root, &[
        ("Element ID", SpreadsheetSemanticProperty::ExternalId),
        ("Element Name", SpreadsheetSemanticProperty::Name),
        ("Package", SpreadsheetSemanticProperty::Owner),
    ]);
    let relations = relationship_map("Reuse and Namespace Rules", file, Some("Relationships"), root, None, &[
        ("Relationship ID", SpreadsheetSemanticProperty::ExternalId),
        ("Rule", SpreadsheetSemanticProperty::RelationshipKind),
        ("From", SpreadsheetSemanticProperty::Source),
        ("To", SpreadsheetSemanticProperty::Target),
        ("Owner", SpreadsheetSemanticProperty::Owner),
        ("Alias", SpreadsheetSemanticProperty::Alias),
        ("Visibility", SpreadsheetSemanticProperty::Visibility),
        ("Extension Point", SpreadsheetSemanticProperty::ExtensionLocation),
        ("Condition", SpreadsheetSemanticProperty::ExtensionCondition),
        ("Name", SpreadsheetSemanticProperty::Name),
        ("Description", SpreadsheetSemanticProperty::Documentation),
    ]);
    SpreadsheetImportMapGroup { mappings: vec![packages, use_cases, signals, relations] }
}

fn rel<'a>(project: &'a Project, external: &str) -> &'a Relationship {
    let key = external_key(NS, external);
    project.relationships.values().find(|r| r.external_id == key).unwrap()
}

#[test]
fn pr47_xlsx_constructs_all_five_kinds_plan_locally_and_preview_is_nonmutating() {
    let (state, root) = workspace("PR47 XLSX");
    let group = xlsx_group(root);
    let before = state.project.lock().unwrap().as_ref().unwrap().clone();
    let preview = preview_spreadsheet_import_group(&group, &state);
    assert!(preview.is_valid(), "{:?}", preview.diagnostics);
    assert_eq!(state.project.lock().unwrap().as_ref().unwrap().relationships.len(), 0);
    assert_eq!(state.project.lock().unwrap().as_ref().unwrap().elements.len(), before.elements.len());
    apply_spreadsheet_import_group(&group, &state).unwrap();
    let guard = state.project.lock().unwrap();
    let project = guard.as_ref().unwrap();
    for (id, kind) in [
        ("INC-1", RelationshipKind::Include),
        ("EXT-1", RelationshipKind::Extend),
        ("PI-1", RelationshipKind::PackageImport),
        ("EI-1", RelationshipKind::ElementImport),
        ("PM-1", RelationshipKind::PackageMerge),
    ] { assert_eq!(rel(project, id).kind, kind); }
    let extend = rel(project, "EXT-1");
    assert_eq!(extend.extension_condition.as_deref(), Some("emergency"));
    assert_eq!(extend.extension_location.as_deref(), Some("EmergencyHandling"));
    let element_import = rel(project, "EI-1");
    assert_eq!(element_import.alias.as_deref(), Some("Command"));
    assert_eq!(element_import.visibility, VisibilityKind::Public);
    assert_eq!(element_import.owner_id, Some(element_import.source_id));
    assert_eq!(rel(project, "PI-1").owner_id, Some(rel(project, "PI-1").source_id));
    assert_eq!(rel(project, "PM-1").owner_id, Some(rel(project, "PM-1").source_id));
    project.validate().unwrap();
}

#[test]
fn pr47_csv_include_and_extend_preserve_direction_condition_location_and_qname_resolution() {
    let (state, root) = workspace("PR47 CSV");
    let package = { let mut g=state.project.lock().unwrap(); g.as_mut().unwrap().create_element(ElementKind::Package,"UC",root).unwrap() };
    {
        let mut g = state.project.lock().unwrap(); let p=g.as_mut().unwrap();
        let base=p.create_element(ElementKind::UseCase,"OperateVehicle",package).unwrap();
        p.element_mut(base).unwrap().extension_points=vec!["EmergencyHandling".into()];
        p.create_element(ElementKind::UseCase,"StartVehicle",package).unwrap();
        p.create_element(ElementKind::UseCase,"EmergencyShutdown",package).unwrap();
    }
    let include = temp_csv("include", "ID,From,To,Owner,Name,Description\nINC-C,PR47 CSV::UC::OperateVehicle,PR47 CSV::UC::StartVehicle,PR47 CSV::UC,reuse,doc\n");
    let extend = temp_csv("extend", "ID,From,To,Owner,Extension Point,Condition,Description\nEXT-C,PR47 CSV::UC::EmergencyShutdown,PR47 CSV::UC::OperateVehicle,PR47 CSV::UC,EmergencyHandling,critical,doc\n");
    let group=SpreadsheetImportMapGroup{mappings:vec![
        relationship_map("Include",include,None,root,Some(RelationshipKind::Include),&[
            ("ID",SpreadsheetSemanticProperty::ExternalId),("From",SpreadsheetSemanticProperty::Source),("To",SpreadsheetSemanticProperty::Target),("Owner",SpreadsheetSemanticProperty::Owner),("Name",SpreadsheetSemanticProperty::Name),("Description",SpreadsheetSemanticProperty::Documentation)]),
        relationship_map("Extend",extend,None,root,Some(RelationshipKind::Extend),&[
            ("ID",SpreadsheetSemanticProperty::ExternalId),("From",SpreadsheetSemanticProperty::Source),("To",SpreadsheetSemanticProperty::Target),("Owner",SpreadsheetSemanticProperty::Owner),("Extension Point",SpreadsheetSemanticProperty::ExtensionLocation),("Condition",SpreadsheetSemanticProperty::ExtensionCondition),("Description",SpreadsheetSemanticProperty::Documentation)]),
    ]};
    apply_spreadsheet_import_group(&group,&state).unwrap();
    let g=state.project.lock().unwrap(); let p=g.as_ref().unwrap();
    let inc=rel(p,"INC-C"); assert_eq!(p.element(inc.source_id).unwrap().name,"OperateVehicle"); assert_eq!(p.element(inc.target_id).unwrap().name,"StartVehicle");
    let ext=rel(p,"EXT-C"); assert_eq!(p.element(ext.source_id).unwrap().name,"EmergencyShutdown"); assert_eq!(p.element(ext.target_id).unwrap().name,"OperateVehicle");
    assert_eq!(ext.extension_condition.as_deref(),Some("critical")); assert_eq!(ext.extension_location.as_deref(),Some("EmergencyHandling"));
}

#[test]
fn pr47_invalid_use_case_endpoints_self_extension_location_and_alias_block() {
    let (state, root)=workspace("PR47 Invalid");
    let (pkg, uc, uc2, block, signal)={let mut g=state.project.lock().unwrap();let p=g.as_mut().unwrap();let pkg=p.create_element(ElementKind::Package,"P",root).unwrap();let uc=p.create_element(ElementKind::UseCase,"A",pkg).unwrap();let uc2=p.create_element(ElementKind::UseCase,"B",pkg).unwrap();let block=p.create_element(ElementKind::Block,"Block",pkg).unwrap();let signal=p.create_element(ElementKind::Signal,"Signal",pkg).unwrap();(pkg,uc,uc2,block,signal)};
    let _=(pkg,uc,uc2,block,signal);
    let bad=temp_csv("bad","ID,Kind,From,To,Owner,Alias,Point,Condition\nI1,Include,A,Block,P,,,\nI2,Include,A,A,P,,,\nE1,Extend,B,A,P,,Missing,x\nEI,ElementImport,P,Signal,P,not-valid!,,\n");
    let map=relationship_map("Bad",bad,None,root,None,&[
        ("ID",SpreadsheetSemanticProperty::ExternalId),("Kind",SpreadsheetSemanticProperty::RelationshipKind),("From",SpreadsheetSemanticProperty::Source),("To",SpreadsheetSemanticProperty::Target),("Owner",SpreadsheetSemanticProperty::Owner),("Alias",SpreadsheetSemanticProperty::Alias),("Point",SpreadsheetSemanticProperty::ExtensionLocation),("Condition",SpreadsheetSemanticProperty::ExtensionCondition)]);
    let preview=preview_spreadsheet_import_group(&SpreadsheetImportMapGroup{mappings:vec![map]},&state);
    assert!(!preview.is_valid());
    assert!(preview.diagnostics.iter().any(|d| d.code=="USE_CASE_RELATIONSHIP_ENDPOINT_KIND_INVALID"));
    assert!(preview.diagnostics.iter().any(|d| d.code=="USE_CASE_RELATIONSHIP_SELF_REFERENCE"));
    assert!(preview.diagnostics.iter().any(|d| d.code=="ELEMENT_IMPORT_ALIAS_INVALID"));
    assert!(preview.diagnostics.iter().any(|d| d.reason.contains("extension point") || d.reason.contains("Missing")));
    assert_eq!(state.project.lock().unwrap().as_ref().unwrap().relationships.len(),0);
}

#[test]
fn pr47_reimport_no_change_updates_stable_ids_and_wrong_kind_collision_blocks() {
    let (state, root)=workspace("PR47 Reimport");
    apply_spreadsheet_import_group(&xlsx_group(root),&state).unwrap();
    let (extend_id, import_id)={let g=state.project.lock().unwrap();let p=g.as_ref().unwrap();(rel(p,"EXT-1").id,rel(p,"EI-1").id)};
    let second=preview_spreadsheet_import_group(&xlsx_group(root),&state); assert!(second.is_valid(),"{:?}",second.diagnostics); assert_eq!(second.totals.update,0); assert!(second.totals.no_change>=5);
    let update=temp_csv("update","ID,Kind,From,To,Owner,Alias,Visibility,Point,Condition,Description\nEXT-1,Extend,UC-EMERG,UC-OPERATE,PKG-UC,,,AlternateHandling,updated,changed\nEI-1,ElementImport,PKG-VEH,SIG-CMD,PKG-VEH,Cmd,Private,,,changed\n");
    let map=relationship_map("Updates",update,None,root,None,&[
        ("ID",SpreadsheetSemanticProperty::ExternalId),("Kind",SpreadsheetSemanticProperty::RelationshipKind),("From",SpreadsheetSemanticProperty::Source),("To",SpreadsheetSemanticProperty::Target),("Owner",SpreadsheetSemanticProperty::Owner),("Alias",SpreadsheetSemanticProperty::Alias),("Visibility",SpreadsheetSemanticProperty::Visibility),("Point",SpreadsheetSemanticProperty::ExtensionLocation),("Condition",SpreadsheetSemanticProperty::ExtensionCondition),("Description",SpreadsheetSemanticProperty::Documentation)]);
    apply_spreadsheet_import_group(&SpreadsheetImportMapGroup{mappings:vec![map]},&state).unwrap();
    {let g=state.project.lock().unwrap();let p=g.as_ref().unwrap();assert_eq!(rel(p,"EXT-1").id,extend_id);assert_eq!(rel(p,"EXT-1").extension_location.as_deref(),Some("AlternateHandling"));assert_eq!(rel(p,"EI-1").id,import_id);assert_eq!(rel(p,"EI-1").alias.as_deref(),Some("Cmd"));assert_eq!(rel(p,"EI-1").visibility,VisibilityKind::Private);}
    let collision=temp_csv("collision","ID,From,To,Owner\nINC-1,PKG-VEH,PKG-COMMON,PKG-VEH\n");
    let map=relationship_map("Collision",collision,None,root,Some(RelationshipKind::PackageImport),&[("ID",SpreadsheetSemanticProperty::ExternalId),("From",SpreadsheetSemanticProperty::Source),("To",SpreadsheetSemanticProperty::Target),("Owner",SpreadsheetSemanticProperty::Owner)]);
    let preview=preview_spreadsheet_import_group(&SpreadsheetImportMapGroup{mappings:vec![map]},&state);assert!(!preview.is_valid());assert!(preview.diagnostics.iter().any(|d| d.code=="RELATIONSHIP_IDENTITY_KIND_MISMATCH"));
}

#[test]
fn pr47_unresolved_ambiguous_duplicate_identity_and_owner_mismatch_block() {
    let (state,root)=workspace("PR47 Resolution");
    {let mut g=state.project.lock().unwrap();let p=g.as_mut().unwrap();let p1=p.create_element(ElementKind::Package,"One",root).unwrap();let p2=p.create_element(ElementKind::Package,"Two",root).unwrap();p.create_element(ElementKind::UseCase,"Same",p1).unwrap();p.create_element(ElementKind::UseCase,"Same",p2).unwrap();p.create_element(ElementKind::UseCase,"Target",p1).unwrap();}
    let csv=temp_csv("resolution","ID,Kind,From,To,Owner\nA,Include,Same,Target,One\nB,Include,Missing,Target,One\nC,PackageImport,One,Two,Two\nC,PackageImport,One,Two,One\n");
    let map=relationship_map("Resolution",csv,None,root,None,&[("ID",SpreadsheetSemanticProperty::ExternalId),("Kind",SpreadsheetSemanticProperty::RelationshipKind),("From",SpreadsheetSemanticProperty::Source),("To",SpreadsheetSemanticProperty::Target),("Owner",SpreadsheetSemanticProperty::Owner)]);
    let preview=preview_spreadsheet_import_group(&SpreadsheetImportMapGroup{mappings:vec![map]},&state);assert!(!preview.is_valid());
    assert!(preview.diagnostics.iter().any(|d| d.code=="SOURCE_AMBIGUOUS"));assert!(preview.diagnostics.iter().any(|d| d.code=="SOURCE_UNRESOLVED"));assert!(preview.diagnostics.iter().any(|d| d.code=="NAMESPACE_RELATIONSHIP_OWNER_INVALID"));assert!(preview.diagnostics.iter().any(|d| d.code=="DUPLICATE_SOURCE_EXTERNAL_ID"));
}

#[test]
fn pr47_late_invalid_extend_rolls_back_entire_map_group() {
    let (state,root)=workspace("PR47 Atomic");
    let packages=temp_csv("packages","ID,Name\nP1,One\nP2,Two\n");
    let usecases=temp_csv("ucs","ID,Name,Owner,Points\nU1,Base,P1,Good\nU2,Extension,P1,\n");
    let relations=temp_csv("rels","ID,Kind,From,To,Owner,Point\nPI,PackageImport,P1,P2,P1,\nE,Extend,U2,U1,P1,Missing\n");
    let group=SpreadsheetImportMapGroup{mappings:vec![
        element_map("Packages",packages,None,ElementKind::Package,root,&[("ID",SpreadsheetSemanticProperty::ExternalId),("Name",SpreadsheetSemanticProperty::Name)]),
        element_map("UseCases",usecases,None,ElementKind::UseCase,root,&[("ID",SpreadsheetSemanticProperty::ExternalId),("Name",SpreadsheetSemanticProperty::Name),("Owner",SpreadsheetSemanticProperty::Owner),("Points",SpreadsheetSemanticProperty::ExtensionPoints)]),
        relationship_map("Relations",relations,None,root,None,&[("ID",SpreadsheetSemanticProperty::ExternalId),("Kind",SpreadsheetSemanticProperty::RelationshipKind),("From",SpreadsheetSemanticProperty::Source),("To",SpreadsheetSemanticProperty::Target),("Owner",SpreadsheetSemanticProperty::Owner),("Point",SpreadsheetSemanticProperty::ExtensionLocation)]),
    ]};
    let preview=preview_spreadsheet_import_group(&group,&state);assert!(!preview.is_valid());assert_eq!(state.project.lock().unwrap().as_ref().unwrap().elements.len(),1);assert_eq!(state.project.lock().unwrap().as_ref().unwrap().relationships.len(),0);assert!(apply_spreadsheet_import_group(&group,&state).is_err());assert_eq!(state.project.lock().unwrap().as_ref().unwrap().elements.len(),1);
}
'''
Path("apps/desktop/src-tauri/src/workspace/spreadsheet_import/pr47_tests.rs").write_text(pr47_tests)

# Native semantic regression: PR47 importer relies on these existing authorities.
core_test = r'''use systems_modeler_core::{ElementKind, ModelError, Project, RelationshipKind, VisibilityKind};

#[test]
fn pr47_native_use_case_and_namespace_relationship_rules_remain_authoritative() {
    let mut project=Project::new("PR47 Core"); let root=project.root_id;
    let uc_pkg=project.create_element(ElementKind::Package,"UseCases",root).unwrap();
    let base=project.create_element(ElementKind::UseCase,"Base",uc_pkg).unwrap();
    project.element_mut(base).unwrap().extension_points=vec!["point".into()];
    let extension=project.create_element(ElementKind::UseCase,"Extension",uc_pkg).unwrap();
    let included=project.create_element(ElementKind::UseCase,"Included",uc_pkg).unwrap();
    let include=project.create_relationship(RelationshipKind::Include,base,included,Some(uc_pkg)).unwrap();
    assert_eq!(project.relationship(include).unwrap().source_id,base);
    let extend=project.create_relationship(RelationshipKind::Extend,extension,base,Some(uc_pkg)).unwrap();
    project.update_extend_relationship(extend,Some("guard".into()),Some("point".into())).unwrap();
    assert!(matches!(project.create_relationship(RelationshipKind::Include,base,base,Some(uc_pkg)),Err(ModelError::SelfUseCaseRelationship)));
    assert!(matches!(project.update_extend_relationship(extend,None,Some("missing".into())),Err(ModelError::ExtensionPointNotFound{..})));

    let vehicle=project.create_element(ElementKind::Package,"Vehicle",root).unwrap();
    let common=project.create_element(ElementKind::Package,"Common",root).unwrap();
    let signal=project.create_element(ElementKind::Signal,"Command",common).unwrap();
    let pi=project.create_package_import(vehicle,common,VisibilityKind::Private).unwrap();
    let ei=project.create_element_import(vehicle,signal,VisibilityKind::Public,Some("Cmd".into())).unwrap();
    let pm=project.create_relationship(RelationshipKind::PackageMerge,vehicle,common,Some(vehicle)).unwrap();
    assert_eq!(project.relationship(pi).unwrap().owner_id,Some(vehicle));
    assert_eq!(project.relationship(ei).unwrap().alias.as_deref(),Some("Cmd"));
    assert_eq!(project.relationship(pm).unwrap().owner_id,Some(vehicle));
    assert!(project.create_relationship(RelationshipKind::PackageImport,vehicle,common,Some(vehicle)).is_err());
    project.validate().unwrap();
}
'''
Path("crates/model-core/tests/pr47_core_namespace_relationships.rs").write_text(core_test)

persistence_test = r'''use systems_modeler_core::{ElementKind, Project, RelationshipKind, VisibilityKind};
use systems_modeler_persistence::ProjectDatabase;

#[test]
fn pr47_persistence_round_trip_preserves_all_five_relationships_and_metadata() {
    let mut p=Project::new("PR47 Persistence");let root=p.root_id;let uc_pkg=p.create_element(ElementKind::Package,"UC",root).unwrap();
    let base=p.create_element(ElementKind::UseCase,"Base",uc_pkg).unwrap();p.element_mut(base).unwrap().extension_points=vec!["point".into()];let included=p.create_element(ElementKind::UseCase,"Included",uc_pkg).unwrap();let ext=p.create_element(ElementKind::UseCase,"Ext",uc_pkg).unwrap();
    let include=p.create_relationship(RelationshipKind::Include,base,included,Some(uc_pkg)).unwrap();let extend=p.create_relationship(RelationshipKind::Extend,ext,base,Some(uc_pkg)).unwrap();p.update_extend_relationship(extend,Some("guard".into()),Some("point".into())).unwrap();
    let a=p.create_element(ElementKind::Package,"A",root).unwrap();let b=p.create_element(ElementKind::Package,"B",root).unwrap();let sig=p.create_element(ElementKind::Signal,"Signal",b).unwrap();let pi=p.create_package_import(a,b,VisibilityKind::Private).unwrap();let ei=p.create_element_import(a,sig,VisibilityKind::Public,Some("Alias".into())).unwrap();let pm=p.create_relationship(RelationshipKind::PackageMerge,a,b,Some(a)).unwrap();
    for (id,key) in [(include,"INC"),(extend,"EXT"),(pi,"PI"),(ei,"EI"),(pm,"PM")] {let r=p.relationships.get_mut(&id).unwrap();r.external_id=format!("catia:pr47::{key}");r.name=format!("name-{key}");r.documentation=format!("doc-{key}");}
    p.validate().unwrap();let mut db=ProjectDatabase::open_in_memory().unwrap();db.save_project(&p).unwrap();let r=db.load_project(p.id).unwrap();
    for (id,kind) in [(include,RelationshipKind::Include),(extend,RelationshipKind::Extend),(pi,RelationshipKind::PackageImport),(ei,RelationshipKind::ElementImport),(pm,RelationshipKind::PackageMerge)] {assert_eq!(r.relationship(id).unwrap().kind,kind);}
    assert_eq!(r.relationship(ei).unwrap().alias.as_deref(),Some("Alias"));assert_eq!(r.relationship(extend).unwrap().extension_condition.as_deref(),Some("guard"));assert_eq!(r.relationship(extend).unwrap().extension_location.as_deref(),Some("point"));assert_eq!(r.relationship(pi).unwrap().visibility,VisibilityKind::Private);r.validate().unwrap();
}
'''
Path("crates/persistence/tests/pr47_core_namespace_relationship_persistence.rs").write_text(persistence_test)

portable_test = r'''use super::*;
use systems_modeler_core::{ElementKind, RelationshipKind, VisibilityKind};

#[test]
fn pr47_portable_json_round_trip_preserves_core_namespace_relationships() {
    let source_state=WorkspaceState::default();let source_activity=ActivityWorkspaceState::default();let mut p=Project::new("PR47 Portable");let root=p.root_id;let uc_pkg=p.create_element(ElementKind::Package,"UC",root).unwrap();let base=p.create_element(ElementKind::UseCase,"Base",uc_pkg).unwrap();p.element_mut(base).unwrap().extension_points=vec!["point".into()];let included=p.create_element(ElementKind::UseCase,"Included",uc_pkg).unwrap();let ext=p.create_element(ElementKind::UseCase,"Ext",uc_pkg).unwrap();let include=p.create_relationship(RelationshipKind::Include,base,included,Some(uc_pkg)).unwrap();let extend=p.create_relationship(RelationshipKind::Extend,ext,base,Some(uc_pkg)).unwrap();p.update_extend_relationship(extend,Some("guard".into()),Some("point".into())).unwrap();let a=p.create_element(ElementKind::Package,"A",root).unwrap();let b=p.create_element(ElementKind::Package,"B",root).unwrap();let sig=p.create_element(ElementKind::Signal,"Signal",b).unwrap();let pi=p.create_package_import(a,b,VisibilityKind::Private).unwrap();let ei=p.create_element_import(a,sig,VisibilityKind::Public,Some("Alias".into())).unwrap();let pm=p.create_relationship(RelationshipKind::PackageMerge,a,b,Some(a)).unwrap();for (id,key) in [(include,"INC"),(extend,"EXT"),(pi,"PI"),(ei,"EI"),(pm,"PM")] {p.relationships.get_mut(&id).unwrap().external_id=format!("catia:pr47::{key}");}p.validate().unwrap();*source_state.project.lock().unwrap()=Some(p);
    let json=export_from_states(&source_state,&source_activity).unwrap();let target_state=WorkspaceState::default();let target_activity=ActivityWorkspaceState::default();import_into_states(&json,&target_state,&target_activity).unwrap();let g=target_state.project.lock().unwrap();let r=g.as_ref().unwrap();for (id,kind) in [(include,RelationshipKind::Include),(extend,RelationshipKind::Extend),(pi,RelationshipKind::PackageImport),(ei,RelationshipKind::ElementImport),(pm,RelationshipKind::PackageMerge)] {assert_eq!(r.relationship(id).unwrap().kind,kind);}assert_eq!(r.relationship(ei).unwrap().alias.as_deref(),Some("Alias"));assert_eq!(r.relationship(extend).unwrap().extension_location.as_deref(),Some("point"));assert_eq!(r.relationship(extend).unwrap().extension_condition.as_deref(),Some("guard"));r.validate().unwrap();
}
'''
Path("apps/desktop/src-tauri/src/workspace/portable_interchange/pr47_core_namespace_relationship_tests.rs").write_text(portable_test)

# -----------------------------------------------------------------------------
# Small business-facing XLSX fixture (OOXML, no third-party Python dependency).
# -----------------------------------------------------------------------------
sheets = [
    ("Packages", [
        ["Package ID","Package Name","Parent"],
        ["PKG-UC","UseCases",""], ["PKG-VEH","VehicleModel",""], ["PKG-COMMON","CommonLibrary",""], ["PKG-EXT","ExtensionLibrary",""],
    ]),
    ("Use Cases", [
        ["Use Case ID","Use Case Name","Package","Extension Points"],
        ["UC-OPERATE","OperateVehicle","PKG-UC","EmergencyHandling;AlternateHandling"],
        ["UC-START","StartVehicle","PKG-UC",""],
        ["UC-EMERG","EmergencyShutdown","PKG-UC",""],
    ]),
    ("Imported Elements", [
        ["Element ID","Element Name","Package"], ["SIG-CMD","CommandSignal","PKG-COMMON"],
    ]),
    ("Relationships", [
        ["Relationship ID","Rule","From","To","Owner","Alias","Visibility","Extension Point","Condition","Name","Description"],
        ["INC-1","Include","UC-OPERATE","UC-START","PKG-UC","","Public","","","Required startup","include doc"],
        ["EXT-1","Extend","UC-EMERG","UC-OPERATE","PKG-UC","","Public","EmergencyHandling","emergency","Emergency reuse","extend doc"],
        ["PI-1","PackageImport","PKG-VEH","PKG-COMMON","PKG-VEH","","Private","","","Common visibility","package import doc"],
        ["EI-1","ElementImport","PKG-VEH","SIG-CMD","PKG-VEH","Command","Public","","","Command import","element import doc"],
        ["PM-1","PackageMerge","PKG-VEH","PKG-EXT","PKG-VEH","","Public","","","Extension merge","package merge doc"],
    ]),
]

def cell(ref, value):
    return f'<c r="{ref}" t="inlineStr"><is><t>{escape(str(value))}</t></is></c>'

def col_name(n):
    out=""
    while n:
        n,rem=divmod(n-1,26);out=chr(65+rem)+out
    return out

fixture=Path("apps/desktop/src-tauri/tests/fixtures/pr47_core_namespace_relationships.xlsx")
fixture.parent.mkdir(parents=True,exist_ok=True)
with ZipFile(fixture,"w",ZIP_DEFLATED) as z:
    z.writestr("[Content_Types].xml", '<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>' + ''.join(f'<Override PartName="/xl/worksheets/sheet{i}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>' for i in range(1,len(sheets)+1)) + '</Types>')
    z.writestr("_rels/.rels", '<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>')
    z.writestr("xl/workbook.xml", '<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>' + ''.join(f'<sheet name="{escape(name)}" sheetId="{i}" r:id="rId{i}"/>' for i,(name,_) in enumerate(sheets,1)) + '</sheets></workbook>')
    z.writestr("xl/_rels/workbook.xml.rels", '<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">' + ''.join(f'<Relationship Id="rId{i}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{i}.xml"/>' for i in range(1,len(sheets)+1)) + '</Relationships>')
    for i,(_,rows) in enumerate(sheets,1):
        row_xml=[]
        for r,row in enumerate(rows,1):
            cells=''.join(cell(f'{col_name(c)}{r}',v) for c,v in enumerate(row,1))
            row_xml.append(f'<row r="{r}">{cells}</row>')
        z.writestr(f"xl/worksheets/sheet{i}.xml", '<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>' + ''.join(row_xml) + '</sheetData></worksheet>')
