use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use uuid::Uuid;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);
        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

id_type!(ProjectId);
id_type!(ElementId);
id_type!(RelationshipId);
id_type!(RelationshipEndId);
id_type!(DiagramId);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElementKind {
    Model,
    Package,
    Block,
    InterfaceBlock,
    ValueType,
    DataType,
    Enumeration,
    EnumerationLiteral,
    ConstraintBlock,
    PartProperty,
    ReferenceProperty,
    ValueProperty,
    ConstraintProperty,
    ProxyPort,
    FullPort,
    Operation,
    Parameter,
    Reception,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipKind {
    Dependency,
    Association,
    /// Retained for compatibility with the foundation API. New SysML composition
    /// should normally be modeled as an Association with a composite end.
    Composition,
    Generalization,
    Realization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AggregationKind {
    #[default]
    None,
    Shared,
    Composite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParameterDirection {
    In,
    Out,
    InOut,
    Return,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Multiplicity {
    pub lower: u32,
    /// None means unbounded (`*`).
    pub upper: Option<u32>,
}

impl Multiplicity {
    pub const ONE: Self = Self {
        lower: 1,
        upper: Some(1),
    };

    pub fn new(lower: u32, upper: Option<u32>) -> Result<Self, ModelError> {
        if let Some(upper) = upper {
            if lower > upper {
                return Err(ModelError::InvalidMultiplicity { lower, upper });
            }
        }
        Ok(Self { lower, upper })
    }

    pub fn notation(&self) -> String {
        match self.upper {
            Some(upper) if self.lower == upper => self.lower.to_string(),
            Some(upper) => format!("{}..{}", self.lower, upper),
            None => format!("{}..*", self.lower),
        }
    }
}

impl Default for Multiplicity {
    fn default() -> Self {
        Self::ONE
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Element {
    pub id: ElementId,
    pub external_id: String,
    pub kind: ElementKind,
    pub name: String,
    pub owner_id: Option<ElementId>,
    pub documentation: String,

    /// Applied stereotype qualified names or stable profile identifiers.
    pub applied_stereotypes: Vec<String>,

    /// Stable semantic type reference used by properties, ports and parameters.
    pub type_id: Option<ElementId>,
    pub multiplicity: Option<Multiplicity>,
    pub aggregation: AggregationKind,
    pub default_value: Option<String>,
    pub is_derived: bool,
    pub is_read_only: bool,
    pub is_conjugated: bool,

    /// Value-type metadata is stored semantically so future parametrics can reuse it.
    pub quantity_kind_external_id: Option<String>,
    pub unit_external_id: Option<String>,

    pub parameter_direction: Option<ParameterDirection>,
    pub literal_value: Option<String>,
}

impl Element {
    fn new(kind: ElementKind, name: String, owner_id: Option<ElementId>) -> Self {
        let id = ElementId::new();
        Self {
            id,
            external_id: format!("EL-{id}"),
            kind,
            name,
            owner_id,
            documentation: String::new(),
            applied_stereotypes: Vec::new(),
            type_id: None,
            multiplicity: None,
            aggregation: AggregationKind::None,
            default_value: None,
            is_derived: false,
            is_read_only: false,
            is_conjugated: false,
            quantity_kind_external_id: None,
            unit_external_id: None,
            parameter_direction: None,
            literal_value: None,
        }
    }

    pub fn is_namespace(&self) -> bool {
        matches!(self.kind, ElementKind::Model | ElementKind::Package)
    }

    pub fn is_classifier(&self) -> bool {
        matches!(
            self.kind,
            ElementKind::Block
                | ElementKind::InterfaceBlock
                | ElementKind::ValueType
                | ElementKind::DataType
                | ElementKind::Enumeration
                | ElementKind::ConstraintBlock
        )
    }

    pub fn is_property(&self) -> bool {
        matches!(
            self.kind,
            ElementKind::PartProperty
                | ElementKind::ReferenceProperty
                | ElementKind::ValueProperty
                | ElementKind::ConstraintProperty
        )
    }

    pub fn is_port(&self) -> bool {
        matches!(self.kind, ElementKind::ProxyPort | ElementKind::FullPort)
    }

    pub fn is_feature(&self) -> bool {
        self.is_property()
            || self.is_port()
            || matches!(
                self.kind,
                ElementKind::Operation | ElementKind::Parameter | ElementKind::Reception
            )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssociationEnd {
    pub id: RelationshipEndId,
    pub classifier_id: ElementId,
    pub role_name: String,
    pub multiplicity: Multiplicity,
    pub navigable: bool,
    pub aggregation: AggregationKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: RelationshipId,
    pub external_id: String,
    pub kind: RelationshipKind,
    pub name: String,
    pub owner_id: Option<ElementId>,
    /// For directed binary relationships: source/specific/client.
    pub source_id: ElementId,
    /// For directed binary relationships: target/general/supplier.
    pub target_id: ElementId,
    pub documentation: String,
    pub applied_stereotypes: Vec<String>,
    /// Association-end semantics. Empty for non-association relationships.
    pub association_ends: Vec<AssociationEnd>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub root_id: ElementId,
    pub elements: HashMap<ElementId, Element>,
    pub relationships: HashMap<RelationshipId, Relationship>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ModelError {
    #[error("element not found: {0}")]
    ElementNotFound(ElementId),
    #[error("relationship not found: {0}")]
    RelationshipNotFound(RelationshipId),
    #[error("owner is invalid for this element: {0}")]
    InvalidOwner(ElementId),
    #[error("relationship endpoint not found: {0}")]
    EndpointNotFound(ElementId),
    #[error("cannot delete an element that still owns children: {0}")]
    OwnerHasChildren(ElementId),
    #[error("cannot delete an element still referenced by relationships or types: {0}")]
    ElementStillReferenced(ElementId),
    #[error("external ID already exists: {0}")]
    DuplicateExternalId(String),
    #[error("invalid multiplicity: lower {lower} exceeds upper {upper}")]
    InvalidMultiplicity { lower: u32, upper: u32 },
    #[error("element kind {kind:?} cannot be owned by {owner:?}")]
    InvalidOwnerKind { kind: ElementKind, owner: ElementKind },
    #[error("element kind {kind:?} cannot be typed by {type_kind:?}")]
    InvalidTypeKind {
        kind: ElementKind,
        type_kind: ElementKind,
    },
    #[error("feature requires a type: {0}")]
    TypeRequired(ElementId),
    #[error("part property must use composite aggregation: {0}")]
    PartMustBeComposite(ElementId),
    #[error("reference property cannot use composite aggregation: {0}")]
    ReferenceCannotBeComposite(ElementId),
    #[error("generalization requires classifier endpoints")]
    GeneralizationRequiresClassifiers,
    #[error("generalization would create an inheritance cycle")]
    GeneralizationCycle,
    #[error("association requires at least two ends")]
    AssociationRequiresTwoEnds,
    #[error("association end classifier not found: {0}")]
    AssociationEndClassifierNotFound(ElementId),
}

impl Project {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let root_id = ElementId::new();
        let mut root = Element::new(ElementKind::Model, name.clone(), None);
        root.id = root_id;
        root.external_id = format!("MODEL-{root_id}");
        let mut elements = HashMap::new();
        elements.insert(root_id, root);
        Self {
            id: ProjectId::new(),
            name,
            root_id,
            elements,
            relationships: HashMap::new(),
        }
    }

    pub fn element(&self, id: ElementId) -> Result<&Element, ModelError> {
        self.elements
            .get(&id)
            .ok_or(ModelError::ElementNotFound(id))
    }

    pub fn element_mut(&mut self, id: ElementId) -> Result<&mut Element, ModelError> {
        self.elements
            .get_mut(&id)
            .ok_or(ModelError::ElementNotFound(id))
    }

    pub fn relationship(&self, id: RelationshipId) -> Result<&Relationship, ModelError> {
        self.relationships
            .get(&id)
            .ok_or(ModelError::RelationshipNotFound(id))
    }

    pub fn children(&self, owner_id: ElementId) -> impl Iterator<Item = &Element> {
        self.elements
            .values()
            .filter(move |element| element.owner_id == Some(owner_id))
    }

    pub fn owned_features(&self, classifier_id: ElementId) -> impl Iterator<Item = &Element> {
        self.children(classifier_id).filter(|element| element.is_feature())
    }

    pub fn create_element(
        &mut self,
        kind: ElementKind,
        name: impl Into<String>,
        owner_id: ElementId,
    ) -> Result<ElementId, ModelError> {
        let owner = self.element(owner_id)?;
        validate_owner_kind(&kind, &owner.kind)?;
        let element = Element::new(kind, name.into(), Some(owner_id));
        let id = element.id;
        self.elements.insert(id, element);
        Ok(id)
    }

    pub fn create_typed_feature(
        &mut self,
        kind: ElementKind,
        name: impl Into<String>,
        owner_id: ElementId,
        type_id: ElementId,
        multiplicity: Multiplicity,
    ) -> Result<ElementId, ModelError> {
        if !matches!(
            kind,
            ElementKind::PartProperty
                | ElementKind::ReferenceProperty
                | ElementKind::ValueProperty
                | ElementKind::ConstraintProperty
                | ElementKind::ProxyPort
                | ElementKind::FullPort
                | ElementKind::Parameter
        ) {
            return Err(ModelError::InvalidOwner(owner_id));
        }
        let id = self.create_element(kind.clone(), name, owner_id)?;
        self.set_element_type(id, type_id)?;
        {
            let element = self.element_mut(id)?;
            element.multiplicity = Some(multiplicity);
            if kind == ElementKind::PartProperty {
                element.aggregation = AggregationKind::Composite;
            }
        }
        self.validate_element(id)?;
        Ok(id)
    }

    pub fn rename_element(
        &mut self,
        id: ElementId,
        name: impl Into<String>,
    ) -> Result<(), ModelError> {
        self.element_mut(id)?.name = name.into();
        Ok(())
    }

    pub fn move_element(&mut self, id: ElementId, new_owner_id: ElementId) -> Result<(), ModelError> {
        let kind = self.element(id)?.kind.clone();
        let owner_kind = self.element(new_owner_id)?.kind.clone();
        validate_owner_kind(&kind, &owner_kind)?;
        self.element_mut(id)?.owner_id = Some(new_owner_id);
        Ok(())
    }

    pub fn set_external_id(
        &mut self,
        id: ElementId,
        external_id: impl Into<String>,
    ) -> Result<(), ModelError> {
        let external_id = external_id.into();
        if self
            .elements
            .values()
            .any(|element| element.id != id && element.external_id == external_id)
            || self
                .relationships
                .values()
                .any(|relationship| relationship.external_id == external_id)
        {
            return Err(ModelError::DuplicateExternalId(external_id));
        }
        self.element_mut(id)?.external_id = external_id;
        Ok(())
    }

    pub fn set_element_type(
        &mut self,
        id: ElementId,
        type_id: ElementId,
    ) -> Result<(), ModelError> {
        let kind = self.element(id)?.kind.clone();
        let type_kind = self.element(type_id)?.kind.clone();
        validate_type_kind(&kind, &type_kind)?;
        self.element_mut(id)?.type_id = Some(type_id);
        Ok(())
    }

    pub fn set_multiplicity(
        &mut self,
        id: ElementId,
        multiplicity: Multiplicity,
    ) -> Result<(), ModelError> {
        let element = self.element_mut(id)?;
        if !element.is_feature() {
            return Err(ModelError::InvalidOwner(id));
        }
        element.multiplicity = Some(multiplicity);
        Ok(())
    }

    pub fn set_aggregation(
        &mut self,
        id: ElementId,
        aggregation: AggregationKind,
    ) -> Result<(), ModelError> {
        let kind = self.element(id)?.kind.clone();
        if kind == ElementKind::PartProperty && aggregation != AggregationKind::Composite {
            return Err(ModelError::PartMustBeComposite(id));
        }
        if kind == ElementKind::ReferenceProperty && aggregation == AggregationKind::Composite {
            return Err(ModelError::ReferenceCannotBeComposite(id));
        }
        self.element_mut(id)?.aggregation = aggregation;
        Ok(())
    }

    pub fn create_relationship(
        &mut self,
        kind: RelationshipKind,
        source_id: ElementId,
        target_id: ElementId,
        owner_id: Option<ElementId>,
    ) -> Result<RelationshipId, ModelError> {
        let source = self.element(source_id)?;
        let target = self.element(target_id)?;
        if matches!(kind, RelationshipKind::Generalization) {
            if !source.is_classifier() || !target.is_classifier() {
                return Err(ModelError::GeneralizationRequiresClassifiers);
            }
            if self.would_create_generalization_cycle(source_id, target_id) {
                return Err(ModelError::GeneralizationCycle);
            }
        }
        if let Some(owner_id) = owner_id {
            let owner = self.element(owner_id)?;
            if !owner.is_namespace() && !owner.is_classifier() {
                return Err(ModelError::InvalidOwner(owner_id));
            }
        }
        let id = RelationshipId::new();
        self.relationships.insert(
            id,
            Relationship {
                id,
                external_id: format!("REL-{id}"),
                kind,
                name: String::new(),
                owner_id,
                source_id,
                target_id,
                documentation: String::new(),
                applied_stereotypes: Vec::new(),
                association_ends: Vec::new(),
            },
        );
        Ok(id)
    }

    pub fn create_association(
        &mut self,
        owner_id: Option<ElementId>,
        ends: Vec<AssociationEnd>,
    ) -> Result<RelationshipId, ModelError> {
        if ends.len() < 2 {
            return Err(ModelError::AssociationRequiresTwoEnds);
        }
        for end in &ends {
            let classifier = self
                .elements
                .get(&end.classifier_id)
                .ok_or(ModelError::AssociationEndClassifierNotFound(end.classifier_id))?;
            if !classifier.is_classifier() {
                return Err(ModelError::AssociationEndClassifierNotFound(end.classifier_id));
            }
        }
        let source_id = ends[0].classifier_id;
        let target_id = ends[1].classifier_id;
        let id = self.create_relationship(
            RelationshipKind::Association,
            source_id,
            target_id,
            owner_id,
        )?;
        self.relationships.get_mut(&id).unwrap().association_ends = ends;
        Ok(id)
    }

    pub fn association_end(
        classifier_id: ElementId,
        role_name: impl Into<String>,
        multiplicity: Multiplicity,
        navigable: bool,
        aggregation: AggregationKind,
    ) -> AssociationEnd {
        AssociationEnd {
            id: RelationshipEndId::new(),
            classifier_id,
            role_name: role_name.into(),
            multiplicity,
            navigable,
            aggregation,
        }
    }

    pub fn delete_element(&mut self, id: ElementId) -> Result<(), ModelError> {
        self.element(id)?;
        if self.children(id).next().is_some() {
            return Err(ModelError::OwnerHasChildren(id));
        }
        let referenced = self.relationships.values().any(|relationship| {
            relationship.source_id == id
                || relationship.target_id == id
                || relationship
                    .association_ends
                    .iter()
                    .any(|end| end.classifier_id == id)
        }) || self.elements.values().any(|element| element.type_id == Some(id));
        if referenced {
            return Err(ModelError::ElementStillReferenced(id));
        }
        self.elements.remove(&id);
        Ok(())
    }

    pub fn validate_element(&self, id: ElementId) -> Result<(), ModelError> {
        let element = self.element(id)?;
        if let Some(owner_id) = element.owner_id {
            let owner = self.element(owner_id)?;
            validate_owner_kind(&element.kind, &owner.kind)?;
        }
        if element.is_property() || element.is_port() || element.kind == ElementKind::Parameter {
            let type_id = element.type_id.ok_or(ModelError::TypeRequired(id))?;
            let type_kind = &self.element(type_id)?.kind;
            validate_type_kind(&element.kind, type_kind)?;
        }
        if element.kind == ElementKind::PartProperty
            && element.aggregation != AggregationKind::Composite
        {
            return Err(ModelError::PartMustBeComposite(id));
        }
        if element.kind == ElementKind::ReferenceProperty
            && element.aggregation == AggregationKind::Composite
        {
            return Err(ModelError::ReferenceCannotBeComposite(id));
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), ModelError> {
        let mut external_ids = HashSet::new();
        for element in self.elements.values() {
            if !external_ids.insert(element.external_id.clone()) {
                return Err(ModelError::DuplicateExternalId(element.external_id.clone()));
            }
            self.validate_element(element.id)?;
        }
        for relationship in self.relationships.values() {
            if !external_ids.insert(relationship.external_id.clone()) {
                return Err(ModelError::DuplicateExternalId(
                    relationship.external_id.clone(),
                ));
            }
            self.element(relationship.source_id)?;
            self.element(relationship.target_id)?;
            for end in &relationship.association_ends {
                self.element(end.classifier_id)
                    .map_err(|_| ModelError::AssociationEndClassifierNotFound(end.classifier_id))?;
            }
            if relationship.kind == RelationshipKind::Generalization {
                let source = self.element(relationship.source_id)?;
                let target = self.element(relationship.target_id)?;
                if !source.is_classifier() || !target.is_classifier() {
                    return Err(ModelError::GeneralizationRequiresClassifiers);
                }
            }
        }
        Ok(())
    }

    pub fn inherited_features(&self, classifier_id: ElementId) -> Result<Vec<&Element>, ModelError> {
        let classifier = self.element(classifier_id)?;
        if !classifier.is_classifier() {
            return Err(ModelError::GeneralizationRequiresClassifiers);
        }
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        self.collect_inherited_features(classifier_id, &mut visited, &mut result);
        Ok(result)
    }

    fn collect_inherited_features<'a>(
        &'a self,
        classifier_id: ElementId,
        visited: &mut HashSet<ElementId>,
        result: &mut Vec<&'a Element>,
    ) {
        if !visited.insert(classifier_id) {
            return;
        }
        for relationship in self.relationships.values().filter(|relationship| {
            relationship.kind == RelationshipKind::Generalization
                && relationship.source_id == classifier_id
        }) {
            let general = relationship.target_id;
            result.extend(self.owned_features(general));
            self.collect_inherited_features(general, visited, result);
        }
    }

    fn would_create_generalization_cycle(
        &self,
        specific_id: ElementId,
        general_id: ElementId,
    ) -> bool {
        if specific_id == general_id {
            return true;
        }
        let mut stack = vec![general_id];
        let mut visited = HashSet::new();
        while let Some(current) = stack.pop() {
            if !visited.insert(current) {
                continue;
            }
            if current == specific_id {
                return true;
            }
            for relationship in self.relationships.values().filter(|relationship| {
                relationship.kind == RelationshipKind::Generalization
                    && relationship.source_id == current
            }) {
                stack.push(relationship.target_id);
            }
        }
        false
    }
}

fn validate_owner_kind(kind: &ElementKind, owner: &ElementKind) -> Result<(), ModelError> {
    let valid = match kind {
        ElementKind::Model => false,
        ElementKind::Package
        | ElementKind::Block
        | ElementKind::InterfaceBlock
        | ElementKind::ValueType
        | ElementKind::DataType
        | ElementKind::Enumeration
        | ElementKind::ConstraintBlock => matches!(owner, ElementKind::Model | ElementKind::Package),
        ElementKind::EnumerationLiteral => matches!(owner, ElementKind::Enumeration),
        ElementKind::PartProperty
        | ElementKind::ReferenceProperty
        | ElementKind::ValueProperty
        | ElementKind::ConstraintProperty
        | ElementKind::ProxyPort
        | ElementKind::FullPort
        | ElementKind::Operation
        | ElementKind::Reception => matches!(
            owner,
            ElementKind::Block
                | ElementKind::InterfaceBlock
                | ElementKind::ConstraintBlock
                | ElementKind::DataType
        ),
        ElementKind::Parameter => matches!(owner, ElementKind::Operation),
    };
    if valid {
        Ok(())
    } else {
        Err(ModelError::InvalidOwnerKind {
            kind: kind.clone(),
            owner: owner.clone(),
        })
    }
}

fn validate_type_kind(kind: &ElementKind, type_kind: &ElementKind) -> Result<(), ModelError> {
    let valid = match kind {
        ElementKind::PartProperty => matches!(type_kind, ElementKind::Block | ElementKind::InterfaceBlock),
        ElementKind::ReferenceProperty => matches!(
            type_kind,
            ElementKind::Block
                | ElementKind::InterfaceBlock
                | ElementKind::DataType
                | ElementKind::ValueType
                | ElementKind::Enumeration
        ),
        ElementKind::ValueProperty => matches!(
            type_kind,
            ElementKind::ValueType | ElementKind::DataType | ElementKind::Enumeration
        ),
        ElementKind::ConstraintProperty => matches!(type_kind, ElementKind::ConstraintBlock),
        ElementKind::ProxyPort | ElementKind::FullPort => matches!(
            type_kind,
            ElementKind::InterfaceBlock | ElementKind::Block | ElementKind::DataType
        ),
        ElementKind::Parameter => matches!(
            type_kind,
            ElementKind::Block
                | ElementKind::InterfaceBlock
                | ElementKind::ValueType
                | ElementKind::DataType
                | ElementKind::Enumeration
        ),
        _ => true,
    };
    if valid {
        Ok(())
    } else {
        Err(ModelError::InvalidTypeKind {
            kind: kind.clone(),
            type_kind: type_kind.clone(),
        })
    }
}

pub mod notation {
    use super::{AggregationKind, Element, ElementKind, Relationship, RelationshipKind};

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum LineStyle {
        Solid,
        Dashed,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum EndDecoration {
        None,
        OpenArrow,
        HollowTriangle,
        FilledDiamond,
        HollowDiamond,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RelationshipNotation {
        pub line: LineStyle,
        pub source_decoration: EndDecoration,
        pub target_decoration: EndDecoration,
    }

    pub fn stereotype_label(kind: &ElementKind) -> Option<&'static str> {
        match kind {
            ElementKind::Block => Some("block"),
            ElementKind::InterfaceBlock => Some("interfaceBlock"),
            ElementKind::ValueType => Some("valueType"),
            ElementKind::ConstraintBlock => Some("constraint"),
            ElementKind::Enumeration => Some("enumeration"),
            _ => None,
        }
    }

    pub fn feature_compartment(kind: &ElementKind) -> Option<&'static str> {
        match kind {
            ElementKind::PartProperty => Some("parts"),
            ElementKind::ReferenceProperty => Some("references"),
            ElementKind::ValueProperty => Some("values"),
            ElementKind::ConstraintProperty => Some("constraints"),
            ElementKind::ProxyPort | ElementKind::FullPort => Some("ports"),
            ElementKind::Operation => Some("operations"),
            ElementKind::Reception => Some("receptions"),
            ElementKind::EnumerationLiteral => Some("literals"),
            _ => None,
        }
    }

    pub fn feature_label(element: &Element, type_name: Option<&str>) -> String {
        let derived = if element.is_derived { "/" } else { "" };
        let mut label = format!("{derived}{}", element.name);
        if let Some(type_name) = type_name {
            label.push_str(" : ");
            label.push_str(type_name);
        }
        if let Some(multiplicity) = element.multiplicity {
            label.push_str(" [");
            label.push_str(&multiplicity.notation());
            label.push(']');
        }
        if let Some(default) = &element.default_value {
            label.push_str(" = ");
            label.push_str(default);
        }
        label
    }

    pub fn relationship_notation(relationship: &Relationship) -> RelationshipNotation {
        match relationship.kind {
            RelationshipKind::Generalization => RelationshipNotation {
                line: LineStyle::Solid,
                source_decoration: EndDecoration::None,
                target_decoration: EndDecoration::HollowTriangle,
            },
            RelationshipKind::Dependency => RelationshipNotation {
                line: LineStyle::Dashed,
                source_decoration: EndDecoration::None,
                target_decoration: EndDecoration::OpenArrow,
            },
            RelationshipKind::Realization => RelationshipNotation {
                line: LineStyle::Dashed,
                source_decoration: EndDecoration::None,
                target_decoration: EndDecoration::HollowTriangle,
            },
            RelationshipKind::Composition => RelationshipNotation {
                line: LineStyle::Solid,
                source_decoration: EndDecoration::FilledDiamond,
                target_decoration: EndDecoration::None,
            },
            RelationshipKind::Association => {
                let source_decoration = relationship
                    .association_ends
                    .first()
                    .map(|end| match end.aggregation {
                        AggregationKind::Composite => EndDecoration::FilledDiamond,
                        AggregationKind::Shared => EndDecoration::HollowDiamond,
                        AggregationKind::None => EndDecoration::None,
                    })
                    .unwrap_or(EndDecoration::None);
                let target_decoration = relationship
                    .association_ends
                    .get(1)
                    .map(|end| match end.aggregation {
                        AggregationKind::Composite => EndDecoration::FilledDiamond,
                        AggregationKind::Shared => EndDecoration::HollowDiamond,
                        AggregationKind::None => EndDecoration::None,
                    })
                    .unwrap_or(EndDecoration::None);
                RelationshipNotation {
                    line: LineStyle::Solid,
                    source_decoration,
                    target_decoration,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn structural_project() -> (Project, ElementId, ElementId, ElementId) {
        let mut project = Project::new("Vehicle");
        let package = project
            .create_element(ElementKind::Package, "Structure", project.root_id)
            .unwrap();
        let vehicle = project
            .create_element(ElementKind::Block, "Vehicle", package)
            .unwrap();
        let powertrain = project
            .create_element(ElementKind::Block, "Powertrain", package)
            .unwrap();
        (project, package, vehicle, powertrain)
    }

    #[test]
    fn builds_owned_structure_and_relationship() {
        let (mut project, package, vehicle, powertrain) = structural_project();
        let relationship = project
            .create_relationship(
                RelationshipKind::Composition,
                vehicle,
                powertrain,
                Some(package),
            )
            .unwrap();
        assert_eq!(project.children(package).count(), 2);
        assert_eq!(project.relationships[&relationship].source_id, vehicle);
    }

    #[test]
    fn creates_typed_bdd_properties_with_correct_semantics() {
        let (mut project, _package, vehicle, powertrain) = structural_project();
        let part = project
            .create_typed_feature(
                ElementKind::PartProperty,
                "powertrain",
                vehicle,
                powertrain,
                Multiplicity::ONE,
            )
            .unwrap();
        let part = project.element(part).unwrap();
        assert_eq!(part.type_id, Some(powertrain));
        assert_eq!(part.aggregation, AggregationKind::Composite);
        assert_eq!(part.multiplicity, Some(Multiplicity::ONE));
    }

    #[test]
    fn rejects_invalid_value_property_type() {
        let (mut project, _package, vehicle, powertrain) = structural_project();
        let result = project.create_typed_feature(
            ElementKind::ValueProperty,
            "mass",
            vehicle,
            powertrain,
            Multiplicity::ONE,
        );
        assert!(matches!(result, Err(ModelError::InvalidTypeKind { .. })));
    }

    #[test]
    fn supports_value_type_unit_and_default_value() {
        let (mut project, package, vehicle, _powertrain) = structural_project();
        let mass_type = project
            .create_element(ElementKind::ValueType, "Mass", package)
            .unwrap();
        {
            let mass_type = project.element_mut(mass_type).unwrap();
            mass_type.quantity_kind_external_id = Some("QK-MASS".into());
            mass_type.unit_external_id = Some("UNIT-KG".into());
        }
        let mass = project
            .create_typed_feature(
                ElementKind::ValueProperty,
                "mass",
                vehicle,
                mass_type,
                Multiplicity::ONE,
            )
            .unwrap();
        project.element_mut(mass).unwrap().default_value = Some("1500".into());
        assert_eq!(project.element(mass).unwrap().default_value.as_deref(), Some("1500"));
    }

    #[test]
    fn association_preserves_role_multiplicity_navigation_and_composition() {
        let (mut project, package, vehicle, powertrain) = structural_project();
        let association = project
            .create_association(
                Some(package),
                vec![
                    Project::association_end(
                        vehicle,
                        "vehicle",
                        Multiplicity::ONE,
                        false,
                        AggregationKind::Composite,
                    ),
                    Project::association_end(
                        powertrain,
                        "powertrains",
                        Multiplicity::new(1, None).unwrap(),
                        true,
                        AggregationKind::None,
                    ),
                ],
            )
            .unwrap();
        let association = project.relationship(association).unwrap();
        assert_eq!(association.association_ends.len(), 2);
        assert_eq!(association.association_ends[1].multiplicity.notation(), "1..*");
        assert!(association.association_ends[1].navigable);
        assert_eq!(
            notation::relationship_notation(association).source_decoration,
            notation::EndDecoration::FilledDiamond
        );
    }

    #[test]
    fn rejects_generalization_cycles() {
        let (mut project, package, vehicle, powertrain) = structural_project();
        let engine = project
            .create_element(ElementKind::Block, "Engine", package)
            .unwrap();
        project
            .create_relationship(RelationshipKind::Generalization, powertrain, vehicle, Some(package))
            .unwrap();
        project
            .create_relationship(RelationshipKind::Generalization, engine, powertrain, Some(package))
            .unwrap();
        let cycle = project.create_relationship(
            RelationshipKind::Generalization,
            vehicle,
            engine,
            Some(package),
        );
        assert_eq!(cycle, Err(ModelError::GeneralizationCycle));
    }

    #[test]
    fn resolves_inherited_features() {
        let (mut project, package, vehicle, powertrain) = structural_project();
        let mass_type = project
            .create_element(ElementKind::ValueType, "Mass", package)
            .unwrap();
        project
            .create_typed_feature(
                ElementKind::ValueProperty,
                "mass",
                vehicle,
                mass_type,
                Multiplicity::ONE,
            )
            .unwrap();
        project
            .create_relationship(RelationshipKind::Generalization, powertrain, vehicle, Some(package))
            .unwrap();
        let inherited = project.inherited_features(powertrain).unwrap();
        assert_eq!(inherited.len(), 1);
        assert_eq!(inherited[0].name, "mass");
    }

    #[test]
    fn notation_uses_sysml_classifier_stereotypes_and_feature_form() {
        let (mut project, package, vehicle, _powertrain) = structural_project();
        let mass_type = project
            .create_element(ElementKind::ValueType, "Mass", package)
            .unwrap();
        let mass = project
            .create_typed_feature(
                ElementKind::ValueProperty,
                "mass",
                vehicle,
                mass_type,
                Multiplicity::new(0, Some(1)).unwrap(),
            )
            .unwrap();
        assert_eq!(notation::stereotype_label(&ElementKind::Block), Some("block"));
        assert_eq!(
            notation::feature_label(project.element(mass).unwrap(), Some("Mass")),
            "mass : Mass [0..1]"
        );
    }

    #[test]
    fn project_validation_enforces_external_id_uniqueness() {
        let (mut project, _package, vehicle, powertrain) = structural_project();
        let duplicate = project.element(vehicle).unwrap().external_id.clone();
        project.element_mut(powertrain).unwrap().external_id = duplicate.clone();
        assert_eq!(project.validate(), Err(ModelError::DuplicateExternalId(duplicate)));
    }
}
