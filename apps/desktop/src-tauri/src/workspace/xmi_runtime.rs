use super::{
    WorkspaceState,
    activity_workspace::ActivityWorkspaceState,
    bulk_model::{
        BuildDiagnosticSeverity, BuildReference, ModelBuildOperation, ModelBuildPlan, external_key,
    },
    history::{self, HistoryState},
    parse_element_id,
    portable_interchange::{PortableProjectV1, portable_from_states},
    xmi_interchange::{
        XmiDiagnostic, XmiDiagnosticSeverity, XmiSemanticDocument, embedded_portable,
        native_element_kind, native_relationship_kind, parse_xmi, serialize_xmi,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
use systems_modeler_core::{
    ElementId, ElementKind, FlowDirection, Multiplicity, ParameterDirection, Project,
    RelationshipId, RelationshipKind, SemanticTarget, StereotypeTargetKind, TagValue, TagValueType,
    VisibilityKind,
};

const MAX_XMI_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum XmiSynchronizationPolicy {
    AdditiveUpdate,
    AuthoritativeXmiScope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XmiImportConfiguration {
    pub source_namespace: String,
    pub target_scope: String,
    pub synchronization: XmiSynchronizationPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum XmiAction {
    Create,
    Update,
    NoChange,
    Remove,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XmiPreviewItem {
    pub action: XmiAction,
    pub xmi_id: String,
    pub xmi_type: String,
    pub detail: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct XmiImportPreview {
    pub applied: bool,
    pub source_namespace: String,
    pub producer: Option<String>,
    pub namespaces: BTreeMap<String, String>,
    pub items: Vec<XmiPreviewItem>,
    pub diagnostics: Vec<XmiDiagnostic>,
    pub create_count: usize,
    pub update_count: usize,
    pub no_change_count: usize,
    pub remove_count: usize,
    pub blocked_count: usize,
}

impl XmiImportPreview {
    fn recount(&mut self) {
        self.create_count = self
            .items
            .iter()
            .filter(|item| item.action == XmiAction::Create)
            .count();
        self.update_count = self
            .items
            .iter()
            .filter(|item| item.action == XmiAction::Update)
            .count();
        self.no_change_count = self
            .items
            .iter()
            .filter(|item| item.action == XmiAction::NoChange)
            .count();
        self.remove_count = self
            .items
            .iter()
            .filter(|item| item.action == XmiAction::Remove)
            .count();
        self.blocked_count = self
            .items
            .iter()
            .filter(|item| item.action == XmiAction::Blocked)
            .count();
    }

    pub fn is_valid(&self) -> bool {
        self.blocked_count == 0
            && !self
                .diagnostics
                .iter()
                .any(|item| item.severity == XmiDiagnosticSeverity::Error)
    }
}

struct PreparedXmiImport {
    document: XmiSemanticDocument,
    configuration: XmiImportConfiguration,
    plan: ModelBuildPlan,
    embedded: Option<PortableProjectV1>,
    remove_elements: Vec<ElementId>,
    remove_relationships: Vec<RelationshipId>,
    preview: XmiImportPreview,
}

fn runtime_diagnostic(code: &str, reason: impl Into<String>) -> XmiDiagnostic {
    XmiDiagnostic {
        severity: XmiDiagnosticSeverity::Error,
        code: code.into(),
        reason: reason.into(),
        file: None,
        line: None,
        column: None,
        namespace: None,
        xmi_id: None,
        xmi_type: None,
        reference: None,
        semantic_target: None,
    }
}

fn blocked(preview: &mut XmiImportPreview, id: &str, kind: &str, reason: impl Into<String>) {
    let reason = reason.into();
    preview.items.push(XmiPreviewItem {
        action: XmiAction::Blocked,
        xmi_id: id.into(),
        xmi_type: kind.into(),
        detail: reason.clone(),
    });
    let mut diagnostic = runtime_diagnostic("SEMANTIC_VALIDATION", reason);
    diagnostic.xmi_id = Some(id.into());
    diagnostic.xmi_type = Some(kind.into());
    preview.diagnostics.push(diagnostic);
}

fn effective_kind(
    document: &XmiSemanticDocument,
    record: &super::xmi_interchange::XmiSemanticRecord,
) -> Option<ElementKind> {
    if let Some(application) = document
        .stereotype_applications
        .iter()
        .find(|application| application.base_reference == record.xmi_id)
    {
        match application.name.as_str() {
            "Block" => return Some(ElementKind::Block),
            "AssociationBlock" => return Some(ElementKind::AssociationBlock),
            "InterfaceBlock" => return Some(ElementKind::InterfaceBlock),
            "ConstraintBlock" => return Some(ElementKind::ConstraintBlock),
            "ValueType" => return Some(ElementKind::ValueType),
            "Requirement" => return Some(ElementKind::Requirement),
            "TestCase" => return Some(ElementKind::TestCase),
            "PartProperty" => return Some(ElementKind::PartProperty),
            "ReferenceProperty" => return Some(ElementKind::ReferenceProperty),
            "ValueProperty" => return Some(ElementKind::ValueProperty),
            "FlowProperty" => return Some(ElementKind::FlowProperty),
            "ConstraintProperty" => return Some(ElementKind::ConstraintProperty),
            "ProxyPort" => return Some(ElementKind::ProxyPort),
            "FullPort" => return Some(ElementKind::FullPort),
            _ => {}
        }
    }
    native_element_kind(record)
}

fn local_type(value: &str) -> &str {
    value.rsplit(':').next().unwrap_or(value)
}

fn record_multiplicity(record: &super::xmi_interchange::XmiSemanticRecord) -> Option<Multiplicity> {
    if !record.attributes.contains_key("lower") && !record.attributes.contains_key("upper") {
        return None;
    }
    let lower = record
        .attributes
        .get("lower")
        .and_then(|value| value.parse().ok())
        .unwrap_or(1);
    let upper = match record.attributes.get("upper").map(String::as_str) {
        Some("*") => None,
        Some(value) => value.parse().ok().or(Some(1)),
        None => Some(1),
    };
    Multiplicity::new(lower, upper).ok()
}

fn record_visibility(record: &super::xmi_interchange::XmiSemanticRecord) -> Option<VisibilityKind> {
    match record.attributes.get("visibility").map(String::as_str) {
        Some("private") => Some(VisibilityKind::Private),
        Some("public") => Some(VisibilityKind::Public),
        _ => None,
    }
}

fn record_parameter_direction(
    record: &super::xmi_interchange::XmiSemanticRecord,
) -> Option<ParameterDirection> {
    match record.attributes.get("direction").map(String::as_str) {
        Some("in") => Some(ParameterDirection::In),
        Some("out") => Some(ParameterDirection::Out),
        Some("inout") => Some(ParameterDirection::InOut),
        Some("return") => Some(ParameterDirection::Return),
        _ => None,
    }
}

fn record_flow_direction(
    record: &super::xmi_interchange::XmiSemanticRecord,
) -> Option<FlowDirection> {
    match record
        .attributes
        .get("direction")
        .or_else(|| record.attributes.get("flowDirection"))
        .map(String::as_str)
    {
        Some("in") => Some(FlowDirection::In),
        Some("out") => Some(FlowDirection::Out),
        Some("inout") => Some(FlowDirection::InOut),
        _ => None,
    }
}

fn profile_owned_record_ids(document: &XmiSemanticDocument) -> BTreeSet<String> {
    let mut owned = document
        .records
        .iter()
        .filter(|record| {
            matches!(
                local_type(&record.xmi_type),
                "Profile" | "Stereotype" | "Extension"
            )
        })
        .map(|record| record.xmi_id.clone())
        .collect::<BTreeSet<_>>();
    loop {
        let before = owned.len();
        for record in &document.records {
            if record
                .owner_id
                .as_ref()
                .is_some_and(|owner| owned.contains(owner))
            {
                owned.insert(record.xmi_id.clone());
            }
        }
        if owned.len() == before {
            return owned;
        }
    }
}

fn ordered_records<'a>(
    document: &'a XmiSemanticDocument,
    roots: &BTreeSet<String>,
) -> Result<Vec<&'a super::xmi_interchange::XmiSemanticRecord>, String> {
    let mut remaining = document
        .records
        .iter()
        .filter(|record| !roots.contains(&record.xmi_id))
        .collect::<Vec<_>>();
    let mut available = roots.clone();
    let mut result = Vec::new();
    while !remaining.is_empty() {
        let before = remaining.len();
        let mut deferred = Vec::new();
        for record in remaining {
            if record
                .owner_id
                .as_ref()
                .is_none_or(|owner| available.contains(owner))
            {
                available.insert(record.xmi_id.clone());
                result.push(record);
            } else {
                deferred.push(record);
            }
        }
        if deferred.len() == before {
            return Err(format!(
                "ownership reference '{}' does not resolve",
                deferred[0].owner_id.as_deref().unwrap_or_default()
            ));
        }
        remaining = deferred;
    }
    Ok(result)
}

fn prepare_xmi_import(
    document: XmiSemanticDocument,
    mut configuration: XmiImportConfiguration,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
) -> PreparedXmiImport {
    configuration.source_namespace = configuration.source_namespace.trim().to_owned();
    let mut preview = XmiImportPreview {
        source_namespace: configuration.source_namespace.clone(),
        producer: document.producer.clone(),
        namespaces: document.namespaces.clone(),
        ..XmiImportPreview::default()
    };
    if configuration.source_namespace.is_empty() {
        preview.diagnostics.push(runtime_diagnostic(
            "SOURCE_NAMESPACE_REQUIRED",
            "XMI source namespace is required and must remain stable across reimport",
        ));
    }
    let embedded = match embedded_portable(&document) {
        Ok(value) => value,
        Err(reason) => {
            preview
                .diagnostics
                .push(runtime_diagnostic("XMI_TYPE_UNSUPPORTED", reason));
            None
        }
    };
    let project_guard = match workspace.project.lock() {
        Ok(guard) => guard,
        Err(_) => {
            preview.diagnostics.push(runtime_diagnostic(
                "SEMANTIC_VALIDATION",
                "project lock poisoned",
            ));
            preview.recount();
            return PreparedXmiImport {
                document,
                configuration: configuration.clone(),
                plan: ModelBuildPlan {
                    source_namespace: configuration.source_namespace,
                    operations: Vec::new(),
                },
                embedded,
                remove_elements: Vec::new(),
                remove_relationships: Vec::new(),
                preview,
            };
        }
    };
    let Some(project) = project_guard.as_ref() else {
        preview
            .diagnostics
            .push(runtime_diagnostic("SEMANTIC_VALIDATION", "no project open"));
        preview.recount();
        return PreparedXmiImport {
            document,
            configuration: configuration.clone(),
            plan: ModelBuildPlan {
                source_namespace: configuration.source_namespace,
                operations: Vec::new(),
            },
            embedded,
            remove_elements: Vec::new(),
            remove_relationships: Vec::new(),
            preview,
        };
    };
    let target_scope = parse_element_id(&configuration.target_scope).unwrap_or(project.root_id);
    if !project
        .elements
        .get(&target_scope)
        .is_some_and(|element| element.is_namespace())
    {
        preview.diagnostics.push(runtime_diagnostic(
            "SEMANTIC_VALIDATION",
            "XMI target scope must be an existing Model, Package, or ModelLibrary",
        ));
    }

    if let Some(portable) = &embedded {
        // `portable_from_states` snapshots `workspace.project` itself. Release the
        // validation guard first: std::sync::Mutex is non-reentrant, so retaining
        // this guard here would self-deadlock on native authored-state XMI.
        drop(project_guard);
        let current = portable_from_states(workspace, activity).ok();
        let incoming = semantic_only(portable.clone());
        let same = current
            .map(semantic_only)
            .and_then(|value| serde_json::to_value(value).ok())
            == serde_json::to_value(&incoming).ok();
        preview.items.push(XmiPreviewItem {
            action: if same {
                XmiAction::NoChange
            } else {
                XmiAction::Update
            },
            xmi_id: "systems-modeler-authored-state".into(),
            xmi_type: "SemanticModel".into(),
            detail: "lossless native semantic payload; diagram geometry is excluded".into(),
        });
        preview.recount();
        return PreparedXmiImport {
            document,
            configuration: configuration.clone(),
            plan: ModelBuildPlan {
                source_namespace: configuration.source_namespace,
                operations: Vec::new(),
            },
            embedded: Some(incoming),
            remove_elements: Vec::new(),
            remove_relationships: Vec::new(),
            preview,
        };
    }

    let roots = document
        .records
        .iter()
        .filter(|record| effective_kind(&document, record) == Some(ElementKind::Model))
        .map(|record| record.xmi_id.clone())
        .collect::<BTreeSet<_>>();
    let profile_owned = profile_owned_record_ids(&document);
    let records = match ordered_records(&document, &roots) {
        Ok(records) => records,
        Err(reason) => {
            preview
                .diagnostics
                .push(runtime_diagnostic("XMI_REFERENCE_UNRESOLVED", reason));
            Vec::new()
        }
    };
    let records = records
        .into_iter()
        .filter(|record| !profile_owned.contains(&record.xmi_id))
        .collect::<Vec<_>>();
    let incoming_elements = records
        .iter()
        .map(|record| record.xmi_id.clone())
        .collect::<BTreeSet<_>>();
    let incoming_relationships = document
        .relationships
        .iter()
        .map(|record| record.xmi_id.clone())
        .collect::<BTreeSet<_>>();
    let mut operations = Vec::new();
    let mut mapped_kinds = HashMap::new();
    for record in &records {
        let Some(kind) = effective_kind(&document, record) else {
            preview.diagnostics.push(XmiDiagnostic {
                severity: XmiDiagnosticSeverity::Warning,
                code: "XMI_TYPE_UNSUPPORTED".into(),
                reason: "semantic type is preserved as provenance but is outside the current native subset".into(),
                file: None,
                line: None,
                column: None,
                namespace: None,
                xmi_id: Some(record.xmi_id.clone()),
                xmi_type: Some(record.xmi_type.clone()),
                reference: None,
                semantic_target: None,
            });
            continue;
        };
        if kind.is_feature_kind() && record.type_reference.is_none() {
            blocked(
                &mut preview,
                &record.xmi_id,
                &record.xmi_type,
                "typed UML/SysML feature requires an exact type reference",
            );
            continue;
        }
        mapped_kinds.insert(record.xmi_id.clone(), kind.clone());
        let key = external_key(&configuration.source_namespace, &record.xmi_id);
        let matches = project
            .elements
            .values()
            .filter(|element| element.external_id == key)
            .collect::<Vec<_>>();
        if matches.len() > 1 {
            blocked(
                &mut preview,
                &record.xmi_id,
                &record.xmi_type,
                "XMI identity is ambiguous in the native model",
            );
            continue;
        }
        let owner = match record.owner_id.as_ref() {
            Some(owner) if roots.contains(owner) => BuildReference::Existing(target_scope),
            Some(owner) => BuildReference::External(owner.clone()),
            None => BuildReference::Existing(target_scope),
        };
        let type_ref = record
            .type_reference
            .as_ref()
            .map(|reference| BuildReference::External(reference.clone()));
        let multiplicity = record_multiplicity(record);
        let visibility = record_visibility(record);
        let parameter_direction = record_parameter_direction(record);
        let flow_direction = record_flow_direction(record);
        let is_conjugated = record
            .attributes
            .get("isConjugated")
            .and_then(|value| value.parse::<bool>().ok());
        let requirement = document.stereotype_applications.iter().find(|application| {
            application.base_reference == record.xmi_id && application.name == "Requirement"
        });
        let requirement_id = requirement
            .and_then(|application| application.tagged_values.get("id"))
            .and_then(|values| values.first())
            .cloned()
            .or_else(|| record.attributes.get("requirementId").cloned());
        let requirement_text = requirement
            .and_then(|application| application.tagged_values.get("text"))
            .and_then(|values| values.first())
            .cloned()
            .or_else(|| record.attributes.get("requirementText").cloned());
        if let Some(existing) = matches.first() {
            if existing.kind != kind {
                blocked(
                    &mut preview,
                    &record.xmi_id,
                    &record.xmi_type,
                    format!(
                        "wrong-kind identity collision: native is {:?}, XMI maps to {kind:?}",
                        existing.kind
                    ),
                );
                continue;
            }
            let changed = existing.name != record.name
                || multiplicity.is_some_and(|value| existing.multiplicity != Some(value))
                || visibility.is_some_and(|value| existing.visibility != value)
                || record
                    .attributes
                    .get("default")
                    .is_some_and(|value| existing.default_value.as_ref() != Some(value))
                || parameter_direction
                    .is_some_and(|value| existing.parameter_direction != Some(value))
                || flow_direction.is_some_and(|value| existing.flow_direction != Some(value))
                || is_conjugated.is_some_and(|value| existing.is_conjugated != value)
                || record
                    .attributes
                    .get("documentation")
                    .is_some_and(|value| existing.documentation != *value)
                || requirement_id
                    .as_ref()
                    .is_some_and(|value| existing.requirement_id.as_ref() != Some(value))
                || requirement_text
                    .as_ref()
                    .is_some_and(|value| existing.requirement_text.as_ref() != Some(value));
            preview.items.push(XmiPreviewItem {
                action: if changed {
                    XmiAction::Update
                } else {
                    XmiAction::NoChange
                },
                xmi_id: record.xmi_id.clone(),
                xmi_type: record.xmi_type.clone(),
                detail: existing.id.to_string(),
            });
            if changed {
                operations.push(ModelBuildOperation::UpdateElementFields {
                    element: BuildReference::Existing(existing.id),
                    name: Some(record.name.clone()),
                    owner: Some(owner.clone()),
                    type_ref,
                    external_id: None,
                    documentation: record.attributes.get("documentation").cloned(),
                    visibility,
                    requirement_id,
                    requirement_text,
                    multiplicity,
                    default_value: record.attributes.get("default").cloned(),
                    parameter_direction,
                    flow_direction,
                    is_conjugated,
                    extension_points: None,
                });
            }
        } else {
            preview.items.push(XmiPreviewItem {
                action: XmiAction::Create,
                xmi_id: record.xmi_id.clone(),
                xmi_type: record.xmi_type.clone(),
                detail: record.name.clone(),
            });
            operations.push(ModelBuildOperation::CreateElement {
                external_id: record.xmi_id.clone(),
                kind,
                name: record.name.clone(),
                owner,
                type_ref,
            });
            if record.attributes.contains_key("documentation")
                || requirement_id.is_some()
                || requirement_text.is_some()
                || multiplicity.is_some()
                || visibility.is_some()
                || record.attributes.contains_key("default")
                || parameter_direction.is_some()
                || flow_direction.is_some()
                || is_conjugated.is_some()
            {
                operations.push(ModelBuildOperation::UpdateElementFields {
                    element: BuildReference::External(record.xmi_id.clone()),
                    name: None,
                    owner: None,
                    type_ref: None,
                    external_id: None,
                    documentation: record.attributes.get("documentation").cloned(),
                    visibility,
                    requirement_id,
                    requirement_text,
                    multiplicity,
                    default_value: record.attributes.get("default").cloned(),
                    parameter_direction,
                    flow_direction,
                    is_conjugated,
                    extension_points: None,
                });
            }
        }
    }
    for record in &document.relationships {
        let Some(kind) = native_relationship_kind(record) else {
            preview.diagnostics.push(XmiDiagnostic {
                severity: XmiDiagnosticSeverity::Warning,
                code: "XMI_TYPE_UNSUPPORTED".into(),
                reason: "relationship is outside the current native subset".into(),
                file: None,
                line: None,
                column: None,
                namespace: None,
                xmi_id: Some(record.xmi_id.clone()),
                xmi_type: Some(record.xmi_type.clone()),
                reference: None,
                semantic_target: None,
            });
            continue;
        };
        if !mapped_kinds.contains_key(&record.source_reference)
            || !mapped_kinds.contains_key(&record.target_reference)
        {
            blocked(
                &mut preview,
                &record.xmi_id,
                &record.xmi_type,
                "relationship endpoint does not map to the current native semantic subset",
            );
            continue;
        }
        let key = external_key(&configuration.source_namespace, &record.xmi_id);
        let matches = project
            .relationships
            .values()
            .filter(|relationship| relationship.external_id == key)
            .collect::<Vec<_>>();
        if matches.len() > 1 {
            blocked(
                &mut preview,
                &record.xmi_id,
                &record.xmi_type,
                "XMI relationship identity is ambiguous",
            );
            continue;
        }
        if let Some(existing) = matches.first() {
            if existing.kind != kind {
                blocked(
                    &mut preview,
                    &record.xmi_id,
                    &record.xmi_type,
                    "wrong-kind relationship identity collision",
                );
                continue;
            }
            let changed = existing.name != record.name;
            preview.items.push(XmiPreviewItem {
                action: if changed {
                    XmiAction::Update
                } else {
                    XmiAction::NoChange
                },
                xmi_id: record.xmi_id.clone(),
                xmi_type: record.xmi_type.clone(),
                detail: existing.id.to_string(),
            });
            if changed {
                operations.push(ModelBuildOperation::UpdateRelationshipFields {
                    relationship: BuildReference::Existing(existing.id),
                    name: Some(record.name.clone()),
                    owner: None,
                    source: None,
                    target: None,
                    external_id: None,
                    documentation: record.attributes.get("documentation").cloned(),
                    visibility: None,
                    source_end: None,
                    target_end: None,
                    alias: None,
                    extension_condition: None,
                    extension_location: None,
                });
            }
        } else {
            let owner = if matches!(
                kind,
                RelationshipKind::PackageImport
                    | RelationshipKind::ElementImport
                    | RelationshipKind::PackageMerge
            ) {
                Some(BuildReference::External(record.source_reference.clone()))
            } else {
                Some(BuildReference::Existing(target_scope))
            };
            preview.items.push(XmiPreviewItem {
                action: XmiAction::Create,
                xmi_id: record.xmi_id.clone(),
                xmi_type: record.xmi_type.clone(),
                detail: format!("{} -> {}", record.source_reference, record.target_reference),
            });
            operations.push(ModelBuildOperation::CreateRelationship {
                external_id: record.xmi_id.clone(),
                kind,
                source: BuildReference::External(record.source_reference.clone()),
                target: BuildReference::External(record.target_reference.clone()),
                owner,
            });
        }
    }

    let mut remove_elements = Vec::new();
    let mut remove_relationships = Vec::new();
    if configuration.synchronization == XmiSynchronizationPolicy::AuthoritativeXmiScope {
        let prefix = format!("{}::", configuration.source_namespace);
        for relationship in project.relationships.values() {
            if let Some(id) = relationship.external_id.strip_prefix(&prefix)
                && !incoming_relationships.contains(id)
            {
                remove_relationships.push(relationship.id);
                preview.items.push(XmiPreviewItem {
                    action: XmiAction::Remove,
                    xmi_id: id.into(),
                    xmi_type: format!("{:?}", relationship.kind),
                    detail: "source-bound authoritative removal".into(),
                });
            }
        }
        let mut probe = project.clone();
        for id in &remove_relationships {
            probe
                .profiles
                .stereotype_applications
                .retain(|_, application| application.target != SemanticTarget::Relationship(*id));
            probe.relationships.remove(id);
        }
        let mut candidates = project
            .elements
            .values()
            .filter_map(|element| {
                element
                    .external_id
                    .strip_prefix(&prefix)
                    .filter(|id| !incoming_elements.contains(*id))
                    .map(|id| (id.to_owned(), element.id, element.kind.clone()))
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|(_, id, _)| std::cmp::Reverse(owner_depth(&probe, *id)));
        for (external_id, id, kind) in candidates {
            match probe.delete_element(id) {
                Ok(()) => {
                    remove_elements.push(id);
                    preview.items.push(XmiPreviewItem {
                        action: XmiAction::Remove,
                        xmi_id: external_id,
                        xmi_type: format!("{kind:?}"),
                        detail: "source-bound authoritative removal".into(),
                    });
                }
                Err(error) => blocked(
                    &mut preview,
                    &external_id,
                    &format!("{kind:?}"),
                    format!("authoritative removal is reference-protected: {error}"),
                ),
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
        preview.diagnostics.push(XmiDiagnostic {
            severity: match build.severity {
                BuildDiagnosticSeverity::Error => XmiDiagnosticSeverity::Error,
                BuildDiagnosticSeverity::Warning => XmiDiagnosticSeverity::Warning,
            },
            code: build.code.into(),
            reason: build.message,
            file: None,
            line: None,
            column: None,
            namespace: None,
            xmi_id: None,
            xmi_type: Some("ModelBuildPlan".into()),
            reference: None,
            semantic_target: None,
        });
    }
    preview.recount();
    PreparedXmiImport {
        document,
        configuration,
        plan,
        embedded: None,
        remove_elements,
        remove_relationships,
        preview,
    }
}

trait FeatureKind {
    fn is_feature_kind(&self) -> bool;
}

impl FeatureKind for ElementKind {
    fn is_feature_kind(&self) -> bool {
        matches!(
            self,
            ElementKind::PartProperty
                | ElementKind::ReferenceProperty
                | ElementKind::ValueProperty
                | ElementKind::FlowProperty
                | ElementKind::ConstraintProperty
                | ElementKind::ConstraintParameter
                | ElementKind::ProxyPort
                | ElementKind::FullPort
                | ElementKind::Parameter
        )
    }
}

fn owner_depth(project: &Project, mut id: ElementId) -> usize {
    let mut depth = 0;
    while let Some(owner) = project
        .elements
        .get(&id)
        .and_then(|element| element.owner_id)
    {
        depth += 1;
        id = owner;
    }
    depth
}

fn semantic_only(mut portable: PortableProjectV1) -> PortableProjectV1 {
    portable.diagrams.clear();
    portable.ibd_diagrams.clear();
    portable.activity.diagrams.clear();
    portable.behavior.diagrams.clear();
    portable
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

fn commit_candidate(
    live: &WorkspaceState,
    live_activity: &ActivityWorkspaceState,
    candidate: &WorkspaceState,
    candidate_activity: &ActivityWorkspaceState,
) -> Result<(), String> {
    *live.project.lock().map_err(|_| "project lock poisoned")? = candidate
        .project
        .lock()
        .map_err(|_| "candidate project lock poisoned")?
        .clone();
    *live.diagrams.lock().map_err(|_| "diagram lock poisoned")? = candidate
        .diagrams
        .lock()
        .map_err(|_| "candidate diagram lock poisoned")?
        .clone();
    *live.ibd_diagrams.lock().map_err(|_| "IBD lock poisoned")? = candidate
        .ibd_diagrams
        .lock()
        .map_err(|_| "candidate IBD lock poisoned")?
        .clone();
    *live.behavior.lock().map_err(|_| "behavior lock poisoned")? = candidate
        .behavior
        .lock()
        .map_err(|_| "candidate behavior lock poisoned")?
        .clone();
    *live
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")? = candidate
        .behavior_diagrams
        .lock()
        .map_err(|_| "candidate behavior diagram lock poisoned")?
        .clone();
    *live_activity
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")? = candidate_activity
        .repository
        .lock()
        .map_err(|_| "candidate Activity repository lock poisoned")?
        .clone();
    *live_activity
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")? = candidate_activity
        .diagrams
        .lock()
        .map_err(|_| "candidate Activity diagram lock poisoned")?
        .clone();
    Ok(())
}

fn replace_embedded_semantics(
    portable: PortableProjectV1,
    candidate: &WorkspaceState,
    candidate_activity: &ActivityWorkspaceState,
) -> Result<(), String> {
    let plan = portable.into_build_plan()?;
    let [ModelBuildOperation::RestorePortableState { state }] = plan.operations.as_slice() else {
        return Err("embedded XMI semantic payload did not produce one atomic restore".into());
    };
    *candidate
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(state.project.clone());
    *candidate
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")? = state.behavior_repository.clone();
    *candidate_activity
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")? = state.activity_repository.clone();
    portable_from_states(candidate, candidate_activity)?;
    Ok(())
}

fn apply_prepared(
    mut prepared: PreparedXmiImport,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
    history_state: Option<&HistoryState>,
) -> XmiImportPreview {
    if !prepared.preview.is_valid() {
        return prepared.preview;
    }
    let (candidate, candidate_activity) = match clone_states(workspace, activity) {
        Ok(value) => value,
        Err(reason) => {
            prepared
                .preview
                .diagnostics
                .push(runtime_diagnostic("SEMANTIC_VALIDATION", reason));
            prepared.preview.recount();
            return prepared.preview;
        }
    };
    let used_embedded = prepared.embedded.is_some();
    let apply_result = if let Some(portable) = prepared.embedded.take() {
        replace_embedded_semantics(portable, &candidate, &candidate_activity)
    } else {
        super::bulk_model::apply_unified_model_build(
            &prepared.plan,
            &candidate,
            &candidate_activity,
        )
        .map(|_| ())
        .map_err(|build| {
            build
                .diagnostics
                .into_iter()
                .map(|item| item.message)
                .collect::<Vec<_>>()
                .join("; ")
        })
    };
    if let Err(reason) = apply_result {
        prepared
            .preview
            .diagnostics
            .push(runtime_diagnostic("SEMANTIC_VALIDATION", reason));
        prepared.preview.recount();
        return prepared.preview;
    }
    {
        let mut project = candidate.project.lock().map_err(|_| ()).ok();
        let Some(project) = project.as_mut().and_then(|guard| guard.as_mut()) else {
            prepared.preview.diagnostics.push(runtime_diagnostic(
                "SEMANTIC_VALIDATION",
                "candidate project unavailable",
            ));
            prepared.preview.recount();
            return prepared.preview;
        };
        for id in &prepared.remove_relationships {
            project
                .profiles
                .stereotype_applications
                .retain(|_, application| application.target != SemanticTarget::Relationship(*id));
            project.relationships.remove(id);
        }
        for id in &prepared.remove_elements {
            if let Err(error) = project.delete_element(*id) {
                prepared.preview.diagnostics.push(runtime_diagnostic(
                    "REFERENCE_PROTECTED_REMOVE",
                    error.to_string(),
                ));
                prepared.preview.recount();
                return prepared.preview;
            }
        }
        if !used_embedded
            && let Err(reason) = apply_external_profiles(
                project,
                &prepared.document,
                &prepared.configuration.source_namespace,
            )
        {
            prepared
                .preview
                .diagnostics
                .push(runtime_diagnostic("SEMANTIC_VALIDATION", reason));
            prepared.preview.recount();
            return prepared.preview;
        }
        if let Err(error) = project.validate() {
            prepared
                .preview
                .diagnostics
                .push(runtime_diagnostic("SEMANTIC_VALIDATION", error.to_string()));
            prepared.preview.recount();
            return prepared.preview;
        }
    }
    if let Err(reason) = portable_from_states(&candidate, &candidate_activity) {
        prepared
            .preview
            .diagnostics
            .push(runtime_diagnostic("SEMANTIC_VALIDATION", reason));
        prepared.preview.recount();
        return prepared.preview;
    }
    if let Some(history_state) = history_state
        && let Err(reason) = history::checkpoint_states(workspace, activity, history_state)
    {
        prepared
            .preview
            .diagnostics
            .push(runtime_diagnostic("SEMANTIC_VALIDATION", reason));
        prepared.preview.recount();
        return prepared.preview;
    }
    if let Err(reason) = commit_candidate(workspace, activity, &candidate, &candidate_activity) {
        prepared
            .preview
            .diagnostics
            .push(runtime_diagnostic("SEMANTIC_VALIDATION", reason));
        prepared.preview.recount();
        return prepared.preview;
    }
    prepared.preview.applied = true;
    prepared.preview
}

fn apply_stereotype_labels(project: &mut Project, document: &XmiSemanticDocument, namespace: &str) {
    for application in &document.stereotype_applications {
        let key = external_key(namespace, &application.base_reference);
        if let Some(element) = project
            .elements
            .values_mut()
            .find(|element| element.external_id == key)
            && !element.applied_stereotypes.contains(&application.name)
        {
            element.applied_stereotypes.push(application.name.clone());
        }
    }
}

fn parse_external_tag_value(value_type: &TagValueType, value: &str) -> Result<TagValue, String> {
    match value_type {
        TagValueType::String => Ok(TagValue::String(value.to_owned())),
        TagValueType::Boolean => value
            .parse::<bool>()
            .map(TagValue::Boolean)
            .map_err(|_| format!("'{value}' is not a Boolean")),
        TagValueType::Integer => value
            .parse::<i64>()
            .map(TagValue::Integer)
            .map_err(|_| format!("'{value}' is not an Integer")),
        TagValueType::Real => value
            .parse::<f64>()
            .map(TagValue::Real)
            .map_err(|_| format!("'{value}' is not a Real")),
        TagValueType::Enumeration { literals }
            if literals.iter().any(|literal| literal == value) =>
        {
            Ok(TagValue::Enumeration(value.to_owned()))
        }
        TagValueType::Enumeration { .. } => Err(format!("'{value}' is not an enumeration literal")),
        TagValueType::SemanticReference => {
            Err("external semantic-reference tag values require an exact native target".into())
        }
    }
}

fn external_tag_type(
    document: &XmiSemanticDocument,
    record: &super::xmi_interchange::XmiSemanticRecord,
) -> TagValueType {
    let declared = record
        .attributes
        .get("tagType")
        .map(String::as_str)
        .or_else(|| {
            record.type_reference.as_ref().and_then(|reference| {
                document
                    .records
                    .iter()
                    .find(|candidate| candidate.xmi_id == *reference)
                    .map(|candidate| candidate.name.as_str())
            })
        })
        .unwrap_or("String");
    match declared.to_ascii_lowercase().as_str() {
        "boolean" => TagValueType::Boolean,
        "integer" | "int" => TagValueType::Integer,
        "real" | "double" | "float" => TagValueType::Real,
        _ => TagValueType::String,
    }
}

fn apply_external_profiles(
    project: &mut Project,
    document: &XmiSemanticDocument,
    namespace: &str,
) -> Result<(), String> {
    for (index, extension) in document.preserved_extensions.iter().enumerate() {
        project.profiles.interchange_extensions.insert(
            format!("{namespace}::xmi-extension::{index}"),
            extension.clone(),
        );
    }
    let profile_records = document
        .records
        .iter()
        .filter(|record| local_type(&record.xmi_type) == "Profile")
        .collect::<Vec<_>>();
    if profile_records.is_empty() {
        apply_stereotype_labels(project, document, namespace);
        return Ok(());
    }
    for profile_record in profile_records {
        let profile_external = format!("{namespace}::profile::{}", profile_record.xmi_id);
        let profile_id = project
            .profiles
            .profiles
            .values()
            .find(|profile| profile.external_id == profile_external)
            .map(|profile| profile.id)
            .map(Ok)
            .unwrap_or_else(|| {
                project.create_profile(
                    &profile_external,
                    &profile_record.name,
                    profile_record.attributes.get("URI").cloned(),
                )
            })?;
        project.apply_profile(
            profile_id,
            project.root_id,
            format!(
                "{namespace}::profile-application::{}",
                profile_record.xmi_id
            ),
        )?;
        for stereotype_record in document.records.iter().filter(|record| {
            record.owner_id.as_deref() == Some(profile_record.xmi_id.as_str())
                && local_type(&record.xmi_type) == "Stereotype"
        }) {
            let matching_applications = document
                .stereotype_applications
                .iter()
                .filter(|application| application.name == stereotype_record.name)
                .collect::<Vec<_>>();
            let mut extends = Vec::new();
            for application in &matching_applications {
                let key = external_key(namespace, &application.base_reference);
                if let Some(element) = project
                    .elements
                    .values()
                    .find(|element| element.external_id == key)
                {
                    let kind = StereotypeTargetKind::Element(element.kind.clone());
                    if !extends.contains(&kind) {
                        extends.push(kind);
                    }
                } else if let Some(relationship) = project
                    .relationships
                    .values()
                    .find(|relationship| relationship.external_id == key)
                {
                    let kind = StereotypeTargetKind::Relationship(relationship.kind.clone());
                    if !extends.contains(&kind) {
                        extends.push(kind);
                    }
                }
            }
            if extends.is_empty() {
                continue;
            }
            let stereotype_external =
                format!("{namespace}::stereotype::{}", stereotype_record.xmi_id);
            let stereotype_id = project
                .profiles
                .stereotypes
                .values()
                .find(|stereotype| stereotype.external_id == stereotype_external)
                .map(|stereotype| stereotype.id)
                .map(Ok)
                .unwrap_or_else(|| {
                    project.create_stereotype(
                        profile_id,
                        &stereotype_external,
                        &stereotype_record.name,
                        extends,
                    )
                })?;
            let mut tags = BTreeMap::new();
            for tag_record in document.records.iter().filter(|record| {
                record.owner_id.as_deref() == Some(stereotype_record.xmi_id.as_str())
                    && local_type(&record.xmi_type) == "Property"
            }) {
                let tag_external = format!("{namespace}::tag::{}", tag_record.xmi_id);
                let value_type = external_tag_type(document, tag_record);
                let definition_id = project
                    .profiles
                    .tag_definitions
                    .values()
                    .find(|definition| definition.external_id == tag_external)
                    .map(|definition| definition.id)
                    .map(Ok)
                    .unwrap_or_else(|| {
                        project.create_tag_definition(
                            stereotype_id,
                            &tag_external,
                            &tag_record.name,
                            value_type.clone(),
                            (
                                tag_record
                                    .attributes
                                    .get("lower")
                                    .and_then(|value| value.parse().ok())
                                    .unwrap_or(0),
                                tag_record.attributes.get("upper").and_then(|value| {
                                    (value != "*").then(|| value.parse().ok()).flatten()
                                }),
                            ),
                            None,
                        )
                    })?;
                tags.insert(tag_record.name.clone(), (definition_id, value_type));
            }
            for application in matching_applications {
                let key = external_key(namespace, &application.base_reference);
                let target = if let Some(element) = project
                    .elements
                    .values()
                    .find(|element| element.external_id == key)
                {
                    SemanticTarget::Element(element.id)
                } else if let Some(relationship) = project
                    .relationships
                    .values()
                    .find(|relationship| relationship.external_id == key)
                {
                    SemanticTarget::Relationship(relationship.id)
                } else {
                    continue;
                };
                let application_id = project.apply_stereotype(
                    stereotype_id,
                    target,
                    format!(
                        "{namespace}::stereotype-application::{}",
                        application.xmi_id
                    ),
                )?;
                for (name, values) in &application.tagged_values {
                    if let Some((definition_id, value_type)) = tags.get(name) {
                        let parsed = values
                            .iter()
                            .map(|value| parse_external_tag_value(value_type, value))
                            .collect::<Result<Vec<_>, _>>()?;
                        project.set_tagged_values(application_id, *definition_id, parsed)?;
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn preview_xmi_xml(
    xml: &str,
    file_name: Option<&str>,
    configuration: XmiImportConfiguration,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
) -> XmiImportPreview {
    match parse_xmi(xml, file_name) {
        Ok(document) => prepare_xmi_import(document, configuration, workspace, activity).preview,
        Err(diagnostics) => {
            let mut preview = XmiImportPreview {
                source_namespace: configuration.source_namespace,
                diagnostics,
                ..XmiImportPreview::default()
            };
            preview.recount();
            preview
        }
    }
}

pub fn apply_xmi_xml(
    xml: &str,
    file_name: Option<&str>,
    configuration: XmiImportConfiguration,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
    history_state: Option<&HistoryState>,
) -> XmiImportPreview {
    match parse_xmi(xml, file_name) {
        Ok(document) => apply_prepared(
            prepare_xmi_import(document, configuration, workspace, activity),
            workspace,
            activity,
            history_state,
        ),
        Err(diagnostics) => {
            let mut preview = XmiImportPreview {
                source_namespace: configuration.source_namespace,
                diagnostics,
                ..XmiImportPreview::default()
            };
            preview.recount();
            preview
        }
    }
}

fn read_xmi(path: &Path) -> Result<(String, String), String> {
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if metadata.len() > MAX_XMI_BYTES {
        return Err("XMI input exceeds the 64 MiB safety limit".into());
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "xmi" | "uml" | "xml") {
        return Err("XMI import accepts .xmi, .uml, or .xml files".into());
    }
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("input.xmi")
        .into();
    let xml = fs::read_to_string(path).map_err(|error| error.to_string())?;
    Ok((name, xml))
}

#[tauri::command]
pub fn preview_xmi_import(
    path: String,
    configuration: XmiImportConfiguration,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
) -> Result<XmiImportPreview, String> {
    let (name, xml) = read_xmi(Path::new(&path))?;
    Ok(preview_xmi_xml(
        &xml,
        Some(&name),
        configuration,
        &workspace,
        &activity,
    ))
}

#[tauri::command]
pub fn apply_xmi_import(
    path: String,
    configuration: XmiImportConfiguration,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history_state: tauri::State<'_, HistoryState>,
) -> Result<XmiImportPreview, String> {
    let (name, xml) = read_xmi(Path::new(&path))?;
    Ok(apply_xmi_xml(
        &xml,
        Some(&name),
        configuration,
        &workspace,
        &activity,
        Some(&history_state),
    ))
}

#[tauri::command]
pub fn stage_xmi_upload(file_name: String, bytes: Vec<u8>) -> Result<String, String> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_XMI_BYTES {
        return Err("XMI upload must be between 1 byte and 64 MiB".into());
    }
    let extension = Path::new(&file_name)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or("XMI file extension is required")?;
    if !matches!(extension.as_str(), "xmi" | "uml" | "xml") {
        return Err("only .xmi, .uml, or .xml uploads are supported".into());
    }
    let path = std::env::temp_dir().join(format!(
        "systems-modeler-xmi-{}.{}",
        uuid::Uuid::new_v4(),
        extension
    ));
    fs::write(&path, bytes).map_err(|error| error.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn discard_staged_xmi(path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("invalid staged XMI path")?;
    if path.parent() != Some(std::env::temp_dir().as_path())
        || !file_name.starts_with("systems-modeler-xmi-")
    {
        return Err("only staged XMI uploads can be discarded".into());
    }
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

#[tauri::command]
pub fn export_xmi(
    path: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
) -> Result<String, String> {
    let portable = semantic_only(portable_from_states(&workspace, &activity)?);
    let xml = serialize_xmi(&portable)?;
    let mut path = PathBuf::from(path);
    if path.extension().and_then(|value| value.to_str()) != Some("xmi") {
        path.set_extension("xmi");
    }
    fs::write(&path, xml).map_err(|error| error.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::super::xmi_interchange::tests::UML_FIXTURE;
    use super::*;
    use crate::workspace::portable_interchange::tests::representative_states;

    #[test]
    fn external_uml_preview_is_nonmutating_and_apply_is_idempotent() {
        let workspace = WorkspaceState::default();
        let activity = ActivityWorkspaceState::default();
        let project = Project::new("Target");
        let root = project.root_id;
        *workspace.project.lock().unwrap() = Some(project);
        let configuration = XmiImportConfiguration {
            source_namespace: "xmi:external".into(),
            target_scope: root.to_string(),
            synchronization: XmiSynchronizationPolicy::AdditiveUpdate,
        };
        let preview = preview_xmi_xml(
            UML_FIXTURE,
            Some("external.uml"),
            configuration.clone(),
            &workspace,
            &activity,
        );
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(preview.create_count, 3);
        assert_eq!(
            workspace
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            1
        );

        let applied = apply_xmi_xml(
            UML_FIXTURE,
            None,
            configuration.clone(),
            &workspace,
            &activity,
            None,
        );
        assert!(applied.applied, "{:?}", applied.diagnostics);
        let repeated = preview_xmi_xml(UML_FIXTURE, None, configuration, &workspace, &activity);
        assert_eq!(repeated.create_count, 0);
        assert_eq!(repeated.no_change_count, 3);
    }

    #[test]
    fn external_profile_becomes_native_profile_and_typed_application() {
        let fixture = include_str!("../../../../../examples/xmi/external-sysml-profile.xmi");
        let workspace = WorkspaceState::default();
        let activity = ActivityWorkspaceState::default();
        let project = Project::new("Target");
        let root = project.root_id;
        *workspace.project.lock().unwrap() = Some(project);
        let configuration = XmiImportConfiguration {
            source_namespace: "xmi:safety".into(),
            target_scope: root.to_string(),
            synchronization: XmiSynchronizationPolicy::AdditiveUpdate,
        };

        let applied = apply_xmi_xml(
            fixture,
            Some("external-sysml-profile.xmi"),
            configuration,
            &workspace,
            &activity,
            None,
        );
        assert!(applied.applied, "{:?}", applied.diagnostics);
        let guard = workspace.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        assert_eq!(project.profiles.profiles.len(), 1);
        assert_eq!(project.profiles.stereotypes.len(), 1);
        assert_eq!(project.profiles.tag_definitions.len(), 1);
        let application = project
            .profiles
            .stereotype_applications
            .values()
            .next()
            .unwrap();
        assert!(
            application
                .tagged_values
                .values()
                .flatten()
                .any(|value| value == &TagValue::Integer(3))
        );
        project.validate().unwrap();
    }

    #[test]
    fn native_semantics_round_trip_losslessly_without_diagram_geometry() {
        let (source, source_activity) = representative_states();
        {
            let mut guard = source.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            let target = project
                .elements
                .values()
                .find(|element| element.kind == ElementKind::Block)
                .unwrap()
                .id;
            let profile = project
                .create_profile("profile:quality", "Quality", None)
                .unwrap();
            let stereotype = project
                .create_stereotype(
                    profile,
                    "stereotype:reviewed",
                    "Reviewed",
                    vec![StereotypeTargetKind::Element(ElementKind::Block)],
                )
                .unwrap();
            let tag = project
                .create_tag_definition(
                    stereotype,
                    "tag:score",
                    "score",
                    TagValueType::Real,
                    (0, Some(1)),
                    None,
                )
                .unwrap();
            project
                .apply_profile(profile, project.root_id, "profile-application:quality")
                .unwrap();
            let application = project
                .apply_stereotype(
                    stereotype,
                    SemanticTarget::Element(target),
                    "stereotype-application:reviewed",
                )
                .unwrap();
            project
                .set_tagged_values(application, tag, vec![TagValue::Real(0.95)])
                .unwrap();
        }
        let portable = semantic_only(portable_from_states(&source, &source_activity).unwrap());
        let first = serialize_xmi(&portable).unwrap();
        let second = serialize_xmi(&portable).unwrap();
        assert_eq!(first, second);
        assert!(!first.contains("edge_routes"));

        let target = WorkspaceState::default();
        let target_activity = ActivityWorkspaceState::default();
        let target_project = Project::new("Blank");
        let root = target_project.root_id;
        *target.project.lock().unwrap() = Some(target_project);
        let applied = apply_xmi_xml(
            &first,
            Some("native-round-trip.xmi"),
            XmiImportConfiguration {
                source_namespace: "xmi:native-round-trip".into(),
                target_scope: root.to_string(),
                synchronization: XmiSynchronizationPolicy::AdditiveUpdate,
            },
            &target,
            &target_activity,
            None,
        );
        assert!(applied.applied, "{:?}", applied.diagnostics);
        let reconstructed = semantic_only(portable_from_states(&target, &target_activity).unwrap());
        assert_eq!(
            serde_json::to_value(portable).unwrap(),
            serde_json::to_value(reconstructed).unwrap()
        );
    }

    #[test]
    fn embedded_xmi_preview_and_apply_complete_without_recursive_project_lock() {
        use std::{sync::mpsc, thread, time::Duration};

        let (source, source_activity) = representative_states();
        let portable = semantic_only(portable_from_states(&source, &source_activity).unwrap());
        let xml = serialize_xmi(&portable).unwrap();
        assert!(xml.contains("sm:authoredState"));

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let target = WorkspaceState::default();
            let target_activity = ActivityWorkspaceState::default();
            let target_project = Project::new("Embedded XMI Target");
            let root = target_project.root_id;
            *target.project.lock().unwrap() = Some(target_project);
            let configuration = XmiImportConfiguration {
                source_namespace: "xmi:embedded-watchdog".into(),
                target_scope: root.to_string(),
                synchronization: XmiSynchronizationPolicy::AdditiveUpdate,
            };

            let preview = preview_xmi_xml(
                &xml,
                Some("embedded-watchdog.xmi"),
                configuration.clone(),
                &target,
                &target_activity,
            );
            let applied = apply_xmi_xml(
                &xml,
                Some("embedded-watchdog.xmi"),
                configuration,
                &target,
                &target_activity,
                None,
            );
            let _ = tx.send((preview, applied));
        });

        let (preview, applied) = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("embedded XMI preview/apply exceeded 10 seconds; probable lock deadlock");
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert!(applied.applied, "{:?}", applied.diagnostics);
    }

    #[test]
    fn authoritative_remove_is_source_bound_and_late_parse_error_is_atomic() {
        let workspace = WorkspaceState::default();
        let activity = ActivityWorkspaceState::default();
        let project = Project::new("Target");
        let root = project.root_id;
        *workspace.project.lock().unwrap() = Some(project);
        let configuration = XmiImportConfiguration {
            source_namespace: "xmi:authority".into(),
            target_scope: root.to_string(),
            synchronization: XmiSynchronizationPolicy::AuthoritativeXmiScope,
        };
        assert!(
            apply_xmi_xml(
                UML_FIXTURE,
                None,
                configuration.clone(),
                &workspace,
                &activity,
                None
            )
            .applied
        );
        // Git may materialize the included fixture with CRLF on Windows. Normalize
        // only the test input used to remove rows so the authoritative-sync assertion
        // exercises the same two semantic removals on every runner.
        let normalized_fixture = UML_FIXTURE.replace("\r\n", "\n");
        let reduced = normalized_fixture
            .replace(
                "    <packagedElement x:type=\"u:Class\" x:id=\"sensor\" name=\"Sensor\" />\n",
                "",
            )
            .replace(
                "    <packagedElement x:type=\"u:Dependency\" x:id=\"uses\" client=\"controller\" supplier=\"sensor\" />\n",
                "",
            );
        let preview = preview_xmi_xml(
            &reduced,
            Some("reduced.xmi"),
            configuration.clone(),
            &workspace,
            &activity,
        );
        assert_eq!(preview.remove_count, 2);
        assert!(
            apply_xmi_xml(
                &reduced,
                None,
                configuration.clone(),
                &workspace,
                &activity,
                None
            )
            .applied
        );

        let before = serde_json::to_value(workspace.project.lock().unwrap().as_ref()).unwrap();
        let invalid = reduced.replace("</x:XMI>", "<broken></x:XMI>");
        let result = apply_xmi_xml(
            &invalid,
            Some("invalid-late.xmi"),
            configuration,
            &workspace,
            &activity,
            None,
        );
        assert!(!result.applied);
        assert_eq!(
            serde_json::to_value(workspace.project.lock().unwrap().as_ref()).unwrap(),
            before
        );
    }
}
