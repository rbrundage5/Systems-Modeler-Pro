use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);
        impl $name {
            pub fn new() -> Self { Self(Uuid::new_v4()) }
        }
        impl Default for $name { fn default() -> Self { Self::new() } }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.0.fmt(f) }
        }
    };
}

id_type!(ProjectId);
id_type!(ElementId);
id_type!(RelationshipId);
id_type!(DiagramId);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElementKind {
    Model,
    Package,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipKind {
    Dependency,
    Association,
    Composition,
    Generalization,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Element {
    pub id: ElementId,
    pub external_id: String,
    pub kind: ElementKind,
    pub name: String,
    pub owner_id: Option<ElementId>,
    pub documentation: String,
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
    #[error("owner must be a model or package: {0}")]
    InvalidOwner(ElementId),
    #[error("relationship endpoint not found: {0}")]
    EndpointNotFound(ElementId),
    #[error("cannot delete an element that still owns children: {0}")]
    OwnerHasChildren(ElementId),
}

impl Project {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let root_id = ElementId::new();
        let root = Element {
            id: root_id,
            external_id: format!("MODEL-{root_id}"),
            kind: ElementKind::Model,
            name: name.clone(),
            owner_id: None,
            documentation: String::new(),
        };
        let mut elements = HashMap::new();
        elements.insert(root_id, root);
        Self { id: ProjectId::new(), name, root_id, elements, relationships: HashMap::new() }
    }

    pub fn element(&self, id: ElementId) -> Result<&Element, ModelError> {
        self.elements.get(&id).ok_or(ModelError::ElementNotFound(id))
    }

    pub fn children(&self, owner_id: ElementId) -> impl Iterator<Item = &Element> {
        self.elements.values().filter(move |element| element.owner_id == Some(owner_id))
    }

    pub fn create_element(
        &mut self,
        kind: ElementKind,
        name: impl Into<String>,
        owner_id: ElementId,
    ) -> Result<ElementId, ModelError> {
        let owner = self.element(owner_id)?;
        if !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {
            return Err(ModelError::InvalidOwner(owner_id));
        }
        let id = ElementId::new();
        let element = Element {
            id,
            external_id: format!("EL-{id}"),
            kind,
            name: name.into(),
            owner_id: Some(owner_id),
            documentation: String::new(),
        };
        self.elements.insert(id, element);
        Ok(id)
    }

    pub fn rename_element(&mut self, id: ElementId, name: impl Into<String>) -> Result<(), ModelError> {
        let element = self.elements.get_mut(&id).ok_or(ModelError::ElementNotFound(id))?;
        element.name = name.into();
        Ok(())
    }

    pub fn create_relationship(
        &mut self,
        kind: RelationshipKind,
        source_id: ElementId,
        target_id: ElementId,
        owner_id: Option<ElementId>,
    ) -> Result<RelationshipId, ModelError> {
        if !self.elements.contains_key(&source_id) { return Err(ModelError::EndpointNotFound(source_id)); }
        if !self.elements.contains_key(&target_id) { return Err(ModelError::EndpointNotFound(target_id)); }
        if let Some(owner_id) = owner_id { self.element(owner_id)?; }
        let id = RelationshipId::new();
        self.relationships.insert(id, Relationship {
            id,
            external_id: format!("REL-{id}"),
            kind,
            name: String::new(),
            owner_id,
            source_id,
            target_id,
            documentation: String::new(),
        });
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_owned_structure_and_relationship() {
        let mut project = Project::new("Vehicle");
        let package = project.create_element(ElementKind::Package, "Structure", project.root_id).unwrap();
        let a = project.create_element(ElementKind::Block, "Vehicle", package).unwrap();
        let b = project.create_element(ElementKind::Block, "Powertrain", package).unwrap();
        let relationship = project.create_relationship(RelationshipKind::Composition, a, b, Some(package)).unwrap();
        assert_eq!(project.children(package).count(), 2);
        assert_eq!(project.relationships[&relationship].source_id, a);
    }
}
