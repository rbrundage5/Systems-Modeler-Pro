//! PR52 ReqIF interchange.
//!
//! ReqIF XML is normalized here before any native semantic mutation. XML DOM
//! nodes are never product semantics. Native requirements remain ordinary
//! `Project` elements/relationships; this module only carries exchange fidelity,
//! provenance, mapping configuration, preview, and deterministic serialization.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use systems_modeler_core::{ElementKind, RelationshipKind};

pub(super) const REQIF_METADATA_KEY: &str = "reqif-exchange-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReqifAction {
    Create,
    Update,
    NoChange,
    Remove,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReqifSynchronizationPolicy {
    Additive,
    AuthoritativeReqifScope,
}

impl Default for ReqifSynchronizationPolicy {
    fn default() -> Self {
        Self::Additive
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReqifNativeField {
    RequirementId,
    RequirementText,
    Name,
    Documentation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReqifImportConfiguration {
    pub source_namespace: String,
    pub target_scope: String,
    #[serde(default)]
    pub synchronization: ReqifSynchronizationPolicy,
    #[serde(default)]
    pub object_type_mappings: BTreeMap<String, ElementKind>,
    #[serde(default)]
    pub relation_type_mappings: BTreeMap<String, RelationshipKind>,
    #[serde(default)]
    pub attribute_mappings: BTreeMap<String, ReqifNativeField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReqifDiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReqifDiagnostic {
    pub severity: ReqifDiagnosticSeverity,
    pub file: Option<String>,
    pub identifier: Option<String>,
    pub record_kind: Option<String>,
    pub attribute_or_type: Option<String>,
    pub source: Option<String>,
    pub target: Option<String>,
    pub native_target: Option<String>,
    pub code: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReqifPreviewItem {
    pub action: ReqifAction,
    pub identifier: String,
    pub kind: String,
    pub detail: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReqifPreviewTotals {
    pub create: usize,
    pub update: usize,
    pub no_change: usize,
    pub remove: usize,
    pub blocked: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReqifImportPreview {
    pub applied: bool,
    pub source_namespace: String,
    pub items: Vec<ReqifPreviewItem>,
    pub diagnostics: Vec<ReqifDiagnostic>,
    pub totals: ReqifPreviewTotals,
}

impl ReqifImportPreview {
    pub fn recount(&mut self) {
        self.totals = ReqifPreviewTotals::default();
        for item in &self.items {
            match item.action {
                ReqifAction::Create => self.totals.create += 1,
                ReqifAction::Update => self.totals.update += 1,
                ReqifAction::NoChange => self.totals.no_change += 1,
                ReqifAction::Remove => self.totals.remove += 1,
                ReqifAction::Blocked => self.totals.blocked += 1,
            }
        }
        if self
            .diagnostics
            .iter()
            .any(|item| item.severity == ReqifDiagnosticSeverity::Error)
            && self.totals.blocked == 0
        {
            self.totals.blocked = 1;
        }
    }

    pub fn is_valid(&self) -> bool {
        self.totals.blocked == 0
            && !self
                .diagnostics
                .iter()
                .any(|item| item.severity == ReqifDiagnosticSeverity::Error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReqifDatatypeKind {
    String,
    Xhtml,
    Boolean,
    Integer,
    Real,
    Date,
    Enumeration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReqifEnumValue {
    pub identifier: String,
    pub long_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReqifDatatype {
    pub identifier: String,
    pub long_name: String,
    pub kind: ReqifDatatypeKind,
    #[serde(default)]
    pub enum_values: Vec<ReqifEnumValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReqifAttributeDefinition {
    pub identifier: String,
    pub long_name: String,
    pub kind: ReqifDatatypeKind,
    pub datatype_identifier: Option<String>,
    #[serde(default)]
    pub multi_valued: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReqifSpecTypeKind {
    SpecObject,
    SpecRelation,
    Specification,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReqifSpecType {
    pub identifier: String,
    pub long_name: String,
    pub kind: ReqifSpecTypeKind,
    #[serde(default)]
    pub attributes: Vec<ReqifAttributeDefinition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReqifValue {
    String(String),
    Xhtml {
        plain_text: String,
        original_xml: String,
    },
    Boolean(bool),
    Integer(i64),
    Real(f64),
    Date(String),
    Enumeration(Vec<String>),
}

impl ReqifValue {
    pub fn readable_text(&self, enum_labels: &BTreeMap<String, String>) -> String {
        match self {
            Self::String(value) | Self::Date(value) => value.clone(),
            Self::Xhtml { plain_text, .. } => plain_text.clone(),
            Self::Boolean(value) => value.to_string(),
            Self::Integer(value) => value.to_string(),
            Self::Real(value) => value.to_string(),
            Self::Enumeration(values) => values
                .iter()
                .map(|id| enum_labels.get(id).cloned().unwrap_or_else(|| id.clone()))
                .collect::<Vec<_>>()
                .join(", "),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReqifAttributeValue {
    pub definition_identifier: String,
    pub value: ReqifValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReqifSpecObject {
    pub identifier: String,
    pub long_name: String,
    pub type_identifier: String,
    #[serde(default)]
    pub values: Vec<ReqifAttributeValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReqifHierarchyNode {
    pub identifier: String,
    pub object_identifier: String,
    #[serde(default)]
    pub children: Vec<ReqifHierarchyNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReqifSpecification {
    pub identifier: String,
    pub long_name: String,
    pub type_identifier: Option<String>,
    #[serde(default)]
    pub children: Vec<ReqifHierarchyNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReqifSpecRelation {
    pub identifier: String,
    pub long_name: String,
    pub type_identifier: String,
    pub source_identifier: String,
    pub target_identifier: String,
    #[serde(default)]
    pub values: Vec<ReqifAttributeValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReqifDocument {
    pub header_identifier: String,
    pub title: String,
    pub creation_time: Option<String>,
    #[serde(default)]
    pub datatypes: Vec<ReqifDatatype>,
    #[serde(default)]
    pub spec_types: Vec<ReqifSpecType>,
    #[serde(default)]
    pub spec_objects: Vec<ReqifSpecObject>,
    #[serde(default)]
    pub specifications: Vec<ReqifSpecification>,
    #[serde(default)]
    pub spec_relations: Vec<ReqifSpecRelation>,
}

impl ReqifDocument {
    pub fn validate_references(&self) -> Vec<ReqifDiagnostic> {
        let datatype_ids = self
            .datatypes
            .iter()
            .map(|item| item.identifier.as_str())
            .collect::<BTreeSet<_>>();
        let type_ids = self
            .spec_types
            .iter()
            .map(|item| item.identifier.as_str())
            .collect::<BTreeSet<_>>();
        let object_ids = self
            .spec_objects
            .iter()
            .map(|item| item.identifier.as_str())
            .collect::<BTreeSet<_>>();
        let attribute_ids = self
            .spec_types
            .iter()
            .flat_map(|item| item.attributes.iter())
            .map(|item| item.identifier.as_str())
            .collect::<BTreeSet<_>>();
        let mut diagnostics = Vec::new();

        for spec_type in &self.spec_types {
            for attribute in &spec_type.attributes {
                if let Some(datatype) = attribute.datatype_identifier.as_deref()
                    && !datatype_ids.contains(datatype)
                {
                    diagnostics.push(reference_diagnostic(
                        "REQIF_TYPE_UNRESOLVED",
                        &attribute.identifier,
                        "ATTRIBUTE-DEFINITION",
                        Some(datatype),
                        "attribute definition references an unknown datatype",
                    ));
                }
            }
        }
        for object in &self.spec_objects {
            if !type_ids.contains(object.type_identifier.as_str()) {
                diagnostics.push(reference_diagnostic(
                    "REQIF_TYPE_UNRESOLVED",
                    &object.identifier,
                    "SPEC-OBJECT",
                    Some(&object.type_identifier),
                    "SPEC-OBJECT references an unknown SPEC-TYPE",
                ));
            }
            for value in &object.values {
                if !attribute_ids.contains(value.definition_identifier.as_str()) {
                    diagnostics.push(reference_diagnostic(
                        "REQIF_REFERENCE_UNRESOLVED",
                        &object.identifier,
                        "SPEC-OBJECT",
                        Some(&value.definition_identifier),
                        "attribute value references an unknown attribute definition",
                    ));
                }
            }
        }
        for relation in &self.spec_relations {
            if !type_ids.contains(relation.type_identifier.as_str()) {
                diagnostics.push(reference_diagnostic(
                    "REQIF_TYPE_UNRESOLVED",
                    &relation.identifier,
                    "SPEC-RELATION",
                    Some(&relation.type_identifier),
                    "SPEC-RELATION references an unknown SPEC-TYPE",
                ));
            }
            if !object_ids.contains(relation.source_identifier.as_str())
                || !object_ids.contains(relation.target_identifier.as_str())
            {
                diagnostics.push(ReqifDiagnostic {
                    severity: ReqifDiagnosticSeverity::Error,
                    file: None,
                    identifier: Some(relation.identifier.clone()),
                    record_kind: Some("SPEC-RELATION".into()),
                    attribute_or_type: Some(relation.type_identifier.clone()),
                    source: Some(relation.source_identifier.clone()),
                    target: Some(relation.target_identifier.clone()),
                    native_target: None,
                    code: "REQIF_REFERENCE_UNRESOLVED".into(),
                    reason: "SPEC-RELATION source or target is unresolved".into(),
                });
            }
        }
        for specification in &self.specifications {
            validate_hierarchy_nodes(
                &specification.children,
                &object_ids,
                &mut BTreeSet::new(),
                &mut diagnostics,
            );
        }
        diagnostics
    }

    pub fn enum_labels(&self) -> BTreeMap<String, String> {
        self.datatypes
            .iter()
            .flat_map(|datatype| datatype.enum_values.iter())
            .map(|item| (item.identifier.clone(), item.long_name.clone()))
            .collect()
    }

    pub fn attribute_definitions(&self) -> BTreeMap<String, &ReqifAttributeDefinition> {
        self.spec_types
            .iter()
            .flat_map(|spec_type| spec_type.attributes.iter())
            .map(|attribute| (attribute.identifier.clone(), attribute))
            .collect()
    }

    pub fn spec_types_by_id(&self) -> BTreeMap<String, &ReqifSpecType> {
        self.spec_types
            .iter()
            .map(|spec_type| (spec_type.identifier.clone(), spec_type))
            .collect()
    }
}

fn reference_diagnostic(
    code: &str,
    identifier: &str,
    record_kind: &str,
    reference: Option<&str>,
    reason: &str,
) -> ReqifDiagnostic {
    ReqifDiagnostic {
        severity: ReqifDiagnosticSeverity::Error,
        file: None,
        identifier: Some(identifier.into()),
        record_kind: Some(record_kind.into()),
        attribute_or_type: reference.map(Into::into),
        source: None,
        target: None,
        native_target: None,
        code: code.into(),
        reason: reason.into(),
    }
}

fn validate_hierarchy_nodes(
    nodes: &[ReqifHierarchyNode],
    object_ids: &BTreeSet<&str>,
    active: &mut BTreeSet<String>,
    diagnostics: &mut Vec<ReqifDiagnostic>,
) {
    for node in nodes {
        if node.identifier.trim().is_empty() || !active.insert(node.identifier.clone()) {
            diagnostics.push(reference_diagnostic(
                "REQIF_HIERARCHY_INVALID",
                &node.identifier,
                "SPEC-HIERARCHY",
                None,
                "hierarchy identifiers must be non-empty and acyclic",
            ));
            continue;
        }
        if !object_ids.contains(node.object_identifier.as_str()) {
            diagnostics.push(reference_diagnostic(
                "REQIF_REFERENCE_UNRESOLVED",
                &node.identifier,
                "SPEC-HIERARCHY",
                Some(&node.object_identifier),
                "SPEC-HIERARCHY references an unknown SPEC-OBJECT",
            ));
        }
        validate_hierarchy_nodes(&node.children, object_ids, active, diagnostics);
        active.remove(&node.identifier);
    }
}

fn reqif_kind_from_tag(tag: &str) -> Option<ReqifDatatypeKind> {
    if tag.ends_with("-STRING") {
        Some(ReqifDatatypeKind::String)
    } else if tag.ends_with("-XHTML") {
        Some(ReqifDatatypeKind::Xhtml)
    } else if tag.ends_with("-BOOLEAN") {
        Some(ReqifDatatypeKind::Boolean)
    } else if tag.ends_with("-INTEGER") {
        Some(ReqifDatatypeKind::Integer)
    } else if tag.ends_with("-REAL") {
        Some(ReqifDatatypeKind::Real)
    } else if tag.ends_with("-DATE") {
        Some(ReqifDatatypeKind::Date)
    } else if tag.ends_with("-ENUMERATION") {
        Some(ReqifDatatypeKind::Enumeration)
    } else {
        None
    }
}

fn child_element<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
    name: &str,
) -> Option<roxmltree::Node<'a, 'input>> {
    node.children()
        .find(|child| child.is_element() && child.tag_name().name() == name)
}

fn first_ref(node: roxmltree::Node<'_, '_>, wrapper: &str) -> Option<String> {
    child_element(node, wrapper)
        .and_then(|container| {
            container.descendants().find(|child| {
                child.is_element() && child.tag_name().name().ends_with("-REF")
            })
        })
        .and_then(|reference| reference.text())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn node_identifier(node: roxmltree::Node<'_, '_>) -> String {
    node.attribute("IDENTIFIER").unwrap_or_default().trim().to_owned()
}

fn node_long_name(node: roxmltree::Node<'_, '_>) -> String {
    node.attribute("LONG-NAME").unwrap_or_default().trim().to_owned()
}

fn parse_datatype(node: roxmltree::Node<'_, '_>) -> Option<ReqifDatatype> {
    let kind = reqif_kind_from_tag(node.tag_name().name())?;
    let enum_values = if kind == ReqifDatatypeKind::Enumeration {
        node.descendants()
            .filter(|child| child.is_element() && child.tag_name().name() == "ENUM-VALUE")
            .map(|child| ReqifEnumValue {
                identifier: node_identifier(child),
                long_name: node_long_name(child),
            })
            .filter(|item| !item.identifier.is_empty())
            .collect()
    } else {
        Vec::new()
    };
    Some(ReqifDatatype {
        identifier: node_identifier(node),
        long_name: node_long_name(node),
        kind,
        enum_values,
    })
}

fn parse_attribute_definition(
    node: roxmltree::Node<'_, '_>,
) -> Option<ReqifAttributeDefinition> {
    let kind = reqif_kind_from_tag(node.tag_name().name())?;
    let datatype_identifier = first_ref(node, "TYPE");
    let multi_valued = node
        .attribute("MULTI-VALUED")
        .is_some_and(|value| value.eq_ignore_ascii_case("true"));
    Some(ReqifAttributeDefinition {
        identifier: node_identifier(node),
        long_name: node_long_name(node),
        kind,
        datatype_identifier,
        multi_valued,
    })
}

fn parse_spec_type(node: roxmltree::Node<'_, '_>) -> Option<ReqifSpecType> {
    let kind = match node.tag_name().name() {
        "SPEC-OBJECT-TYPE" => ReqifSpecTypeKind::SpecObject,
        "SPEC-RELATION-TYPE" => ReqifSpecTypeKind::SpecRelation,
        "SPECIFICATION-TYPE" => ReqifSpecTypeKind::Specification,
        _ => return None,
    };
    let attributes = child_element(node, "SPEC-ATTRIBUTES")
        .into_iter()
        .flat_map(|container| container.children().filter(|child| child.is_element()))
        .filter_map(parse_attribute_definition)
        .collect();
    Some(ReqifSpecType {
        identifier: node_identifier(node),
        long_name: node_long_name(node),
        kind,
        attributes,
    })
}

fn readable_xhtml(node: roxmltree::Node<'_, '_>) -> String {
    node.descendants()
        .filter_map(|child| child.text())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_attribute_value(
    node: roxmltree::Node<'_, '_>,
    xml: &str,
) -> Result<Option<ReqifAttributeValue>, String> {
    let Some(kind) = reqif_kind_from_tag(node.tag_name().name()) else {
        return Ok(None);
    };
    let definition_identifier = first_ref(node, "DEFINITION")
        .ok_or_else(|| "attribute value has no DEFINITION reference".to_string())?;
    let scalar = node.attribute("THE-VALUE").unwrap_or_default().trim();
    let value = match kind {
        ReqifDatatypeKind::String => ReqifValue::String(scalar.into()),
        ReqifDatatypeKind::Boolean => ReqifValue::Boolean(match scalar {
            "true" | "1" | "TRUE" => true,
            "false" | "0" | "FALSE" => false,
            other => return Err(format!("invalid ReqIF boolean value: {other}")),
        }),
        ReqifDatatypeKind::Integer => ReqifValue::Integer(
            scalar
                .parse()
                .map_err(|_| format!("invalid ReqIF integer value: {scalar}"))?,
        ),
        ReqifDatatypeKind::Real => ReqifValue::Real(
            scalar
                .parse()
                .map_err(|_| format!("invalid ReqIF real value: {scalar}"))?,
        ),
        ReqifDatatypeKind::Date => ReqifValue::Date(scalar.into()),
        ReqifDatatypeKind::Enumeration => ReqifValue::Enumeration(
            node.descendants()
                .filter(|child| {
                    child.is_element() && child.tag_name().name() == "ENUM-VALUE-REF"
                })
                .filter_map(|child| child.text())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
        ),
        ReqifDatatypeKind::Xhtml => {
            let xhtml = child_element(node, "THE-VALUE");
            let original_xml = xhtml
                .map(|child| xml[child.range()].to_owned())
                .unwrap_or_default();
            ReqifValue::Xhtml {
                plain_text: xhtml.map(readable_xhtml).unwrap_or_default(),
                original_xml,
            }
        }
    };
    Ok(Some(ReqifAttributeValue {
        definition_identifier,
        value,
    }))
}

fn parse_values(node: roxmltree::Node<'_, '_>, xml: &str) -> Result<Vec<ReqifAttributeValue>, String> {
    child_element(node, "VALUES")
        .into_iter()
        .flat_map(|container| container.children().filter(|child| child.is_element()))
        .filter_map(|child| match parse_attribute_value(child, xml) {
            Ok(Some(value)) => Some(Ok(value)),
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

fn parse_hierarchy(node: roxmltree::Node<'_, '_>) -> Result<ReqifHierarchyNode, String> {
    let identifier = node_identifier(node);
    let object_identifier = first_ref(node, "OBJECT")
        .ok_or_else(|| format!("SPEC-HIERARCHY {identifier} has no OBJECT reference"))?;
    let children = child_element(node, "CHILDREN")
        .into_iter()
        .flat_map(|container| container.children().filter(|child| child.is_element()))
        .filter(|child| child.tag_name().name() == "SPEC-HIERARCHY")
        .map(parse_hierarchy)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ReqifHierarchyNode {
        identifier,
        object_identifier,
        children,
    })
}

fn direct_section<'a, 'input>(
    root: roxmltree::Node<'a, 'input>,
    name: &str,
) -> Option<roxmltree::Node<'a, 'input>> {
    root.descendants()
        .find(|node| node.is_element() && node.tag_name().name() == name)
}

pub fn parse_reqif(xml: &str, file: Option<&str>) -> Result<ReqifDocument, Vec<ReqifDiagnostic>> {
    let document = roxmltree::Document::parse(xml).map_err(|error| {
        vec![ReqifDiagnostic {
            severity: ReqifDiagnosticSeverity::Error,
            file: file.map(Into::into),
            identifier: None,
            record_kind: None,
            attribute_or_type: None,
            source: None,
            target: None,
            native_target: None,
            code: "REQIF_XML_INVALID".into(),
            reason: error.to_string(),
        }]
    })?;
    let root = document.root_element();
    if root.tag_name().name() != "REQ-IF" {
        return Err(vec![ReqifDiagnostic {
            severity: ReqifDiagnosticSeverity::Error,
            file: file.map(Into::into),
            identifier: None,
            record_kind: None,
            attribute_or_type: None,
            source: None,
            target: None,
            native_target: None,
            code: "REQIF_XML_INVALID".into(),
            reason: "document root is not REQ-IF".into(),
        }]);
    }

    let header = direct_section(root, "REQ-IF-HEADER");
    let header_identifier = header.map(node_identifier).unwrap_or_default();
    let title = header.map(node_long_name).unwrap_or_default();
    let creation_time = header
        .and_then(|item| child_element(item, "CREATION-TIME"))
        .and_then(|item| item.text())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    let datatypes = direct_section(root, "DATATYPES")
        .into_iter()
        .flat_map(|container| container.children().filter(|child| child.is_element()))
        .filter_map(parse_datatype)
        .collect::<Vec<_>>();
    let spec_types = direct_section(root, "SPEC-TYPES")
        .into_iter()
        .flat_map(|container| container.children().filter(|child| child.is_element()))
        .filter_map(parse_spec_type)
        .collect::<Vec<_>>();

    let mut parse_errors = Vec::new();
    let mut spec_objects = Vec::new();
    if let Some(container) = direct_section(root, "SPEC-OBJECTS") {
        for node in container
            .children()
            .filter(|child| child.is_element() && child.tag_name().name() == "SPEC-OBJECT")
        {
            let identifier = node_identifier(node);
            match (first_ref(node, "TYPE"), parse_values(node, xml)) {
                (Some(type_identifier), Ok(values)) => spec_objects.push(ReqifSpecObject {
                    identifier,
                    long_name: node_long_name(node),
                    type_identifier,
                    values,
                }),
                (None, _) => parse_errors.push(reference_diagnostic(
                    "REQIF_TYPE_UNRESOLVED",
                    &identifier,
                    "SPEC-OBJECT",
                    None,
                    "SPEC-OBJECT has no TYPE reference",
                )),
                (_, Err(reason)) => parse_errors.push(reference_diagnostic(
                    "REQIF_ATTRIBUTE_INVALID",
                    &identifier,
                    "SPEC-OBJECT",
                    None,
                    &reason,
                )),
            }
        }
    }

    let mut specifications = Vec::new();
    if let Some(container) = direct_section(root, "SPECIFICATIONS") {
        for node in container
            .children()
            .filter(|child| child.is_element() && child.tag_name().name() == "SPECIFICATION")
        {
            let children = child_element(node, "CHILDREN")
                .into_iter()
                .flat_map(|items| items.children().filter(|child| child.is_element()))
                .filter(|child| child.tag_name().name() == "SPEC-HIERARCHY")
                .map(parse_hierarchy)
                .collect::<Result<Vec<_>, _>>();
            match children {
                Ok(children) => specifications.push(ReqifSpecification {
                    identifier: node_identifier(node),
                    long_name: node_long_name(node),
                    type_identifier: first_ref(node, "TYPE"),
                    children,
                }),
                Err(reason) => parse_errors.push(reference_diagnostic(
                    "REQIF_HIERARCHY_INVALID",
                    &node_identifier(node),
                    "SPECIFICATION",
                    None,
                    &reason,
                )),
            }
        }
    }

    let mut spec_relations = Vec::new();
    if let Some(container) = direct_section(root, "SPEC-RELATIONS") {
        for node in container
            .children()
            .filter(|child| child.is_element() && child.tag_name().name() == "SPEC-RELATION")
        {
            let identifier = node_identifier(node);
            let parsed = (
                first_ref(node, "TYPE"),
                first_ref(node, "SOURCE"),
                first_ref(node, "TARGET"),
                parse_values(node, xml),
            );
            match parsed {
                (Some(type_identifier), Some(source_identifier), Some(target_identifier), Ok(values)) => {
                    spec_relations.push(ReqifSpecRelation {
                        identifier,
                        long_name: node_long_name(node),
                        type_identifier,
                        source_identifier,
                        target_identifier,
                        values,
                    });
                }
                (_, _, _, Err(reason)) => parse_errors.push(reference_diagnostic(
                    "REQIF_ATTRIBUTE_INVALID",
                    &identifier,
                    "SPEC-RELATION",
                    None,
                    &reason,
                )),
                _ => parse_errors.push(reference_diagnostic(
                    "REQIF_REFERENCE_UNRESOLVED",
                    &identifier,
                    "SPEC-RELATION",
                    None,
                    "SPEC-RELATION requires TYPE, SOURCE, and TARGET references",
                )),
            }
        }
    }

    let normalized = ReqifDocument {
        header_identifier,
        title,
        creation_time,
        datatypes,
        spec_types,
        spec_objects,
        specifications,
        spec_relations,
    };
    parse_errors.extend(normalized.validate_references());
    for item in &mut parse_errors {
        item.file = file.map(Into::into);
    }
    if parse_errors.is_empty() {
        Ok(normalized)
    } else {
        Err(parse_errors)
    }
}

pub fn detected_object_kind(document: &ReqifDocument, type_identifier: &str) -> Option<ElementKind> {
    let spec_type = document
        .spec_types
        .iter()
        .find(|item| item.identifier == type_identifier)?;
    if spec_type.kind != ReqifSpecTypeKind::SpecObject {
        return None;
    }
    let name = spec_type.long_name.to_ascii_lowercase();
    if name.contains("test case") || name.contains("testcase") || name.contains("verification case") {
        Some(ElementKind::TestCase)
    } else if name.contains("requirement") || name.contains("requirement") || name.contains("spec object") {
        Some(ElementKind::Requirement)
    } else {
        None
    }
}

pub fn detected_relation_kind(
    document: &ReqifDocument,
    type_identifier: &str,
) -> Option<RelationshipKind> {
    let spec_type = document
        .spec_types
        .iter()
        .find(|item| item.identifier == type_identifier)?;
    if spec_type.kind != ReqifSpecTypeKind::SpecRelation {
        return None;
    }
    let normalized = spec_type
        .long_name
        .to_ascii_lowercase()
        .replace([' ', '-', '_'], "");
    match normalized.as_str() {
        "trace" => Some(RelationshipKind::Trace),
        "derivereqt" | "deriverequirement" => Some(RelationshipKind::DeriveRequirement),
        "satisfy" => Some(RelationshipKind::Satisfy),
        "verify" => Some(RelationshipKind::Verify),
        "refine" => Some(RelationshipKind::Refine),
        "copy" => Some(RelationshipKind::Copy),
        "dependency" => Some(RelationshipKind::Dependency),
        _ => None,
    }
}

pub fn detected_attribute_mapping(
    definition: &ReqifAttributeDefinition,
) -> Option<ReqifNativeField> {
    let normalized = definition
        .long_name
        .to_ascii_lowercase()
        .replace([' ', '-', '_', '.'], "");
    match normalized.as_str() {
        "requirementid" | "reqid" | "identifier" => Some(ReqifNativeField::RequirementId),
        "requirementtext" | "reqiftext" | "text" | "description" => {
            Some(ReqifNativeField::RequirementText)
        }
        "name" | "title" | "longname" => Some(ReqifNativeField::Name),
        "documentation" | "notes" | "note" => Some(ReqifNativeField::Documentation),
        _ => None,
    }
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn datatype_tag(kind: ReqifDatatypeKind) -> &'static str {
    match kind {
        ReqifDatatypeKind::String => "STRING",
        ReqifDatatypeKind::Xhtml => "XHTML",
        ReqifDatatypeKind::Boolean => "BOOLEAN",
        ReqifDatatypeKind::Integer => "INTEGER",
        ReqifDatatypeKind::Real => "REAL",
        ReqifDatatypeKind::Date => "DATE",
        ReqifDatatypeKind::Enumeration => "ENUMERATION",
    }
}

pub fn serialize_reqif(document: &ReqifDocument) -> String {
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<REQ-IF xmlns=\"http://www.omg.org/spec/ReqIF/20110401/reqif.xsd\">\n",
    );
    out.push_str("  <THE-HEADER><REQ-IF-HEADER");
    out.push_str(&format!(
        " IDENTIFIER=\"{}\" LONG-NAME=\"{}\">",
        escape_xml(&document.header_identifier),
        escape_xml(&document.title)
    ));
    if let Some(time) = &document.creation_time {
        out.push_str(&format!("<CREATION-TIME>{}</CREATION-TIME>", escape_xml(time)));
    }
    out.push_str("</REQ-IF-HEADER></THE-HEADER>\n  <CORE-CONTENT><REQ-IF-CONTENT>\n");
    out.push_str("    <DATATYPES>\n");
    for datatype in &document.datatypes {
        let suffix = datatype_tag(datatype.kind);
        out.push_str(&format!(
            "      <DATATYPE-DEFINITION-{suffix} IDENTIFIER=\"{}\" LONG-NAME=\"{}\">",
            escape_xml(&datatype.identifier),
            escape_xml(&datatype.long_name)
        ));
        if datatype.kind == ReqifDatatypeKind::Enumeration {
            out.push_str("<SPECIFIED-VALUES>");
            for item in &datatype.enum_values {
                out.push_str(&format!(
                    "<ENUM-VALUE IDENTIFIER=\"{}\" LONG-NAME=\"{}\"/>",
                    escape_xml(&item.identifier),
                    escape_xml(&item.long_name)
                ));
            }
            out.push_str("</SPECIFIED-VALUES>");
        }
        out.push_str(&format!("</DATATYPE-DEFINITION-{suffix}>\n"));
    }
    out.push_str("    </DATATYPES>\n    <SPEC-TYPES>\n");
    for spec_type in &document.spec_types {
        let tag = match spec_type.kind {
            ReqifSpecTypeKind::SpecObject => "SPEC-OBJECT-TYPE",
            ReqifSpecTypeKind::SpecRelation => "SPEC-RELATION-TYPE",
            ReqifSpecTypeKind::Specification => "SPECIFICATION-TYPE",
        };
        out.push_str(&format!(
            "      <{tag} IDENTIFIER=\"{}\" LONG-NAME=\"{}\"><SPEC-ATTRIBUTES>",
            escape_xml(&spec_type.identifier),
            escape_xml(&spec_type.long_name)
        ));
        for attribute in &spec_type.attributes {
            let suffix = datatype_tag(attribute.kind);
            out.push_str(&format!(
                "<ATTRIBUTE-DEFINITION-{suffix} IDENTIFIER=\"{}\" LONG-NAME=\"{}\"{}>",
                escape_xml(&attribute.identifier),
                escape_xml(&attribute.long_name),
                if attribute.multi_valued { " MULTI-VALUED=\"true\"" } else { "" }
            ));
            if let Some(datatype) = &attribute.datatype_identifier {
                out.push_str(&format!(
                    "<TYPE><DATATYPE-DEFINITION-{suffix}-REF>{}</DATATYPE-DEFINITION-{suffix}-REF></TYPE>",
                    escape_xml(datatype)
                ));
            }
            out.push_str(&format!("</ATTRIBUTE-DEFINITION-{suffix}>"));
        }
        out.push_str(&format!("</SPEC-ATTRIBUTES></{tag}>\n"));
    }
    out.push_str("    </SPEC-TYPES>\n    <SPEC-OBJECTS>\n");
    for object in &document.spec_objects {
        out.push_str(&format!(
            "      <SPEC-OBJECT IDENTIFIER=\"{}\" LONG-NAME=\"{}\"><TYPE><SPEC-OBJECT-TYPE-REF>{}</SPEC-OBJECT-TYPE-REF></TYPE><VALUES>",
            escape_xml(&object.identifier),
            escape_xml(&object.long_name),
            escape_xml(&object.type_identifier)
        ));
        serialize_values(&mut out, &object.values);
        out.push_str("</VALUES></SPEC-OBJECT>\n");
    }
    out.push_str("    </SPEC-OBJECTS>\n    <SPEC-RELATIONS>\n");
    for relation in &document.spec_relations {
        out.push_str(&format!(
            "      <SPEC-RELATION IDENTIFIER=\"{}\" LONG-NAME=\"{}\"><TYPE><SPEC-RELATION-TYPE-REF>{}</SPEC-RELATION-TYPE-REF></TYPE><SOURCE><SPEC-OBJECT-REF>{}</SPEC-OBJECT-REF></SOURCE><TARGET><SPEC-OBJECT-REF>{}</SPEC-OBJECT-REF></TARGET><VALUES>",
            escape_xml(&relation.identifier),
            escape_xml(&relation.long_name),
            escape_xml(&relation.type_identifier),
            escape_xml(&relation.source_identifier),
            escape_xml(&relation.target_identifier)
        ));
        serialize_values(&mut out, &relation.values);
        out.push_str("</VALUES></SPEC-RELATION>\n");
    }
    out.push_str("    </SPEC-RELATIONS>\n    <SPECIFICATIONS>\n");
    for specification in &document.specifications {
        out.push_str(&format!(
            "      <SPECIFICATION IDENTIFIER=\"{}\" LONG-NAME=\"{}\">",
            escape_xml(&specification.identifier),
            escape_xml(&specification.long_name)
        ));
        if let Some(type_identifier) = &specification.type_identifier {
            out.push_str(&format!(
                "<TYPE><SPECIFICATION-TYPE-REF>{}</SPECIFICATION-TYPE-REF></TYPE>",
                escape_xml(type_identifier)
            ));
        }
        out.push_str("<CHILDREN>");
        for child in &specification.children {
            serialize_hierarchy(&mut out, child);
        }
        out.push_str("</CHILDREN></SPECIFICATION>\n");
    }
    out.push_str("    </SPECIFICATIONS>\n  </REQ-IF-CONTENT></CORE-CONTENT>\n</REQ-IF>\n");
    out
}

fn serialize_values(out: &mut String, values: &[ReqifAttributeValue]) {
    for attribute in values {
        let (suffix, scalar) = match &attribute.value {
            ReqifValue::String(value) => ("STRING", Some(value.clone())),
            ReqifValue::Boolean(value) => ("BOOLEAN", Some(value.to_string())),
            ReqifValue::Integer(value) => ("INTEGER", Some(value.to_string())),
            ReqifValue::Real(value) => ("REAL", Some(value.to_string())),
            ReqifValue::Date(value) => ("DATE", Some(value.clone())),
            ReqifValue::Xhtml { .. } => ("XHTML", None),
            ReqifValue::Enumeration(_) => ("ENUMERATION", None),
        };
        out.push_str(&format!("<ATTRIBUTE-VALUE-{suffix}"));
        if let Some(scalar) = scalar {
            out.push_str(&format!(" THE-VALUE=\"{}\"", escape_xml(&scalar)));
        }
        out.push('>');
        out.push_str(&format!(
            "<DEFINITION><ATTRIBUTE-DEFINITION-{suffix}-REF>{}</ATTRIBUTE-DEFINITION-{suffix}-REF></DEFINITION>",
            escape_xml(&attribute.definition_identifier)
        ));
        match &attribute.value {
            ReqifValue::Xhtml {
                plain_text,
                original_xml,
            } => {
                if original_xml.trim().starts_with("<THE-VALUE") {
                    out.push_str(original_xml);
                } else {
                    out.push_str(&format!("<THE-VALUE>{}</THE-VALUE>", escape_xml(plain_text)));
                }
            }
            ReqifValue::Enumeration(values) => {
                out.push_str("<VALUES>");
                for value in values {
                    out.push_str(&format!(
                        "<ENUM-VALUE-REF>{}</ENUM-VALUE-REF>",
                        escape_xml(value)
                    ));
                }
                out.push_str("</VALUES>");
            }
            _ => {}
        }
        out.push_str(&format!("</ATTRIBUTE-VALUE-{suffix}>"));
    }
}

fn serialize_hierarchy(out: &mut String, node: &ReqifHierarchyNode) {
    out.push_str(&format!(
        "<SPEC-HIERARCHY IDENTIFIER=\"{}\"><OBJECT><SPEC-OBJECT-REF>{}</SPEC-OBJECT-REF></OBJECT>",
        escape_xml(&node.identifier),
        escape_xml(&node.object_identifier)
    ));
    if !node.children.is_empty() {
        out.push_str("<CHILDREN>");
        for child in &node.children {
            serialize_hierarchy(out, child);
        }
        out.push_str("</CHILDREN>");
    }
    out.push_str("</SPEC-HIERARCHY>");
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReqifExchangeState {
    #[serde(default)]
    pub sources: BTreeMap<String, ReqifSourceState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReqifSourceState {
    pub document: ReqifDocument,
    #[serde(default)]
    pub element_bindings: BTreeMap<String, String>,
    #[serde(default)]
    pub relationship_bindings: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd">
<THE-HEADER><REQ-IF-HEADER IDENTIFIER="H" LONG-NAME="Example"><CREATION-TIME>2026-09-02T00:00:00Z</CREATION-TIME></REQ-IF-HEADER></THE-HEADER>
<CORE-CONTENT><REQ-IF-CONTENT>
<DATATYPES>
<DATATYPE-DEFINITION-STRING IDENTIFIER="DT-S" LONG-NAME="String"/>
<DATATYPE-DEFINITION-ENUMERATION IDENTIFIER="DT-E" LONG-NAME="Priority"><SPECIFIED-VALUES><ENUM-VALUE IDENTIFIER="EV-H" LONG-NAME="High"/><ENUM-VALUE IDENTIFIER="EV-L" LONG-NAME="Low"/></SPECIFIED-VALUES></DATATYPE-DEFINITION-ENUMERATION>
</DATATYPES>
<SPEC-TYPES>
<SPEC-OBJECT-TYPE IDENTIFIER="T-REQ" LONG-NAME="Requirement"><SPEC-ATTRIBUTES><ATTRIBUTE-DEFINITION-STRING IDENTIFIER="A-ID" LONG-NAME="Requirement ID"><TYPE><DATATYPE-DEFINITION-STRING-REF>DT-S</DATATYPE-DEFINITION-STRING-REF></TYPE></ATTRIBUTE-DEFINITION-STRING><ATTRIBUTE-DEFINITION-STRING IDENTIFIER="A-TEXT" LONG-NAME="Requirement Text"><TYPE><DATATYPE-DEFINITION-STRING-REF>DT-S</DATATYPE-DEFINITION-STRING-REF></TYPE></ATTRIBUTE-DEFINITION-STRING><ATTRIBUTE-DEFINITION-ENUMERATION IDENTIFIER="A-PRI" LONG-NAME="Priority"><TYPE><DATATYPE-DEFINITION-ENUMERATION-REF>DT-E</DATATYPE-DEFINITION-ENUMERATION-REF></TYPE></ATTRIBUTE-DEFINITION-ENUMERATION></SPEC-ATTRIBUTES></SPEC-OBJECT-TYPE>
<SPEC-RELATION-TYPE IDENTIFIER="T-TRACE" LONG-NAME="Trace"><SPEC-ATTRIBUTES/></SPEC-RELATION-TYPE>
<SPECIFICATION-TYPE IDENTIFIER="T-SPEC" LONG-NAME="Specification"><SPEC-ATTRIBUTES/></SPECIFICATION-TYPE>
</SPEC-TYPES>
<SPEC-OBJECTS>
<SPEC-OBJECT IDENTIFIER="R-1" LONG-NAME="First"><TYPE><SPEC-OBJECT-TYPE-REF>T-REQ</SPEC-OBJECT-TYPE-REF></TYPE><VALUES><ATTRIBUTE-VALUE-STRING THE-VALUE="REQ-1"><DEFINITION><ATTRIBUTE-DEFINITION-STRING-REF>A-ID</ATTRIBUTE-DEFINITION-STRING-REF></DEFINITION></ATTRIBUTE-VALUE-STRING><ATTRIBUTE-VALUE-STRING THE-VALUE="Text"><DEFINITION><ATTRIBUTE-DEFINITION-STRING-REF>A-TEXT</ATTRIBUTE-DEFINITION-STRING-REF></DEFINITION></ATTRIBUTE-VALUE-STRING><ATTRIBUTE-VALUE-ENUMERATION><DEFINITION><ATTRIBUTE-DEFINITION-ENUMERATION-REF>A-PRI</ATTRIBUTE-DEFINITION-ENUMERATION-REF></DEFINITION><VALUES><ENUM-VALUE-REF>EV-H</ENUM-VALUE-REF></VALUES></ATTRIBUTE-VALUE-ENUMERATION></VALUES></SPEC-OBJECT>
<SPEC-OBJECT IDENTIFIER="R-2" LONG-NAME="Second"><TYPE><SPEC-OBJECT-TYPE-REF>T-REQ</SPEC-OBJECT-TYPE-REF></TYPE><VALUES/></SPEC-OBJECT>
</SPEC-OBJECTS>
<SPEC-RELATIONS><SPEC-RELATION IDENTIFIER="REL-1" LONG-NAME="trace"><TYPE><SPEC-RELATION-TYPE-REF>T-TRACE</SPEC-RELATION-TYPE-REF></TYPE><SOURCE><SPEC-OBJECT-REF>R-1</SPEC-OBJECT-REF></SOURCE><TARGET><SPEC-OBJECT-REF>R-2</SPEC-OBJECT-REF></TARGET><VALUES/></SPEC-RELATION></SPEC-RELATIONS>
<SPECIFICATIONS><SPECIFICATION IDENTIFIER="S-1" LONG-NAME="Requirements"><TYPE><SPECIFICATION-TYPE-REF>T-SPEC</SPECIFICATION-TYPE-REF></TYPE><CHILDREN><SPEC-HIERARCHY IDENTIFIER="SH-1"><OBJECT><SPEC-OBJECT-REF>R-1</SPEC-OBJECT-REF></OBJECT><CHILDREN><SPEC-HIERARCHY IDENTIFIER="SH-2"><OBJECT><SPEC-OBJECT-REF>R-2</SPEC-OBJECT-REF></OBJECT></SPEC-HIERARCHY></CHILDREN></SPEC-HIERARCHY></CHILDREN></SPECIFICATION></SPECIFICATIONS>
</REQ-IF-CONTENT></CORE-CONTENT></REQ-IF>"#;

    #[test]
    fn parses_core_reqif_and_preserves_enum_identity_and_hierarchy() {
        let parsed = parse_reqif(MINIMAL, Some("external.reqif")).expect("parse");
        assert_eq!(parsed.header_identifier, "H");
        assert_eq!(parsed.spec_objects.len(), 2);
        assert_eq!(parsed.spec_relations.len(), 1);
        assert_eq!(parsed.specifications[0].children[0].children.len(), 1);
        assert_eq!(parsed.datatypes[1].enum_values[0].identifier, "EV-H");
        assert_eq!(detected_object_kind(&parsed, "T-REQ"), Some(ElementKind::Requirement));
        assert_eq!(detected_relation_kind(&parsed, "T-TRACE"), Some(RelationshipKind::Trace));
    }

    #[test]
    fn deterministic_serialization_reparses() {
        let parsed = parse_reqif(MINIMAL, None).expect("parse");
        let first = serialize_reqif(&parsed);
        let second = serialize_reqif(&parsed);
        assert_eq!(first, second);
        let reparsed = parse_reqif(&first, None).expect("reparse");
        assert_eq!(reparsed.spec_objects.len(), parsed.spec_objects.len());
        assert_eq!(reparsed.spec_relations.len(), parsed.spec_relations.len());
    }

    #[test]
    fn malformed_xml_is_diagnostic() {
        let diagnostics = parse_reqif("<REQ-IF>", Some("bad.reqif")).unwrap_err();
        assert_eq!(diagnostics[0].code, "REQIF_XML_INVALID");
        assert_eq!(diagnostics[0].file.as_deref(), Some("bad.reqif"));
    }
}
