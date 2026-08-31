from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing anchor: {label}")
    return text.replace(old, new, 1)


# ---------------------------------------------------------------------------
# PR36 bulk-model extension: relationship field/update operation only.
# ---------------------------------------------------------------------------
bulk_path = ROOT / "apps/desktop/src-tauri/src/workspace/bulk_model.rs"
bulk = bulk_path.read_text(encoding="utf-8")

bulk = replace_once(
    bulk,
    '''pub type DiagramReference = BuildReference<DiagramId>;\n\n#[derive(Debug, Clone)]\npub enum ModelBuildOperation {\n''',
    '''pub type DiagramReference = BuildReference<DiagramId>;\n\n#[derive(Debug, Clone, Default)]\npub struct AssociationEndBuildFields {\n    pub role_name: Option<String>,\n    pub multiplicity: Option<Multiplicity>,\n    pub navigable: Option<bool>,\n    pub aggregation: Option<AggregationKind>,\n}\n\n#[derive(Debug, Clone)]\npub enum ModelBuildOperation {\n''',
    "bulk association end fields",
)

bulk = replace_once(
    bulk,
    '''    CreateRelationship {\n        external_id: String,\n        kind: RelationshipKind,\n        source: ElementReference,\n        target: ElementReference,\n        owner: Option<ElementReference>,\n    },\n    CreateDiagram {\n''',
    '''    CreateRelationship {\n        external_id: String,\n        kind: RelationshipKind,\n        source: ElementReference,\n        target: ElementReference,\n        owner: Option<ElementReference>,\n    },\n    /// PR40 mapped relationship update path. All endpoint/owner resolution and\n    /// mutation stays in the PR36 candidate so preview/apply remain atomic.\n    UpdateRelationshipFields {\n        relationship: RelationshipReference,\n        name: Option<String>,\n        owner: Option<ElementReference>,\n        source: Option<ElementReference>,\n        target: Option<ElementReference>,\n        external_id: Option<String>,\n        documentation: Option<String>,\n        visibility: Option<VisibilityKind>,\n        source_end: Option<AssociationEndBuildFields>,\n        target_end: Option<AssociationEndBuildFields>,\n    },\n    CreateDiagram {\n''',
    "bulk relationship update variant",
)

bulk = replace_once(
    bulk,
    '''        ModelBuildOperation::CreateRelationship { external_id, .. } => {\n            format!("CREATE relationship {external_id}")\n        }\n        ModelBuildOperation::CreateDiagram {\n''',
    '''        ModelBuildOperation::CreateRelationship { external_id, .. } => {\n            format!("CREATE relationship {external_id}")\n        }\n        ModelBuildOperation::UpdateRelationshipFields { .. } => {\n            "UPDATE mapped relationship fields".into()\n        }\n        ModelBuildOperation::CreateDiagram {\n''',
    "bulk relationship operation description",
)

bulk = replace_once(
    bulk,
    '''                    relationship_ids.insert(key, id);\n                }\n                ModelBuildOperation::CreateDiagram {\n''',
    '''                    relationship_ids.insert(key, id);\n                }\n                ModelBuildOperation::UpdateRelationshipFields {\n                    relationship,\n                    name,\n                    owner,\n                    source,\n                    target,\n                    external_id,\n                    documentation,\n                    visibility,\n                    source_end,\n                    target_end,\n                } => {\n                    let id = resolve_relationship(\n                        &project,\n                        &relationship_ids,\n                        namespace,\n                        relationship,\n                        index,\n                    )?;\n                    let current = project.relationship(id).map_err(|cause| {\n                        error("SEMANTIC_VALIDATION", Some(index), cause.to_string())\n                    })?;\n                    let kind = current.kind.clone();\n                    let next_source = source\n                        .as_ref()\n                        .map(|reference| {\n                            resolve_element(&project, &element_ids, namespace, reference, index)\n                        })\n                        .transpose()?\n                        .unwrap_or(current.source_id);\n                    let next_target = target\n                        .as_ref()\n                        .map(|reference| {\n                            resolve_element(&project, &element_ids, namespace, reference, index)\n                        })\n                        .transpose()?\n                        .unwrap_or(current.target_id);\n                    let next_owner = owner\n                        .as_ref()\n                        .map(|reference| {\n                            resolve_element(&project, &element_ids, namespace, reference, index)\n                        })\n                        .transpose()?\n                        .or(current.owner_id);\n\n                    let mut next_association_ends = if kind == RelationshipKind::Association {\n                        if current.association_ends.len() < 2 {\n                            return Err(error(\n                                "SEMANTIC_VALIDATION",\n                                Some(index),\n                                "Association update requires two existing semantic ends",\n                            ));\n                        }\n                        let mut ends = current.association_ends.clone();\n                        ends[0].classifier_id = next_source;\n                        ends[1].classifier_id = next_target;\n                        if let Some(fields) = source_end {\n                            if let Some(value) = &fields.role_name {\n                                ends[0].role_name = value.clone();\n                            }\n                            if let Some(value) = fields.multiplicity {\n                                ends[0].multiplicity = value;\n                            }\n                            if let Some(value) = fields.navigable {\n                                ends[0].navigable = value;\n                            }\n                            if let Some(value) = fields.aggregation {\n                                ends[0].aggregation = value;\n                            }\n                        }\n                        if let Some(fields) = target_end {\n                            if let Some(value) = &fields.role_name {\n                                ends[1].role_name = value.clone();\n                            }\n                            if let Some(value) = fields.multiplicity {\n                                ends[1].multiplicity = value;\n                            }\n                            if let Some(value) = fields.navigable {\n                                ends[1].navigable = value;\n                            }\n                            if let Some(value) = fields.aggregation {\n                                ends[1].aggregation = value;\n                            }\n                        }\n                        Some(ends)\n                    } else {\n                        if source_end.is_some() || target_end.is_some() {\n                            return Err(error(\n                                "SEMANTIC_VALIDATION",\n                                Some(index),\n                                "Association-end fields are valid only for Association",\n                            ));\n                        }\n                        None\n                    };\n\n                    // Reuse the existing model-core creation validator on a candidate clone\n                    // before reconnecting an existing relationship. This preserves existing\n                    // Generalization cycle checks, Dependency endpoint checks, ownership\n                    // rules, and Association classifier validation without a second importer\n                    // validation model.\n                    let mut validation_project = project.clone();\n                    validation_project.relationships.remove(&id);\n                    if kind == RelationshipKind::Association {\n                        validation_project\n                            .create_association(\n                                next_owner,\n                                next_association_ends.clone().unwrap_or_default(),\n                            )\n                            .map_err(|cause| {\n                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())\n                            })?;\n                    } else {\n                        validation_project\n                            .create_relationship(\n                                kind.clone(),\n                                next_source,\n                                next_target,\n                                next_owner,\n                            )\n                            .map_err(|cause| {\n                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())\n                            })?;\n                    }\n\n                    let next_external_id = external_id.as_ref().map(|value| {\n                        external_key(namespace, value)\n                    });\n                    if let Some(key) = &next_external_id {\n                        if project\n                            .elements\n                            .values()\n                            .any(|element| element.external_id == *key)\n                            || project.relationships.values().any(|candidate| {\n                                candidate.id != id && candidate.external_id == *key\n                            })\n                        {\n                            return Err(error(\n                                "DUPLICATE_EXTERNAL_ID",\n                                Some(index),\n                                format!("external ID already exists: {key}"),\n                            ));\n                        }\n                    }\n\n                    let relationship = project.relationships.get_mut(&id).unwrap();\n                    relationship.source_id = next_source;\n                    relationship.target_id = next_target;\n                    relationship.owner_id = next_owner;\n                    if let Some(value) = name {\n                        relationship.name = value.clone();\n                    }\n                    if let Some(value) = documentation {\n                        relationship.documentation = value.clone();\n                    }\n                    if let Some(value) = visibility {\n                        relationship.visibility = *value;\n                    }\n                    if let Some(value) = next_external_id {\n                        relationship.external_id = value;\n                    }\n                    if let Some(ends) = next_association_ends.take() {\n                        relationship.association_ends = ends;\n                    }\n                    project.validate().map_err(|cause| {\n                        error("SEMANTIC_VALIDATION", Some(index), cause.to_string())\n                    })?;\n                }\n                ModelBuildOperation::CreateDiagram {\n''',
    "bulk relationship update candidate handler",
)

bulk_path.write_text(bulk, encoding="utf-8")


# ---------------------------------------------------------------------------
# PR38/39 spreadsheet mapping extension for PR40 relationships.
# ---------------------------------------------------------------------------
path = ROOT / "apps/desktop/src-tauri/src/workspace/spreadsheet_import.rs"
text = path.read_text(encoding="utf-8")

text = replace_once(
    text,
    '''        BuildDiagnostic, BuildDiagnosticSeverity, BuildReference, ElementReference,\n        ModelBuildOperation, ModelBuildPlan, ModelBuildResult, apply_model_build, external_key,\n        preview_model_build,\n''',
    '''        AssociationEndBuildFields, BuildDiagnostic, BuildDiagnosticSeverity, BuildReference,\n        ElementReference, ModelBuildOperation, ModelBuildPlan, ModelBuildResult,\n        RelationshipReference, apply_model_build, external_key, preview_model_build,\n''',
    "spreadsheet bulk imports",
)
text = replace_once(
    text,
    '''use systems_modeler_core::{\n    Element, ElementId, ElementKind, FlowDirection, Multiplicity, Project, VisibilityKind,\n};\n''',
    '''use systems_modeler_core::{\n    AggregationKind, Element, ElementId, ElementKind, FlowDirection, Multiplicity, Project,\n    Relationship, RelationshipId, RelationshipKind, VisibilityKind,\n};\n''',
    "spreadsheet model imports",
)

text = replace_once(
    text,
    '''    RequirementId,\n    RequirementText,\n}\n''',
    '''    RequirementId,\n    RequirementText,\n    RelationshipKind,\n    Source,\n    Target,\n    SourceEndRole,\n    TargetEndRole,\n    SourceMultiplicity,\n    TargetMultiplicity,\n    SourceNavigable,\n    TargetNavigable,\n    SourceAggregation,\n    TargetAggregation,\n}\n''',
    "relationship semantic properties",
)

text = replace_once(
    text,
    '''#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]\npub enum SpreadsheetSearchScope {\n    TargetOnly,\n    TargetRecursive,\n}\n\n#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\npub struct SpreadsheetColumnMapping {\n''',
    '''#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]\npub enum SpreadsheetSearchScope {\n    TargetOnly,\n    TargetRecursive,\n}\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]\npub enum SpreadsheetRelationshipIdentityPolicy {\n    #[default]\n    ExternalId,\n    KindSourceTarget,\n    KindSourceTargetAssociationEnds,\n}\n\n#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\npub struct SpreadsheetColumnMapping {\n''',
    "relationship identity policy",
)

text = replace_once(
    text,
    '''    pub element_kind: ElementKind,\n    /// Stable semantic target. Display names are never persisted as target identity.\n    pub target_scope: ElementId,\n''',
    '''    pub element_kind: ElementKind,\n    /// PR40 relationship mappings set this to a supported relationship kind, or map\n    /// the controlled `RelationshipKind` property for mixed relationship rows. The\n    /// legacy `element_kind` field remains for backwards-compatible PR38/39 maps.\n    #[serde(default)]\n    pub relationship_kind: Option<RelationshipKind>,\n    #[serde(default)]\n    pub relationship_identity: SpreadsheetRelationshipIdentityPolicy,\n    /// Stable semantic target. Display names are never persisted as target identity.\n    pub target_scope: ElementId,\n''',
    "relationship map fields",
)

text = replace_once(
    text,
    '''    pub element_kind: Option<ElementKind>,\n    pub source_namespace: Option<String>,\n''',
    '''    pub element_kind: Option<ElementKind>,\n    pub relationship_kind: Option<RelationshipKind>,\n    pub source_endpoint: Option<String>,\n    pub target_endpoint: Option<String>,\n    pub source_namespace: Option<String>,\n''',
    "relationship diagnostic fields",
)

text = replace_once(
    text,
    '''    pub element_kind: ElementKind,\n    pub identification_value: Option<String>,\n    pub action: SpreadsheetRowAction,\n''',
    '''    pub element_kind: ElementKind,\n    pub relationship_kind: Option<RelationshipKind>,\n    pub identification_value: Option<String>,\n    pub action: SpreadsheetRowAction,\n''',
    "relationship row preview field",
)

text = replace_once(
    text,
    '''    element_kind: ElementKind,\n    source_namespace: String,\n    identification_value: Option<String>,\n}\n''',
    '''    element_kind: ElementKind,\n    relationship_kind: Option<RelationshipKind>,\n    source_endpoint: Option<String>,\n    target_endpoint: Option<String>,\n    source_namespace: String,\n    identification_value: Option<String>,\n}\n''',
    "relationship row context fields",
)

text = replace_once(
    text,
    '''fn is_feature_kind(kind: &ElementKind) -> bool {\n    matches!(\n        kind,\n        ElementKind::PartProperty\n            | ElementKind::ReferenceProperty\n            | ElementKind::ValueProperty\n            | ElementKind::FlowProperty\n            | ElementKind::ConstraintProperty\n            | ElementKind::ConstraintParameter\n    )\n}\n\nfn diagnostic(\n''',
    '''fn is_feature_kind(kind: &ElementKind) -> bool {\n    matches!(\n        kind,\n        ElementKind::PartProperty\n            | ElementKind::ReferenceProperty\n            | ElementKind::ValueProperty\n            | ElementKind::FlowProperty\n            | ElementKind::ConstraintProperty\n            | ElementKind::ConstraintParameter\n    )\n}\n\nfn supported_relationship_kind(kind: &RelationshipKind) -> bool {\n    matches!(\n        kind,\n        RelationshipKind::Association\n            | RelationshipKind::Generalization\n            | RelationshipKind::Dependency\n            | RelationshipKind::Realization\n    )\n}\n\nfn is_relationship_map(map: &SpreadsheetImportMap) -> bool {\n    map.relationship_kind.is_some()\n        || map\n            .column_mappings\n            .iter()\n            .any(|mapping| mapping.property == SpreadsheetSemanticProperty::RelationshipKind)\n}\n\nfn reference_error_code(\n    property: SpreadsheetSemanticProperty,\n    ambiguous: bool,\n) -> &'static str {\n    match (property, ambiguous) {\n        (SpreadsheetSemanticProperty::Owner, true) => "OWNER_AMBIGUOUS",\n        (SpreadsheetSemanticProperty::Owner, false) => "OWNER_UNRESOLVED",\n        (SpreadsheetSemanticProperty::Type, true) => "TYPE_AMBIGUOUS",\n        (SpreadsheetSemanticProperty::Type, false) => "TYPE_UNRESOLVED",\n        (SpreadsheetSemanticProperty::Source, true) => "SOURCE_AMBIGUOUS",\n        (SpreadsheetSemanticProperty::Source, false) => "SOURCE_UNRESOLVED",\n        (SpreadsheetSemanticProperty::Target, true) => "TARGET_AMBIGUOUS",\n        (SpreadsheetSemanticProperty::Target, false) => "TARGET_UNRESOLVED",\n        (_, true) => "REFERENCE_AMBIGUOUS",\n        (_, false) => "REFERENCE_UNRESOLVED",\n    }\n}\n\nfn diagnostic(\n''',
    "relationship helpers",
)

text = replace_once(
    text,
    '''        element_kind: map.map(|mapping| mapping.element_kind.clone()),\n        source_namespace: map.map(|mapping| mapping.source_namespace.clone()),\n''',
    '''        element_kind: map.map(|mapping| mapping.element_kind.clone()),\n        relationship_kind: map.and_then(|mapping| mapping.relationship_kind.clone()),\n        source_endpoint: None,\n        target_endpoint: None,\n        source_namespace: map.map(|mapping| mapping.source_namespace.clone()),\n''',
    "diagnostic relationship defaults",
)

# Make semantic-reference diagnostics endpoint-aware instead of treating every non-owner as Type.
text = text.replace(
    '''                if property == SpreadsheetSemanticProperty::Owner {\n                    "OWNER_AMBIGUOUS"\n                } else {\n                    "TYPE_AMBIGUOUS"\n                },\n''',
    '''                reference_error_code(property, true),\n''',
)
text = text.replace(
    '''            if property == SpreadsheetSemanticProperty::Owner {\n                "OWNER_UNRESOLVED"\n            } else {\n                "TYPE_UNRESOLVED"\n            },\n''',
    '''            reference_error_code(property, false),\n''',
)
text = text.replace(
    '''            if property == SpreadsheetSemanticProperty::Owner {\n                "OWNER_AMBIGUOUS"\n            } else {\n                "TYPE_AMBIGUOUS"\n            },\n''',
    '''            reference_error_code(property, true),\n''',
)

# Relationship maps take the controlled relationship path and skip legacy element validation.
validate_anchor = '''    if !supported_kind(&map.element_kind) {\n'''
relationship_validation = '''    let has_property = |property| {\n        map.column_mappings\n            .iter()\n            .any(|mapping| mapping.property == property)\n    };\n    if is_relationship_map(map) {\n        if let Some(kind) = &map.relationship_kind\n            && !supported_relationship_kind(kind)\n        {\n            return Err(diagnostic(\n                Some(map), None, mapped_column_name(map, SpreadsheetSemanticProperty::RelationshipKind),\n                Some(SpreadsheetSemanticProperty::RelationshipKind), None,\n                "RELATIONSHIP_KIND_UNSUPPORTED",\n                format!("{:?} is outside the PR40 relationship scope", kind),\n            ));\n        }\n        if map.relationship_kind.is_none() && !has_property(SpreadsheetSemanticProperty::RelationshipKind) {\n            return Err(diagnostic(\n                Some(map), None, None, Some(SpreadsheetSemanticProperty::RelationshipKind), None,\n                "RELATIONSHIP_KIND_REQUIRED",\n                "relationship mappings require a configured relationship_kind or mapped RelationshipKind column",\n            ));\n        }\n        for property in [SpreadsheetSemanticProperty::Source, SpreadsheetSemanticProperty::Target, SpreadsheetSemanticProperty::Owner] {\n            if !has_property(property) {\n                return Err(diagnostic(\n                    Some(map), None, None, Some(property), None,\n                    "RELATIONSHIP_COLUMN_REQUIRED",\n                    format!("PR40 relationship mappings require a mapped {:?} column", property),\n                ));\n            }\n        }\n        if map.relationship_identity == SpreadsheetRelationshipIdentityPolicy::ExternalId\n            && !has_property(SpreadsheetSemanticProperty::ExternalId)\n        {\n            return Err(diagnostic(\n                Some(map), None, None, Some(SpreadsheetSemanticProperty::ExternalId), None,\n                "RELATIONSHIP_EXTERNAL_ID_REQUIRED",\n                "ExternalId relationship identity requires a mapped External ID column",\n            ));\n        }\n        if [\n            SpreadsheetSemanticProperty::Type, SpreadsheetSemanticProperty::Multiplicity,\n            SpreadsheetSemanticProperty::DefaultValue, SpreadsheetSemanticProperty::FlowDirection,\n            SpreadsheetSemanticProperty::RequirementId, SpreadsheetSemanticProperty::RequirementText,\n        ].into_iter().any(has_property) {\n            return Err(diagnostic(\n                Some(map), None, None, None, None,\n                "SEMANTIC_PROPERTY_INVALID",\n                "element/feature-only mapped fields cannot be used by PR40 relationship mappings",\n            ));\n        }\n        let association_fields = [\n            SpreadsheetSemanticProperty::SourceEndRole, SpreadsheetSemanticProperty::TargetEndRole,\n            SpreadsheetSemanticProperty::SourceMultiplicity, SpreadsheetSemanticProperty::TargetMultiplicity,\n            SpreadsheetSemanticProperty::SourceNavigable, SpreadsheetSemanticProperty::TargetNavigable,\n            SpreadsheetSemanticProperty::SourceAggregation, SpreadsheetSemanticProperty::TargetAggregation,\n        ];\n        if map.relationship_kind.as_ref().is_some_and(|kind| *kind != RelationshipKind::Association)\n            && association_fields.into_iter().any(has_property)\n        {\n            return Err(diagnostic(\n                Some(map), None, None, None, None,\n                "ASSOCIATION_FIELD_INVALID",\n                "Association-end fields can be mapped only for Association rows",\n            ));\n        }\n        let target = project.element(map.target_scope).map_err(|_| {\n            diagnostic(\n                Some(map), None, None, None, None, "TARGET_SCOPE_UNRESOLVED",\n                format!("target scope {} does not resolve", map.target_scope),\n            )\n        })?;\n        if !target.is_namespace() {\n            return Err(diagnostic(\n                Some(map), None, None, None, None, "TARGET_SCOPE_INVALID",\n                format!("target '{}' ({:?}) is not a semantic namespace", target.name, target.kind),\n            ));\n        }\n        return Ok(());\n    }\n\n'''
if validate_anchor not in text:
    raise SystemExit("missing relationship validate insertion anchor")
text = text.replace(validate_anchor, relationship_validation + validate_anchor, 1)

# Remove duplicate local has_property declaration in the legacy path after insertion.
legacy_has = '''    let has_property = |property| {\n        map.column_mappings\n            .iter()\n            .any(|mapping| mapping.property == property)\n    };\n    if map.element_kind != ElementKind::Requirement\n'''
if legacy_has not in text:
    raise SystemExit("legacy has_property anchor missing")
text = text.replace(legacy_has, '''    if map.element_kind != ElementKind::Requirement\n''', 1)

# Relationship parsing/planning helpers go immediately before legacy mapped-field updates.
helper_anchor = '''#[allow(clippy::too_many_arguments)]\nfn mapped_field_changes(\n'''
relationship_helpers = r'''fn parse_relationship_kind_value(
    map: &SpreadsheetImportMap,
    row: usize,
    value: &str,
) -> Result<RelationshipKind, SpreadsheetImportDiagnostic> {
    let kind = match value.trim().to_ascii_lowercase().as_str() {
        "association" => RelationshipKind::Association,
        "generalization" => RelationshipKind::Generalization,
        "dependency" => RelationshipKind::Dependency,
        "realization" => RelationshipKind::Realization,
        _ => {
            return Err(diagnostic(
                Some(map), Some(row), mapped_column_name(map, SpreadsheetSemanticProperty::RelationshipKind),
                Some(SpreadsheetSemanticProperty::RelationshipKind), Some(value.trim().to_string()),
                "RELATIONSHIP_KIND_UNSUPPORTED",
                format!("relationship kind '{}' is outside PR40; expected Association, Generalization, Dependency, or Realization", value.trim()),
            ));
        }
    };
    Ok(kind)
}

fn relationship_kind_for_row(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
) -> Result<RelationshipKind, SpreadsheetImportDiagnostic> {
    let mapped = non_empty_value(values, SpreadsheetSemanticProperty::RelationshipKind)
        .map(|value| parse_relationship_kind_value(map, row, value))
        .transpose()?;
    match (map.relationship_kind.clone(), mapped) {
        (Some(configured), Some(mapped)) if configured != mapped => Err(diagnostic(
            Some(map), Some(row), mapped_column_name(map, SpreadsheetSemanticProperty::RelationshipKind),
            Some(SpreadsheetSemanticProperty::RelationshipKind), Some(format!("{:?}", mapped)),
            "RELATIONSHIP_KIND_MISMATCH",
            format!("row relationship kind {:?} does not match configured {:?}", mapped, configured),
        )),
        (Some(configured), _) => Ok(configured),
        (None, Some(mapped)) => Ok(mapped),
        (None, None) => Err(diagnostic(
            Some(map), Some(row), mapped_column_name(map, SpreadsheetSemanticProperty::RelationshipKind),
            Some(SpreadsheetSemanticProperty::RelationshipKind), None,
            "RELATIONSHIP_KIND_REQUIRED", "relationship kind is blank",
        )),
    }
}

fn parse_navigable(
    map: &SpreadsheetImportMap,
    row: usize,
    property: SpreadsheetSemanticProperty,
    value: &str,
) -> Result<bool, SpreadsheetImportDiagnostic> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "1" => Ok(true),
        "false" | "no" | "0" => Ok(false),
        _ => Err(diagnostic(
            Some(map), Some(row), mapped_column_name(map, property), Some(property), None,
            "NAVIGABILITY_INVALID", format!("navigability '{}' must be true/false, yes/no, or 1/0", value),
        )),
    }
}

fn parse_aggregation(
    map: &SpreadsheetImportMap,
    row: usize,
    property: SpreadsheetSemanticProperty,
    value: &str,
) -> Result<AggregationKind, SpreadsheetImportDiagnostic> {
    match value.trim().to_ascii_lowercase().as_str() {
        "none" => Ok(AggregationKind::None),
        "shared" | "aggregation" => Ok(AggregationKind::Shared),
        "composite" | "composition" => Ok(AggregationKind::Composite),
        _ => Err(diagnostic(
            Some(map), Some(row), mapped_column_name(map, property), Some(property), None,
            "AGGREGATION_INVALID", format!("aggregation '{}' must be none, shared, or composite", value),
        )),
    }
}

fn parse_end_fields(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    role_property: SpreadsheetSemanticProperty,
    multiplicity_property: SpreadsheetSemanticProperty,
    navigable_property: SpreadsheetSemanticProperty,
    aggregation_property: SpreadsheetSemanticProperty,
) -> Result<Option<AssociationEndBuildFields>, SpreadsheetImportDiagnostic> {
    let mapped = |property| values.contains_key(&property);
    if ![role_property, multiplicity_property, navigable_property, aggregation_property]
        .into_iter().any(mapped)
    {
        return Ok(None);
    }
    let role_name = values.get(&role_property).cloned();
    let multiplicity = match non_empty_value(values, multiplicity_property) {
        Some(value) => Some(super::parametrics::parse_multiplicity(value).map_err(|reason| {
            diagnostic(
                Some(map), Some(row), mapped_column_name(map, multiplicity_property), Some(multiplicity_property), None,
                "MULTIPLICITY_INVALID", format!("association-end multiplicity '{}' is invalid: {}", value, reason),
            )
        })?),
        None => None,
    };
    let navigable = match non_empty_value(values, navigable_property) {
        Some(value) => Some(parse_navigable(map, row, navigable_property, value)?),
        None => None,
    };
    let aggregation = match non_empty_value(values, aggregation_property) {
        Some(value) => Some(parse_aggregation(map, row, aggregation_property, value)?),
        None => None,
    };
    Ok(Some(AssociationEndBuildFields { role_name, multiplicity, navigable, aggregation }))
}

fn resolve_relationship_endpoint(
    map: &SpreadsheetImportMap,
    project: &Project,
    planned: &[PlannedElement],
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    property: SpreadsheetSemanticProperty,
    label: &str,
) -> Result<ResolvedOwner, SpreadsheetImportDiagnostic> {
    let requested = non_empty_value(values, property).ok_or_else(|| diagnostic(
        Some(map), None, mapped_column_name(map, property), Some(property), None,
        if property == SpreadsheetSemanticProperty::Source { "SOURCE_REQUIRED" } else { "TARGET_REQUIRED" },
        format!("{label} endpoint is blank"),
    ))?;
    resolve_semantic_reference(map, project, planned, requested, property, label).map_err(|mut error| {
        if property == SpreadsheetSemanticProperty::Source {
            error.source_endpoint = Some(requested.to_string());
        } else {
            error.target_endpoint = Some(requested.to_string());
        }
        error
    })
}

fn relationship_in_scope(
    map: &SpreadsheetImportMap,
    project: &Project,
    relationship: &Relationship,
) -> bool {
    let Some(owner_id) = relationship.owner_id else { return false; };
    match map.search_scope {
        SpreadsheetSearchScope::TargetOnly => owner_id == map.target_scope,
        SpreadsheetSearchScope::TargetRecursive => {
            owner_id == map.target_scope || distance_from_target(project, owner_id, map.target_scope).is_some()
        }
    }
}

fn find_relationship_by_external_id<'a>(
    map: &SpreadsheetImportMap,
    project: &'a Project,
    external_id: &str,
    kind: &RelationshipKind,
) -> Result<Option<&'a Relationship>, SpreadsheetImportDiagnostic> {
    let key = external_key(&map.source_namespace, external_id);
    if project.elements.values().any(|element| element.external_id == key) {
        return Err(diagnostic(
            Some(map), None, mapped_column_name(map, SpreadsheetSemanticProperty::ExternalId),
            Some(SpreadsheetSemanticProperty::ExternalId), Some(external_id.to_string()),
            "RELATIONSHIP_IDENTITY_KIND_MISMATCH", "relationship external ID is already used by an element",
        ));
    }
    let matches = project.relationships.values()
        .filter(|relationship| relationship.external_id == key)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Ok(None),
        [relationship] if &relationship.kind != kind => Err(diagnostic(
            Some(map), None, mapped_column_name(map, SpreadsheetSemanticProperty::ExternalId),
            Some(SpreadsheetSemanticProperty::ExternalId), Some(external_id.to_string()),
            "RELATIONSHIP_IDENTITY_KIND_MISMATCH",
            format!("external ID identifies {:?}, not {:?}", relationship.kind, kind),
        )),
        [relationship] if !relationship_in_scope(map, project, relationship) => Err(diagnostic(
            Some(map), None, mapped_column_name(map, SpreadsheetSemanticProperty::ExternalId),
            Some(SpreadsheetSemanticProperty::ExternalId), Some(external_id.to_string()),
            "RELATIONSHIP_OUTSIDE_SCOPE", "relationship external ID exists outside the configured target/search scope",
        )),
        [relationship] => Ok(Some(*relationship)),
        _ => Err(diagnostic(
            Some(map), None, mapped_column_name(map, SpreadsheetSemanticProperty::ExternalId),
            Some(SpreadsheetSemanticProperty::ExternalId), Some(external_id.to_string()),
            "AMBIGUOUS_RELATIONSHIP", "relationship external ID resolves to more than one relationship",
        )),
    }
}

fn end_fields_match(
    end: &systems_modeler_core::AssociationEnd,
    fields: &Option<AssociationEndBuildFields>,
) -> bool {
    let Some(fields) = fields else { return true; };
    fields.role_name.as_ref().is_none_or(|value| end.role_name == *value)
        && fields.multiplicity.is_none_or(|value| end.multiplicity == value)
        && fields.navigable.is_none_or(|value| end.navigable == value)
        && fields.aggregation.is_none_or(|value| end.aggregation == value)
}

fn find_relationship_by_fallback<'a>(
    map: &SpreadsheetImportMap,
    project: &'a Project,
    kind: &RelationshipKind,
    source: &ResolvedOwner,
    target: &ResolvedOwner,
    source_end: &Option<AssociationEndBuildFields>,
    target_end: &Option<AssociationEndBuildFields>,
) -> Result<Option<&'a Relationship>, SpreadsheetImportDiagnostic> {
    let (BuildReference::Existing(source_id), BuildReference::Existing(target_id)) =
        (&source.reference, &target.reference)
    else {
        return Ok(None);
    };
    let matches = project.relationships.values()
        .filter(|relationship| relationship_in_scope(map, project, relationship))
        .filter(|relationship| relationship.kind == *kind && relationship.source_id == *source_id && relationship.target_id == *target_id)
        .filter(|relationship| {
            if map.relationship_identity != SpreadsheetRelationshipIdentityPolicy::KindSourceTargetAssociationEnds
                || *kind != RelationshipKind::Association
            {
                return true;
            }
            relationship.association_ends.len() >= 2
                && end_fields_match(&relationship.association_ends[0], source_end)
                && end_fields_match(&relationship.association_ends[1], target_end)
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Ok(None),
        [relationship] => Ok(Some(*relationship)),
        _ => Err(diagnostic(
            Some(map), None, None, None, None, "AMBIGUOUS_RELATIONSHIP",
            format!("configured fallback identity matches {} {:?} relationships", matches.len(), kind),
        )),
    }
}

fn fallback_relationship_external_id(
    policy: SpreadsheetRelationshipIdentityPolicy,
    kind: &RelationshipKind,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
) -> String {
    let source = non_empty_value(values, SpreadsheetSemanticProperty::Source).unwrap_or_default();
    let target = non_empty_value(values, SpreadsheetSemanticProperty::Target).unwrap_or_default();
    let mut identity = format!("fallback::{kind:?}::{source}=>{target}");
    if policy == SpreadsheetRelationshipIdentityPolicy::KindSourceTargetAssociationEnds
        && *kind == RelationshipKind::Association
    {
        for property in [
            SpreadsheetSemanticProperty::SourceEndRole, SpreadsheetSemanticProperty::TargetEndRole,
            SpreadsheetSemanticProperty::SourceMultiplicity, SpreadsheetSemanticProperty::TargetMultiplicity,
            SpreadsheetSemanticProperty::SourceNavigable, SpreadsheetSemanticProperty::TargetNavigable,
            SpreadsheetSemanticProperty::SourceAggregation, SpreadsheetSemanticProperty::TargetAggregation,
        ] {
            identity.push('|');
            identity.push_str(non_empty_value(values, property).unwrap_or_default());
        }
    }
    identity
}

fn relationship_reference_matches(reference: &ElementReference, existing: ElementId) -> bool {
    matches!(reference, BuildReference::Existing(id) if *id == existing)
}

fn association_end_changed(
    end: &systems_modeler_core::AssociationEnd,
    fields: &Option<AssociationEndBuildFields>,
) -> bool {
    !end_fields_match(end, fields)
}

#[allow(clippy::too_many_arguments)]
fn relationship_field_changes(
    map: &SpreadsheetImportMap,
    row: usize,
    relationship: &Relationship,
    effective_external_id: &str,
    source: &ResolvedOwner,
    target: &ResolvedOwner,
    owner: &ResolvedOwner,
    source_end: Option<AssociationEndBuildFields>,
    target_end: Option<AssociationEndBuildFields>,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
) -> Result<(bool, ModelBuildOperation), SpreadsheetImportDiagnostic> {
    let name = values.get(&SpreadsheetSemanticProperty::Name).cloned();
    let documentation = values.get(&SpreadsheetSemanticProperty::Documentation).cloned();
    let visibility = values.get(&SpreadsheetSemanticProperty::Visibility)
        .map(|value| parse_visibility(map, row, value)).transpose()?;
    let external_id_explicit = non_empty_value(values, SpreadsheetSemanticProperty::ExternalId).is_some();
    let external_changed = relationship.external_id != external_key(&map.source_namespace, effective_external_id);
    let owner_changed = !relationship_reference_matches(&owner.reference, relationship.owner_id.unwrap_or(project_sentinel_element_id()));
    let source_changed = !relationship_reference_matches(&source.reference, relationship.source_id);
    let target_changed = !relationship_reference_matches(&target.reference, relationship.target_id);
    let association_changed = if relationship.kind == RelationshipKind::Association {
        relationship.association_ends.len() < 2
            || relationship.association_ends.get(0).is_some_and(|end| association_end_changed(end, &source_end))
            || relationship.association_ends.get(1).is_some_and(|end| association_end_changed(end, &target_end))
    } else {
        false
    };
    let changed = source_changed || target_changed || owner_changed || association_changed
        || name.as_ref().is_some_and(|value| relationship.name != *value)
        || documentation.as_ref().is_some_and(|value| relationship.documentation != *value)
        || visibility.is_some_and(|value| relationship.visibility != value)
        || external_changed;
    Ok((changed, ModelBuildOperation::UpdateRelationshipFields {
        relationship: BuildReference::Existing(relationship.id),
        name,
        owner: Some(owner.reference.clone()),
        source: Some(source.reference.clone()),
        target: Some(target.reference.clone()),
        external_id: (external_changed || external_id_explicit).then(|| effective_external_id.to_string()),
        documentation,
        visibility,
        source_end,
        target_end,
    }))
}

fn project_sentinel_element_id() -> ElementId {
    ElementId(uuid::Uuid::nil())
}

struct RelationshipRowPlan {
    action: SpreadsheetRowAction,
    operations: Vec<ModelBuildOperation>,
}

fn plan_relationship_row(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    project: &Project,
    planned: &[PlannedElement],
    seen_source_external_ids: &mut HashSet<String>,
) -> Result<RelationshipRowPlan, SpreadsheetImportDiagnostic> {
    let kind = relationship_kind_for_row(map, row, values)?;
    let source = resolve_relationship_endpoint(map, project, planned, values, SpreadsheetSemanticProperty::Source, "Source")?;
    let target = resolve_relationship_endpoint(map, project, planned, values, SpreadsheetSemanticProperty::Target, "Target")?;
    let owner_text = non_empty_value(values, SpreadsheetSemanticProperty::Owner).ok_or_else(|| diagnostic(
        Some(map), Some(row), mapped_column_name(map, SpreadsheetSemanticProperty::Owner),
        Some(SpreadsheetSemanticProperty::Owner), None, "RELATIONSHIP_OWNER_REQUIRED",
        "PR40 requires an explicit semantic Owner because model-core has no automatic owner rule for these relationship kinds",
    ))?;
    let owner = resolve_owner(map, project, planned, Some(owner_text))?;

    let source_end = parse_end_fields(
        map, row, values,
        SpreadsheetSemanticProperty::SourceEndRole, SpreadsheetSemanticProperty::SourceMultiplicity,
        SpreadsheetSemanticProperty::SourceNavigable, SpreadsheetSemanticProperty::SourceAggregation,
    )?;
    let target_end = parse_end_fields(
        map, row, values,
        SpreadsheetSemanticProperty::TargetEndRole, SpreadsheetSemanticProperty::TargetMultiplicity,
        SpreadsheetSemanticProperty::TargetNavigable, SpreadsheetSemanticProperty::TargetAggregation,
    )?;
    if kind != RelationshipKind::Association {
        let association_value_present = [
            SpreadsheetSemanticProperty::SourceEndRole, SpreadsheetSemanticProperty::TargetEndRole,
            SpreadsheetSemanticProperty::SourceMultiplicity, SpreadsheetSemanticProperty::TargetMultiplicity,
            SpreadsheetSemanticProperty::SourceNavigable, SpreadsheetSemanticProperty::TargetNavigable,
            SpreadsheetSemanticProperty::SourceAggregation, SpreadsheetSemanticProperty::TargetAggregation,
        ].into_iter().any(|property| non_empty_value(values, property).is_some());
        if association_value_present {
            return Err(diagnostic(
                Some(map), Some(row), None, None, None, "ASSOCIATION_FIELD_INVALID",
                format!("Association-end values cannot be applied to {:?}", kind),
            ));
        }
    }

    if let Some(value) = non_empty_value(values, SpreadsheetSemanticProperty::RelationshipKind) {
        let parsed = parse_relationship_kind_value(map, row, value)?;
        if parsed != kind {
            return Err(diagnostic(
                Some(map), Some(row), mapped_column_name(map, SpreadsheetSemanticProperty::RelationshipKind),
                Some(SpreadsheetSemanticProperty::RelationshipKind), Some(value.to_string()),
                "RELATIONSHIP_KIND_MISMATCH", format!("mapped kind {:?} does not match row {:?}", kind, parsed),
            ));
        }
    }

    let explicit_external_id = non_empty_value(values, SpreadsheetSemanticProperty::ExternalId);
    if explicit_external_id.is_none() && map.relationship_identity == SpreadsheetRelationshipIdentityPolicy::ExternalId {
        return Err(diagnostic(
            Some(map), Some(row), mapped_column_name(map, SpreadsheetSemanticProperty::ExternalId),
            Some(SpreadsheetSemanticProperty::ExternalId), None, "RELATIONSHIP_EXTERNAL_ID_REQUIRED",
            "relationship External ID is blank and no fallback relationship identity policy was configured",
        ));
    }
    let effective_external_id = explicit_external_id
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| fallback_relationship_external_id(map.relationship_identity, &kind, values));
    let key = external_key(&map.source_namespace, &effective_external_id);
    if !seen_source_external_ids.insert(key.clone()) {
        return Err(diagnostic(
            Some(map), Some(row), mapped_column_name(map, SpreadsheetSemanticProperty::ExternalId),
            Some(SpreadsheetSemanticProperty::ExternalId), Some(effective_external_id),
            "DUPLICATE_SOURCE_EXTERNAL_ID", format!("source relationship identity '{key}' appears more than once in this import group"),
        ));
    }

    let existing = if let Some(external_id) = explicit_external_id {
        find_relationship_by_external_id(map, project, external_id, &kind)?
    } else {
        let synthesized = find_relationship_by_external_id(map, project, &effective_external_id, &kind)?;
        if synthesized.is_some() {
            synthesized
        } else {
            find_relationship_by_fallback(map, project, &kind, &source, &target, &source_end, &target_end)?
        }
    };

    if let Some(existing) = existing {
        let (changed, operation) = relationship_field_changes(
            map, row, existing, &effective_external_id, &source, &target, &owner,
            source_end, target_end, values,
        )?;
        return Ok(RelationshipRowPlan {
            action: if changed { SpreadsheetRowAction::Update } else { SpreadsheetRowAction::NoChange },
            operations: changed.then_some(operation).into_iter().collect(),
        });
    }

    let mut operations = vec![ModelBuildOperation::CreateRelationship {
        external_id: effective_external_id.clone(),
        kind: kind.clone(),
        source: source.reference.clone(),
        target: target.reference.clone(),
        owner: Some(owner.reference.clone()),
    }];
    let name = values.get(&SpreadsheetSemanticProperty::Name).cloned();
    let documentation = values.get(&SpreadsheetSemanticProperty::Documentation).cloned();
    let visibility = values.get(&SpreadsheetSemanticProperty::Visibility)
        .map(|value| parse_visibility(map, row, value)).transpose()?;
    if name.is_some() || documentation.is_some() || visibility.is_some()
        || source_end.is_some() || target_end.is_some()
    {
        operations.push(ModelBuildOperation::UpdateRelationshipFields {
            relationship: RelationshipReference::External(effective_external_id),
            name,
            owner: None,
            source: None,
            target: None,
            external_id: None,
            documentation,
            visibility,
            source_end,
            target_end,
        });
    }
    Ok(RelationshipRowPlan { action: SpreadsheetRowAction::Create, operations })
}

'''
if helper_anchor not in text:
    raise SystemExit("relationship helper insertion anchor missing")
text = text.replace(helper_anchor, relationship_helpers + helper_anchor, 1)

# Relationship-aware identification/preview/context helpers.
text = replace_once(
    text,
    '''fn identification_value(\n    map: &SpreadsheetImportMap,\n    values: &BTreeMap<SpreadsheetSemanticProperty, String>,\n) -> Option<String> {\n    let property = match map.identification_property {\n''',
    '''fn identification_value(\n    map: &SpreadsheetImportMap,\n    values: &BTreeMap<SpreadsheetSemanticProperty, String>,\n) -> Option<String> {\n    if is_relationship_map(map) {\n        return non_empty_value(values, SpreadsheetSemanticProperty::ExternalId)\n            .map(ToOwned::to_owned)\n            .or_else(|| {\n                let source = non_empty_value(values, SpreadsheetSemanticProperty::Source)?;\n                let target = non_empty_value(values, SpreadsheetSemanticProperty::Target)?;\n                Some(format!("{source} -> {target}"))\n            });\n    }\n    let property = match map.identification_property {\n''',
    "relationship identification preview",
)

text = replace_once(
    text,
    '''        row,\n        element_kind: map.element_kind.clone(),\n        identification_value: identification_value(map, values),\n        action,\n''',
    '''        row,\n        element_kind: map.element_kind.clone(),\n        relationship_kind: if is_relationship_map(map) {\n            relationship_kind_for_row(map, row, values).ok()\n        } else {\n            None\n        },\n        identification_value: identification_value(map, values),\n        action,\n''',
    "relationship row preview value",
)

text = replace_once(
    text,
    '''        row,\n        element_kind: map.element_kind.clone(),\n        source_namespace: map.source_namespace.clone(),\n        identification_value: identification_value(map, values),\n''',
    '''        row,\n        element_kind: map.element_kind.clone(),\n        relationship_kind: if is_relationship_map(map) {\n            relationship_kind_for_row(map, row, values).ok()\n        } else {\n            None\n        },\n        source_endpoint: non_empty_value(values, SpreadsheetSemanticProperty::Source).map(ToOwned::to_owned),\n        target_endpoint: non_empty_value(values, SpreadsheetSemanticProperty::Target).map(ToOwned::to_owned),\n        source_namespace: map.source_namespace.clone(),\n        identification_value: identification_value(map, values),\n''',
    "relationship row context values",
)

# Handle relationship rows before the legacy element path while sharing the same plan and planned elements.
row_anchor = '''            if let Some(external_id) =\n                non_empty_value(&values, SpreadsheetSemanticProperty::ExternalId)\n            {\n'''
relationship_row_branch = '''            if is_relationship_map(map) {\n                match plan_relationship_row(\n                    map,\n                    row.row_number,\n                    &values,\n                    project,\n                    &planned,\n                    &mut seen_source_external_ids,\n                ) {\n                    Ok(planned_relationship) => {\n                        preview.rows.push(row_preview(\n                            map,\n                            row.row_number,\n                            &values,\n                            planned_relationship.action,\n                        ));\n                        let context = row_context(map, row.row_number, &values);\n                        for operation in planned_relationship.operations {\n                            operations.push(operation);\n                            operation_contexts.push(context.clone());\n                        }\n                    }\n                    Err(mut error) => {\n                        error.row = Some(row.row_number);\n                        error.relationship_kind = relationship_kind_for_row(map, row.row_number, &values).ok();\n                        error.source_endpoint = non_empty_value(&values, SpreadsheetSemanticProperty::Source).map(ToOwned::to_owned);\n                        error.target_endpoint = non_empty_value(&values, SpreadsheetSemanticProperty::Target).map(ToOwned::to_owned);\n                        block_row(error);\n                    }\n                }\n                continue;\n            }\n\n'''
if row_anchor not in text:
    raise SystemExit("relationship row insertion anchor missing")
text = text.replace(row_anchor, relationship_row_branch + row_anchor, 1)

# Build diagnostics include relationship context when available.
text = replace_once(
    text,
    '''        element_kind: context.map(|context| context.element_kind.clone()),\n        source_namespace: context.map(|context| context.source_namespace.clone()),\n''',
    '''        element_kind: context.map(|context| context.element_kind.clone()),\n        relationship_kind: context.and_then(|context| context.relationship_kind.clone()),\n        source_endpoint: context.and_then(|context| context.source_endpoint.clone()),\n        target_endpoint: context.and_then(|context| context.target_endpoint.clone()),\n        source_namespace: context.map(|context| context.source_namespace.clone()),\n''',
    "build diagnostic relationship context",
)

# Backwards-compatible existing test helper fields.
text = replace_once(
    text,
    '''            element_kind: kind,\n            target_scope: target,\n''',
    '''            element_kind: kind,\n            relationship_kind: None,\n            relationship_identity: SpreadsheetRelationshipIdentityPolicy::ExternalId,\n            target_scope: target,\n''',
    "existing test map relationship defaults",
)

path.write_text(text, encoding="utf-8")
print("PR40 core patch applied")
