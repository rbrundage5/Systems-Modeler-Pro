use super::{
    WorkspaceState,
    activity_workspace::ActivityWorkspaceState,
    bulk_model::{
        BuildDiagnosticSeverity, BuildReference, ModelBuildOperation, ModelBuildPlan, external_key,
    },
    history::{self, HistoryState},
    parse_element_id,
    reqif_interchange::{
        REQIF_METADATA_KEY, ReqifAction, ReqifAttributeDefinition, ReqifAttributeValue,
        ReqifDatatype, ReqifDatatypeKind, ReqifDiagnostic, ReqifDiagnosticSeverity, ReqifDocument,
        ReqifExchangeState, ReqifHierarchyNode, ReqifImportConfiguration, ReqifImportPreview,
        ReqifNativeField, ReqifPreviewItem, ReqifSourceState, ReqifSpecObject, ReqifSpecRelation,
        ReqifSpecType, ReqifSpecTypeKind, ReqifSynchronizationPolicy, ReqifValue,
        detected_attribute_mapping, detected_object_kind, detected_relation_kind, parse_reqif,
        serialize_reqif,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};
use systems_modeler_core::{
    Element, ElementId, ElementKind, Project, Relationship, RelationshipId, RelationshipKind,
};
use systems_modeler_persistence::ProjectDatabase;

const MAX_REQIF_BYTES: u64 = 64 * 1024 * 1024;
const GENERATED_STRING_DATATYPE: &str = "SM-DT-STRING";
const GENERATED_REQUIREMENT_TYPE: &str = "SM-T-REQUIREMENT";
const GENERATED_TESTCASE_TYPE: &str = "SM-T-TESTCASE";
const GENERATED_SPECIFICATION_TYPE: &str = "SM-T-SPECIFICATION";
const GENERATED_NAME_ATTRIBUTE: &str = "SM-A-NAME";
const GENERATED_REQUIREMENT_ID_ATTRIBUTE: &str = "SM-A-REQUIREMENT-ID";
const GENERATED_REQUIREMENT_TEXT_ATTRIBUTE: &str = "SM-A-REQUIREMENT-TEXT";
const GENERATED_DOCUMENTATION_ATTRIBUTE: &str = "SM-A-DOCUMENTATION";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReqifFilePayload {
    pub file_name: String,
    pub xml: String,
}

#[derive(Debug, Clone)]
struct MappedObjectFields {
    name: String,
    documentation: Option<String>,
    requirement_id: Option<String>,
    requirement_text: Option<String>,
}

#[derive(Debug, Clone)]
struct PreparedReqifImport {
    document: ReqifDocument,
    plan: ModelBuildPlan,
    preview: ReqifImportPreview,
    remove_elements: Vec<ElementId>,
    remove_relationships: Vec<RelationshipId>,
    configuration: ReqifImportConfiguration,
}

fn diagnostic(
    severity: ReqifDiagnosticSeverity,
    code: &str,
    reason: impl Into<String>,
    identifier: Option<&str>,
    kind: Option<&str>,
) -> ReqifDiagnostic {
    ReqifDiagnostic {
        severity,
        file: None,
        identifier: identifier.map(Into::into),
        record_kind: kind.map(Into::into),
        attribute_or_type: None,
        source: None,
        target: None,
        native_target: None,
        code: code.into(),
        reason: reason.into(),
    }
}

fn blocked_item(identifier: &str, kind: &str, detail: impl Into<String>) -> ReqifPreviewItem {
    ReqifPreviewItem {
        action: ReqifAction::Blocked,
        identifier: identifier.into(),
        kind: kind.into(),
        detail: detail.into(),
    }
}

fn find_element_by_external<'a>(project: &'a Project, key: &str) -> Vec<&'a Element> {
    project
        .elements
        .values()
        .filter(|element| element.external_id == key)
        .collect()
}

fn find_relationship_by_external<'a>(
    project: &'a Project,
    key: &str,
) -> Vec<&'a Relationship> {
    project
        .relationships
        .values()
        .filter(|relationship| relationship.external_id == key)
        .collect()
}

fn config_object_kind(
    document: &ReqifDocument,
    configuration: &ReqifImportConfiguration,
    type_identifier: &str,
) -> Option<ElementKind> {
    let spec_type = document
        .spec_types
        .iter()
        .find(|item| item.identifier == type_identifier);
    configuration
        .object_type_mappings
        .get(type_identifier)
        .cloned()
        .or_else(|| {
            spec_type.and_then(|item| configuration.object_type_mappings.get(&item.long_name).cloned())
        })
        .or_else(|| detected_object_kind(document, type_identifier))
}

fn config_relation_kind(
    document: &ReqifDocument,
    configuration: &ReqifImportConfiguration,
    type_identifier: &str,
) -> Option<RelationshipKind> {
    let spec_type = document
        .spec_types
        .iter()
        .find(|item| item.identifier == type_identifier);
    configuration
        .relation_type_mappings
        .get(type_identifier)
        .cloned()
        .or_else(|| {
            spec_type.and_then(|item| configuration.relation_type_mappings.get(&item.long_name).cloned())
        })
        .or_else(|| detected_relation_kind(document, type_identifier))
}

fn configured_field(
    configuration: &ReqifImportConfiguration,
    definition: &ReqifAttributeDefinition,
) -> Option<ReqifNativeField> {
    configuration
        .attribute_mappings
        .get(&definition.identifier)
        .copied()
        .or_else(|| configuration.attribute_mappings.get(&definition.long_name).copied())
        .or_else(|| detected_attribute_mapping(definition))
}

fn mapped_object_fields(
    document: &ReqifDocument,
    configuration: &ReqifImportConfiguration,
    object: &ReqifSpecObject,
) -> MappedObjectFields {
    let definitions = document.attribute_definitions();
    let labels = document.enum_labels();
    let mut name = (!object.long_name.trim().is_empty()).then(|| object.long_name.clone());
    let mut documentation = None;
    let mut requirement_id = None;
    let mut requirement_text = None;
    for attribute in &object.values {
        let Some(definition) = definitions.get(&attribute.definition_identifier) else {
            continue;
        };
        let Some(field) = configured_field(configuration, definition) else {
            continue;
        };
        let value = attribute.value.readable_text(&labels);
        match field {
            ReqifNativeField::RequirementId => {
                requirement_id = (!value.trim().is_empty()).then_some(value)
            }
            ReqifNativeField::RequirementText => requirement_text = Some(value),
            ReqifNativeField::Name => name = (!value.trim().is_empty()).then_some(value),
            ReqifNativeField::Documentation => documentation = Some(value),
        }
    }
    MappedObjectFields {
        name: name.unwrap_or_else(|| object.identifier.clone()),
        documentation,
        requirement_id,
        requirement_text,
    }
}

fn object_needs_update(element: &Element, fields: &MappedObjectFields) -> bool {
    element.name != fields.name
        || fields
            .documentation
            .as_ref()
            .is_some_and(|value| element.documentation != *value)
        || fields
            .requirement_id
            .as_ref()
            .is_some_and(|value| element.requirement_id.as_ref() != Some(value))
        || fields
            .requirement_text
            .as_ref()
            .is_some_and(|value| element.requirement_text.as_ref() != Some(value))
}

fn relationship_needs_update(
    relationship: &Relationship,
    relation: &ReqifSpecRelation,
    source_id: ElementId,
    target_id: ElementId,
) -> bool {
    relationship.name != relation.long_name
        || relationship.source_id != source_id
        || relationship.target_id != target_id
}

fn resolve_live_object_id(
    project: &Project,
    namespace: &str,
    identifier: &str,
) -> Option<ElementId> {
    let key = external_key(namespace, identifier);
    let matches = find_element_by_external(project, &key);
    (matches.len() == 1).then_some(matches[0].id)
}

fn prepare_reqif_import(
    document: ReqifDocument,
    mut configuration: ReqifImportConfiguration,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
) -> PreparedReqifImport {
    let mut preview = ReqifImportPreview {
        applied: false,
        source_namespace: configuration.source_namespace.clone(),
        ..ReqifImportPreview::default()
    };
    if configuration.source_namespace.trim().is_empty() {
        preview.diagnostics.push(diagnostic(
            ReqifDiagnosticSeverity::Error,
            "SOURCE_NAMESPACE_REQUIRED",
            "ReqIF source_namespace is required and must not be derived solely from a temporary file path",
            None,
            None,
        ));
    }
    configuration.source_namespace = configuration.source_namespace.trim().to_owned();

    let project_guard = match workspace.project.lock() {
        Ok(guard) => guard,
        Err(_) => {
            preview.diagnostics.push(diagnostic(
                ReqifDiagnosticSeverity::Error,
                "SEMANTIC_VALIDATION",
                "project lock poisoned",
                None,
                None,
            ));
            preview.recount();
            return PreparedReqifImport {
                document,
                plan: ModelBuildPlan {
                    source_namespace: configuration.source_namespace.clone(),
                    operations: Vec::new(),
                },
                preview,
                remove_elements: Vec::new(),
                remove_relationships: Vec::new(),
                configuration,
            };
        }
    };
    let Some(project) = project_guard.as_ref() else {
        preview.diagnostics.push(diagnostic(
            ReqifDiagnosticSeverity::Error,
            "SEMANTIC_VALIDATION",
            "no project open",
            None,
            None,
        ));
        preview.recount();
        return PreparedReqifImport {
            document,
            plan: ModelBuildPlan {
                source_namespace: configuration.source_namespace.clone(),
                operations: Vec::new(),
            },
            preview,
            remove_elements: Vec::new(),
            remove_relationships: Vec::new(),
            configuration,
        };
    };
    let target_scope = match parse_element_id(&configuration.target_scope) {
        Ok(id) => id,
        Err(reason) => {
            preview.diagnostics.push(diagnostic(
                ReqifDiagnosticSeverity::Error,
                "SEMANTIC_VALIDATION",
                reason,
                None,
                None,
            ));
            project.root_id
        }
    };
    if let Ok(owner) = project.element(target_scope) {
        if !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {
            preview.diagnostics.push(diagnostic(
                ReqifDiagnosticSeverity::Error,
                "SEMANTIC_VALIDATION",
                "ReqIF target scope must be an existing Model or Package",
                None,
                None,
            ));
        }
    } else {
        preview.diagnostics.push(diagnostic(
            ReqifDiagnosticSeverity::Error,
            "SEMANTIC_VALIDATION",
            "ReqIF target scope does not exist",
            None,
            None,
        ));
    }

    let object_ids = document
        .spec_objects
        .iter()
        .map(|object| object.identifier.clone())
        .collect::<BTreeSet<_>>();
    if object_ids.len() != document.spec_objects.len() {
        preview.diagnostics.push(diagnostic(
            ReqifDiagnosticSeverity::Error,
            "REQIF_REFERENCE_UNRESOLVED",
            "duplicate SPEC-OBJECT IDENTIFIER values are not allowed",
            None,
            Some("SPEC-OBJECT"),
        ));
    }
    let relation_ids = document
        .spec_relations
        .iter()
        .map(|relation| relation.identifier.clone())
        .collect::<BTreeSet<_>>();
    if relation_ids.len() != document.spec_relations.len() {
        preview.diagnostics.push(diagnostic(
            ReqifDiagnosticSeverity::Error,
            "REQIF_REFERENCE_UNRESOLVED",
            "duplicate SPEC-RELATION IDENTIFIER values are not allowed",
            None,
            Some("SPEC-RELATION"),
        ));
    }

    let mut operations = Vec::new();
    let mut mapped_object_kinds = BTreeMap::new();
    for object in &document.spec_objects {
        let Some(kind) = config_object_kind(&document, &configuration, &object.type_identifier) else {
            preview.items.push(blocked_item(
                &object.identifier,
                "SPEC-OBJECT",
                "SPEC-OBJECT type requires an explicit Requirement/TestCase mapping",
            ));
            preview.diagnostics.push(ReqifDiagnostic {
                severity: ReqifDiagnosticSeverity::Error,
                file: None,
                identifier: Some(object.identifier.clone()),
                record_kind: Some("SPEC-OBJECT".into()),
                attribute_or_type: Some(object.type_identifier.clone()),
                source: None,
                target: None,
                native_target: None,
                code: "REQIF_TYPE_UNRESOLVED".into(),
                reason: "unsupported or unmapped SPEC-OBJECT type".into(),
            });
            continue;
        };
        if !matches!(kind, ElementKind::Requirement | ElementKind::TestCase) {
            preview.items.push(blocked_item(
                &object.identifier,
                "SPEC-OBJECT",
                "PR52 ReqIF object mappings are restricted to Requirement and TestCase",
            ));
            preview.diagnostics.push(diagnostic(
                ReqifDiagnosticSeverity::Error,
                "REQIF_TYPE_UNRESOLVED",
                "configured SPEC-OBJECT type is outside the PR52 Requirement/TestCase scope",
                Some(&object.identifier),
                Some("SPEC-OBJECT"),
            ));
            continue;
        }
        mapped_object_kinds.insert(object.identifier.clone(), kind.clone());
        let fields = mapped_object_fields(&document, &configuration, object);
        let key = external_key(&configuration.source_namespace, &object.identifier);
        let existing = find_element_by_external(project, &key);
        if existing.len() > 1 {
            preview.items.push(blocked_item(
                &object.identifier,
                "SPEC-OBJECT",
                "ReqIF identity is ambiguous in the native project",
            ));
            preview.diagnostics.push(diagnostic(
                ReqifDiagnosticSeverity::Error,
                "SEMANTIC_VALIDATION",
                "multiple native elements share this ReqIF external identity",
                Some(&object.identifier),
                Some("SPEC-OBJECT"),
            ));
            continue;
        }
        if let Some(existing) = existing.first() {
            if existing.kind != kind {
                preview.items.push(blocked_item(
                    &object.identifier,
                    "SPEC-OBJECT",
                    format!(
                        "wrong-kind collision: ReqIF maps to {kind:?}, native identity is {:?}",
                        existing.kind
                    ),
                ));
                preview.diagnostics.push(ReqifDiagnostic {
                    severity: ReqifDiagnosticSeverity::Error,
                    file: None,
                    identifier: Some(object.identifier.clone()),
                    record_kind: Some("SPEC-OBJECT".into()),
                    attribute_or_type: Some(object.type_identifier.clone()),
                    source: None,
                    target: None,
                    native_target: Some(existing.id.to_string()),
                    code: "WRONG_KIND_COLLISION".into(),
                    reason: "stable ReqIF identity already belongs to another native kind".into(),
                });
                continue;
            }
            let action = if object_needs_update(existing, &fields) {
                ReqifAction::Update
            } else {
                ReqifAction::NoChange
            };
            preview.items.push(ReqifPreviewItem {
                action,
                identifier: object.identifier.clone(),
                kind: format!("{:?}", kind),
                detail: existing.id.to_string(),
            });
            if action == ReqifAction::Update {
                operations.push(ModelBuildOperation::UpdateElementFields {
                    element: BuildReference::External(object.identifier.clone()),
                    name: Some(fields.name),
                    owner: None,
                    type_ref: None,
                    external_id: None,
                    documentation: fields.documentation,
                    visibility: None,
                    requirement_id: (kind == ElementKind::Requirement)
                        .then_some(fields.requirement_id)
                        .flatten(),
                    requirement_text: (kind == ElementKind::Requirement)
                        .then_some(fields.requirement_text)
                        .flatten(),
                    multiplicity: None,
                    default_value: None,
                    parameter_direction: None,
                    flow_direction: None,
                    is_conjugated: None,
                    extension_points: None,
                });
            }
        } else {
            preview.items.push(ReqifPreviewItem {
                action: ReqifAction::Create,
                identifier: object.identifier.clone(),
                kind: format!("{:?}", kind),
                detail: fields.name.clone(),
            });
            operations.push(ModelBuildOperation::CreateElement {
                external_id: object.identifier.clone(),
                kind: kind.clone(),
                name: fields.name.clone(),
                owner: BuildReference::Existing(target_scope),
                type_ref: None,
            });
            if fields.documentation.is_some()
                || (kind == ElementKind::Requirement
                    && (fields.requirement_id.is_some() || fields.requirement_text.is_some()))
            {
                operations.push(ModelBuildOperation::UpdateElementFields {
                    element: BuildReference::External(object.identifier.clone()),
                    name: None,
                    owner: None,
                    type_ref: None,
                    external_id: None,
                    documentation: fields.documentation,
                    visibility: None,
                    requirement_id: (kind == ElementKind::Requirement)
                        .then_some(fields.requirement_id)
                        .flatten(),
                    requirement_text: (kind == ElementKind::Requirement)
                        .then_some(fields.requirement_text)
                        .flatten(),
                    multiplicity: None,
                    default_value: None,
                    parameter_direction: None,
                    flow_direction: None,
                    is_conjugated: None,
                    extension_points: None,
                });
            }
        }
    }

    for relation in &document.spec_relations {
        let Some(kind) = config_relation_kind(&document, &configuration, &relation.type_identifier)
        else {
            preview.items.push(ReqifPreviewItem {
                action: ReqifAction::NoChange,
                identifier: relation.identifier.clone(),
                kind: "SPEC-RELATION".into(),
                detail: "preserved as ReqIF exchange metadata; no native relation mapping selected"
                    .into(),
            });
            preview.diagnostics.push(ReqifDiagnostic {
                severity: ReqifDiagnosticSeverity::Warning,
                file: None,
                identifier: Some(relation.identifier.clone()),
                record_kind: Some("SPEC-RELATION".into()),
                attribute_or_type: Some(relation.type_identifier.clone()),
                source: Some(relation.source_identifier.clone()),
                target: Some(relation.target_identifier.clone()),
                native_target: None,
                code: "REQIF_RELATION_MAPPING_REQUIRED".into(),
                reason: "relation type is preserved but not converted to a native relationship"
                    .into(),
            });
            continue;
        };
        if !matches!(
            kind,
            RelationshipKind::Trace
                | RelationshipKind::DeriveRequirement
                | RelationshipKind::Satisfy
                | RelationshipKind::Verify
                | RelationshipKind::Refine
                | RelationshipKind::Copy
                | RelationshipKind::Dependency
        ) {
            preview.items.push(blocked_item(
                &relation.identifier,
                "SPEC-RELATION",
                "configured relation kind is outside the PR52 traceability scope",
            ));
            preview.diagnostics.push(diagnostic(
                ReqifDiagnosticSeverity::Error,
                "REQIF_RELATION_MAPPING_REQUIRED",
                "configured relation kind is not supported by PR52",
                Some(&relation.identifier),
                Some("SPEC-RELATION"),
            ));
            continue;
        }
        if !mapped_object_kinds.contains_key(&relation.source_identifier)
            || !mapped_object_kinds.contains_key(&relation.target_identifier)
        {
            preview.items.push(blocked_item(
                &relation.identifier,
                "SPEC-RELATION",
                "mapped relation endpoint is not mapped to a native SPEC-OBJECT",
            ));
            preview.diagnostics.push(ReqifDiagnostic {
                severity: ReqifDiagnosticSeverity::Error,
                file: None,
                identifier: Some(relation.identifier.clone()),
                record_kind: Some("SPEC-RELATION".into()),
                attribute_or_type: Some(relation.type_identifier.clone()),
                source: Some(relation.source_identifier.clone()),
                target: Some(relation.target_identifier.clone()),
                native_target: None,
                code: "REQIF_REFERENCE_UNRESOLVED".into(),
                reason: "native relation endpoints cannot be resolved".into(),
            });
            continue;
        }
        let key = external_key(&configuration.source_namespace, &relation.identifier);
        let existing = find_relationship_by_external(project, &key);
        if existing.len() > 1 {
            preview.items.push(blocked_item(
                &relation.identifier,
                "SPEC-RELATION",
                "ReqIF relationship identity is ambiguous in the native project",
            ));
            continue;
        }
        if let Some(existing) = existing.first() {
            if existing.kind != kind {
                preview.items.push(blocked_item(
                    &relation.identifier,
                    "SPEC-RELATION",
                    format!(
                        "wrong-kind collision: ReqIF maps to {kind:?}, native identity is {:?}",
                        existing.kind
                    ),
                ));
                preview.diagnostics.push(diagnostic(
                    ReqifDiagnosticSeverity::Error,
                    "WRONG_KIND_COLLISION",
                    "stable ReqIF relationship identity belongs to another native kind",
                    Some(&relation.identifier),
                    Some("SPEC-RELATION"),
                ));
                continue;
            }
            let source_id = resolve_live_object_id(
                project,
                &configuration.source_namespace,
                &relation.source_identifier,
            );
            let target_id = resolve_live_object_id(
                project,
                &configuration.source_namespace,
                &relation.target_identifier,
            );
            let action = match (source_id, target_id) {
                (Some(source), Some(target))
                    if relationship_needs_update(existing, relation, source, target) =>
                {
                    ReqifAction::Update
                }
                (Some(_), Some(_)) => ReqifAction::NoChange,
                _ => ReqifAction::Update,
            };
            preview.items.push(ReqifPreviewItem {
                action,
                identifier: relation.identifier.clone(),
                kind: format!("{:?}", kind),
                detail: existing.id.to_string(),
            });
            if action == ReqifAction::Update {
                operations.push(ModelBuildOperation::UpdateRelationshipFields {
                    relationship: BuildReference::External(relation.identifier.clone()),
                    name: Some(relation.long_name.clone()),
                    owner: Some(BuildReference::Existing(target_scope)),
                    source: Some(BuildReference::External(relation.source_identifier.clone())),
                    target: Some(BuildReference::External(relation.target_identifier.clone())),
                    external_id: None,
                    documentation: None,
                    visibility: None,
                    source_end: None,
                    target_end: None,
                    alias: None,
                    extension_condition: None,
                    extension_location: None,
                });
            }
        } else {
            preview.items.push(ReqifPreviewItem {
                action: ReqifAction::Create,
                identifier: relation.identifier.clone(),
                kind: format!("{:?}", kind),
                detail: format!(
                    "{} -> {}",
                    relation.source_identifier, relation.target_identifier
                ),
            });
            operations.push(ModelBuildOperation::CreateRelationship {
                external_id: relation.identifier.clone(),
                kind,
                source: BuildReference::External(relation.source_identifier.clone()),
                target: BuildReference::External(relation.target_identifier.clone()),
                owner: Some(BuildReference::Existing(target_scope)),
            });
            if !relation.long_name.is_empty() {
                operations.push(ModelBuildOperation::UpdateRelationshipFields {
                    relationship: BuildReference::External(relation.identifier.clone()),
                    name: Some(relation.long_name.clone()),
                    owner: None,
                    source: None,
                    target: None,
                    external_id: None,
                    documentation: None,
                    visibility: None,
                    source_end: None,
                    target_end: None,
                    alias: None,
                    extension_condition: None,
                    extension_location: None,
                });
            }
        }
    }

    let mut remove_elements = Vec::new();
    let mut remove_relationships = Vec::new();
    if configuration.synchronization == ReqifSynchronizationPolicy::AuthoritativeReqifScope {
        let exchange = workspace
            .reqif_exchange
            .lock()
            .map(|state| state.clone())
            .unwrap_or_default();
        if let Some(previous) = exchange.sources.get(&configuration.source_namespace) {
            for (identifier, native_id) in &previous.relationship_bindings {
                if relation_ids.contains(identifier) {
                    continue;
                }
                if let Ok(id) = uuid::Uuid::parse_str(native_id).map(RelationshipId)
                    && project.relationships.get(&id).is_some_and(|relationship| {
                        relationship.external_id
                            == external_key(&configuration.source_namespace, identifier)
                    })
                {
                    remove_relationships.push(id);
                    preview.items.push(ReqifPreviewItem {
                        action: ReqifAction::Remove,
                        identifier: identifier.clone(),
                        kind: "SPEC-RELATION".into(),
                        detail: native_id.clone(),
                    });
                }
            }
            let mut deletion_probe = project.clone();
            for id in &remove_relationships {
                deletion_probe.relationships.remove(id);
            }
            for (identifier, native_id) in &previous.element_bindings {
                if object_ids.contains(identifier) {
                    continue;
                }
                let Ok(id) = uuid::Uuid::parse_str(native_id).map(ElementId) else {
                    continue;
                };
                let provenance_matches = deletion_probe.elements.get(&id).is_some_and(|element| {
                    element.external_id == external_key(&configuration.source_namespace, identifier)
                });
                if !provenance_matches {
                    continue;
                }
                match deletion_probe.delete_element(id) {
                    Ok(()) => {
                        remove_elements.push(id);
                        preview.items.push(ReqifPreviewItem {
                            action: ReqifAction::Remove,
                            identifier: identifier.clone(),
                            kind: "SPEC-OBJECT".into(),
                            detail: native_id.clone(),
                        });
                    }
                    Err(error) => {
                        preview.items.push(blocked_item(
                            identifier,
                            "SPEC-OBJECT",
                            "authoritative removal is reference-protected",
                        ));
                        preview.diagnostics.push(ReqifDiagnostic {
                            severity: ReqifDiagnosticSeverity::Error,
                            file: None,
                            identifier: Some(identifier.clone()),
                            record_kind: Some("SPEC-OBJECT".into()),
                            attribute_or_type: None,
                            source: None,
                            target: None,
                            native_target: Some(native_id.clone()),
                            code: "REFERENCE_PROTECTED_REMOVE".into(),
                            reason: error.to_string(),
                        });
                    }
                }
            }
        }
    }
    drop(project_guard);

    let plan = ModelBuildPlan {
        source_namespace: configuration.source_namespace.clone(),
        operations,
    };
    let build_preview = super::bulk_model::preview_unified_model_build(&plan, workspace, activity);
    for build in build_preview.diagnostics {
        let severity = match build.severity {
            BuildDiagnosticSeverity::Error => ReqifDiagnosticSeverity::Error,
            BuildDiagnosticSeverity::Warning => ReqifDiagnosticSeverity::Warning,
        };
        preview.diagnostics.push(ReqifDiagnostic {
            severity,
            file: None,
            identifier: None,
            record_kind: Some("ModelBuildPlan".into()),
            attribute_or_type: None,
            source: None,
            target: None,
            native_target: None,
            code: build.code.into(),
            reason: build.message,
        });
    }
    preview.recount();
    PreparedReqifImport {
        document,
        plan,
        preview,
        remove_elements,
        remove_relationships,
        configuration,
    }
}

fn clone_states(
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
) -> Result<(WorkspaceState, ActivityWorkspaceState), String> {
    Ok((
        WorkspaceState {
            project: Mutex::new(
                workspace
                    .project
                    .lock()
                    .map_err(|_| "project lock poisoned")?
                    .clone(),
            ),
            diagrams: Mutex::new(
                workspace
                    .diagrams
                    .lock()
                    .map_err(|_| "diagram lock poisoned")?
                    .clone(),
            ),
            ibd_diagrams: Mutex::new(
                workspace
                    .ibd_diagrams
                    .lock()
                    .map_err(|_| "IBD lock poisoned")?
                    .clone(),
            ),
            behavior: Mutex::new(
                workspace
                    .behavior
                    .lock()
                    .map_err(|_| "behavior lock poisoned")?
                    .clone(),
            ),
            behavior_diagrams: Mutex::new(
                workspace
                    .behavior_diagrams
                    .lock()
                    .map_err(|_| "behavior diagram lock poisoned")?
                    .clone(),
            ),
            current_file: Mutex::new(
                workspace
                    .current_file
                    .lock()
                    .map_err(|_| "project path lock poisoned")?
                    .clone(),
            ),
            reqif_exchange: Mutex::new(
                workspace
                    .reqif_exchange
                    .lock()
                    .map_err(|_| "ReqIF exchange lock poisoned")?
                    .clone(),
            ),
        },
        ActivityWorkspaceState {
            repository: Mutex::new(
                activity
                    .repository
                    .lock()
                    .map_err(|_| "Activity repository lock poisoned")?
                    .clone(),
            ),
            diagrams: Mutex::new(
                activity
                    .diagrams
                    .lock()
                    .map_err(|_| "Activity diagram lock poisoned")?
                    .clone(),
            ),
        },
    ))
}

fn cleanup_removed_relationship_presentations(
    workspace: &WorkspaceState,
    removed: &BTreeSet<String>,
) -> Result<(), String> {
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?;
    for diagram in &mut *diagrams {
        diagram
            .edges
            .retain(|edge| !removed.contains(&edge.relationship_id));
    }
    Ok(())
}

fn cleanup_removed_element_presentations(
    workspace: &WorkspaceState,
    removed: &BTreeSet<String>,
) -> Result<(), String> {
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?;
    for diagram in &mut *diagrams {
        let removed_presentations = diagram
            .nodes
            .iter()
            .filter(|node| removed.contains(&node.element_id))
            .map(|node| node.id.clone())
            .collect::<BTreeSet<_>>();
        diagram
            .nodes
            .retain(|node| !removed.contains(&node.element_id));
        diagram.edges.retain(|edge| {
            !removed_presentations.contains(&edge.source_node_id)
                && !removed_presentations.contains(&edge.target_node_id)
        });
    }
    Ok(())
}

fn bind_source_state(
    candidate: &WorkspaceState,
    configuration: &ReqifImportConfiguration,
    document: ReqifDocument,
) -> Result<(), String> {
    let project = candidate
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?;
    let project = project.as_ref().ok_or("no project open")?;
    let mut element_bindings = BTreeMap::new();
    for object in &document.spec_objects {
        if config_object_kind(&document, configuration, &object.type_identifier).is_none() {
            continue;
        }
        let key = external_key(&configuration.source_namespace, &object.identifier);
        if let Some(element) = project
            .elements
            .values()
            .find(|element| element.external_id == key)
        {
            element_bindings.insert(object.identifier.clone(), element.id.to_string());
        }
    }
    let mut relationship_bindings = BTreeMap::new();
    for relation in &document.spec_relations {
        if config_relation_kind(&document, configuration, &relation.type_identifier).is_none() {
            continue;
        }
        let key = external_key(&configuration.source_namespace, &relation.identifier);
        if let Some(relationship) = project
            .relationships
            .values()
            .find(|relationship| relationship.external_id == key)
        {
            relationship_bindings.insert(relation.identifier.clone(), relationship.id.to_string());
        }
    }
    drop(project);
    candidate
        .reqif_exchange
        .lock()
        .map_err(|_| "ReqIF exchange lock poisoned")?
        .sources
        .insert(
            configuration.source_namespace.clone(),
            ReqifSourceState {
                document,
                element_bindings,
                relationship_bindings,
            },
        );
    Ok(())
}

fn commit_candidate(
    live: &WorkspaceState,
    live_activity: &ActivityWorkspaceState,
    candidate: &WorkspaceState,
    candidate_activity: &ActivityWorkspaceState,
) -> Result<(), String> {
    let project = candidate
        .project
        .lock()
        .map_err(|_| "candidate project lock poisoned")?
        .clone();
    let diagrams = candidate
        .diagrams
        .lock()
        .map_err(|_| "candidate diagram lock poisoned")?
        .clone();
    let ibd_diagrams = candidate
        .ibd_diagrams
        .lock()
        .map_err(|_| "candidate IBD lock poisoned")?
        .clone();
    let behavior = candidate
        .behavior
        .lock()
        .map_err(|_| "candidate behavior lock poisoned")?
        .clone();
    let behavior_diagrams = candidate
        .behavior_diagrams
        .lock()
        .map_err(|_| "candidate behavior diagram lock poisoned")?
        .clone();
    let current_file = candidate
        .current_file
        .lock()
        .map_err(|_| "candidate project path lock poisoned")?
        .clone();
    let reqif_exchange = candidate
        .reqif_exchange
        .lock()
        .map_err(|_| "candidate ReqIF exchange lock poisoned")?
        .clone();
    let activity_repository = candidate_activity
        .repository
        .lock()
        .map_err(|_| "candidate Activity repository lock poisoned")?
        .clone();
    let activity_diagrams = candidate_activity
        .diagrams
        .lock()
        .map_err(|_| "candidate Activity diagram lock poisoned")?
        .clone();

    *live.project.lock().map_err(|_| "project lock poisoned")? = project;
    *live.diagrams.lock().map_err(|_| "diagram lock poisoned")? = diagrams;
    *live
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")? = ibd_diagrams;
    *live.behavior.lock().map_err(|_| "behavior lock poisoned")? = behavior;
    *live
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")? = behavior_diagrams;
    *live
        .current_file
        .lock()
        .map_err(|_| "project path lock poisoned")? = current_file;
    *live
        .reqif_exchange
        .lock()
        .map_err(|_| "ReqIF exchange lock poisoned")? = reqif_exchange;
    *live_activity
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")? = activity_repository;
    *live_activity
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")? = activity_diagrams;
    Ok(())
}

fn apply_prepared(
    mut prepared: PreparedReqifImport,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
    history_state: Option<&HistoryState>,
) -> ReqifImportPreview {
    if !prepared.preview.is_valid() {
        return prepared.preview;
    }
    let (candidate, candidate_activity) = match clone_states(workspace, activity) {
        Ok(states) => states,
        Err(reason) => {
            prepared.preview.diagnostics.push(diagnostic(
                ReqifDiagnosticSeverity::Error,
                "SEMANTIC_VALIDATION",
                reason,
                None,
                None,
            ));
            prepared.preview.recount();
            return prepared.preview;
        }
    };
    if let Err(build_preview) =
        super::bulk_model::apply_unified_model_build(&prepared.plan, &candidate, &candidate_activity)
    {
        for build in build_preview.diagnostics {
            prepared.preview.diagnostics.push(diagnostic(
                match build.severity {
                    BuildDiagnosticSeverity::Error => ReqifDiagnosticSeverity::Error,
                    BuildDiagnosticSeverity::Warning => ReqifDiagnosticSeverity::Warning,
                },
                build.code,
                build.message,
                None,
                Some("ModelBuildPlan"),
            ));
        }
        prepared.preview.recount();
        return prepared.preview;
    }

    let removed_relationship_strings = prepared
        .remove_relationships
        .iter()
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();
    {
        let mut project = match candidate.project.lock() {
            Ok(project) => project,
            Err(_) => {
                prepared.preview.diagnostics.push(diagnostic(
                    ReqifDiagnosticSeverity::Error,
                    "SEMANTIC_VALIDATION",
                    "candidate project lock poisoned",
                    None,
                    None,
                ));
                prepared.preview.recount();
                return prepared.preview;
            }
        };
        let Some(project) = project.as_mut() else {
            prepared.preview.diagnostics.push(diagnostic(
                ReqifDiagnosticSeverity::Error,
                "SEMANTIC_VALIDATION",
                "candidate project disappeared",
                None,
                None,
            ));
            prepared.preview.recount();
            return prepared.preview;
        };
        for id in &prepared.remove_relationships {
            project.relationships.remove(id);
        }
    }
    if let Err(reason) =
        cleanup_removed_relationship_presentations(&candidate, &removed_relationship_strings)
    {
        prepared.preview.diagnostics.push(diagnostic(
            ReqifDiagnosticSeverity::Error,
            "SEMANTIC_VALIDATION",
            reason,
            None,
            None,
        ));
        prepared.preview.recount();
        return prepared.preview;
    }

    let removed_element_strings = prepared
        .remove_elements
        .iter()
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();
    {
        let mut project = match candidate.project.lock() {
            Ok(project) => project,
            Err(_) => {
                prepared.preview.diagnostics.push(diagnostic(
                    ReqifDiagnosticSeverity::Error,
                    "SEMANTIC_VALIDATION",
                    "candidate project lock poisoned",
                    None,
                    None,
                ));
                prepared.preview.recount();
                return prepared.preview;
            }
        };
        let Some(project) = project.as_mut() else {
            prepared.preview.diagnostics.push(diagnostic(
                ReqifDiagnosticSeverity::Error,
                "SEMANTIC_VALIDATION",
                "candidate project disappeared",
                None,
                None,
            ));
            prepared.preview.recount();
            return prepared.preview;
        };
        for id in &prepared.remove_elements {
            if let Err(error) = project.delete_element(*id) {
                prepared.preview.diagnostics.push(diagnostic(
                    ReqifDiagnosticSeverity::Error,
                    "REFERENCE_PROTECTED_REMOVE",
                    error.to_string(),
                    None,
                    Some("SPEC-OBJECT"),
                ));
                prepared.preview.recount();
                return prepared.preview;
            }
        }
    }
    if let Err(reason) = cleanup_removed_element_presentations(&candidate, &removed_element_strings) {
        prepared.preview.diagnostics.push(diagnostic(
            ReqifDiagnosticSeverity::Error,
            "SEMANTIC_VALIDATION",
            reason,
            None,
            None,
        ));
        prepared.preview.recount();
        return prepared.preview;
    }

    if let Err(reason) = bind_source_state(
        &candidate,
        &prepared.configuration,
        prepared.document.clone(),
    ) {
        prepared.preview.diagnostics.push(diagnostic(
            ReqifDiagnosticSeverity::Error,
            "SEMANTIC_VALIDATION",
            reason,
            None,
            None,
        ));
        prepared.preview.recount();
        return prepared.preview;
    }
    if let Err(reason) = super::portable_interchange::portable_from_states(&candidate, &candidate_activity)
    {
        prepared.preview.diagnostics.push(diagnostic(
            ReqifDiagnosticSeverity::Error,
            "SEMANTIC_VALIDATION",
            reason,
            None,
            None,
        ));
        prepared.preview.recount();
        return prepared.preview;
    }
    if let Some(history_state) = history_state {
        if let Err(reason) = history::checkpoint_states(workspace, activity, history_state) {
            prepared.preview.diagnostics.push(diagnostic(
                ReqifDiagnosticSeverity::Error,
                "SEMANTIC_VALIDATION",
                reason,
                None,
                None,
            ));
            prepared.preview.recount();
            return prepared.preview;
        }
    }
    if let Err(reason) = commit_candidate(workspace, activity, &candidate, &candidate_activity) {
        prepared.preview.diagnostics.push(diagnostic(
            ReqifDiagnosticSeverity::Error,
            "SEMANTIC_VALIDATION",
            reason,
            None,
            None,
        ));
        prepared.preview.recount();
        return prepared.preview;
    }
    prepared.preview.applied = true;
    prepared.preview
}

pub fn preview_reqif_xml(
    xml: &str,
    file_name: Option<&str>,
    configuration: ReqifImportConfiguration,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
) -> ReqifImportPreview {
    match parse_reqif(xml, file_name) {
        Ok(document) => prepare_reqif_import(document, configuration, workspace, activity).preview,
        Err(mut diagnostics) => {
            for item in &mut diagnostics {
                if item.file.is_none() {
                    item.file = file_name.map(Into::into);
                }
            }
            let mut preview = ReqifImportPreview {
                source_namespace: configuration.source_namespace,
                diagnostics,
                ..ReqifImportPreview::default()
            };
            preview.recount();
            preview
        }
    }
}

pub fn apply_reqif_xml(
    xml: &str,
    file_name: Option<&str>,
    configuration: ReqifImportConfiguration,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
    history_state: Option<&HistoryState>,
) -> ReqifImportPreview {
    match parse_reqif(xml, file_name) {
        Ok(document) => apply_prepared(
            prepare_reqif_import(document, configuration, workspace, activity),
            workspace,
            activity,
            history_state,
        ),
        Err(diagnostics) => {
            let mut preview = ReqifImportPreview {
                source_namespace: configuration.source_namespace,
                diagnostics,
                ..ReqifImportPreview::default()
            };
            preview.recount();
            preview
        }
    }
}

fn read_reqif_file(path: &Path) -> Result<ReqifFilePayload, String> {
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if metadata.len() > MAX_REQIF_BYTES {
        return Err("ReqIF input exceeds the 64 MiB safety limit".into());
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if extension == "reqif" {
        return Ok(ReqifFilePayload {
            file_name: path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("input.reqif")
                .into(),
            xml: fs::read_to_string(path).map_err(|error| error.to_string())?,
        });
    }
    if extension != "reqifz" {
        return Err("ReqIF import supports .reqif and .reqifz".into());
    }
    let file = fs::File::open(path).map_err(|error| error.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;
    let mut candidates = Vec::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|error| error.to_string())?;
        if entry.is_file() && entry.name().to_ascii_lowercase().ends_with(".reqif") {
            candidates.push((entry.name().to_owned(), entry.size()));
        }
    }
    candidates.sort_by(|left, right| left.0.cmp(&right.0));
    let (name, size) = candidates
        .into_iter()
        .next()
        .ok_or("ReqIFZ container has no .reqif document")?;
    if size > MAX_REQIF_BYTES {
        return Err("ReqIFZ document exceeds the 64 MiB safety limit".into());
    }
    let mut entry = archive.by_name(&name).map_err(|error| error.to_string())?;
    let mut xml = String::new();
    entry
        .read_to_string(&mut xml)
        .map_err(|error| error.to_string())?;
    Ok(ReqifFilePayload {
        file_name: name,
        xml,
    })
}

#[tauri::command]
pub fn preview_reqif_import(
    path: String,
    configuration: ReqifImportConfiguration,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
) -> Result<ReqifImportPreview, String> {
    let payload = read_reqif_file(Path::new(&path))?;
    Ok(preview_reqif_xml(
        &payload.xml,
        Some(&payload.file_name),
        configuration,
        &workspace,
        &activity,
    ))
}

#[tauri::command]
pub fn apply_reqif_import(
    path: String,
    configuration: ReqifImportConfiguration,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history_state: tauri::State<'_, HistoryState>,
) -> Result<ReqifImportPreview, String> {
    let payload = read_reqif_file(Path::new(&path))?;
    Ok(apply_reqif_xml(
        &payload.xml,
        Some(&payload.file_name),
        configuration,
        &workspace,
        &activity,
        Some(&history_state),
    ))
}

fn generated_attribute(identifier: &str, long_name: &str) -> super::reqif_interchange::ReqifAttributeDefinition {
    super::reqif_interchange::ReqifAttributeDefinition {
        identifier: identifier.into(),
        long_name: long_name.into(),
        kind: ReqifDatatypeKind::String,
        datatype_identifier: Some(GENERATED_STRING_DATATYPE.into()),
        multi_valued: false,
    }
}

fn generated_object_type(identifier: &str, long_name: &str) -> ReqifSpecType {
    ReqifSpecType {
        identifier: identifier.into(),
        long_name: long_name.into(),
        kind: ReqifSpecTypeKind::SpecObject,
        attributes: vec![
            generated_attribute(GENERATED_NAME_ATTRIBUTE, "Name"),
            generated_attribute(GENERATED_REQUIREMENT_ID_ATTRIBUTE, "Requirement ID"),
            generated_attribute(GENERATED_REQUIREMENT_TEXT_ATTRIBUTE, "Requirement Text"),
            generated_attribute(GENERATED_DOCUMENTATION_ATTRIBUTE, "Documentation"),
        ],
    }
}

fn generated_native_values(element: &Element) -> Vec<ReqifAttributeValue> {
    let mut values = vec![ReqifAttributeValue {
        definition_identifier: GENERATED_NAME_ATTRIBUTE.into(),
        value: ReqifValue::String(element.name.clone()),
    }];
    if let Some(value) = &element.requirement_id {
        values.push(ReqifAttributeValue {
            definition_identifier: GENERATED_REQUIREMENT_ID_ATTRIBUTE.into(),
            value: ReqifValue::String(value.clone()),
        });
    }
    if let Some(value) = &element.requirement_text {
        values.push(ReqifAttributeValue {
            definition_identifier: GENERATED_REQUIREMENT_TEXT_ATTRIBUTE.into(),
            value: ReqifValue::String(value.clone()),
        });
    }
    if !element.documentation.is_empty() {
        values.push(ReqifAttributeValue {
            definition_identifier: GENERATED_DOCUMENTATION_ATTRIBUTE.into(),
            value: ReqifValue::String(element.documentation.clone()),
        });
    }
    values
}

fn update_imported_object(
    mut object: ReqifSpecObject,
    element: &Element,
    source: &ReqifSourceState,
) -> ReqifSpecObject {
    object.long_name = element.name.clone();
    let definitions = source.document.attribute_definitions();
    let configuration = source.configuration.as_ref();
    for value in &mut object.values {
        let Some(definition) = definitions.get(&value.definition_identifier) else {
            continue;
        };
        let field = configuration
            .and_then(|config| configured_field(config, definition))
            .or_else(|| detected_attribute_mapping(definition));
        let replacement = match field {
            Some(ReqifNativeField::RequirementId) => element.requirement_id.clone(),
            Some(ReqifNativeField::RequirementText) => element.requirement_text.clone(),
            Some(ReqifNativeField::Name) => Some(element.name.clone()),
            Some(ReqifNativeField::Documentation) => Some(element.documentation.clone()),
            None => None,
        };
        if let Some(replacement) = replacement {
            value.value = match &value.value {
                ReqifValue::Xhtml { plain_text, .. } if plain_text == &replacement => {
                    value.value.clone()
                }
                ReqifValue::Xhtml { .. } => ReqifValue::Xhtml {
                    plain_text: replacement,
                    original_xml: String::new(),
                },
                ReqifValue::Boolean(_) => replacement
                    .parse::<bool>()
                    .map(ReqifValue::Boolean)
                    .unwrap_or_else(|_| ReqifValue::String(replacement)),
                ReqifValue::Integer(_) => replacement
                    .parse::<i64>()
                    .map(ReqifValue::Integer)
                    .unwrap_or_else(|_| ReqifValue::String(replacement)),
                ReqifValue::Real(_) => replacement
                    .parse::<f64>()
                    .map(ReqifValue::Real)
                    .unwrap_or_else(|_| ReqifValue::String(replacement)),
                ReqifValue::Date(_) => ReqifValue::Date(replacement),
                ReqifValue::Enumeration(_) => value.value.clone(),
                ReqifValue::String(_) => ReqifValue::String(replacement),
            };
        }
    }
    object
}

fn native_scope_contains(project: &Project, scope: ElementId, element: &Element) -> bool {
    let mut current = Some(element.id);
    let mut visited = BTreeSet::new();
    while let Some(id) = current {
        if id == scope {
            return true;
        }
        if !visited.insert(id) {
            break;
        }
        current = project.elements.get(&id).and_then(|item| item.owner_id);
    }
    false
}

fn filter_hierarchy(
    nodes: &[ReqifHierarchyNode],
    included: &BTreeSet<String>,
) -> Vec<ReqifHierarchyNode> {
    nodes
        .iter()
        .filter_map(|node| {
            let children = filter_hierarchy(&node.children, included);
            included.contains(&node.object_identifier).then(|| ReqifHierarchyNode {
                identifier: node.identifier.clone(),
                object_identifier: node.object_identifier.clone(),
                children,
            })
        })
        .collect()
}

fn deterministic_relation_type(kind: &RelationshipKind) -> String {
    format!("SM-RT-{:?}", kind).to_ascii_uppercase()
}

fn export_document(
    workspace: &WorkspaceState,
    scope: ElementId,
) -> Result<ReqifDocument, String> {
    let project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?;
    let project = project.as_ref().ok_or("no project open")?;
    project.element(scope).map_err(|error| error.to_string())?;
    let exchange = workspace
        .reqif_exchange
        .lock()
        .map_err(|_| "ReqIF exchange lock poisoned")?
        .clone();

    let mut datatypes = BTreeMap::new();
    datatypes.insert(
        GENERATED_STRING_DATATYPE.to_string(),
        ReqifDatatype {
            identifier: GENERATED_STRING_DATATYPE.into(),
            long_name: "String".into(),
            kind: ReqifDatatypeKind::String,
            enum_values: Vec::new(),
        },
    );
    let mut spec_types = BTreeMap::new();
    spec_types.insert(
        GENERATED_REQUIREMENT_TYPE.to_string(),
        generated_object_type(GENERATED_REQUIREMENT_TYPE, "Requirement"),
    );
    spec_types.insert(
        GENERATED_TESTCASE_TYPE.to_string(),
        generated_object_type(GENERATED_TESTCASE_TYPE, "TestCase"),
    );
    spec_types.insert(
        GENERATED_SPECIFICATION_TYPE.to_string(),
        ReqifSpecType {
            identifier: GENERATED_SPECIFICATION_TYPE.into(),
            long_name: "Specification".into(),
            kind: ReqifSpecTypeKind::Specification,
            attributes: Vec::new(),
        },
    );

    let selected = project
        .elements
        .values()
        .filter(|element| {
            matches!(element.kind, ElementKind::Requirement | ElementKind::TestCase)
                && native_scope_contains(project, scope, element)
        })
        .collect::<Vec<_>>();
    let selected_native_ids = selected
        .iter()
        .map(|element| element.id.to_string())
        .collect::<BTreeSet<_>>();

    let mut binding_lookup: BTreeMap<String, (&str, &ReqifSourceState)> = BTreeMap::new();
    for (namespace, source) in &exchange.sources {
        for (identifier, native_id) in &source.element_bindings {
            if selected_native_ids.contains(native_id) {
                binding_lookup
                    .entry(native_id.clone())
                    .or_insert((identifier.as_str(), source));
            }
        }
    }
    let mut object_ids_by_native = BTreeMap::new();
    let mut used_identifiers = BTreeSet::new();
    let mut spec_objects = Vec::new();
    for element in selected {
        if let Some((identifier, source)) = binding_lookup.get(&element.id.to_string()) {
            if !used_identifiers.contains(*identifier)
                && let Some(original) = source
                    .document
                    .spec_objects
                    .iter()
                    .find(|object| object.identifier == **identifier)
            {
                for datatype in &source.document.datatypes {
                    datatypes
                        .entry(datatype.identifier.clone())
                        .or_insert_with(|| datatype.clone());
                }
                for spec_type in &source.document.spec_types {
                    spec_types
                        .entry(spec_type.identifier.clone())
                        .or_insert_with(|| spec_type.clone());
                }
                let object = update_imported_object(original.clone(), element, source);
                used_identifiers.insert(object.identifier.clone());
                object_ids_by_native.insert(element.id.to_string(), object.identifier.clone());
                spec_objects.push(object);
                continue;
            }
        }
        let identifier = format!("SM-E-{}", element.id);
        used_identifiers.insert(identifier.clone());
        object_ids_by_native.insert(element.id.to_string(), identifier.clone());
        spec_objects.push(ReqifSpecObject {
            identifier,
            long_name: element.name.clone(),
            type_identifier: if element.kind == ElementKind::Requirement {
                GENERATED_REQUIREMENT_TYPE.into()
            } else {
                GENERATED_TESTCASE_TYPE.into()
            },
            values: generated_native_values(element),
        });
    }
    spec_objects.sort_by(|left, right| left.identifier.cmp(&right.identifier));

    let mut spec_relations = Vec::new();
    let mut used_relation_identifiers = BTreeSet::new();
    let eligible_kinds = |kind: &RelationshipKind| {
        matches!(
            kind,
            RelationshipKind::Trace
                | RelationshipKind::DeriveRequirement
                | RelationshipKind::Satisfy
                | RelationshipKind::Verify
                | RelationshipKind::Refine
                | RelationshipKind::Copy
                | RelationshipKind::Dependency
        )
    };
    for relationship in project.relationships.values() {
        if !eligible_kinds(&relationship.kind) {
            continue;
        }
        let Some(source_identifier) = object_ids_by_native.get(&relationship.source_id.to_string())
        else {
            continue;
        };
        let Some(target_identifier) = object_ids_by_native.get(&relationship.target_id.to_string())
        else {
            continue;
        };
        let mut imported = None;
        for source in exchange.sources.values() {
            if let Some((identifier, _)) = source
                .relationship_bindings
                .iter()
                .find(|(_, native_id)| **native_id == relationship.id.to_string())
                && !used_relation_identifiers.contains(identifier)
            {
                imported = source
                    .document
                    .spec_relations
                    .iter()
                    .find(|relation| relation.identifier == *identifier)
                    .cloned();
                if imported.is_some() {
                    for datatype in &source.document.datatypes {
                        datatypes
                            .entry(datatype.identifier.clone())
                            .or_insert_with(|| datatype.clone());
                    }
                    for spec_type in &source.document.spec_types {
                        spec_types
                            .entry(spec_type.identifier.clone())
                            .or_insert_with(|| spec_type.clone());
                    }
                }
                break;
            }
        }
        if let Some(mut relation) = imported {
            relation.long_name = relationship.name.clone();
            relation.source_identifier = source_identifier.clone();
            relation.target_identifier = target_identifier.clone();
            used_relation_identifiers.insert(relation.identifier.clone());
            spec_relations.push(relation);
        } else {
            let type_identifier = deterministic_relation_type(&relationship.kind);
            spec_types.entry(type_identifier.clone()).or_insert(ReqifSpecType {
                identifier: type_identifier.clone(),
                long_name: format!("{:?}", relationship.kind),
                kind: ReqifSpecTypeKind::SpecRelation,
                attributes: Vec::new(),
            });
            let identifier = format!("SM-R-{}", relationship.id);
            used_relation_identifiers.insert(identifier.clone());
            spec_relations.push(ReqifSpecRelation {
                identifier,
                long_name: relationship.name.clone(),
                type_identifier,
                source_identifier: source_identifier.clone(),
                target_identifier: target_identifier.clone(),
                values: Vec::new(),
            });
        }
    }

    // Exchange-only unknown relation types remain round-trippable as long as both
    // endpoints are still selected. Bound relations whose native semantic object
    // was manually removed are intentionally not resurrected.
    let included_object_ids = spec_objects
        .iter()
        .map(|object| object.identifier.clone())
        .collect::<BTreeSet<_>>();
    for source in exchange.sources.values() {
        for relation in &source.document.spec_relations {
            if source.relationship_bindings.contains_key(&relation.identifier)
                || used_relation_identifiers.contains(&relation.identifier)
                || !included_object_ids.contains(&relation.source_identifier)
                || !included_object_ids.contains(&relation.target_identifier)
            {
                continue;
            }
            for datatype in &source.document.datatypes {
                datatypes
                    .entry(datatype.identifier.clone())
                    .or_insert_with(|| datatype.clone());
            }
            for spec_type in &source.document.spec_types {
                spec_types
                    .entry(spec_type.identifier.clone())
                    .or_insert_with(|| spec_type.clone());
            }
            used_relation_identifiers.insert(relation.identifier.clone());
            spec_relations.push(relation.clone());
        }
    }
    spec_relations.sort_by(|left, right| left.identifier.cmp(&right.identifier));

    let mut specifications = Vec::new();
    for (namespace, source) in &exchange.sources {
        let source_object_ids = source
            .element_bindings
            .iter()
            .filter_map(|(identifier, native_id)| {
                object_ids_by_native
                    .get(native_id)
                    .is_some_and(|exported| exported == identifier)
                    .then_some(identifier.clone())
            })
            .collect::<BTreeSet<_>>();
        for specification in &source.document.specifications {
            let children = filter_hierarchy(&specification.children, &source_object_ids);
            if !children.is_empty() {
                let mut specification = specification.clone();
                specification.identifier = format!(
                    "{}-{}",
                    specification.identifier,
                    stable_namespace_suffix(namespace)
                );
                specification.children = children;
                specifications.push(specification);
            }
        }
    }
    let hierarchy_ids = specifications
        .iter()
        .flat_map(|specification| hierarchy_object_ids(&specification.children))
        .collect::<BTreeSet<_>>();
    let generated_children = spec_objects
        .iter()
        .filter(|object| !hierarchy_ids.contains(&object.identifier))
        .enumerate()
        .map(|(index, object)| ReqifHierarchyNode {
            identifier: format!("SM-HIER-{:04}-{}", index + 1, object.identifier),
            object_identifier: object.identifier.clone(),
            children: Vec::new(),
        })
        .collect::<Vec<_>>();
    if !generated_children.is_empty() {
        specifications.push(super::reqif_interchange::ReqifSpecification {
            identifier: format!("SM-SPEC-{}", scope),
            long_name: project
                .element(scope)
                .map(|element| element.name.clone())
                .unwrap_or_else(|_| "Requirements".into()),
            type_identifier: Some(GENERATED_SPECIFICATION_TYPE.into()),
            children: generated_children,
        });
    }
    specifications.sort_by(|left, right| left.identifier.cmp(&right.identifier));

    Ok(ReqifDocument {
        header_identifier: format!("SM-H-{}", project.id),
        title: format!("{} ReqIF", project.name),
        // A stable value keeps deterministic byte-for-byte exports. ReqIF exchange
        // identity is not based on CREATION-TIME.
        creation_time: Some("1970-01-01T00:00:00Z".into()),
        datatypes: datatypes.into_values().collect(),
        spec_types: spec_types.into_values().collect(),
        spec_objects,
        specifications,
        spec_relations,
    })
}

fn stable_namespace_suffix(namespace: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in namespace.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn hierarchy_object_ids(nodes: &[ReqifHierarchyNode]) -> Vec<String> {
    let mut result = Vec::new();
    for node in nodes {
        result.push(node.object_identifier.clone());
        result.extend(hierarchy_object_ids(&node.children));
    }
    result
}

pub fn export_reqif_xml(scope: ElementId, workspace: &WorkspaceState) -> Result<String, String> {
    let document = export_document(workspace, scope)?;
    let diagnostics = document.validate_references();
    if diagnostics
        .iter()
        .any(|item| item.severity == ReqifDiagnosticSeverity::Error)
    {
        return Err(format!("generated ReqIF failed validation: {diagnostics:?}"));
    }
    Ok(serialize_reqif(&document))
}

fn write_reqif_file(path: &Path, xml: &str) -> Result<PathBuf, String> {
    let mut output = path.to_path_buf();
    if output.extension().is_none() {
        output.set_extension("reqif");
    }
    let extension = output
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if extension == "reqif" {
        fs::write(&output, xml).map_err(|error| error.to_string())?;
        return Ok(output);
    }
    if extension != "reqifz" {
        return Err("ReqIF export supports .reqif and .reqifz".into());
    }
    let file = fs::File::create(&output).map_err(|error| error.to_string())?;
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);
    writer
        .start_file("requirements.reqif", options)
        .map_err(|error| error.to_string())?;
    writer
        .write_all(xml.as_bytes())
        .map_err(|error| error.to_string())?;
    writer.finish().map_err(|error| error.to_string())?;
    Ok(output)
}

#[tauri::command]
pub fn export_reqif(
    path: String,
    scope_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let scope = parse_element_id(&scope_id)?;
    let xml = export_reqif_xml(scope, &workspace)?;
    let path = write_reqif_file(Path::new(&path), &xml)?;
    Ok(path.to_string_lossy().into_owned())
}

pub(super) fn save_reqif_metadata(
    database: &mut ProjectDatabase,
    project: &Project,
    exchange: &ReqifExchangeState,
) -> Result<(), String> {
    let payload = serde_json::to_string(exchange).map_err(|error| error.to_string())?;
    database
        .save_metadata(project.id, REQIF_METADATA_KEY, &payload)
        .map_err(|error| error.to_string())
}

pub(super) fn load_reqif_metadata(
    database: &ProjectDatabase,
    project: &Project,
) -> Result<ReqifExchangeState, String> {
    match database
        .load_metadata(project.id, REQIF_METADATA_KEY)
        .map_err(|error| error.to_string())?
    {
        Some(payload) => serde_json::from_str(&payload)
            .map_err(|error| format!("invalid saved ReqIF exchange metadata: {error}")),
        None => Ok(ReqifExchangeState::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use systems_modeler_core::Project;

    fn configuration(project: &Project) -> ReqifImportConfiguration {
        ReqifImportConfiguration {
            source_namespace: "reqif:test-source".into(),
            target_scope: project.root_id.to_string(),
            synchronization: ReqifSynchronizationPolicy::Additive,
            object_type_mappings: BTreeMap::new(),
            relation_type_mappings: BTreeMap::new(),
            attribute_mappings: BTreeMap::new(),
        }
    }

    fn fixture() -> &'static str {
        include_str!("../../../../../examples/reqif/external-representative.reqif")
    }

    fn states() -> (WorkspaceState, ActivityWorkspaceState) {
        let workspace = WorkspaceState::default();
        *workspace.project.lock().unwrap() = Some(Project::new("ReqIF Test"));
        (workspace, ActivityWorkspaceState::default())
    }

    #[test]
    fn external_fixture_preview_apply_and_identical_reimport_are_stable() {
        let (workspace, activity) = states();
        let project = workspace.project.lock().unwrap().clone().unwrap();
        let config = configuration(&project);
        let preview = preview_reqif_xml(fixture(), Some("external.reqif"), config.clone(), &workspace, &activity);
        assert!(preview.is_valid(), "{:#?}", preview.diagnostics);
        assert!(preview.totals.create >= 4);
        let applied = apply_reqif_xml(fixture(), Some("external.reqif"), config.clone(), &workspace, &activity, None);
        assert!(applied.applied, "{:#?}", applied.diagnostics);
        let reimport = preview_reqif_xml(fixture(), Some("external.reqif"), config, &workspace, &activity);
        assert!(reimport.is_valid(), "{:#?}", reimport.diagnostics);
        assert_eq!(reimport.totals.create, 0);
        assert_eq!(reimport.totals.update, 0);
        assert!(reimport.totals.no_change >= 4);
    }

    #[test]
    fn preview_is_non_mutating_and_late_invalid_relation_rolls_back() {
        let (workspace, activity) = states();
        let project = workspace.project.lock().unwrap().clone().unwrap();
        let config = configuration(&project);
        let before = workspace.project.lock().unwrap().clone().unwrap().elements.len();
        let _ = preview_reqif_xml(fixture(), None, config.clone(), &workspace, &activity);
        assert_eq!(workspace.project.lock().unwrap().as_ref().unwrap().elements.len(), before);
        let invalid = fixture().replace(
            "<SPEC-OBJECT-REF>REQ-2</SPEC-OBJECT-REF></TARGET>",
            "<SPEC-OBJECT-REF>MISSING</SPEC-OBJECT-REF></TARGET>",
        );
        let result = apply_reqif_xml(&invalid, None, config, &workspace, &activity, None);
        assert!(!result.applied);
        assert_eq!(workspace.project.lock().unwrap().as_ref().unwrap().elements.len(), before);
    }

    #[test]
    fn native_export_is_deterministic_and_reimports_into_blank_project() {
        let (workspace, activity) = states();
        let project = workspace.project.lock().unwrap().clone().unwrap();
        let config = configuration(&project);
        let applied = apply_reqif_xml(fixture(), None, config, &workspace, &activity, None);
        assert!(applied.applied, "{:#?}", applied.diagnostics);
        let scope = workspace.project.lock().unwrap().as_ref().unwrap().root_id;
        let first = export_reqif_xml(scope, &workspace).unwrap();
        let second = export_reqif_xml(scope, &workspace).unwrap();
        assert_eq!(first, second);

        let (blank, blank_activity) = states();
        let blank_project = blank.project.lock().unwrap().clone().unwrap();
        let imported = apply_reqif_xml(
            &first,
            None,
            ReqifImportConfiguration {
                source_namespace: "reqif:roundtrip".into(),
                target_scope: blank_project.root_id.to_string(),
                synchronization: ReqifSynchronizationPolicy::Additive,
                object_type_mappings: BTreeMap::new(),
                relation_type_mappings: BTreeMap::new(),
                attribute_mappings: BTreeMap::new(),
            },
            &blank,
            &blank_activity,
            None,
        );
        assert!(imported.applied, "{:#?}", imported.diagnostics);
        let native = blank.project.lock().unwrap();
        assert!(native.as_ref().unwrap().elements.values().any(|element| element.kind == ElementKind::Requirement));
        assert!(native.as_ref().unwrap().relationships.values().any(|relationship| relationship.kind == RelationshipKind::Trace));
    }
}
