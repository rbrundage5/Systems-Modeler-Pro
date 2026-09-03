//! Namespace-aware, standards-shaped semantic XMI adapter for PR53.
//!
//! The parser produces a neutral IR and never mutates the repository. The runtime layer turns
//! this IR into a `ModelBuildPlan`, previews it, and commits a validated candidate atomically.

use super::portable_interchange::{PortableProjectV1, PortableSemanticProjectV1};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use systems_modeler_core::{
    Element, ElementKind, Relationship, RelationshipKind, SemanticTarget, TagValue, TagValueType,
};
use systems_modeler_core::behavior::{Region, VertexKind};

pub const UML_NS: &str = "http://www.omg.org/spec/UML/20131001";
pub const XMI_NS: &str = "http://www.omg.org/spec/XMI/20131001";
pub const SYSML_NS: &str = "http://www.omg.org/spec/SysML/20150709/SysML";
pub const SM_NS: &str = "https://systems-modeler.dev/xmi/semantic/1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum XmiDiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct XmiDiagnostic {
    pub severity: XmiDiagnosticSeverity,
    pub code: String,
    pub reason: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub namespace: Option<String>,
    pub xmi_id: Option<String>,
    pub xmi_type: Option<String>,
    pub reference: Option<String>,
    pub semantic_target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct XmiSemanticRecord {
    pub xmi_id: String,
    pub xmi_type: String,
    pub name: String,
    pub owner_id: Option<String>,
    pub type_reference: Option<String>,
    pub attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct XmiRelationshipRecord {
    pub xmi_id: String,
    pub xmi_type: String,
    pub name: String,
    pub owner_id: Option<String>,
    pub source_reference: String,
    pub target_reference: String,
    pub attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct XmiStereotypeRecord {
    pub xmi_id: String,
    pub namespace: String,
    pub name: String,
    pub base_reference: String,
    pub base_metaclass: String,
    pub tagged_values: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct XmiBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct XmiDiagramNodeRecord {
    pub xmi_id: String,
    pub semantic_reference: String,
    pub bounds: Option<XmiBounds>,
    pub notation: Option<String>,
    pub compartments: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct XmiDiagramEdgeRecord {
    pub xmi_id: String,
    pub semantic_reference: String,
    pub source_presentation_reference: String,
    pub target_presentation_reference: String,
    pub waypoints: Vec<(f64, f64)>,
    pub label_anchor: Option<(f64, f64)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct XmiDiagramRecord {
    pub xmi_id: String,
    pub name: String,
    pub family: String,
    pub owner_reference: String,
    pub context_reference: Option<String>,
    pub parent_diagram_reference: Option<String>,
    pub producer_namespace: Option<String>,
    pub nodes: Vec<XmiDiagramNodeRecord>,
    pub edges: Vec<XmiDiagramEdgeRecord>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct XmiSemanticDocument {
    pub namespaces: BTreeMap<String, String>,
    pub producer: Option<String>,
    pub records: Vec<XmiSemanticRecord>,
    pub relationships: Vec<XmiRelationshipRecord>,
    pub stereotype_applications: Vec<XmiStereotypeRecord>,
    pub diagrams: Vec<XmiDiagramRecord>,
    pub embedded_portable_json: Option<String>,
    pub preserved_extensions: Vec<String>,
}

fn parse_finite(value: Option<&str>) -> Option<f64> {
    value?.parse::<f64>().ok().filter(|number| number.is_finite())
}

fn normalized_family(value: &str) -> Option<&'static str> {
    let compact = value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect::<String>();
    match compact.as_str() {
        "bdd" | "blockdefinition" | "blockdefinitiondiagram" => Some("bdd"),
        "ibd" | "internalblock" | "internalblockdiagram" => Some("ibd"),
        "req" | "requirement" | "requirementdiagram" => Some("requirement"),
        "uc" | "usecase" | "usecasediagram" => Some("use-case"),
        "pkg" | "package" | "packagediagram" => Some("package"),
        "act" | "activity" | "activitydiagram" => Some("activity"),
        "stm" | "statemachine" | "statemachinediagram" => Some("state-machine"),
        "sd" | "sequence" | "sequencediagram" | "interaction" => Some("sequence"),
        "par" | "parametric" | "parametricdiagram" => Some("parametric"),
        _ => None,
    }
}

fn presentation_reference(node: roxmltree::Node<'_, '_>) -> Option<String> {
    ["element", "semanticElement", "modelElement", "subject", "semantic"]
        .into_iter()
        .find_map(|name| local_attribute(node, name).map(ToOwned::to_owned))
}

fn parse_bounds(node: roxmltree::Node<'_, '_>) -> Option<XmiBounds> {
    let bounds = node
        .children()
        .find(|child| child.is_element() && child.tag_name().name() == "Bounds")
        .unwrap_or(node);
    let result = XmiBounds {
        x: parse_finite(local_attribute(bounds, "x"))?,
        y: parse_finite(local_attribute(bounds, "y"))?,
        width: parse_finite(local_attribute(bounds, "width"))?,
        height: parse_finite(local_attribute(bounds, "height"))?,
    };
    (result.width > 0.0 && result.height > 0.0).then_some(result)
}

fn parse_diagrams(parsed: &roxmltree::Document<'_>) -> Vec<XmiDiagramRecord> {
    let mut diagrams = Vec::new();
    for node in parsed.descendants().filter(roxmltree::Node::is_element) {
        let local = node.tag_name().name();
        let declared = local_attribute(node, "family")
            .or_else(|| local_attribute(node, "diagramType"))
            .or_else(|| xmi_attribute(node, "type"));
        if local != "Diagram" && !declared.is_some_and(|value| value.contains("Diagram")) {
            continue;
        }
        let Some(xmi_id) = xmi_attribute(node, "id").or_else(|| local_attribute(node, "id"))
        else {
            continue;
        };
        let Some(family) = declared.and_then(normalized_family) else {
            continue;
        };
        let owner_reference = local_attribute(node, "owner")
            .or_else(|| local_attribute(node, "ownerElement"))
            .unwrap_or_default()
            .to_owned();
        let mut record = XmiDiagramRecord {
            xmi_id: xmi_id.to_owned(),
            name: local_attribute(node, "name").unwrap_or(family).to_owned(),
            family: family.to_owned(),
            owner_reference,
            context_reference: local_attribute(node, "context")
                .or_else(|| local_attribute(node, "semanticContext"))
                .map(ToOwned::to_owned),
            parent_diagram_reference: local_attribute(node, "parentDiagram").map(ToOwned::to_owned),
            producer_namespace: node.tag_name().namespace().map(ToOwned::to_owned),
            nodes: Vec::new(),
            edges: Vec::new(),
        };
        for child in node.descendants().filter(roxmltree::Node::is_element) {
            let child_local = child.tag_name().name();
            let child_type = xmi_attribute(child, "type").unwrap_or_default();
            if matches!(child_local, "Node" | "Shape") || child_type.ends_with("Shape") {
                if let (Some(id), Some(semantic_reference)) = (
                    xmi_attribute(child, "id").or_else(|| local_attribute(child, "id")),
                    presentation_reference(child),
                ) {
                    record.nodes.push(XmiDiagramNodeRecord {
                        xmi_id: id.to_owned(),
                        semantic_reference,
                        bounds: parse_bounds(child),
                        notation: local_attribute(child, "notation").map(ToOwned::to_owned),
                        compartments: BTreeMap::new(),
                    });
                }
            } else if child_local == "Edge" || child_type.ends_with("Edge") {
                let Some(id) = xmi_attribute(child, "id").or_else(|| local_attribute(child, "id")) else { continue };
                let Some(semantic_reference) = presentation_reference(child) else { continue };
                let Some(source) = local_attribute(child, "source") else { continue };
                let Some(target) = local_attribute(child, "target") else { continue };
                let waypoints = child
                    .children()
                    .filter(|point| point.is_element() && point.tag_name().name() == "waypoint")
                    .filter_map(|point| Some((parse_finite(local_attribute(point, "x"))?, parse_finite(local_attribute(point, "y"))?)))
                    .collect();
                let label_anchor = child
                    .children()
                    .find(|point| point.is_element() && point.tag_name().name() == "labelAnchor")
                    .and_then(|point| Some((parse_finite(local_attribute(point, "x"))?, parse_finite(local_attribute(point, "y"))?)));
                record.edges.push(XmiDiagramEdgeRecord {
                    xmi_id: id.to_owned(), semantic_reference,
                    source_presentation_reference: source.to_owned(),
                    target_presentation_reference: target.to_owned(),
                    waypoints, label_anchor,
                });
            }
        }
        record.nodes.sort_by(|left, right| left.xmi_id.cmp(&right.xmi_id));
        record.edges.sort_by(|left, right| left.xmi_id.cmp(&right.xmi_id));
        diagrams.push(record);
    }
    diagrams.sort_by(|left, right| left.xmi_id.cmp(&right.xmi_id));
    diagrams
}

fn diagnostic(code: &str, reason: impl Into<String>, file: Option<&str>) -> XmiDiagnostic {
    XmiDiagnostic {
        severity: XmiDiagnosticSeverity::Error,
        code: code.into(),
        reason: reason.into(),
        file: file.map(Into::into),
        line: None,
        column: None,
        namespace: None,
        xmi_id: None,
        xmi_type: None,
        reference: None,
        semantic_target: None,
    }
}

fn is_xmi_namespace(namespace: &str) -> bool {
    namespace == XMI_NS || namespace.contains("/XMI/") || namespace.ends_with("/XMI")
}

fn xmi_attribute<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
    local_name: &str,
) -> Option<&'a str> {
    node.attributes()
        .find(|attribute| {
            attribute.name() == local_name && attribute.namespace().is_some_and(is_xmi_namespace)
        })
        .map(|attribute| attribute.value())
}

fn local_attribute<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
    local_name: &str,
) -> Option<&'a str> {
    node.attributes()
        .find(|attribute| attribute.name() == local_name)
        .map(|attribute| attribute.value())
}

fn semantic_type_reference<'a, 'input>(node: roxmltree::Node<'a, 'input>) -> Option<&'a str> {
    node.attributes()
        .find(|attribute| {
            attribute.name() == "type" && !attribute.namespace().is_some_and(is_xmi_namespace)
        })
        .map(|attribute| attribute.value())
}

fn qualified_type(node: roxmltree::Node<'_, '_>) -> String {
    xmi_attribute(node, "type")
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            let local = node.tag_name().name();
            match node.tag_name().namespace() {
                Some(namespace) if namespace.contains("SysML") => format!("sysml:{local}"),
                Some(namespace) if namespace.contains("UML") => format!("uml:{local}"),
                _ => local.to_owned(),
            }
        })
}

fn semantic_local_type(value: &str) -> &str {
    value.rsplit(':').next().unwrap_or(value)
}

fn relationship_type(value: &str, local_tag: &str) -> bool {
    matches!(
        semantic_local_type(value),
        "Association"
            | "Dependency"
            | "Realization"
            | "Abstraction"
            | "PackageImport"
            | "ElementImport"
            | "PackageMerge"
            | "Include"
            | "Extend"
            | "Connector"
            | "InformationFlow"
    ) || matches!(
        local_tag,
        "generalization"
            | "packageImport"
            | "elementImport"
            | "packageMerge"
            | "include"
            | "extend"
    )
}

fn record_type(value: &str) -> bool {
    matches!(
        semantic_local_type(value),
        "Model"
            | "Package"
            | "Profile"
            | "Class"
            | "Interface"
            | "DataType"
            | "PrimitiveType"
            | "Enumeration"
            | "EnumerationLiteral"
            | "Signal"
            | "InstanceSpecification"
            | "Property"
            | "Port"
            | "Operation"
            | "Parameter"
            | "Reception"
            | "Actor"
            | "UseCase"
            | "Activity"
            | "StateMachine"
            | "Region"
            | "State"
            | "FinalState"
            | "Pseudostate"
            | "Transition"
            | "Interaction"
            | "Lifeline"
            | "Message"
            | "ExecutionSpecification"
            | "CombinedFragment"
            | "InteractionOperand"
            | "StateInvariant"
            | "Stereotype"
            | "Extension"
    )
}

fn nearest_semantic_owner(node: roxmltree::Node<'_, '_>) -> Option<String> {
    node.ancestors()
        .skip(1)
        .find_map(|ancestor| xmi_attribute(ancestor, "id").map(ToOwned::to_owned))
}

fn attribute_map(node: roxmltree::Node<'_, '_>) -> BTreeMap<String, String> {
    node.attributes()
        .filter(|attribute| {
            !(matches!(attribute.name(), "id" | "type")
                && attribute.namespace().is_some_and(is_xmi_namespace))
        })
        .map(|attribute| (attribute.name().to_owned(), attribute.value().to_owned()))
        .collect()
}

fn relationship_references(node: roxmltree::Node<'_, '_>) -> Option<(String, String)> {
    let owner = nearest_semantic_owner(node);
    let source = local_attribute(node, "source")
        .or_else(|| local_attribute(node, "client"))
        .or_else(|| local_attribute(node, "includingCase"))
        .or_else(|| local_attribute(node, "extension"))
        .map(ToOwned::to_owned)
        .or_else(|| owner.clone());
    let target = local_attribute(node, "target")
        .or_else(|| local_attribute(node, "supplier"))
        .or_else(|| local_attribute(node, "general"))
        .or_else(|| local_attribute(node, "importedPackage"))
        .or_else(|| local_attribute(node, "importedElement"))
        .or_else(|| local_attribute(node, "mergedPackage"))
        .or_else(|| local_attribute(node, "addition"))
        .or_else(|| local_attribute(node, "extendedCase"))
        .map(ToOwned::to_owned);
    source.zip(target)
}

pub fn parse_xmi(xml: &str, file: Option<&str>) -> Result<XmiSemanticDocument, Vec<XmiDiagnostic>> {
    let parsed = roxmltree::Document::parse(xml).map_err(|error| {
        let position = error.pos();
        let mut item = diagnostic("XMI_XML_INVALID", error.to_string(), file);
        item.line = Some(position.row);
        item.column = Some(position.col);
        vec![item]
    })?;
    let root = parsed.root_element();
    let mut document = XmiSemanticDocument::default();
    for namespace in root.namespaces() {
        document.namespaces.insert(
            namespace.name().unwrap_or_default().to_owned(),
            namespace.uri().to_owned(),
        );
    }
    document.producer = parsed.descendants().find_map(|node| {
        (node.tag_name().name() == "Documentation")
            .then(|| local_attribute(node, "exporter").map(ToOwned::to_owned))
            .flatten()
    });
    document.diagrams = parse_diagrams(&parsed);

    let mut ids = BTreeSet::new();
    let mut diagnostics = Vec::new();
    for node in parsed.descendants().filter(roxmltree::Node::is_element) {
        if node.tag_name().namespace() == Some(SM_NS) && node.tag_name().name() == "authoredState" {
            document.embedded_portable_json = node.text().map(ToOwned::to_owned);
            continue;
        }
        if node.tag_name().name() == "Extension" {
            document
                .preserved_extensions
                .push(xml[node.range()].to_owned());
            continue;
        }
        let Some(id) = xmi_attribute(node, "id") else {
            continue;
        };
        if !ids.insert(id.to_owned()) {
            let mut item = diagnostic(
                "XMI_REFERENCE_AMBIGUOUS",
                format!("duplicate xmi:id '{id}'"),
                file,
            );
            item.xmi_id = Some(id.into());
            diagnostics.push(item);
            continue;
        }
        let namespace = node.tag_name().namespace().unwrap_or_default();
        let base_attribute = node
            .attributes()
            .find(|attribute| attribute.name().starts_with("base_"));
        if (namespace.contains("SysML") || base_attribute.is_some())
            && let Some(base) = base_attribute
        {
            let mut tags = BTreeMap::new();
            for attribute in node.attributes() {
                if attribute.namespace().is_some_and(is_xmi_namespace)
                    || attribute.name().starts_with("base_")
                {
                    continue;
                }
                tags.insert(
                    attribute.name().to_owned(),
                    attribute
                        .value()
                        .split_whitespace()
                        .map(ToOwned::to_owned)
                        .collect(),
                );
            }
            document.stereotype_applications.push(XmiStereotypeRecord {
                xmi_id: id.into(),
                namespace: namespace.into(),
                name: node.tag_name().name().into(),
                base_reference: base.value().into(),
                base_metaclass: base.name().trim_start_matches("base_").into(),
                tagged_values: tags,
            });
            continue;
        }
        let xmi_type = qualified_type(node);
        if relationship_type(&xmi_type, node.tag_name().name()) {
            if let Some((source, target)) = relationship_references(node) {
                document.relationships.push(XmiRelationshipRecord {
                    xmi_id: id.into(),
                    xmi_type,
                    name: local_attribute(node, "name").unwrap_or_default().into(),
                    owner_id: nearest_semantic_owner(node),
                    source_reference: source,
                    target_reference: target,
                    attributes: attribute_map(node),
                });
            } else {
                let mut item = diagnostic(
                    "XMI_REFERENCE_UNRESOLVED",
                    "relationship lacks exact source/target references",
                    file,
                );
                item.xmi_id = Some(id.into());
                item.xmi_type = Some(xmi_type);
                diagnostics.push(item);
            }
        } else if record_type(&xmi_type) {
            document.records.push(XmiSemanticRecord {
                xmi_id: id.into(),
                xmi_type,
                name: local_attribute(node, "name").unwrap_or_default().into(),
                owner_id: nearest_semantic_owner(node),
                type_reference: semantic_type_reference(node).map(ToOwned::to_owned),
                attributes: attribute_map(node),
            });
        }
    }
    let known = document
        .records
        .iter()
        .map(|record| record.xmi_id.as_str())
        .chain(
            document
                .relationships
                .iter()
                .map(|record| record.xmi_id.as_str()),
        )
        .collect::<BTreeSet<_>>();
    for relationship in &document.relationships {
        for reference in [
            &relationship.source_reference,
            &relationship.target_reference,
        ] {
            if !known.contains(reference.as_str()) {
                let mut item = diagnostic(
                    "XMI_REFERENCE_UNRESOLVED",
                    format!("required reference '{reference}' does not resolve"),
                    file,
                );
                item.xmi_id = Some(relationship.xmi_id.clone());
                item.xmi_type = Some(relationship.xmi_type.clone());
                item.reference = Some(reference.clone());
                diagnostics.push(item);
            }
        }
    }
    if diagnostics
        .iter()
        .any(|item| item.severity == XmiDiagnosticSeverity::Error)
    {
        Err(diagnostics)
    } else {
        document
            .records
            .sort_by(|left, right| left.xmi_id.cmp(&right.xmi_id));
        document
            .relationships
            .sort_by(|left, right| left.xmi_id.cmp(&right.xmi_id));
        document
            .stereotype_applications
            .sort_by(|left, right| left.xmi_id.cmp(&right.xmi_id));
        Ok(document)
    }
}

pub fn native_element_kind(record: &XmiSemanticRecord) -> Option<ElementKind> {
    let stereotype = record
        .attributes
        .get("systemsModelerKind")
        .map(String::as_str);
    match stereotype.unwrap_or_else(|| semantic_local_type(&record.xmi_type)) {
        "Model" => Some(ElementKind::Model),
        "Package" => Some(ElementKind::Package),
        "ModelLibrary" => Some(ElementKind::ModelLibrary),
        "Class" | "Block" => Some(ElementKind::Block),
        "AssociationBlock" => Some(ElementKind::AssociationBlock),
        "Interface" | "InterfaceBlock" => Some(ElementKind::InterfaceBlock),
        "ConstraintBlock" => Some(ElementKind::ConstraintBlock),
        "DataType" => Some(ElementKind::DataType),
        "PrimitiveType" => Some(ElementKind::PrimitiveType),
        "Enumeration" => Some(ElementKind::Enumeration),
        "EnumerationLiteral" => Some(ElementKind::EnumerationLiteral),
        "Signal" => Some(ElementKind::Signal),
        "ValueType" => Some(ElementKind::ValueType),
        "Unit" => Some(ElementKind::Unit),
        "QuantityKind" => Some(ElementKind::QuantityKind),
        "InstanceSpecification" => Some(ElementKind::InstanceSpecification),
        "PartProperty" => Some(ElementKind::PartProperty),
        "ReferenceProperty" => Some(ElementKind::ReferenceProperty),
        "ValueProperty" => Some(ElementKind::ValueProperty),
        "FlowProperty" => Some(ElementKind::FlowProperty),
        "ConstraintProperty" => Some(ElementKind::ConstraintProperty),
        "ConstraintParameter" => Some(ElementKind::ConstraintParameter),
        "Property" => Some(ElementKind::ValueProperty),
        "Port" | "FullPort" => Some(ElementKind::FullPort),
        "ProxyPort" => Some(ElementKind::ProxyPort),
        "Operation" => Some(ElementKind::Operation),
        "Parameter" => Some(ElementKind::Parameter),
        "Reception" => Some(ElementKind::Reception),
        "Requirement" => Some(ElementKind::Requirement),
        "TestCase" => Some(ElementKind::TestCase),
        "Actor" => Some(ElementKind::Actor),
        "UseCase" => Some(ElementKind::UseCase),
        "Comment" => Some(ElementKind::Comment),
        _ => None,
    }
}

pub fn native_relationship_kind(record: &XmiRelationshipRecord) -> Option<RelationshipKind> {
    match record
        .attributes
        .get("systemsModelerKind")
        .map(String::as_str)
        .unwrap_or_else(|| semantic_local_type(&record.xmi_type))
    {
        "Dependency" => Some(RelationshipKind::Dependency),
        "PackageImport" => Some(RelationshipKind::PackageImport),
        "ElementImport" => Some(RelationshipKind::ElementImport),
        "PackageMerge" => Some(RelationshipKind::PackageMerge),
        "Association" => Some(RelationshipKind::Association),
        "Generalization" => Some(RelationshipKind::Generalization),
        "Realization" | "Abstraction" => Some(RelationshipKind::Realization),
        "Allocate" => Some(RelationshipKind::Allocate),
        "Connector" => Some(RelationshipKind::Connector),
        "InformationFlow" | "ItemFlow" => Some(RelationshipKind::ItemFlow),
        "DeriveRequirement" => Some(RelationshipKind::DeriveRequirement),
        "Satisfy" => Some(RelationshipKind::Satisfy),
        "Verify" => Some(RelationshipKind::Verify),
        "Refine" => Some(RelationshipKind::Refine),
        "Trace" => Some(RelationshipKind::Trace),
        "Copy" => Some(RelationshipKind::Copy),
        "Include" => Some(RelationshipKind::Include),
        "Extend" => Some(RelationshipKind::Extend),
        "BindingConnector" => Some(RelationshipKind::BindingConnector),
        _ => None,
    }
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn xml_id(value: &str) -> String {
    let mut result = String::with_capacity(value.len() + 2);
    if !value
        .chars()
        .next()
        .is_some_and(|character| character == '_' || character.is_alphabetic())
    {
        result.push_str("id_");
    }
    for character in value.chars() {
        if character == '_' || character == '-' || character == '.' || character.is_alphanumeric() {
            result.push(character);
        } else {
            result.push('_');
        }
    }
    result
}

fn element_xmi_type(kind: &ElementKind) -> &'static str {
    match kind {
        ElementKind::Model => "uml:Model",
        ElementKind::Package | ElementKind::ModelLibrary => "uml:Package",
        ElementKind::Block
        | ElementKind::AssociationBlock
        | ElementKind::ConstraintBlock
        | ElementKind::Requirement
        | ElementKind::TestCase => "uml:Class",
        ElementKind::InterfaceBlock => "uml:Interface",
        ElementKind::ValueType | ElementKind::DataType => "uml:DataType",
        ElementKind::PrimitiveType | ElementKind::Unit | ElementKind::QuantityKind => {
            "uml:PrimitiveType"
        }
        ElementKind::Enumeration => "uml:Enumeration",
        ElementKind::EnumerationLiteral => "uml:EnumerationLiteral",
        ElementKind::Signal => "uml:Signal",
        ElementKind::InstanceSpecification => "uml:InstanceSpecification",
        ElementKind::Slot => "uml:Slot",
        ElementKind::PartProperty
        | ElementKind::ReferenceProperty
        | ElementKind::ValueProperty
        | ElementKind::FlowProperty
        | ElementKind::ConstraintProperty
        | ElementKind::ConstraintParameter => "uml:Property",
        ElementKind::ProxyPort | ElementKind::FullPort => "uml:Port",
        ElementKind::Operation => "uml:Operation",
        ElementKind::Parameter => "uml:Parameter",
        ElementKind::Reception => "uml:Reception",
        ElementKind::Actor => "uml:Actor",
        ElementKind::UseCase => "uml:UseCase",
        ElementKind::Comment => "uml:Comment",
    }
}

fn relationship_xmi_type(kind: &RelationshipKind) -> &'static str {
    match kind {
        RelationshipKind::Association | RelationshipKind::Composition => "uml:Association",
        RelationshipKind::Generalization => "uml:Generalization",
        RelationshipKind::Realization => "uml:Realization",
        RelationshipKind::PackageImport => "uml:PackageImport",
        RelationshipKind::ElementImport => "uml:ElementImport",
        RelationshipKind::PackageMerge => "uml:PackageMerge",
        RelationshipKind::Include => "uml:Include",
        RelationshipKind::Extend => "uml:Extend",
        RelationshipKind::Connector | RelationshipKind::BindingConnector => "uml:Connector",
        RelationshipKind::ItemFlow => "uml:InformationFlow",
        _ => "uml:Dependency",
    }
}

fn element_tag(element: &Element) -> &'static str {
    if element.is_feature() {
        match element.kind {
            ElementKind::Operation => "ownedOperation",
            ElementKind::Parameter => "ownedParameter",
            _ => "ownedAttribute",
        }
    } else {
        "packagedElement"
    }
}

fn write_element_tree(
    xml: &mut String,
    project: &PortableSemanticProjectV1,
    element: &Element,
    ids: &BTreeMap<String, String>,
    indent: &str,
) {
    let xmi_id = ids[&element.id.to_string()].as_str();
    xml.push_str(&format!(
        "{indent}<{} xmi:type=\"{}\" xmi:id=\"{}\" name=\"{}\" sm:systemsModelerKind=\"{:?}\" sm:externalId=\"{}\"",
        element_tag(element),
        element_xmi_type(&element.kind),
        xmi_id,
        xml_escape(&element.name),
        element.kind,
        xml_escape(&element.external_id)
    ));
    if let Some(type_id) = element.type_id
        && let Some(target) = ids.get(&type_id.to_string())
    {
        xml.push_str(&format!(" type=\"{target}\""));
    }
    if let Some(multiplicity) = element.multiplicity {
        xml.push_str(&format!(
            " lower=\"{}\" upper=\"{}\"",
            multiplicity.lower,
            multiplicity
                .upper
                .map_or_else(|| "*".into(), |value| value.to_string())
        ));
    }
    xml.push_str(&format!(
        " visibility=\"{}\"",
        match element.visibility {
            systems_modeler_core::VisibilityKind::Public => "public",
            systems_modeler_core::VisibilityKind::Private => "private",
        }
    ));
    if let Some(default) = &element.default_value {
        xml.push_str(&format!(" default=\"{}\"", xml_escape(default)));
    }
    if let Some(direction) = element.parameter_direction {
        xml.push_str(&format!(
            " direction=\"{}\"",
            match direction {
                systems_modeler_core::ParameterDirection::In => "in",
                systems_modeler_core::ParameterDirection::Out => "out",
                systems_modeler_core::ParameterDirection::InOut => "inout",
                systems_modeler_core::ParameterDirection::Return => "return",
            }
        ));
    } else if let Some(direction) = element.flow_direction {
        xml.push_str(&format!(
            " sm:flowDirection=\"{}\"",
            match direction {
                systems_modeler_core::FlowDirection::In => "in",
                systems_modeler_core::FlowDirection::Out => "out",
                systems_modeler_core::FlowDirection::InOut => "inout",
            }
        ));
    }
    if element.is_conjugated {
        xml.push_str(" sm:isConjugated=\"true\"");
    }
    if let Some(requirement_id) = &element.requirement_id {
        xml.push_str(&format!(
            " sm:requirementId=\"{}\"",
            xml_escape(requirement_id)
        ));
    }
    if let Some(requirement_text) = &element.requirement_text {
        xml.push_str(&format!(
            " sm:requirementText=\"{}\"",
            xml_escape(requirement_text)
        ));
    }
    if !element.documentation.is_empty() {
        xml.push_str(&format!(
            " sm:documentation=\"{}\"",
            xml_escape(&element.documentation)
        ));
    }
    let mut children = project
        .elements
        .iter()
        .filter(|candidate| candidate.owner_id == Some(element.id))
        .collect::<Vec<_>>();
    children.sort_by(|left, right| left.external_id.cmp(&right.external_id));
    let mut relationships = project
        .relationships
        .iter()
        .filter(|candidate| candidate.owner_id == Some(element.id))
        .collect::<Vec<_>>();
    relationships.sort_by(|left, right| left.external_id.cmp(&right.external_id));
    if children.is_empty() && relationships.is_empty() {
        xml.push_str(" />\n");
        return;
    }
    xml.push_str(">\n");
    let child_indent = format!("{indent}  ");
    for child in children {
        write_element_tree(xml, project, child, ids, &child_indent);
    }
    for relationship in relationships {
        write_relationship(xml, relationship, ids, &child_indent);
    }
    xml.push_str(&format!("{indent}</{}>\n", element_tag(element)));
}

fn write_relationship(
    xml: &mut String,
    relationship: &Relationship,
    ids: &BTreeMap<String, String>,
    indent: &str,
) {
    let Some(source) = ids.get(&relationship.source_id.to_string()) else {
        return;
    };
    let Some(target) = ids.get(&relationship.target_id.to_string()) else {
        return;
    };
    xml.push_str(&format!(
        "{indent}<packagedElement xmi:type=\"{}\" xmi:id=\"{}\" name=\"{}\" client=\"{}\" supplier=\"{}\" sm:systemsModelerKind=\"{:?}\" sm:externalId=\"{}\" />\n",
        relationship_xmi_type(&relationship.kind),
        xml_id(&format!("relationship_{}", relationship.external_id)),
        xml_escape(&relationship.name),
        source,
        target,
        relationship.kind,
        xml_escape(&relationship.external_id)
    ));
}

fn tag_type_name(value_type: &TagValueType) -> &'static str {
    match value_type {
        TagValueType::String => "String",
        TagValueType::Boolean => "Boolean",
        TagValueType::Integer => "Integer",
        TagValueType::Real => "Real",
        TagValueType::Enumeration { .. } => "Enumeration",
        TagValueType::SemanticReference => "SemanticReference",
    }
}

fn tag_value_text(value: &TagValue, ids: &BTreeMap<String, String>) -> String {
    match value {
        TagValue::String(value) | TagValue::Enumeration(value) => value.clone(),
        TagValue::Boolean(value) => value.to_string(),
        TagValue::Integer(value) => value.to_string(),
        TagValue::Real(value) => value.to_string(),
        TagValue::SemanticReference(SemanticTarget::Element(id)) => ids
            .get(&id.to_string())
            .cloned()
            .unwrap_or_else(|| id.to_string()),
        TagValue::SemanticReference(SemanticTarget::Relationship(id)) => id.to_string(),
    }
}

fn write_profiles(
    xml: &mut String,
    project: &PortableSemanticProjectV1,
    ids: &BTreeMap<String, String>,
) {
    let mut profiles = project.profiles.profiles.values().collect::<Vec<_>>();
    profiles.sort_by(|left, right| left.external_id.cmp(&right.external_id));
    for profile in profiles {
        let profile_id = xml_id(&format!("profile_{}", profile.external_id));
        xml.push_str(&format!(
            "    <packagedElement xmi:type=\"uml:Profile\" xmi:id=\"{}\" name=\"{}\"{}>\n",
            profile_id,
            xml_escape(&profile.name),
            profile
                .uri
                .as_ref()
                .map(|uri| format!(" URI=\"{}\"", xml_escape(uri)))
                .unwrap_or_default()
        ));
        let mut stereotypes = project
            .profiles
            .stereotypes
            .values()
            .filter(|stereotype| stereotype.profile_id == profile.id)
            .collect::<Vec<_>>();
        stereotypes.sort_by(|left, right| left.external_id.cmp(&right.external_id));
        for stereotype in stereotypes {
            xml.push_str(&format!(
                "      <packagedElement xmi:type=\"uml:Stereotype\" xmi:id=\"{}\" name=\"{}\">\n",
                xml_id(&format!("stereotype_{}", stereotype.external_id)),
                xml_escape(&stereotype.name)
            ));
            let mut definitions = project
                .profiles
                .tag_definitions
                .values()
                .filter(|definition| definition.stereotype_id == stereotype.id)
                .collect::<Vec<_>>();
            definitions.sort_by(|left, right| left.external_id.cmp(&right.external_id));
            for definition in definitions {
                xml.push_str(&format!(
                    "        <ownedAttribute xmi:type=\"uml:Property\" xmi:id=\"{}\" name=\"{}\" lower=\"{}\" upper=\"{}\" sm:tagType=\"{}\" />\n",
                    xml_id(&format!("tag_{}", definition.external_id)),
                    xml_escape(&definition.name),
                    definition.lower,
                    definition.upper.map_or_else(|| "*".into(), |value| value.to_string()),
                    tag_type_name(&definition.value_type)
                ));
            }
            xml.push_str("      </packagedElement>\n");
        }
        xml.push_str("    </packagedElement>\n");
    }

    let mut applications = project
        .profiles
        .stereotype_applications
        .values()
        .collect::<Vec<_>>();
    applications.sort_by(|left, right| left.external_id.cmp(&right.external_id));
    for application in applications {
        let Some(stereotype) = project.profiles.stereotypes.get(&application.stereotype_id) else {
            continue;
        };
        let (base_name, target) = match application.target {
            SemanticTarget::Element(id) => ("Element", ids.get(&id.to_string()).cloned()),
            SemanticTarget::Relationship(id) => (
                "Relationship",
                project
                    .relationships
                    .iter()
                    .find(|relationship| relationship.id == id)
                    .map(|relationship| {
                        xml_id(&format!("relationship_{}", relationship.external_id))
                    }),
            ),
        };
        let Some(target) = target else { continue };
        xml.push_str(&format!(
            "  <sm:{} xmi:id=\"{}\" base_{}=\"{}\" sm:definition=\"{}\"",
            xml_id(&stereotype.name),
            xml_id(&format!(
                "stereotype_application_{}",
                application.external_id
            )),
            base_name,
            target,
            xml_id(&format!("stereotype_{}", stereotype.external_id))
        ));
        let mut values = application.tagged_values.iter().collect::<Vec<_>>();
        values.sort_by_key(|(definition_id, _)| definition_id.to_string());
        for (definition_id, values) in values {
            if let Some(definition) = project.profiles.tag_definitions.get(definition_id) {
                let value = values
                    .iter()
                    .map(|value| tag_value_text(value, ids))
                    .collect::<Vec<_>>()
                    .join(" ");
                xml.push_str(&format!(
                    " {}=\"{}\"",
                    xml_id(&definition.name),
                    xml_escape(&value)
                ));
            }
        }
        xml.push_str(" />\n");
    }
}

fn write_waypoints(xml: &mut String, points: &[super::DiagramPoint], indent: &str) {
    for point in points {
        xml.push_str(&format!(
            "{indent}<sm:waypoint x=\"{}\" y=\"{}\" />\n",
            point.x, point.y
        ));
    }
}

fn transition_endpoints(regions: &[Region], semantic_id: &str) -> Option<(String, String)> {
    for region in regions {
        if let Some(transition) = region
            .transitions
            .iter()
            .find(|transition| transition.id.to_string() == semantic_id)
        {
            return Some((
                transition.source_id.to_string(),
                transition.target_id.to_string(),
            ));
        }
        for vertex in &region.vertices {
            if let VertexKind::State(state) = &vertex.kind
                && let Some(endpoints) = transition_endpoints(&state.regions, semantic_id)
            {
                return Some(endpoints);
            }
        }
    }
    None
}

fn behavior_edge_endpoints(
    portable: &PortableProjectV1,
    diagram: &super::behavior_workspace::BehaviorDiagram,
    semantic_id: &str,
) -> Option<(String, String)> {
    match &diagram.kind {
        super::behavior_workspace::BehaviorDiagramKind::StateMachine => portable
            .behavior
            .state_machines
            .values()
            .find(|machine| machine.id.to_string() == diagram.semantic_id)
            .and_then(|machine| transition_endpoints(&machine.regions, semantic_id)),
        super::behavior_workspace::BehaviorDiagramKind::Sequence => portable
            .behavior
            .interactions
            .values()
            .find(|interaction| interaction.id.to_string() == diagram.semantic_id)
            .and_then(|interaction| {
                interaction
                    .messages
                    .iter()
                    .find(|message| message.id.to_string() == semantic_id)
                    .and_then(|message| {
                        Some((
                            message.send_event.as_ref()?.lifeline_id.to_string(),
                            message.receive_event.as_ref()?.lifeline_id.to_string(),
                        ))
                    })
            }),
    }
}

fn write_presentations(xml: &mut String, portable: &PortableProjectV1) {
    xml.push_str("  <xmi:Extension extender=\"Systems-Modeler-Pro diagram interchange\">\n    <sm:diagrams version=\"1\">\n");
    for diagram in &portable.diagrams {
        xml.push_str(&format!(
            "      <sm:Diagram xmi:id=\"{}\" name=\"{}\" family=\"{}\" owner=\"{}\"{}>\n",
            xml_id(&diagram.id), xml_escape(&diagram.name), xml_escape(&diagram.family),
            xml_escape(&diagram.owner_id),
            diagram.semantic_context_id.as_ref().map(|value| format!(" context=\"{}\"", xml_escape(value))).unwrap_or_default()
        ));
        for node in &diagram.nodes {
            xml.push_str(&format!(
                "        <sm:Node xmi:id=\"{}\" semanticElement=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{} />\n",
                xml_id(&node.id), xml_escape(&node.element_id), node.x, node.y, node.width, node.height,
                node.actor_notation.as_ref().map(|value| format!(" notation=\"{}\"", xml_escape(value))).unwrap_or_default()
            ));
        }
        for edge in &diagram.edges {
            xml.push_str(&format!(
                "        <sm:Edge xmi:id=\"{}\" semanticElement=\"{}\" source=\"{}\" target=\"{}\">\n",
                xml_id(&edge.id), xml_escape(&edge.relationship_id), xml_id(&edge.source_node_id), xml_id(&edge.target_node_id)
            ));
            write_waypoints(xml, &edge.points, "          ");
            if let Some(anchor) = edge.label_anchor {
                xml.push_str(&format!("          <sm:labelAnchor x=\"{}\" y=\"{}\" />\n", anchor.x, anchor.y));
            }
            xml.push_str("        </sm:Edge>\n");
        }
        xml.push_str("      </sm:Diagram>\n");
    }
    for diagram in &portable.ibd_diagrams {
        xml.push_str(&format!(
            "      <sm:Diagram xmi:id=\"{}\" name=\"{}\" family=\"ibd\" owner=\"{}\" context=\"{}\">\n",
            xml_id(&diagram.id), xml_escape(&diagram.name), xml_escape(&diagram.owner_id), xml_escape(&diagram.context_block_id)
        ));
        for property in &diagram.properties {
            xml.push_str(&format!("        <sm:Node xmi:id=\"{}\" semanticElement=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" />\n", xml_id(&property.id), xml_escape(&property.element_id), property.x, property.y, property.width, property.height));
            for port in &property.ports {
                xml.push_str(&format!("        <sm:Node xmi:id=\"{}\" semanticElement=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" notation=\"nested-port\" />\n", xml_id(&port.id), xml_escape(&port.element_id), port.x - port.size / 2.0, port.y - port.size / 2.0, port.size, port.size));
            }
        }
        for port in &diagram.boundary_ports {
            xml.push_str(&format!("        <sm:Node xmi:id=\"{}\" semanticElement=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" notation=\"boundary-port\" />\n", xml_id(&port.id), xml_escape(&port.element_id), port.x - port.size / 2.0, port.y - port.size / 2.0, port.size, port.size));
        }
        for edge in &diagram.connectors {
            xml.push_str(&format!("        <sm:Edge xmi:id=\"{}\" semanticElement=\"{}\" source=\"{}\" target=\"{}\">\n", xml_id(&edge.id), xml_escape(&edge.relationship_id), xml_id(&edge.source_presentation_id), xml_id(&edge.target_presentation_id)));
            write_waypoints(xml, &edge.points, "          ");
            xml.push_str("        </sm:Edge>\n");
        }
        xml.push_str("      </sm:Diagram>\n");
    }
    for diagram in &portable.activity.diagrams {
        xml.push_str(&format!("      <sm:Diagram xmi:id=\"{}\" name=\"{}\" family=\"activity\" owner=\"{}\" context=\"{}\">\n", xml_id(&diagram.id), xml_escape(&diagram.name), xml_escape(&diagram.owner_id), xml_escape(&diagram.activity_id)));
        for node in &diagram.nodes {
            xml.push_str(&format!("        <sm:Node xmi:id=\"{}\" semanticElement=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" />\n", xml_id(&node.id), xml_escape(&node.activity_node_id), node.x, node.y, node.width, node.height));
        }
        for edge in &diagram.edges {
            xml.push_str(&format!("        <sm:Edge xmi:id=\"{}\" semanticElement=\"{}\" source=\"{}\" target=\"{}\">\n", xml_id(&edge.id), xml_escape(&edge.activity_edge_id), xml_id(&edge.source_node_id), xml_id(&edge.target_node_id)));
            write_waypoints(xml, &edge.points, "          ");
            xml.push_str("        </sm:Edge>\n");
        }
        xml.push_str("      </sm:Diagram>\n");
    }
    for diagram in &portable.behavior.diagrams {
        let family = match &diagram.kind {
            super::behavior_workspace::BehaviorDiagramKind::StateMachine => "state-machine",
            super::behavior_workspace::BehaviorDiagramKind::Sequence => "sequence",
        };
        xml.push_str(&format!("      <sm:Diagram xmi:id=\"{}\" name=\"{}\" family=\"{}\" owner=\"{}\" context=\"{}\">\n", xml_id(&diagram.id), xml_escape(&diagram.name), family, xml_escape(&diagram.owner_id), xml_escape(&diagram.context_id)));
        for node in &diagram.state_nodes {
            xml.push_str(&format!("        <sm:Node xmi:id=\"{}\" semanticElement=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" />\n", xml_id(&format!("{}-{}", diagram.id, node.vertex_id)), xml_escape(&node.vertex_id), node.x, node.y, node.width, node.height));
        }
        for lifeline in &diagram.lifelines {
            xml.push_str(&format!("        <sm:Node xmi:id=\"{}\" semanticElement=\"{}\" x=\"{}\" y=\"{}\" width=\"140\" height=\"{}\" notation=\"lifeline\" />\n", xml_id(&format!("{}-{}", diagram.id, lifeline.lifeline_id)), xml_escape(&lifeline.lifeline_id), lifeline.x, lifeline.timeline_start_y, lifeline.timeline_end_y - lifeline.timeline_start_y));
        }
        for edge in &diagram.edge_routes {
            let Some((source, target)) =
                behavior_edge_endpoints(portable, diagram, &edge.semantic_id)
            else {
                continue;
            };
            xml.push_str(&format!("        <sm:Edge xmi:id=\"{}\" semanticElement=\"{}\" source=\"{}\" target=\"{}\">\n", xml_id(&format!("{}-{}", diagram.id, edge.semantic_id)), xml_escape(&edge.semantic_id), xml_id(&format!("{}-{source}", diagram.id)), xml_id(&format!("{}-{target}", diagram.id))));
            write_waypoints(xml, &edge.points, "          ");
            xml.push_str("        </sm:Edge>\n");
        }
        xml.push_str("      </sm:Diagram>\n");
    }
    xml.push_str("    </sm:diagrams>\n  </xmi:Extension>\n");
}

pub fn serialize_xmi(portable: &PortableProjectV1) -> Result<String, String> {
    let project = &portable.project;
    let mut ids = BTreeMap::new();
    for element in &project.elements {
        ids.insert(
            element.id.to_string(),
            xml_id(&format!("element_{}", element.external_id)),
        );
    }
    let root = project
        .elements
        .iter()
        .find(|element| element.id == project.root_id)
        .ok_or("project root missing during XMI export")?;
    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<xmi:XMI xmlns:xmi=\"{XMI_NS}\" xmlns:uml=\"{UML_NS}\" xmlns:sysml=\"{SYSML_NS}\" xmlns:sm=\"{SM_NS}\" xmi:version=\"2.5.1\">\n  <xmi:Documentation exporter=\"Systems-Modeler-Pro\" exporterVersion=\"PR53\" />\n  <uml:Model xmi:id=\"{}\" name=\"{}\" sm:externalId=\"{}\">\n",
        ids[&root.id.to_string()],
        xml_escape(&project.name),
        xml_escape(&root.external_id)
    );
    let mut elements = project
        .elements
        .iter()
        .filter(|element| element.owner_id == Some(project.root_id))
        .collect::<Vec<_>>();
    elements.sort_by(|left, right| left.external_id.cmp(&right.external_id));
    for element in elements {
        write_element_tree(&mut xml, project, element, &ids, "    ");
    }
    let mut relationships = project
        .relationships
        .iter()
        .filter(|relationship| {
            relationship.owner_id.is_none() || relationship.owner_id == Some(project.root_id)
        })
        .collect::<Vec<_>>();
    relationships.sort_by(|left, right| left.external_id.cmp(&right.external_id));
    for relationship in relationships {
        write_relationship(&mut xml, relationship, &ids, "    ");
    }
    write_profiles(&mut xml, project, &ids);
    for activity in &portable.activity.activities {
        xml.push_str(&format!(
            "    <packagedElement xmi:type=\"uml:Activity\" xmi:id=\"{}\" name=\"{}\" sm:externalId=\"{}\" />\n",
            xml_id(&format!("activity_{}", activity.external_id)), xml_escape(&activity.name), xml_escape(&activity.external_id)
        ));
    }
    for machine in &portable.behavior.state_machines {
        xml.push_str(&format!(
            "    <packagedElement xmi:type=\"uml:StateMachine\" xmi:id=\"{}\" name=\"{}\" sm:externalId=\"{}\" />\n",
            xml_id(&format!("state_machine_{}", machine.external_id)), xml_escape(&machine.name), xml_escape(&machine.external_id)
        ));
    }
    for interaction in &portable.behavior.interactions {
        xml.push_str(&format!(
            "    <packagedElement xmi:type=\"uml:Interaction\" xmi:id=\"{}\" name=\"{}\" sm:externalId=\"{}\" />\n",
            xml_id(&format!("interaction_{}", interaction.external_id)), xml_escape(&interaction.name), xml_escape(&interaction.external_id)
        ));
    }
    xml.push_str("  </uml:Model>\n");
    for element in &project.elements {
        for label in &element.applied_stereotypes {
            if project
                .profiles
                .stereotype_applications
                .values()
                .any(|application| {
                    application.target == SemanticTarget::Element(element.id)
                        && project
                            .profiles
                            .stereotypes
                            .get(&application.stereotype_id)
                            .is_some_and(|stereotype| stereotype.name == *label)
                })
            {
                continue;
            }
            let tag = label.replace(|character: char| !character.is_alphanumeric(), "_");
            xml.push_str(&format!(
                "  <sysml:{tag} xmi:id=\"{}\" base_{}=\"{}\" />\n",
                xml_id(&format!("stereotype-{}-{label}", element.external_id)),
                semantic_local_type(element_xmi_type(&element.kind)),
                ids[&element.id.to_string()]
            ));
        }
    }
    for (identity, extension) in &project.profiles.interchange_extensions {
        xml.push_str(&format!(
            "  <xmi:Extension extender=\"Systems-Modeler-Pro preserved {}\"><sm:preservedXml>{}</sm:preservedXml></xmi:Extension>\n",
            xml_escape(identity),
            xml_escape(extension)
        ));
    }
    write_presentations(&mut xml, portable);
    let portable_json = serde_json::to_string(portable).map_err(|error| error.to_string())?;
    xml.push_str(&format!(
        "  <xmi:Extension extender=\"Systems-Modeler-Pro\"><sm:authoredState>{}</sm:authoredState></xmi:Extension>\n</xmi:XMI>\n",
        xml_escape(&portable_json)
    ));
    Ok(xml)
}

pub fn embedded_portable(
    document: &XmiSemanticDocument,
) -> Result<Option<PortableProjectV1>, String> {
    document
        .embedded_portable_json
        .as_deref()
        .map(|json| {
            serde_json::from_str(json)
                .map_err(|error| format!("invalid Systems-Modeler XMI semantic extension: {error}"))
        })
        .transpose()
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    pub(crate) const UML_FIXTURE: &str =
        include_str!("../../../../../examples/xmi/external-uml.xmi");
    pub(crate) const GENERIC_DI_FIXTURE: &str =
        include_str!("../../../../../examples/xmi/generic-uml-di.xmi");

    const SYSML_FIXTURE: &str = r#"<?xml version="1.0"?>
<xmi:XMI xmlns:xmi="http://www.omg.org/spec/XMI/20131001" xmlns:uml="http://www.omg.org/spec/UML/20131001" xmlns:s="http://www.omg.org/spec/SysML/20150709/SysML">
  <uml:Model xmi:id="m1" name="System">
    <packagedElement xmi:type="uml:Class" xmi:id="r1" name="Safe" />
  </uml:Model>
  <s:Requirement xmi:id="sa1" base_Class="r1" id="REQ-1" text="The system shall be safe." />
</xmi:XMI>"#;

    #[test]
    fn namespace_prefixes_are_not_authoritative() {
        let document = parse_xmi(UML_FIXTURE, Some("external.uml")).unwrap();
        assert_eq!(document.records.len(), 3);
        assert_eq!(document.relationships.len(), 1);
        assert_eq!(document.relationships[0].source_reference, "controller");
    }

    #[test]
    fn producer_neutral_di_is_normalized_to_shared_presentation_ir() {
        let document = parse_xmi(GENERIC_DI_FIXTURE, Some("generic-uml-di.xmi")).unwrap();
        assert_eq!(document.diagrams.len(), 1);
        let diagram = &document.diagrams[0];
        assert_eq!(diagram.family, "bdd");
        assert_eq!(diagram.nodes.len(), 2);
        assert_eq!(diagram.edges.len(), 1);
        assert_eq!(diagram.nodes[0].bounds.unwrap().width, 190.0);
        assert_eq!(diagram.edges[0].waypoints.len(), 2);
        assert_eq!(
            diagram.edges[0].source_presentation_reference,
            "controller-shape"
        );
    }

    #[test]
    fn semantic_type_reference_is_not_confused_with_xmi_metaclass() {
        let fixture = r#"<?xml version="1.0"?>
<x:XMI xmlns:x="http://www.omg.org/spec/XMI/20131001" xmlns:u="http://www.omg.org/spec/UML/20131001">
  <u:Model x:id="m1" name="TypedModel">
    <u:Class x:id="c1" name="Classifier">
      <u:Property x:id="p1" name="typedProperty" type="c1" />
    </u:Class>
  </u:Model>
</x:XMI>"#;
        let document = parse_xmi(fixture, Some("typed-property.xmi")).unwrap();
        let property = document
            .records
            .iter()
            .find(|record| record.xmi_id == "p1")
            .expect("typed property record");
        assert_eq!(property.xmi_type, "uml:Property");
        assert_eq!(property.type_reference.as_deref(), Some("c1"));
        assert_eq!(
            property.attributes.get("type").map(String::as_str),
            Some("c1")
        );
    }

    #[test]
    fn sysml_stereotype_identity_and_tags_are_preserved() {
        let document = parse_xmi(SYSML_FIXTURE, None).unwrap();
        assert_eq!(document.stereotype_applications.len(), 1);
        let application = &document.stereotype_applications[0];
        assert_eq!(application.name, "Requirement");
        assert_eq!(application.base_reference, "r1");
        assert_eq!(application.tagged_values["id"], ["REQ-1"]);
    }

    #[test]
    fn unresolved_required_reference_is_blocked() {
        let invalid = UML_FIXTURE.replace("supplier=\"sensor\"", "supplier=\"missing\"");
        let diagnostics = parse_xmi(&invalid, Some("bad.xmi")).unwrap_err();
        assert!(
            diagnostics
                .iter()
                .any(|item| item.code == "XMI_REFERENCE_UNRESOLVED")
        );
    }

    #[test]
    fn external_profile_definitions_and_typed_tags_are_parsed() {
        let fixture = include_str!("../../../../../examples/xmi/external-sysml-profile.xmi");
        let document = parse_xmi(fixture, Some("external-sysml-profile.xmi")).unwrap();
        assert!(
            document
                .records
                .iter()
                .any(|record| record.xmi_type.ends_with(":Profile"))
        );
        assert!(
            document
                .records
                .iter()
                .any(|record| record.xmi_type.ends_with(":Stereotype"))
        );
        assert_eq!(
            document.stereotype_applications[0].tagged_values["sil"],
            ["3"]
        );
    }
}
