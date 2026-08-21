use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use uuid::Uuid;

pub mod behavior;
pub mod ibd;
pub use behavior::*;
pub use ibd::{Connector, ConnectorEnd, ConnectorKind, ItemFlow};

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
    AssociationBlock,
    InterfaceBlock,
    ConstraintBlock,
    ValueType,
    DataType,
    PrimitiveType,
    Enumeration,
    EnumerationLiteral,
    Signal,
    Unit,
    QuantityKind,
    InstanceSpecification,
    Slot,
    PartProperty,
    ReferenceProperty,
    ValueProperty,
    FlowProperty,
    ConstraintProperty,
    ProxyPort,
    FullPort,
    Operation,
    Parameter,
    Reception,
    Requirement,
    TestCase,
    Comment,
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
    Connector,
    ItemFlow,
    DeriveRequirement,
    Satisfy,
    Verify,
    Refine,
    Trace,
    Copy,
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
pub enum FlowDirection {
    In,
    Out,
    InOut,
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
        if let Some(upper) = upper
            && lower > upper
        {
            return Err(ModelError::InvalidMultiplicity { lower, upper });
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
    pub applied_stereotypes: Vec<String>,
    pub type_id: Option<ElementId>,
    pub multiplicity: Option<Multiplicity>,
    pub aggregation: AggregationKind,
    pub default_value: Option<String>,
    pub is_derived: bool,
    pub is_read_only: bool,
    pub is_conjugated: bool,
    pub quantity_kind_external_id: Option<String>,
    pub unit_external_id: Option<String>,
    pub parameter_direction: Option<ParameterDirection>,
    pub literal_value: Option<String>,
    #[serde(default)]
    pub flow_direction: Option<FlowDirection>,
    /// Human-readable requirement identifier. This is deliberately separate
    /// from both the immutable semantic UUID and the import-facing External ID.
    #[serde(default)]
    pub requirement_id: Option<String>,
    #[serde(default)]
    pub requirement_text: Option<String>,
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
            flow_direction: None,
            requirement_id: None,
            requirement_text: None,
        }
    }

    pub fn is_namespace(&self) -> bool {
        matches!(self.kind, ElementKind::Model | ElementKind::Package)
    }

    pub fn is_classifier(&self) -> bool {
        matches!(
            self.kind,
            ElementKind::Block
                | ElementKind::AssociationBlock
                | ElementKind::InterfaceBlock
                | ElementKind::ConstraintBlock
                | ElementKind::ValueType
                | ElementKind::DataType
                | ElementKind::PrimitiveType
                | ElementKind::Enumeration
                | ElementKind::Signal
                | ElementKind::Requirement
                | ElementKind::TestCase
        )
    }

    pub fn is_property(&self) -> bool {
        matches!(
            self.kind,
            ElementKind::PartProperty
                | ElementKind::ReferenceProperty
                | ElementKind::ValueProperty
                | ElementKind::FlowProperty
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
                ElementKind::Operation
                    | ElementKind::Parameter
                    | ElementKind::Reception
                    | ElementKind::Slot
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
    pub source_id: ElementId,
    pub target_id: ElementId,
    pub documentation: String,
    pub applied_stereotypes: Vec<String>,
    pub association_ends: Vec<AssociationEnd>,
    #[serde(default)]
    pub connector: Option<Connector>,
    #[serde(default)]
    pub item_flow: Option<ItemFlow>,
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
    InvalidOwnerKind {
        kind: ElementKind,
        owner: ElementKind,
    },
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
    #[error("value type quantity kind does not resolve to QuantityKind: {0}")]
    InvalidQuantityKindReference(String),
    #[error("value type unit does not resolve to Unit: {0}")]
    InvalidUnitReference(String),
    #[error("IBD context must be a Block or AssociationBlock: {0}")]
    InvalidIbdContext(ElementId),
    #[error(
        "invalid nested connector property path at {0}; select a property/port that is actually reachable from this IBD context"
    )]
    InvalidConnectorPath(ElementId),
    #[error(
        "connector endpoint must be a PartProperty, ReferenceProperty, ProxyPort, or FullPort: {0}; select an internal structural property or a valid port"
    )]
    ConnectorEndpointMustBePortOrProperty(ElementId),
    #[error("connector cannot connect an endpoint to itself; select a different second endpoint")]
    ConnectorSelfConnection,
    #[error(
        "assembly connector requires two internal endpoints; select an internal Part/Reference Property or one of its nested ports for endpoint 1, then a second internal property/port. Do not use an outer Block boundary port"
    )]
    AssemblyRequiresInternalEnds,
    #[error(
        "delegation connector requires exactly one outer Block boundary port and one internal Part/Reference Property or nested port; select one endpoint of each kind"
    )]
    DelegationRequiresBoundaryAndInternal,
    #[error(
        "connector endpoint types are incompatible: {source_id} vs {target_id}; choose endpoints with compatible semantic types"
    )]
    IncompatibleConnectorTypes {
        source_id: ElementId,
        target_id: ElementId,
    },
    #[error("FullPort cannot be conjugated: {0}")]
    FullPortCannotBeConjugated(ElementId),
    #[error(
        "item flow requires at least one conveyed classifier; select an existing Connector, then choose the classifier conveyed by that Connector"
    )]
    ItemFlowRequiresConveyedItem,
    #[error("invalid conveyed item: {0}; ItemFlow conveyed items must be semantic classifiers")]
    InvalidConveyedItem(ElementId),
    #[error("relationship is not a connector: {0}")]
    RelationshipIsNotConnector(RelationshipId),
    #[error("item flow must realize an existing connector: {0}")]
    ItemFlowConnectorNotFound(RelationshipId),
    #[error("invalid endpoint kinds for {relationship:?}: {source_kind:?} -> {target_kind:?}")]
    InvalidTraceabilityEndpoints {
        relationship: RelationshipKind,
        source_kind: ElementKind,
        target_kind: ElementKind,
    },
    #[error("requirement ID cannot be empty: {0}")]
    EmptyRequirementId(ElementId),
    #[error("requirement ID already exists in this project: {0}")]
    DuplicateRequirementId(String),
    #[error("copied Requirement text is read-only; edit its supplier Requirement: {0}")]
    CopiedRequirementIsReadOnly(ElementId),
    #[error("Requirement traceability relationships cannot connect an element to itself")]
    SelfTraceabilityRelationship,
    #[error("duplicate Requirement traceability relationship: {relationship:?} {source} -> {target}")]
    DuplicateTraceabilityRelationship {
        relationship: RelationshipKind,
        source: ElementId,
        target: ElementId,
    },
    #[error("Requirement traceability relationships must be owned by a Model or Package: {0}")]
    InvalidTraceabilityOwner(ElementId),
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
        self.children(classifier_id)
            .filter(|element| element.is_feature())
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
                | ElementKind::FlowProperty
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

    pub fn move_element(
        &mut self,
        id: ElementId,
        new_owner_id: ElementId,
    ) -> Result<(), ModelError> {
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
        let traceability = is_traceability_relationship(&kind);
        if traceability && source_id == target_id {
            return Err(ModelError::SelfTraceabilityRelationship);
        }
        validate_traceability_endpoints(&kind, &source.kind, &target.kind)?;
        if traceability && self.relationships.values().any(|relationship| {
            relationship.kind == kind
                && relationship.source_id == source_id
                && relationship.target_id == target_id
        }) {
            return Err(ModelError::DuplicateTraceabilityRelationship {
                relationship: kind,
                source: source_id,
                target: target_id,
            });
        }
        let copied_requirement_text = (kind == RelationshipKind::Copy)
            .then(|| target.requirement_text.clone())
            .flatten();
        if kind == RelationshipKind::Generalization {
            if !source.is_classifier() || !target.is_classifier() {
                return Err(ModelError::GeneralizationRequiresClassifiers);
            }
            if self.would_create_generalization_cycle(source_id, target_id) {
                return Err(ModelError::GeneralizationCycle);
            }
        }
        if let Some(owner_id) = owner_id {
            let owner = self.element(owner_id)?;
            if traceability && !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {
                return Err(ModelError::InvalidTraceabilityOwner(owner_id));
            }
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
                connector: None,
                item_flow: None,
            },
        );
        if let Some(text) = copied_requirement_text {
            self.element_mut(source_id)?.requirement_text = Some(text);
        }
        Ok(id)
    }

    pub fn create_requirement(
        &mut self,
        name: impl Into<String>,
        requirement_id: impl Into<String>,
        text: impl Into<String>,
        owner_id: ElementId,
    ) -> Result<ElementId, ModelError> {
        let id = self.create_element(ElementKind::Requirement, name, owner_id)?;
        if let Err(error) = self.update_requirement(id, requirement_id, text) {
            self.elements.remove(&id);
            return Err(error);
        }
        Ok(id)
    }

    pub fn update_requirement(
        &mut self,
        id: ElementId,
        requirement_id: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<(), ModelError> {
        let requirement_id = requirement_id.into();
        if requirement_id.trim().is_empty() {
            return Err(ModelError::EmptyRequirementId(id));
        }
        if self.elements.values().any(|element| {
            element.id != id
                && element.kind == ElementKind::Requirement
                && element.requirement_id.as_deref() == Some(requirement_id.as_str())
        }) {
            return Err(ModelError::DuplicateRequirementId(requirement_id));
        }
        if self.relationships.values().any(|relationship| {
            relationship.kind == RelationshipKind::Copy && relationship.source_id == id
        }) {
            return Err(ModelError::CopiedRequirementIsReadOnly(id));
        }
        let text = text.into();
        {
            let requirement = self.element_mut(id)?;
            if requirement.kind != ElementKind::Requirement {
                return Err(ModelError::InvalidOwner(id));
            }
            requirement.requirement_id = Some(requirement_id);
            requirement.requirement_text = Some(text.clone());
        }
        let copied_clients: Vec<_> = self.relationships.values()
            .filter(|relationship| relationship.kind == RelationshipKind::Copy && relationship.target_id == id)
            .map(|relationship| relationship.source_id)
            .collect();
        for client_id in copied_clients {
            self.element_mut(client_id)?.requirement_text = Some(text.clone());
        }
        Ok(())
    }

    pub fn create_connector(&mut self, connector: Connector) -> Result<RelationshipId, ModelError> {
        self.validate_connector(&connector)?;
        let source_id = connector.source.port_id.unwrap_or(connector.source.role_id);
        let target_id = connector.target.port_id.unwrap_or(connector.target.role_id);
        let id = self.create_relationship(
            RelationshipKind::Connector,
            source_id,
            target_id,
            Some(connector.context_id),
        )?;
        self.relationships.get_mut(&id).unwrap().connector = Some(connector);
        Ok(id)
    }

    pub fn create_item_flow(&mut self, flow: ItemFlow) -> Result<RelationshipId, ModelError> {
        let connector = self
            .relationship(flow.connector_id)
            .map_err(|_| ModelError::ItemFlowConnectorNotFound(flow.connector_id))?;
        if connector.kind != RelationshipKind::Connector || connector.connector.is_none() {
            return Err(ModelError::ItemFlowConnectorNotFound(flow.connector_id));
        }
        self.validate_item_flow(&flow)?;
        let context_id = connector.connector.as_ref().unwrap().context_id;
        let source_id = flow.source.port_id.unwrap_or(flow.source.role_id);
        let target_id = flow.target.port_id.unwrap_or(flow.target.role_id);
        let id = self.create_relationship(
            RelationshipKind::ItemFlow,
            source_id,
            target_id,
            Some(context_id),
        )?;
        self.relationships.get_mut(&id).unwrap().item_flow = Some(flow);
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
            let classifier = self.elements.get(&end.classifier_id).ok_or(
                ModelError::AssociationEndClassifierNotFound(end.classifier_id),
            )?;
            if !classifier.is_classifier() {
                return Err(ModelError::AssociationEndClassifierNotFound(
                    end.classifier_id,
                ));
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
                || relationship.connector.as_ref().is_some_and(|connector| {
                    connector.source.role_id == id
                        || connector.source.port_id == Some(id)
                        || connector.source.property_path.contains(&id)
                        || connector.target.role_id == id
                        || connector.target.port_id == Some(id)
                        || connector.target.property_path.contains(&id)
                })
                || relationship.item_flow.as_ref().is_some_and(|flow| {
                    flow.conveyed_item_ids.contains(&id)
                        || flow.source.role_id == id
                        || flow.source.port_id == Some(id)
                        || flow.target.role_id == id
                        || flow.target.port_id == Some(id)
                })
        }) || self
            .elements
            .values()
            .any(|element| element.type_id == Some(id));
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
        } else if element.kind == ElementKind::InstanceSpecification
            && let Some(type_id) = element.type_id
        {
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
        if element.kind == ElementKind::FullPort && element.is_conjugated {
            return Err(ModelError::FullPortCannotBeConjugated(id));
        }
        if element.kind == ElementKind::ValueType {
            if let Some(quantity_kind) = &element.quantity_kind_external_id
                && !self.elements.values().any(|candidate| {
                    candidate.external_id == *quantity_kind
                        && candidate.kind == ElementKind::QuantityKind
                })
            {
                return Err(ModelError::InvalidQuantityKindReference(
                    quantity_kind.clone(),
                ));
            }
            if let Some(unit) = &element.unit_external_id
                && !self.elements.values().any(|candidate| {
                    candidate.external_id == *unit && candidate.kind == ElementKind::Unit
                })
            {
                return Err(ModelError::InvalidUnitReference(unit.clone()));
            }
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), ModelError> {
        let mut external_ids = HashSet::new();
        let mut requirement_ids = HashSet::new();
        for element in self.elements.values() {
            if !external_ids.insert(element.external_id.clone()) {
                return Err(ModelError::DuplicateExternalId(element.external_id.clone()));
            }
            self.validate_element(element.id)?;
            if element.kind == ElementKind::Requirement {
                let requirement_id = element
                    .requirement_id
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or(ModelError::EmptyRequirementId(element.id))?;
                if !requirement_ids.insert(requirement_id.to_owned()) {
                    return Err(ModelError::DuplicateRequirementId(requirement_id.to_owned()));
                }
            }
        }
        for relationship in self.relationships.values() {
            if !external_ids.insert(relationship.external_id.clone()) {
                return Err(ModelError::DuplicateExternalId(
                    relationship.external_id.clone(),
                ));
            }
            self.element(relationship.source_id)?;
            self.element(relationship.target_id)?;
            if is_traceability_relationship(&relationship.kind) {
                if relationship.source_id == relationship.target_id {
                    return Err(ModelError::SelfTraceabilityRelationship);
                }
                if let Some(owner_id) = relationship.owner_id
                    && !matches!(
                        self.element(owner_id)?.kind,
                        ElementKind::Model | ElementKind::Package
                    )
                {
                    return Err(ModelError::InvalidTraceabilityOwner(owner_id));
                }
                if self.relationships.values().any(|candidate| {
                    candidate.id != relationship.id
                        && candidate.kind == relationship.kind
                        && candidate.source_id == relationship.source_id
                        && candidate.target_id == relationship.target_id
                }) {
                    return Err(ModelError::DuplicateTraceabilityRelationship {
                        relationship: relationship.kind.clone(),
                        source: relationship.source_id,
                        target: relationship.target_id,
                    });
                }
            }
            validate_traceability_endpoints(
                &relationship.kind,
                &self.element(relationship.source_id)?.kind,
                &self.element(relationship.target_id)?.kind,
            )?;
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
            match relationship.kind {
                RelationshipKind::Connector => {
                    let connector = relationship
                        .connector
                        .as_ref()
                        .ok_or(ModelError::RelationshipIsNotConnector(relationship.id))?;
                    self.validate_connector(connector)?;
                }
                RelationshipKind::ItemFlow => {
                    let flow = relationship
                        .item_flow
                        .as_ref()
                        .ok_or(ModelError::ItemFlowRequiresConveyedItem)?;
                    self.validate_item_flow(flow)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn inherited_features(
        &self,
        classifier_id: ElementId,
    ) -> Result<Vec<&Element>, ModelError> {
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
    let namespace_owned = matches!(owner, ElementKind::Model | ElementKind::Package);
    let classifier_owner = matches!(
        owner,
        ElementKind::Block
            | ElementKind::AssociationBlock
            | ElementKind::InterfaceBlock
            | ElementKind::ConstraintBlock
            | ElementKind::DataType
            | ElementKind::Signal
    );
    let valid = match kind {
        ElementKind::Model => false,
        ElementKind::Package
        | ElementKind::Block
        | ElementKind::AssociationBlock
        | ElementKind::InterfaceBlock
        | ElementKind::ConstraintBlock
        | ElementKind::ValueType
        | ElementKind::DataType
        | ElementKind::PrimitiveType
        | ElementKind::Enumeration
        | ElementKind::Signal
        | ElementKind::Unit
        | ElementKind::QuantityKind
        | ElementKind::InstanceSpecification
        | ElementKind::Requirement
        | ElementKind::TestCase => namespace_owned,
        ElementKind::Comment => namespace_owned || classifier_owner,
        ElementKind::EnumerationLiteral => matches!(owner, ElementKind::Enumeration),
        ElementKind::Slot => matches!(owner, ElementKind::InstanceSpecification),
        ElementKind::PartProperty | ElementKind::ReferenceProperty => {
            matches!(owner, ElementKind::Block | ElementKind::AssociationBlock)
        }
        ElementKind::ValueProperty => matches!(
            owner,
            ElementKind::Block
                | ElementKind::AssociationBlock
                | ElementKind::ConstraintBlock
                | ElementKind::DataType
                | ElementKind::ValueType
        ),
        ElementKind::FlowProperty => {
            matches!(owner, ElementKind::Block | ElementKind::InterfaceBlock)
        }
        ElementKind::ConstraintProperty => matches!(
            owner,
            ElementKind::Block | ElementKind::AssociationBlock | ElementKind::ConstraintBlock
        ),
        ElementKind::ProxyPort | ElementKind::FullPort => matches!(
            owner,
            ElementKind::Block | ElementKind::AssociationBlock | ElementKind::InterfaceBlock
        ),
        ElementKind::Operation | ElementKind::Reception => classifier_owner,
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

fn is_traceability_relationship(relationship: &RelationshipKind) -> bool {
    matches!(
        relationship,
        RelationshipKind::DeriveRequirement
            | RelationshipKind::Satisfy
            | RelationshipKind::Verify
            | RelationshipKind::Refine
            | RelationshipKind::Trace
            | RelationshipKind::Copy
    )
}

fn validate_traceability_endpoints(
    relationship: &RelationshipKind,
    source: &ElementKind,
    target: &ElementKind,
) -> Result<(), ModelError> {
    use RelationshipKind as R;
    let is_requirement = |kind: &ElementKind| *kind == ElementKind::Requirement;
    let valid = match relationship {
        R::DeriveRequirement | R::Copy => is_requirement(source) && is_requirement(target),
        R::Satisfy => !is_requirement(source) && is_requirement(target),
        R::Verify => *source == ElementKind::TestCase && is_requirement(target),
        R::Refine => is_requirement(source) ^ is_requirement(target),
        R::Trace => true,
        _ => true,
    };
    if valid {
        Ok(())
    } else {
        Err(ModelError::InvalidTraceabilityEndpoints {
            relationship: relationship.clone(),
            source_kind: source.clone(),
            target_kind: target.clone(),
        })
    }
}

fn validate_type_kind(kind: &ElementKind, type_kind: &ElementKind) -> Result<(), ModelError> {
    let any_classifier = matches!(
        type_kind,
        ElementKind::Block
            | ElementKind::AssociationBlock
            | ElementKind::InterfaceBlock
            | ElementKind::ConstraintBlock
            | ElementKind::ValueType
            | ElementKind::DataType
            | ElementKind::PrimitiveType
            | ElementKind::Enumeration
            | ElementKind::Signal
    );
    let valid = match kind {
        ElementKind::PartProperty => matches!(
            type_kind,
            ElementKind::Block | ElementKind::AssociationBlock | ElementKind::InterfaceBlock
        ),
        ElementKind::ReferenceProperty => any_classifier,
        ElementKind::ValueProperty => matches!(
            type_kind,
            ElementKind::ValueType
                | ElementKind::DataType
                | ElementKind::PrimitiveType
                | ElementKind::Enumeration
        ),
        ElementKind::FlowProperty => any_classifier,
        ElementKind::ConstraintProperty => matches!(type_kind, ElementKind::ConstraintBlock),
        ElementKind::ProxyPort | ElementKind::FullPort => matches!(
            type_kind,
            ElementKind::InterfaceBlock
                | ElementKind::Block
                | ElementKind::AssociationBlock
                | ElementKind::DataType
        ),
        ElementKind::Parameter | ElementKind::InstanceSpecification => any_classifier,
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
            ElementKind::Block | ElementKind::AssociationBlock => Some("block"),
            ElementKind::InterfaceBlock => Some("interfaceBlock"),
            ElementKind::ConstraintBlock => Some("constraint"),
            ElementKind::ValueType => Some("valueType"),
            ElementKind::Enumeration => Some("enumeration"),
            ElementKind::Unit => Some("unit"),
            ElementKind::QuantityKind => Some("quantityKind"),
            _ => None,
        }
    }

    pub fn feature_compartment(kind: &ElementKind) -> Option<&'static str> {
        match kind {
            ElementKind::PartProperty => Some("parts"),
            ElementKind::ReferenceProperty => Some("references"),
            ElementKind::ValueProperty => Some("values"),
            ElementKind::FlowProperty => Some("flowProperties"),
            ElementKind::ConstraintProperty => Some("constraints"),
            ElementKind::ProxyPort | ElementKind::FullPort => Some("ports"),
            ElementKind::Operation => Some("operations"),
            ElementKind::Reception => Some("receptions"),
            ElementKind::EnumerationLiteral => Some("literals"),
            ElementKind::Slot => Some("slots"),
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
            RelationshipKind::DeriveRequirement
            | RelationshipKind::Satisfy
            | RelationshipKind::Verify
            | RelationshipKind::Refine
            | RelationshipKind::Trace
            | RelationshipKind::Copy => RelationshipNotation {
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
            RelationshipKind::Connector | RelationshipKind::ItemFlow => RelationshipNotation {
                line: LineStyle::Solid,
                source_decoration: EndDecoration::None,
                target_decoration: EndDecoration::None,
            },
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
    }

    #[test]
    fn association_preserves_composition_notation() {
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
        assert_eq!(
            notation::relationship_notation(project.relationship(association).unwrap())
                .source_decoration,
            notation::EndDecoration::FilledDiamond
        );
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
            .create_relationship(
                RelationshipKind::Generalization,
                powertrain,
                vehicle,
                Some(package),
            )
            .unwrap();
        assert_eq!(project.inherited_features(powertrain).unwrap().len(), 1);
    }

    #[test]
    fn supports_extended_bdd_semantic_families() {
        let (mut project, package, vehicle, _powertrain) = structural_project();
        let primitive = project
            .create_element(ElementKind::PrimitiveType, "Real", package)
            .unwrap();
        let signal = project
            .create_element(ElementKind::Signal, "Status", package)
            .unwrap();
        let association_block = project
            .create_element(ElementKind::AssociationBlock, "Connection", package)
            .unwrap();
        let flow = project
            .create_typed_feature(
                ElementKind::FlowProperty,
                "status",
                vehicle,
                signal,
                Multiplicity::ONE,
            )
            .unwrap();
        project.element_mut(flow).unwrap().flow_direction = Some(FlowDirection::Out);
        assert!(project.element(primitive).unwrap().is_classifier());
        assert!(project.element(association_block).unwrap().is_classifier());
        project.validate().unwrap();
    }

    #[test]
    fn validates_value_type_quantity_kind_and_unit_references() {
        let (mut project, package, _vehicle, _powertrain) = structural_project();
        let quantity_kind = project
            .create_element(ElementKind::QuantityKind, "Mass", package)
            .unwrap();
        let unit = project
            .create_element(ElementKind::Unit, "kg", package)
            .unwrap();
        let value_type = project
            .create_element(ElementKind::ValueType, "MassValue", package)
            .unwrap();
        let qk_external = project.element(quantity_kind).unwrap().external_id.clone();
        let unit_external = project.element(unit).unwrap().external_id.clone();
        {
            let value_type = project.element_mut(value_type).unwrap();
            value_type.quantity_kind_external_id = Some(qk_external);
            value_type.unit_external_id = Some(unit_external);
        }
        project.validate().unwrap();
    }
}
