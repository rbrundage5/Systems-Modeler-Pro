use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use uuid::Uuid;

pub mod behavior;
pub mod ibd;
pub mod parametrics;
pub use behavior::*;
pub use ibd::{Connector, ConnectorEnd, ConnectorKind, ItemFlow};
pub use parametrics::{
    BindingConnector, BindingEndpoint, ParametricEvaluationReport, ParametricEvaluationScope,
    ParametricValueUpdate, evaluate_parametrics,
};

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
    ModelLibrary,
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
    ConstraintParameter,
    ProxyPort,
    FullPort,
    Operation,
    Parameter,
    Reception,
    Requirement,
    TestCase,
    Actor,
    UseCase,
    Comment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipKind {
    Dependency,
    PackageImport,
    ElementImport,
    PackageMerge,
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
    /// Required reuse from the including Use Case (source) to the included Use Case (target).
    Include,
    /// Optional behavior from the extending Use Case (source) to the extended Use Case (target).
    Extend,
    /// Equality binding between compatible value/constraint endpoints.
    BindingConnector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum VisibilityKind {
    #[default]
    Public,
    Private,
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
    #[serde(default)]
    pub visibility: VisibilityKind,
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
    /// Named insertion locations owned by a Use Case.
    #[serde(default)]
    pub extension_points: Vec<String>,
    /// Structured Use Case specification kept separately from general documentation.
    #[serde(default)]
    pub use_case_specification: String,
    /// Optional classifier represented as the system/subject for this Use Case.
    #[serde(default)]
    pub represented_classifier_id: Option<ElementId>,
    /// Reusable equation owned by a ConstraintBlock.
    #[serde(default)]
    pub constraint_expression: String,
    /// Canonical dimension signature owned by a QuantityKind (for example `M*L^2*T^-2`).
    #[serde(default)]
    pub quantity_dimension: Option<String>,
    /// Display symbol owned by a Unit.
    #[serde(default)]
    pub unit_symbol: Option<String>,
    /// Multiplicative conversion from this Unit to the QuantityKind canonical unit.
    #[serde(default = "default_unit_scale")]
    pub unit_scale_to_base: f64,
}

fn default_unit_scale() -> f64 {
    1.0
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
            visibility: VisibilityKind::Public,
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
            extension_points: Vec::new(),
            use_case_specification: String::new(),
            represented_classifier_id: None,
            constraint_expression: String::new(),
            quantity_dimension: None,
            unit_symbol: None,
            unit_scale_to_base: 1.0,
        }
    }

    pub fn is_namespace(&self) -> bool {
        matches!(
            self.kind,
            ElementKind::Model | ElementKind::Package | ElementKind::ModelLibrary
        )
    }

    pub fn is_packageable(&self) -> bool {
        !self.is_feature()
            && !matches!(
                self.kind,
                ElementKind::EnumerationLiteral | ElementKind::Slot | ElementKind::Comment
            )
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
                | ElementKind::Actor
                | ElementKind::UseCase
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
                | ElementKind::ConstraintParameter
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
                    | ElementKind::ConstraintParameter
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
    #[serde(default)]
    pub visibility: VisibilityKind,
    /// Optional local name introduced by an ElementImport.
    #[serde(default)]
    pub alias: Option<String>,
    pub applied_stereotypes: Vec<String>,
    pub association_ends: Vec<AssociationEnd>,
    #[serde(default)]
    pub connector: Option<Connector>,
    #[serde(default)]
    pub item_flow: Option<ItemFlow>,
    /// Optional guard for an Extend relationship.
    #[serde(default)]
    pub extension_condition: Option<String>,
    /// Optional named extension point on the extended (target) Use Case.
    #[serde(default)]
    pub extension_location: Option<String>,
    /// Typed semantic ends for a SysML Binding Connector.
    #[serde(default)]
    pub binding: Option<BindingConnector>,
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
    #[error("the project root cannot be moved or deleted: {0}")]
    ProtectedProjectRoot(ElementId),
    #[error("moving element {element_id} under {new_owner_id} would create an ownership cycle")]
    OwnershipCycle {
        element_id: ElementId,
        new_owner_id: ElementId,
    },
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
    #[error("Actor and Use Case generalization endpoints must have the same semantic kind")]
    InvalidUseCaseGeneralization,
    #[error("invalid endpoints for {relationship:?}: {source_kind:?} -> {target_kind:?}")]
    InvalidUseCaseRelationshipEndpoints {
        relationship: RelationshipKind,
        source_kind: ElementKind,
        target_kind: ElementKind,
    },
    #[error("Use Case relationships cannot connect an element to itself")]
    SelfUseCaseRelationship,
    #[error("extension point '{location}' is not owned by extended Use Case {use_case_id}")]
    ExtensionPointNotFound {
        use_case_id: ElementId,
        location: String,
    },
    #[error("Use Case extension point names must be non-empty and unique: {0}")]
    InvalidExtensionPoints(ElementId),
    #[error("represented Use Case subject is not a valid classifier: {0}")]
    InvalidUseCaseSubject(ElementId),
    #[error("association requires at least two ends")]
    AssociationRequiresTwoEnds,
    #[error("association end classifier not found: {0}")]
    AssociationEndClassifierNotFound(ElementId),
    #[error("value type quantity kind does not resolve to QuantityKind: {0}")]
    InvalidQuantityKindReference(String),
    #[error("value type unit does not resolve to Unit: {0}")]
    InvalidUnitReference(String),
    #[error("ConstraintBlock expression is invalid for {element_id}: {reason}")]
    InvalidConstraintExpression {
        element_id: ElementId,
        reason: String,
    },
    #[error("QuantityKind dimension is invalid for {0}")]
    InvalidQuantityDimension(ElementId),
    #[error("Unit scale must be finite and greater than zero: {0}")]
    InvalidUnitScale(ElementId),
    #[error("binding endpoint is invalid: {0}")]
    InvalidBindingEndpoint(String),
    #[error("binding endpoint types are incompatible: {source_id} vs {target_id}")]
    IncompatibleBindingTypes {
        source_id: ElementId,
        target_id: ElementId,
    },
    #[error("binding connector cannot connect an endpoint to itself")]
    BindingSelfConnection,
    #[error("an equivalent BindingConnector already exists")]
    DuplicateBindingConnector,
    #[error("relationship is not a BindingConnector: {0}")]
    RelationshipIsNotBindingConnector(RelationshipId),
    #[error("parametric evaluation failed: {0}")]
    ParametricEvaluation(String),
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
    #[error(
        "duplicate Requirement traceability relationship: {relationship:?} {source_id} -> {target_id}"
    )]
    DuplicateTraceabilityRelationship {
        relationship: RelationshipKind,
        source_id: ElementId,
        target_id: ElementId,
    },
    #[error("Requirement traceability relationships require a Model or Package owner")]
    MissingTraceabilityOwner,
    #[error("Requirement traceability relationships must be owned by a Model or Package: {0}")]
    InvalidTraceabilityOwner(ElementId),
    #[error("{diagnostic}")]
    InvalidPackageRelationshipEndpoints {
        relationship: RelationshipKind,
        source_name: String,
        target_name: String,
        diagnostic: String,
    },
    #[error("{relationship:?} cannot connect '{element}' to itself")]
    SelfPackageRelationship {
        relationship: RelationshipKind,
        element: String,
    },
    #[error("an equivalent {relationship:?} already exists: '{source_name}' -> '{target_name}'")]
    DuplicatePackageRelationship {
        relationship: RelationshipKind,
        source_name: String,
        target_name: String,
    },
    #[error("{relationship:?} from '{source_name}' must be owned by that importing namespace, not '{owner_name}'")]
    InvalidPackageRelationshipOwner {
        relationship: RelationshipKind,
        source_name: String,
        owner_name: String,
    },
    #[error("ElementImport alias '{0}' is not a valid identifier")]
    InvalidElementImportAlias(String),
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
                | ElementKind::ConstraintParameter
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
        if id == self.root_id {
            return Err(ModelError::ProtectedProjectRoot(id));
        }
        let kind = self.element(id)?.kind.clone();
        let owner_kind = self.element(new_owner_id)?.kind.clone();
        validate_owner_kind(&kind, &owner_kind)?;
        let mut ancestor = Some(new_owner_id);
        let mut visited = HashSet::new();
        while let Some(candidate) = ancestor {
            if candidate == id || !visited.insert(candidate) {
                return Err(ModelError::OwnershipCycle {
                    element_id: id,
                    new_owner_id,
                });
            }
            ancestor = self.element(candidate)?.owner_id;
        }
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
        let package_relationship =
            is_package_relationship(&kind) || kind == RelationshipKind::Dependency;
        if package_relationship {
            validate_package_relationship_endpoints(&kind, source, target)?;
            if source_id == target_id {
                return Err(ModelError::SelfPackageRelationship {
                    relationship: kind,
                    element: source.name.clone(),
                });
            }
            if self.relationships.values().any(|relationship| {
                relationship.kind == kind
                    && relationship.source_id == source_id
                    && relationship.target_id == target_id
            }) {
                return Err(ModelError::DuplicatePackageRelationship {
                    relationship: kind,
                    source_name: source.name.clone(),
                    target_name: target.name.clone(),
                });
            }
            if kind != RelationshipKind::Dependency && owner_id != Some(source_id) {
                let owner = owner_id
                    .and_then(|id| self.element(id).ok())
                    .map(|element| element.name.clone())
                    .unwrap_or_else(|| "no semantic owner".into());
                return Err(ModelError::InvalidPackageRelationshipOwner {
                    relationship: kind,
                    source_name: source.name.clone(),
                    owner_name: owner,
                });
            }
        }
        let traceability = is_traceability_relationship(&kind);
        validate_use_case_relationship_endpoints(&kind, &source.kind, &target.kind)?;
        if matches!(kind, RelationshipKind::Include | RelationshipKind::Extend)
            && source_id == target_id
        {
            return Err(ModelError::SelfUseCaseRelationship);
        }
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
                source_id,
                target_id,
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
            if (matches!(source.kind, ElementKind::Actor | ElementKind::UseCase)
                || matches!(target.kind, ElementKind::Actor | ElementKind::UseCase))
                && source.kind != target.kind
            {
                return Err(ModelError::InvalidUseCaseGeneralization);
            }
        }
        if traceability && owner_id.is_none() {
            return Err(ModelError::MissingTraceabilityOwner);
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
                visibility: VisibilityKind::Public,
                alias: None,
                applied_stereotypes: Vec::new(),
                association_ends: Vec::new(),
                connector: None,
                item_flow: None,
                extension_condition: None,
                extension_location: None,
                binding: None,
            },
        );
        if let Some(text) = copied_requirement_text {
            self.element_mut(source_id)?.requirement_text = Some(text);
        }
        Ok(id)
    }

    pub fn create_package_import(
        &mut self,
        importing_namespace_id: ElementId,
        imported_package_id: ElementId,
        visibility: VisibilityKind,
    ) -> Result<RelationshipId, ModelError> {
        let id = self.create_relationship(
            RelationshipKind::PackageImport,
            importing_namespace_id,
            imported_package_id,
            Some(importing_namespace_id),
        )?;
        self.relationships.get_mut(&id).unwrap().visibility = visibility;
        Ok(id)
    }

    pub fn create_element_import(
        &mut self,
        importing_namespace_id: ElementId,
        imported_element_id: ElementId,
        visibility: VisibilityKind,
        alias: Option<String>,
    ) -> Result<RelationshipId, ModelError> {
        let alias = normalize_element_import_alias(alias)?;
        let id = self.create_relationship(
            RelationshipKind::ElementImport,
            importing_namespace_id,
            imported_element_id,
            Some(importing_namespace_id),
        )?;
        let relationship = self.relationships.get_mut(&id).unwrap();
        relationship.visibility = visibility;
        relationship.alias = alias;
        Ok(id)
    }

    pub fn update_use_case(
        &mut self,
        id: ElementId,
        specification: impl Into<String>,
        extension_points: Vec<String>,
        represented_classifier_id: Option<ElementId>,
    ) -> Result<(), ModelError> {
        if self.element(id)?.kind != ElementKind::UseCase {
            return Err(ModelError::InvalidUseCaseSubject(id));
        }
        if let Some(subject_id) = represented_classifier_id {
            let subject = self.element(subject_id)?;
            if !subject.is_classifier()
                || matches!(subject.kind, ElementKind::Actor | ElementKind::UseCase)
            {
                return Err(ModelError::InvalidUseCaseSubject(subject_id));
            }
        }
        let mut unique = HashSet::new();
        let extension_points = extension_points
            .into_iter()
            .map(|point| point.trim().to_owned())
            .filter(|point| !point.is_empty() && unique.insert(point.clone()))
            .collect();
        let use_case = self.element_mut(id)?;
        use_case.use_case_specification = specification.into();
        use_case.extension_points = extension_points;
        use_case.represented_classifier_id = represented_classifier_id;
        Ok(())
    }

    pub fn update_extend_relationship(
        &mut self,
        id: RelationshipId,
        condition: Option<String>,
        extension_location: Option<String>,
    ) -> Result<(), ModelError> {
        let relationship = self.relationship(id)?;
        if relationship.kind != RelationshipKind::Extend {
            return Err(ModelError::InvalidUseCaseRelationshipEndpoints {
                relationship: relationship.kind.clone(),
                source_kind: self.element(relationship.source_id)?.kind.clone(),
                target_kind: self.element(relationship.target_id)?.kind.clone(),
            });
        }
        if let Some(location) = extension_location.as_deref().filter(|value| !value.trim().is_empty())
        {
            let target = self.element(relationship.target_id)?;
            if !target.extension_points.iter().any(|point| point == location) {
                return Err(ModelError::ExtensionPointNotFound {
                    use_case_id: target.id,
                    location: location.to_owned(),
                });
            }
        }
        let relationship = self.relationships.get_mut(&id).unwrap();
        relationship.extension_condition = condition.filter(|value| !value.trim().is_empty());
        relationship.extension_location = extension_location.filter(|value| !value.trim().is_empty());
        Ok(())
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
        if id == self.root_id {
            return Err(ModelError::ProtectedProjectRoot(id));
        }
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
                || relationship.binding.as_ref().is_some_and(|binding| {
                    binding.source.role_id == id
                        || binding.source.parameter_id == Some(id)
                        || binding.target.role_id == id
                        || binding.target.parameter_id == Some(id)
                })
        }) || self
            .elements
            .values()
            .any(|element| {
                element.type_id == Some(id) || element.represented_classifier_id == Some(id)
            });
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
        if matches!(
            element.kind,
            ElementKind::ValueType
                | ElementKind::ValueProperty
                | ElementKind::ConstraintParameter
                | ElementKind::Unit
        ) {
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
        if element.kind == ElementKind::ConstraintBlock {
            parametrics::validate_constraint_block(self, element)?;
        }
        if element.kind == ElementKind::QuantityKind
            && let Some(dimension) = element.quantity_dimension.as_deref()
            && !parametrics::validate_dimension(dimension)
        {
            return Err(ModelError::InvalidQuantityDimension(element.id));
        }
        if element.kind == ElementKind::Unit
            && (!element.unit_scale_to_base.is_finite() || element.unit_scale_to_base <= 0.0)
        {
            return Err(ModelError::InvalidUnitScale(element.id));
        }
        if element.kind == ElementKind::UseCase {
            if let Some(subject_id) = element.represented_classifier_id {
                let subject = self.element(subject_id)?;
                if !subject.is_classifier()
                    || matches!(subject.kind, ElementKind::Actor | ElementKind::UseCase)
                {
                    return Err(ModelError::InvalidUseCaseSubject(subject_id));
                }
            }
            let mut extension_points = HashSet::new();
            if element
                .extension_points
                .iter()
                .any(|point| point.trim().is_empty() || !extension_points.insert(point))
            {
                return Err(ModelError::InvalidExtensionPoints(element.id));
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
                if relationship.owner_id.is_none() {
                    return Err(ModelError::MissingTraceabilityOwner);
                }
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
                        source_id: relationship.source_id,
                        target_id: relationship.target_id,
                    });
                }
            }
            validate_traceability_endpoints(
                &relationship.kind,
                &self.element(relationship.source_id)?.kind,
                &self.element(relationship.target_id)?.kind,
            )?;
            let source = self.element(relationship.source_id)?;
            let target = self.element(relationship.target_id)?;
            validate_use_case_relationship_endpoints(
                &relationship.kind,
                &source.kind,
                &target.kind,
            )?;
            let package_relationship = is_package_relationship(&relationship.kind)
                || relationship.kind == RelationshipKind::Dependency;
            if package_relationship {
                validate_package_relationship_endpoints(&relationship.kind, source, target)?;
                if relationship.source_id == relationship.target_id {
                    return Err(ModelError::SelfPackageRelationship {
                        relationship: relationship.kind.clone(),
                        element: source.name.clone(),
                    });
                }
                if relationship.kind != RelationshipKind::Dependency
                    && relationship.owner_id != Some(relationship.source_id)
                {
                    let owner = relationship
                        .owner_id
                        .and_then(|id| self.element(id).ok())
                        .map(|element| element.name.clone())
                        .unwrap_or_else(|| "no semantic owner".into());
                    return Err(ModelError::InvalidPackageRelationshipOwner {
                        relationship: relationship.kind.clone(),
                        source_name: source.name.clone(),
                        owner_name: owner,
                    });
                }
                if self.relationships.values().any(|candidate| {
                    candidate.id != relationship.id
                        && candidate.kind == relationship.kind
                        && candidate.source_id == relationship.source_id
                        && candidate.target_id == relationship.target_id
                }) {
                    return Err(ModelError::DuplicatePackageRelationship {
                        relationship: relationship.kind.clone(),
                        source_name: source.name.clone(),
                        target_name: target.name.clone(),
                    });
                }
                if relationship.kind == RelationshipKind::ElementImport {
                    normalize_element_import_alias(relationship.alias.clone())?;
                } else if relationship.alias.is_some() {
                    return Err(ModelError::InvalidElementImportAlias(
                        relationship.alias.clone().unwrap_or_default(),
                    ));
                }
            }
            if matches!(relationship.kind, RelationshipKind::Include | RelationshipKind::Extend)
                && relationship.source_id == relationship.target_id
            {
                return Err(ModelError::SelfUseCaseRelationship);
            }
            if relationship.kind == RelationshipKind::Extend
                && let Some(location) = relationship
                    .extension_location
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                && !target.extension_points.iter().any(|point| point == location)
            {
                return Err(ModelError::ExtensionPointNotFound {
                    use_case_id: target.id,
                    location: location.to_owned(),
                });
            }
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
                if (matches!(source.kind, ElementKind::Actor | ElementKind::UseCase)
                    || matches!(target.kind, ElementKind::Actor | ElementKind::UseCase))
                    && source.kind != target.kind
                {
                    return Err(ModelError::InvalidUseCaseGeneralization);
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
                RelationshipKind::BindingConnector => {
                    self.validate_binding_connector(relationship)?;
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

fn is_package_relationship(kind: &RelationshipKind) -> bool {
    matches!(
        kind,
        RelationshipKind::PackageImport
            | RelationshipKind::ElementImport
            | RelationshipKind::PackageMerge
    )
}

fn package_namespace(kind: &ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Model | ElementKind::Package | ElementKind::ModelLibrary
    )
}

fn validate_package_relationship_endpoints(
    relationship: &RelationshipKind,
    source: &Element,
    target: &Element,
) -> Result<(), ModelError> {
    let diagnostic = match relationship {
        RelationshipKind::PackageImport if !package_namespace(&source.kind) => Some(format!(
            "Package Import requires a Package-compatible importing namespace; '{}' is a {:?}.",
            source.name, source.kind
        )),
        RelationshipKind::PackageImport if !package_namespace(&target.kind) => Some(format!(
            "Package Import requires a Package-compatible imported namespace; '{}' is a {:?}.",
            target.name, target.kind
        )),
        RelationshipKind::ElementImport if !package_namespace(&source.kind) => Some(format!(
            "Element Import requires a Package-compatible importing namespace; '{}' is a {:?}.",
            source.name, source.kind
        )),
        RelationshipKind::ElementImport if !target.is_packageable() => Some(format!(
            "Element Import requires a packageable imported element; '{}' is a {:?}.",
            target.name, target.kind
        )),
        RelationshipKind::PackageMerge
            if !matches!(source.kind, ElementKind::Package | ElementKind::ModelLibrary) =>
        {
            Some(format!(
                "UML Package Merge requires a receiving Package; '{}' is a {:?}.",
                source.name, source.kind
            ))
        }
        RelationshipKind::PackageMerge
            if !matches!(target.kind, ElementKind::Package | ElementKind::ModelLibrary) =>
        {
            Some(format!(
                "UML Package Merge requires a merged Package; '{}' is a {:?}.",
                target.name, target.kind
            ))
        }
        RelationshipKind::Dependency if !source.is_packageable() => Some(format!(
            "Dependency requires a packageable source; '{}' is a {:?}.",
            source.name, source.kind
        )),
        RelationshipKind::Dependency if !target.is_packageable() => Some(format!(
            "Dependency requires a packageable target; '{}' is a {:?}.",
            target.name, target.kind
        )),
        _ => None,
    };
    match diagnostic {
        None => Ok(()),
        Some(diagnostic) => Err(ModelError::InvalidPackageRelationshipEndpoints {
            relationship: relationship.clone(),
            source_name: source.name.clone(),
            target_name: target.name.clone(),
            diagnostic,
        }),
    }
}

fn normalize_element_import_alias(alias: Option<String>) -> Result<Option<String>, ModelError> {
    let Some(alias) = alias
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    let mut characters = alias.chars();
    let starts_valid = characters
        .next()
        .is_some_and(|character| character == '_' || character.is_alphabetic());
    if !starts_valid
        || !characters.all(|character| character == '_' || character.is_alphanumeric())
    {
        return Err(ModelError::InvalidElementImportAlias(alias));
    }
    Ok(Some(alias))
}

fn validate_owner_kind(kind: &ElementKind, owner: &ElementKind) -> Result<(), ModelError> {
    let namespace_owned = package_namespace(owner);
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
        | ElementKind::ModelLibrary
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
        | ElementKind::TestCase
        | ElementKind::Actor
        | ElementKind::UseCase => namespace_owned,
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
        ElementKind::ConstraintParameter => matches!(owner, ElementKind::ConstraintBlock),
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

fn validate_use_case_relationship_endpoints(
    relationship: &RelationshipKind,
    source: &ElementKind,
    target: &ElementKind,
) -> Result<(), ModelError> {
    let valid = match relationship {
        RelationshipKind::Include | RelationshipKind::Extend => {
            *source == ElementKind::UseCase && *target == ElementKind::UseCase
        }
        RelationshipKind::Association
            if matches!(source, ElementKind::Actor | ElementKind::UseCase)
                || matches!(target, ElementKind::Actor | ElementKind::UseCase) =>
        {
            matches!(
                (source, target),
                (ElementKind::Actor, ElementKind::UseCase)
                    | (ElementKind::UseCase, ElementKind::Actor)
            )
        }
        _ => true,
    };
    if valid {
        Ok(())
    } else {
        Err(ModelError::InvalidUseCaseRelationshipEndpoints {
            relationship: relationship.clone(),
            source_kind: source.clone(),
            target_kind: target.clone(),
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
        ElementKind::ConstraintParameter => matches!(
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
        ElementKind::Reception => matches!(type_kind, ElementKind::Signal),
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
            ElementKind::ModelLibrary => Some("modelLibrary"),
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
            ElementKind::ConstraintParameter => Some("parameters"),
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
            RelationshipKind::Dependency
            | RelationshipKind::PackageImport
            | RelationshipKind::ElementImport
            | RelationshipKind::PackageMerge => RelationshipNotation {
                line: LineStyle::Dashed,
                source_decoration: EndDecoration::None,
                target_decoration: EndDecoration::OpenArrow,
            },
            RelationshipKind::Include | RelationshipKind::Extend => RelationshipNotation {
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
            RelationshipKind::Connector
            | RelationshipKind::ItemFlow
            | RelationshipKind::BindingConnector => RelationshipNotation {
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
