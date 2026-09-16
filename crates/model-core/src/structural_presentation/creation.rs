//! Typed creation of existing top-level BDD elements. Owned features and
//! Requirement ID/text use their existing specialized commands.
use crate::{ElementId, ElementKind, ModelError, Project};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BddElementKind {
    Block,
    AssociationBlock,
    InterfaceBlock,
    ConstraintBlock,
    ValueType,
    DataType,
    PrimitiveType,
    Enumeration,
    Signal,
    Unit,
    QuantityKind,
    InstanceSpecification,
    Comment,
    TestCase,
    Actor,
    UseCase,
}

impl BddElementKind {
    pub fn model_kind(self) -> ElementKind {
        match self {
            Self::Block => ElementKind::Block,
            Self::AssociationBlock => ElementKind::AssociationBlock,
            Self::InterfaceBlock => ElementKind::InterfaceBlock,
            Self::ConstraintBlock => ElementKind::ConstraintBlock,
            Self::ValueType => ElementKind::ValueType,
            Self::DataType => ElementKind::DataType,
            Self::PrimitiveType => ElementKind::PrimitiveType,
            Self::Enumeration => ElementKind::Enumeration,
            Self::Signal => ElementKind::Signal,
            Self::Unit => ElementKind::Unit,
            Self::QuantityKind => ElementKind::QuantityKind,
            Self::InstanceSpecification => ElementKind::InstanceSpecification,
            Self::Comment => ElementKind::Comment,
            Self::TestCase => ElementKind::TestCase,
            Self::Actor => ElementKind::Actor,
            Self::UseCase => ElementKind::UseCase,
        }
    }

    pub fn from_model_kind(kind: &ElementKind) -> Option<Self> {
        match kind {
            ElementKind::Block => Some(Self::Block),
            ElementKind::AssociationBlock => Some(Self::AssociationBlock),
            ElementKind::InterfaceBlock => Some(Self::InterfaceBlock),
            ElementKind::ConstraintBlock => Some(Self::ConstraintBlock),
            ElementKind::ValueType => Some(Self::ValueType),
            ElementKind::DataType => Some(Self::DataType),
            ElementKind::PrimitiveType => Some(Self::PrimitiveType),
            ElementKind::Enumeration => Some(Self::Enumeration),
            ElementKind::Signal => Some(Self::Signal),
            ElementKind::Unit => Some(Self::Unit),
            ElementKind::QuantityKind => Some(Self::QuantityKind),
            ElementKind::InstanceSpecification => Some(Self::InstanceSpecification),
            ElementKind::Comment => Some(Self::Comment),
            ElementKind::TestCase => Some(Self::TestCase),
            ElementKind::Actor => Some(Self::Actor),
            ElementKind::UseCase => Some(Self::UseCase),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateBddElement {
    pub kind: BddElementKind,
    pub owner: ElementId,
    pub name: String,
}

impl CreateBddElement {
    pub fn apply(&self, project: &mut Project) -> Result<ElementId, ModelError> {
        project.create_element(self.kind.model_kind(), &self.name, self.owner)
    }
}
