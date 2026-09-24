//! Authored structural presentation contract shared by native clients and servers.
//! This is the existing desktop wire/storage shape; it contains no session, view,
//! selection, file-path or runtime state. Commands remain separately scoped.

use crate::{
    AggregationKind, BindingEndpoint, DiagramId, ElementId, ElementKind, Project, Relationship,
    RelationshipId, RelationshipKind,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const PACKAGE_MIN_WIDTH: f64 = 120.0;
const PACKAGE_MIN_HEIGHT: f64 = 70.0;

pub type DiagramPoint = crate::GeometryPoint;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramNode {
    pub id: String,
    pub element_id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// Presentation-only Actor notation. Semantic Actor identity is unchanged.
    #[serde(default)]
    pub actor_notation: Option<String>,
    /// Diagram positions for definition-owned ConstraintParameters on this usage.
    #[serde(default)]
    pub parameter_presentations: Vec<ConstraintParameterPresentation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintParameterPresentation {
    pub id: String,
    pub parameter_id: String,
    pub offset_x: f64,
    pub offset_y: f64,
    pub size: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UseCaseSubjectBoundary {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramEdge {
    pub id: String,
    pub relationship_id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub points: Vec<DiagramPoint>,
    #[serde(default)]
    pub label_anchor: Option<DiagramPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BddDiagram {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    #[serde(default = "default_bdd_family")]
    pub family: String,
    /// Presentation context is intentionally independent of repository ownership.
    #[serde(default)]
    pub semantic_context_id: Option<String>,
    /// Movable/resizable presentation of the represented Use Case subject.
    #[serde(default)]
    pub subject_boundary: Option<UseCaseSubjectBoundary>,
    pub nodes: Vec<DiagramNode>,
    #[serde(default)]
    pub edges: Vec<DiagramEdge>,
}

fn default_bdd_family() -> String {
    "bdd".into()
}

pub fn parametric_endpoint_matches(
    diagram: &BddDiagram,
    presentation_id: &str,
    endpoint: &BindingEndpoint,
) -> bool {
    diagram.nodes.iter().any(|node| {
        if node.id == presentation_id {
            return endpoint.parameter_id.is_none()
                && node.element_id == endpoint.role_id.to_string();
        }
        node.element_id == endpoint.role_id.to_string()
            && node.parameter_presentations.iter().any(|parameter| {
                parameter.id == presentation_id
                    && endpoint.parameter_id.map(|id| id.to_string())
                        == Some(parameter.parameter_id.clone())
            })
    })
}

fn parse_element_id(value: &str) -> Result<ElementId, String> {
    uuid::Uuid::parse_str(value)
        .map(ElementId)
        .map_err(|_| format!("invalid element id: {value}"))
}

fn parse_diagram_id(value: &str) -> Result<DiagramId, String> {
    uuid::Uuid::parse_str(value)
        .map(DiagramId)
        .map_err(|_| format!("invalid diagram id: {value}"))
}

fn parse_relationship_id(value: &str) -> Result<RelationshipId, String> {
    uuid::Uuid::parse_str(value)
        .map(RelationshipId)
        .map_err(|_| format!("invalid relationship id: {value}"))
}

pub fn relationship_display_kind(relationship: &Relationship) -> &'static str {
    if relationship.kind == RelationshipKind::Association {
        if relationship
            .association_ends
            .iter()
            .any(|end| end.aggregation == AggregationKind::Composite)
        {
            return "Composition";
        }
        if relationship
            .association_ends
            .iter()
            .any(|end| end.aggregation == AggregationKind::Shared)
        {
            return "Aggregation";
        }
        return "Association";
    }
    match relationship.kind {
        RelationshipKind::Dependency => "Dependency",
        RelationshipKind::PackageImport => "PackageImport",
        RelationshipKind::ElementImport => "ElementImport",
        RelationshipKind::PackageMerge => "PackageMerge",
        RelationshipKind::Association => "Association",
        RelationshipKind::Composition => "Composition",
        RelationshipKind::Generalization => "Generalization",
        RelationshipKind::Realization => "Realization",
        RelationshipKind::Allocate => "Allocate",
        RelationshipKind::Connector => "Connector",
        RelationshipKind::ItemFlow => "ItemFlow",
        RelationshipKind::DeriveRequirement => "DeriveRequirement",
        RelationshipKind::Satisfy => "Satisfy",
        RelationshipKind::Verify => "Verify",
        RelationshipKind::Refine => "Refine",
        RelationshipKind::Trace => "Trace",
        RelationshipKind::Copy => "Copy",
        RelationshipKind::Include => "Include",
        RelationshipKind::Extend => "Extend",
        RelationshipKind::BindingConnector => "BindingConnector",
    }
}

pub fn validate_structural_diagrams(
    project: &Project,
    diagrams: &[BddDiagram],
) -> Result<(), String> {
    let mut diagram_ids = HashSet::new();
    let mut node_ids = HashSet::new();
    let mut edge_ids = HashSet::new();
    for diagram in diagrams {
        parse_diagram_id(&diagram.id)?;
        if !diagram_ids.insert(&diagram.id) {
            return Err(format!("duplicate diagram id: {}", diagram.id));
        }
        let owner_id = parse_element_id(&diagram.owner_id)?;
        let owner = project
            .element(owner_id)
            .map_err(|error| error.to_string())?;
        if !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {
            return Err(format!(
                "BDD owner is not a Model or Package: {}",
                diagram.owner_id
            ));
        }
        let semantic_context_id = diagram
            .semantic_context_id
            .as_deref()
            .map(parse_element_id)
            .transpose()?;
        if let Some(context_id) = semantic_context_id {
            let context = project
                .element(context_id)
                .map_err(|error| error.to_string())?;
            if diagram.family == "use-case"
                && (!context.is_classifier()
                    || matches!(context.kind, ElementKind::Actor | ElementKind::UseCase))
            {
                return Err(
                    "Use Case diagram context is not a represented system classifier".into(),
                );
            }
            if diagram.family == "parametric"
                && !matches!(
                    context.kind,
                    ElementKind::Block
                        | ElementKind::AssociationBlock
                        | ElementKind::ConstraintBlock
                )
            {
                return Err("Parametric diagram context must be a Block or ConstraintBlock".into());
            }
        } else if diagram.family == "parametric" {
            return Err("Parametric Diagram requires a semantic context".into());
        }
        if diagram.family != "use-case" && diagram.subject_boundary.is_some() {
            return Err("subject boundaries are only valid on Use Case Diagrams".into());
        }
        if diagram.semantic_context_id.is_none() && diagram.subject_boundary.is_some() {
            return Err("Use Case subject boundary requires a semantic context".into());
        }
        if let Some(boundary) = diagram.subject_boundary.as_ref() {
            if uuid::Uuid::parse_str(&boundary.id).is_err() || !node_ids.insert(&boundary.id) {
                return Err(format!(
                    "invalid or duplicate Use Case subject-boundary id: {}",
                    boundary.id
                ));
            }
            if !boundary.x.is_finite()
                || !boundary.y.is_finite()
                || !boundary.width.is_finite()
                || !boundary.height.is_finite()
                || boundary.x < 0.0
                || boundary.y < 42.0
                || boundary.width < 280.0
                || boundary.height < 220.0
            {
                return Err("invalid Use Case subject-boundary geometry".into());
            }
        }
        for node in &diagram.nodes {
            if uuid::Uuid::parse_str(&node.id).is_err() {
                return Err(format!("invalid diagram node id: {}", node.id));
            }
            if !node_ids.insert(&node.id) {
                return Err(format!("duplicate diagram node id: {}", node.id));
            }
            let element_id = parse_element_id(&node.element_id)?;
            let element = project
                .element(element_id)
                .map_err(|error| error.to_string())?;
            if diagram.family == "use-case"
                && !matches!(element.kind, ElementKind::Actor | ElementKind::UseCase)
            {
                return Err(format!(
                    "element kind {:?} is not valid on a Use Case Diagram",
                    element.kind
                ));
            }
            if diagram.family == "parametric" {
                if !matches!(
                    element.kind,
                    ElementKind::ConstraintProperty | ElementKind::ValueProperty
                ) {
                    return Err(format!(
                        "element kind {:?} is not valid on a Parametric Diagram",
                        element.kind
                    ));
                }
                project
                    .validate_parametric_role(
                        semantic_context_id.ok_or("Parametric Diagram requires a semantic context")?,
                        element_id,
                    )
                    .map_err(|error| error.to_string())?;
                if element.kind == ElementKind::ValueProperty
                    && !node.parameter_presentations.is_empty()
                {
                    return Err("ValueProperty presentations cannot own parameter endpoints".into());
                }
                if element.kind == ElementKind::ConstraintProperty {
                    let constraint_block_id =
                        element.type_id.ok_or("ConstraintProperty has no type")?;
                    let expected_parameters: HashSet<_> = project
                        .children(constraint_block_id)
                        .filter(|parameter| parameter.kind == ElementKind::ConstraintParameter)
                        .map(|parameter| parameter.id.to_string())
                        .collect();
                    let presented_parameters: HashSet<_> = node
                        .parameter_presentations
                        .iter()
                        .map(|parameter| parameter.parameter_id.clone())
                        .collect();
                    if presented_parameters != expected_parameters {
                        return Err("ConstraintProperty presentation must expose every definition-owned parameter exactly once".into());
                    }
                    for parameter in &node.parameter_presentations {
                        let max_x = (node.width - parameter.size).max(0.0);
                        let max_y = (node.height - parameter.size).max(0.0);
                        let on_boundary = parameter.offset_x.abs() < f64::EPSILON
                            || (parameter.offset_x - max_x).abs() < f64::EPSILON
                            || parameter.offset_y.abs() < f64::EPSILON
                            || (parameter.offset_y - max_y).abs() < f64::EPSILON;
                        if uuid::Uuid::parse_str(&parameter.id).is_err()
                            || !node_ids.insert(&parameter.id)
                            || !parameter.offset_x.is_finite()
                            || !parameter.offset_y.is_finite()
                            || !parameter.size.is_finite()
                            || parameter.size < 10.0
                            || parameter.offset_x < 0.0
                            || parameter.offset_x > max_x
                            || parameter.offset_y < 0.0
                            || parameter.offset_y > max_y
                            || !on_boundary
                        {
                            return Err("invalid ConstraintParameter presentation geometry".into());
                        }
                        let semantic = project
                            .element(parse_element_id(&parameter.parameter_id)?)
                            .map_err(|error| error.to_string())?;
                        if semantic.kind != ElementKind::ConstraintParameter
                            || semantic.owner_id != Some(constraint_block_id)
                        {
                            return Err("ConstraintParameter presentation does not match its reusable definition".into());
                        }
                    }
                }
            }
            if let Some(notation) = node.actor_notation.as_deref()
                && (element.kind != ElementKind::Actor
                    || !matches!(notation, "stick" | "rectangle"))
            {
                return Err(format!(
                    "invalid Actor notation for presentation {}",
                    node.id
                ));
            }
            if element.kind == ElementKind::UseCase
                && let Some(boundary) = diagram.subject_boundary.as_ref()
                && (node.x < boundary.x
                    || node.y < boundary.y
                    || node.x + node.width > boundary.x + boundary.width
                    || node.y + node.height > boundary.y + boundary.height)
            {
                return Err(format!(
                    "Use Case presentation {} is outside its subject boundary",
                    node.id
                ));
            }
        }
        for edge in &diagram.edges {
            if uuid::Uuid::parse_str(&edge.id).is_err() {
                return Err(format!("invalid diagram edge id: {}", edge.id));
            }
            if !edge_ids.insert(&edge.id) {
                return Err(format!("duplicate diagram edge id: {}", edge.id));
            }
            let relationship_id = parse_relationship_id(&edge.relationship_id)?;
            let relationship = project
                .relationship(relationship_id)
                .map_err(|error| error.to_string())?;
            if matches!(
                relationship.kind,
                RelationshipKind::Connector | RelationshipKind::ItemFlow
            ) {
                return Err(
                    "Connector and ItemFlow presentations belong on an IBD, not a BDD".into(),
                );
            }
            if diagram.family == "parametric"
                && relationship.kind != RelationshipKind::BindingConnector
            {
                return Err("only BindingConnectors are valid on a Parametric Diagram".into());
            }
            if diagram.family != "parametric"
                && relationship.kind == RelationshipKind::BindingConnector
            {
                return Err("BindingConnector presentations belong on a Parametric Diagram".into());
            }
            if diagram.family == "use-case"
                && !matches!(
                    relationship.kind,
                    RelationshipKind::Association
                        | RelationshipKind::Include
                        | RelationshipKind::Extend
                        | RelationshipKind::Generalization
                )
            {
                return Err(format!(
                    "relationship kind {:?} is not valid on a Use Case Diagram",
                    relationship.kind
                ));
            }
            if diagram.family == "parametric" {
                project
                    .validate_binding_in_context(
                        relationship,
                        semantic_context_id.ok_or("Parametric Diagram requires a semantic context")?,
                    )
                    .map_err(|error| error.to_string())?;
                let binding = relationship
                    .binding
                    .as_ref()
                    .ok_or("BindingConnector has no semantic endpoint details")?;
                if !parametric_endpoint_matches(diagram, &edge.source_node_id, &binding.source)
                    || !parametric_endpoint_matches(diagram, &edge.target_node_id, &binding.target)
                {
                    return Err(format!(
                        "diagram binding endpoints do not match semantic relationship: {}",
                        edge.relationship_id
                    ));
                }
            } else {
                let source = diagram
                    .nodes
                    .iter()
                    .find(|node| node.id == edge.source_node_id)
                    .ok_or_else(|| {
                        format!("edge source node not found: {}", edge.source_node_id)
                    })?;
                let target = diagram
                    .nodes
                    .iter()
                    .find(|node| node.id == edge.target_node_id)
                    .ok_or_else(|| {
                        format!("edge target node not found: {}", edge.target_node_id)
                    })?;
                if source.element_id != relationship.source_id.to_string()
                    || target.element_id != relationship.target_id.to_string()
                {
                    return Err(format!(
                        "diagram edge endpoints do not match semantic relationship: {}",
                        edge.relationship_id
                    ));
                }
            }
            if edge.points.len() < 2 {
                return Err(format!("diagram edge has no usable route: {}", edge.id));
            }
        }
        if diagram.family == "package" {
            validate_package_diagram(project, diagram)?;
        }
    }
    Ok(())
}

pub fn package_presentable(element: &crate::Element) -> bool {
    element.is_packageable() || element.kind == ElementKind::Comment
}

pub fn package_relationship_kind(kind: &RelationshipKind) -> bool {
    matches!(
        kind,
        RelationshipKind::PackageImport
            | RelationshipKind::ElementImport
            | RelationshipKind::Dependency
    )
}

pub fn package_relationship_name(kind: &RelationshipKind) -> &'static str {
    match kind {
        RelationshipKind::PackageImport => "PackageImport",
        RelationshipKind::ElementImport => "ElementImport",
        RelationshipKind::Dependency => "Dependency",
        _ => "Unsupported",
    }
}

pub fn dependency_endpoints(
    project: &Project,
    source_id: ElementId,
    target_id: ElementId,
) -> Result<(), String> {
    let source = project
        .element(source_id)
        .map_err(|error| error.to_string())?;
    let target = project
        .element(target_id)
        .map_err(|error| error.to_string())?;
    if !source.is_packageable() || !target.is_packageable() {
        return Err(format!(
            "Package-level Dependency requires packageable semantic endpoints; received '{}' ({:?}) -> '{}' ({:?})",
            source.name, source.kind, target.name, target.kind
        ));
    }
    if source_id == target_id {
        return Err(format!(
            "Dependency cannot connect '{}' to itself",
            source.name
        ));
    }
    Ok(())
}

pub fn validate_package_diagram(project: &Project, diagram: &BddDiagram) -> Result<(), String> {
    if diagram.family != "package" {
        return Err("target diagram is not a Package Diagram".into());
    }
    let owner = project
        .element(parse_element_id(&diagram.owner_id)?)
        .map_err(|error| error.to_string())?;
    if !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {
        return Err(format!(
            "Package Diagram owner '{}' must be a Model or Package",
            owner.name
        ));
    }

    let mut presentation_ids = HashSet::new();
    for node in &diagram.nodes {
        if !presentation_ids.insert(&node.id) {
            return Err(format!("duplicate Package presentation id: {}", node.id));
        }
        let element = project
            .element(parse_element_id(&node.element_id)?)
            .map_err(|error| error.to_string())?;
        if !package_presentable(element) {
            return Err(format!(
                "'{}' ({:?}) cannot be presented on a Package Diagram",
                element.name, element.kind
            ));
        }
        if !node.x.is_finite()
            || !node.y.is_finite()
            || !node.width.is_finite()
            || !node.height.is_finite()
            || node.x < 0.0
            || node.y < 42.0
            || node.width < PACKAGE_MIN_WIDTH
            || node.height < PACKAGE_MIN_HEIGHT
        {
            return Err(format!(
                "invalid Package presentation geometry for '{}'",
                element.name
            ));
        }
    }

    for edge in &diagram.edges {
        if !presentation_ids.insert(&edge.id) {
            return Err(format!(
                "duplicate Package relationship presentation id: {}",
                edge.id
            ));
        }
        let relationship = project
            .relationship(parse_relationship_id(&edge.relationship_id)?)
            .map_err(|error| error.to_string())?;
        if !package_relationship_kind(&relationship.kind) {
            return Err(format!(
                "{} is not valid on a Package Diagram",
                relationship_display_kind(relationship)
            ));
        }
        let source = diagram
            .nodes
            .iter()
            .find(|node| node.id == edge.source_node_id)
            .ok_or("Package relationship presentation references a missing source presentation")?;
        let target = diagram
            .nodes
            .iter()
            .find(|node| node.id == edge.target_node_id)
            .ok_or("Package relationship presentation references a missing target presentation")?;
        if source.element_id != relationship.source_id.to_string()
            || target.element_id != relationship.target_id.to_string()
        {
            let semantic_source = project
                .element(relationship.source_id)
                .map_err(|error| error.to_string())?;
            let semantic_target = project
                .element(relationship.target_id)
                .map_err(|error| error.to_string())?;
            return Err(format!(
                "{} presentation endpoints do not match its Rust semantic endpoints '{} -> {}'",
                package_relationship_name(&relationship.kind),
                semantic_source.name,
                semantic_target.name,
            ));
        }
        if relationship.kind == RelationshipKind::Dependency {
            dependency_endpoints(project, relationship.source_id, relationship.target_id)?;
        }
        if edge.points.len() < 2
            || edge
                .points
                .iter()
                .any(|point| !point.x.is_finite() || !point.y.is_finite())
            || edge.label_anchor.is_none()
        {
            return Err(format!(
                "{} presentation has invalid route or label geometry",
                package_relationship_name(&relationship.kind)
            ));
        }
    }
    Ok(())
}

pub mod creation;
pub mod geometry;
