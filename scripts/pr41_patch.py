from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
path = ROOT / "apps/desktop/src-tauri/src/workspace/spreadsheet_import.rs"
text = path.read_text(encoding="utf-8")


def replace_once(old: str, new: str, label: str) -> None:
    global text
    if old not in text:
        raise SystemExit(f"missing anchor: {label}")
    text = text.replace(old, new, 1)


replace_once(
    '''struct PlannedElement {\n    external_id: String,\n    kind: ElementKind,\n    qualified_name: String,\n    depth_from_target: usize,\n}\n''',
    '''struct PlannedElement {\n    external_id: String,\n    kind: ElementKind,\n    qualified_name: String,\n    requirement_id: Option<String>,\n    depth_from_target: usize,\n}\n''',
    "planned requirement identity",
)

replace_once(
    '''fn supported_relationship_kind(kind: &RelationshipKind) -> bool {\n    matches!(\n        kind,\n        RelationshipKind::Association\n            | RelationshipKind::Generalization\n            | RelationshipKind::Dependency\n            | RelationshipKind::Realization\n    )\n}\n''',
    '''fn supported_relationship_kind(kind: &RelationshipKind) -> bool {\n    matches!(\n        kind,\n        RelationshipKind::Association\n            | RelationshipKind::Generalization\n            | RelationshipKind::Dependency\n            | RelationshipKind::Realization\n            | RelationshipKind::DeriveRequirement\n            | RelationshipKind::Satisfy\n            | RelationshipKind::Verify\n            | RelationshipKind::Refine\n            | RelationshipKind::Trace\n            | RelationshipKind::Copy\n    )\n}\n\nfn is_pr41_traceability_kind(kind: &RelationshipKind) -> bool {\n    matches!(\n        kind,\n        RelationshipKind::DeriveRequirement\n            | RelationshipKind::Satisfy\n            | RelationshipKind::Verify\n            | RelationshipKind::Refine\n            | RelationshipKind::Trace\n            | RelationshipKind::Copy\n    )\n}\n''',
    "supported PR41 relationship kinds",
)

replace_once(
    '''        for property in [\n            SpreadsheetSemanticProperty::Source,\n            SpreadsheetSemanticProperty::Target,\n            SpreadsheetSemanticProperty::Owner,\n        ] {\n''',
    '''        for property in [\n            SpreadsheetSemanticProperty::Source,\n            SpreadsheetSemanticProperty::Target,\n        ] {\n''',
    "relationship owner column optional at map level",
)

replace_once(
    '''                format!("{:?} is outside the PR40 relationship scope", kind),\n''',
    '''                format!("{:?} is outside the PR40/PR41 relationship scope", kind),\n''',
    "configured relationship scope diagnostic",
)

replace_once(
    '''    label: &str,\n) -> Result<ResolvedOwner, SpreadsheetImportDiagnostic> {\n''',
    '''    label: &str,\n    allow_requirement_id: bool,\n) -> Result<ResolvedOwner, SpreadsheetImportDiagnostic> {\n''',
    "semantic reference requirement mode",
)

qname_anchor = '''    let mut existing = project\n        .elements\n        .values()\n        .filter(|element| {\n            reference_in_scope(project, element.id, map.target_scope, map.search_scope)\n        })\n        .filter_map(|element| {\n'''
requirement_block = '''    if allow_requirement_id {\n        let mut existing_requirement = project\n            .elements\n            .values()\n            .filter(|element| {\n                reference_in_scope(project, element.id, map.target_scope, map.search_scope)\n            })\n            .filter(|element| {\n                element.kind == ElementKind::Requirement\n                    && element.requirement_id.as_deref() == Some(requested)\n            })\n            .collect::<Vec<_>>();\n        existing_requirement.sort_by_key(|element| element.id.to_string());\n        existing_requirement.dedup_by_key(|element| element.id);\n        let mut pending_requirement = planned\n            .iter()\n            .filter(|element| match map.search_scope {\n                SpreadsheetSearchScope::TargetOnly => element.depth_from_target == 1,\n                SpreadsheetSearchScope::TargetRecursive => element.depth_from_target >= 1,\n            })\n            .filter(|element| {\n                element.kind == ElementKind::Requirement\n                    && element.requirement_id.as_deref() == Some(requested)\n            })\n            .collect::<Vec<_>>();\n        pending_requirement.sort_by(|left, right| left.external_id.cmp(&right.external_id));\n        pending_requirement.dedup_by(|left, right| left.external_id == right.external_id);\n        match (\n            existing_requirement.as_slice(),\n            pending_requirement.as_slice(),\n        ) {\n            ([element], []) => {\n                let qualified_name = project\n                    .qualified_name(element.id)\n                    .unwrap_or_else(|_| element.name.clone());\n                return Ok(ResolvedOwner {\n                    reference: BuildReference::Existing(element.id),\n                    qualified_name,\n                    kind: element.kind.clone(),\n                    depth_from_target: distance_from_target(\n                        project,\n                        element.id,\n                        map.target_scope,\n                    )\n                    .unwrap_or(0),\n                });\n            }\n            ([], [element]) => {\n                return Ok(ResolvedOwner {\n                    reference: BuildReference::External(element.external_id.clone()),\n                    qualified_name: element.qualified_name.clone(),\n                    kind: element.kind.clone(),\n                    depth_from_target: element.depth_from_target,\n                });\n            }\n            ([], []) => {}\n            _ => {\n                return Err(diagnostic(\n                    Some(map),\n                    None,\n                    mapped_column_name(map, property),\n                    Some(property),\n                    Some(requested.into()),\n                    "AMBIGUOUS_REQUIREMENT_ID",\n                    format!(\n                        "Requirement ID '{requested}' resolves to {} Requirements within the configured search scope; use External ID or a more specific identifier",\n                        existing_requirement.len() + pending_requirement.len()\n                    ),\n                ));\n            }\n        }\n    }\n\n'''
replace_once(qname_anchor, requirement_block + qname_anchor, "Requirement ID endpoint lookup")

replace_once(
    '''                "{label} '{requested}' could not be resolved by namespaced External ID or exact qualified name within {:?} search scope",\n                map.search_scope\n''',
    '''                "{label} '{requested}' could not be resolved by {} within {:?} search scope",\n                if allow_requirement_id {\n                    "namespaced External ID, exact Requirement ID, or exact qualified name"\n                } else {\n                    "namespaced External ID or exact qualified name"\n                },\n                map.search_scope\n''',
    "endpoint unresolved modes diagnostic",
)

replace_once(
    '''        SpreadsheetSemanticProperty::Owner,\n        "Owner",\n    )\n}\n''',
    '''        SpreadsheetSemanticProperty::Owner,\n        "Owner",\n        false,\n    )\n}\n''',
    "owner reference mode",
)
replace_once(
    '''        SpreadsheetSemanticProperty::Type,\n        "Type",\n    )\n}\n''',
    '''        SpreadsheetSemanticProperty::Type,\n        "Type",\n        false,\n    )\n}\n''',
    "type reference mode",
)

replace_once(
    '''    values: &BTreeMap<SpreadsheetSemanticProperty, String>,\n    property: SpreadsheetSemanticProperty,\n    label: &str,\n) -> Result<ResolvedOwner, SpreadsheetImportDiagnostic> {\n''',
    '''    values: &BTreeMap<SpreadsheetSemanticProperty, String>,\n    kind: &RelationshipKind,\n    property: SpreadsheetSemanticProperty,\n    label: &str,\n) -> Result<ResolvedOwner, SpreadsheetImportDiagnostic> {\n''',
    "relationship endpoint kind-aware signature",
)
replace_once(
    '''    resolve_semantic_reference(map, project, planned, requested, property, label).map_err(\n        |mut error| {\n''',
    '''    let allow_requirement_id = match (kind, property) {\n        (\n            RelationshipKind::DeriveRequirement | RelationshipKind::Copy,\n            SpreadsheetSemanticProperty::Source | SpreadsheetSemanticProperty::Target,\n        ) => true,\n        (\n            RelationshipKind::Satisfy\n            | RelationshipKind::Verify\n            | RelationshipKind::Refine,\n            SpreadsheetSemanticProperty::Target,\n        ) => true,\n        (\n            RelationshipKind::Trace,\n            SpreadsheetSemanticProperty::Source | SpreadsheetSemanticProperty::Target,\n        ) => true,\n        _ => false,\n    };\n    resolve_semantic_reference(\n        map,\n        project,\n        planned,\n        requested,\n        property,\n        label,\n        allow_requirement_id,\n    )\n    .map_err(\n        |mut error| {\n''',
    "relationship Requirement ID resolution",
)

replace_once(
    '''        values,\n        SpreadsheetSemanticProperty::Source,\n        "Source",\n    )?;\n''',
    '''        values,\n        &kind,\n        SpreadsheetSemanticProperty::Source,\n        "Source",\n    )?;\n''',
    "source endpoint kind",
)
replace_once(
    '''        values,\n        SpreadsheetSemanticProperty::Target,\n        "Target",\n    )?;\n''',
    '''        values,\n        &kind,\n        SpreadsheetSemanticProperty::Target,\n        "Target",\n    )?;\n''',
    "target endpoint kind",
)

replace_once(
    '''    let owner_text = non_empty_value(values, SpreadsheetSemanticProperty::Owner).ok_or_else(|| diagnostic(\n        Some(map), Some(row), mapped_column_name(map, SpreadsheetSemanticProperty::Owner),\n        Some(SpreadsheetSemanticProperty::Owner), None, "RELATIONSHIP_OWNER_REQUIRED",\n        "PR40 requires an explicit semantic Owner because model-core has no automatic owner rule for these relationship kinds",\n    ))?;\n    let owner = resolve_owner(map, project, planned, Some(owner_text))?;\n''',
    '''    let owner = if let Some(owner_text) =\n        non_empty_value(values, SpreadsheetSemanticProperty::Owner)\n    {\n        resolve_owner(map, project, planned, Some(owner_text))?\n    } else if is_pr41_traceability_kind(&kind) {\n        let inferred = resolve_owner(map, project, planned, None)?;\n        if map.target_scope == project.root_id {\n            return Err(diagnostic(\n                Some(map),\n                Some(row),\n                mapped_column_name(map, SpreadsheetSemanticProperty::Owner),\n                Some(SpreadsheetSemanticProperty::Owner),\n                None,\n                "RELATIONSHIP_OWNER_REQUIRED",\n                "PR41 does not infer a loose root owner; map Owner explicitly or configure a package target scope that contains the relationship endpoints",\n            ));\n        }\n        inferred\n    } else {\n        return Err(diagnostic(\n            Some(map),\n            Some(row),\n            mapped_column_name(map, SpreadsheetSemanticProperty::Owner),\n            Some(SpreadsheetSemanticProperty::Owner),\n            None,\n            "RELATIONSHIP_OWNER_REQUIRED",\n            "PR40 core relationships still require an explicit semantic Owner",\n        ));\n    };\n''',
    "traceability owner handling",
)

replace_once(
    '''        "realization" => RelationshipKind::Realization,\n        _ => {\n''',
    '''        "realization" => RelationshipKind::Realization,\n        "deriverequirement" | "derivereqt" => RelationshipKind::DeriveRequirement,\n        "satisfy" => RelationshipKind::Satisfy,\n        "verify" => RelationshipKind::Verify,\n        "refine" => RelationshipKind::Refine,\n        "trace" => RelationshipKind::Trace,\n        "copy" => RelationshipKind::Copy,\n        _ => {\n''',
    "PR41 relationship aliases",
)
replace_once(
    '''                    "relationship kind '{}' is outside PR40; expected Association, Generalization, Dependency, or Realization",\n                    value.trim()\n''',
    '''                    "relationship kind '{}' is outside PR40/PR41; expected Association, Generalization, Dependency, Realization, DeriveRequirement/deriveReqt, Satisfy, Verify, Refine, Trace, or Copy",\n                    value.trim()\n''',
    "relationship kind error list",
)

replace_once(
    '''            planned.push(PlannedElement {\n                external_id: external_id.to_string(),\n                kind: map.element_kind.clone(),\n                qualified_name,\n                depth_from_target: owner.depth_from_target + 1,\n            });\n''',
    '''            planned.push(PlannedElement {\n                external_id: external_id.to_string(),\n                kind: map.element_kind.clone(),\n                qualified_name,\n                requirement_id: (map.element_kind == ElementKind::Requirement)\n                    .then(|| {\n                        non_empty_value(&values, SpreadsheetSemanticProperty::RequirementId)\n                            .map(ToOwned::to_owned)\n                    })\n                    .flatten(),\n                depth_from_target: owner.depth_from_target + 1,\n            });\n''',
    "plan-local Requirement ID",
)

path.write_text(text, encoding="utf-8")
