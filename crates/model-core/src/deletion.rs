//! Staged model deletion. References are derived from semantic records, never views.
use crate::{ElementId, ModelError, Project, RelationshipId, SemanticTarget};
use std::collections::HashSet;

pub struct ElementDeletion {
    pub project: Project,
    pub elements: HashSet<ElementId>,
    pub relationships: HashSet<RelationshipId>,
}

impl Project {
    pub fn stage_connected_element_deletion(&self, id: ElementId) -> Result<ElementDeletion, String> {
        if id == self.root_id {
            return Err(ModelError::ProtectedProjectRoot(id).to_string());
        }
        self.element(id).map_err(|error| error.to_string())?;
        let mut elements = HashSet::from([id]);
        loop {
            let owned: Vec<_> = self.elements.values()
                .filter(|element| element.owner_id.is_some_and(|owner| elements.contains(&owner)))
                .map(|element| element.id).collect();
            let count = elements.len();
            elements.extend(owned);
            if elements.len() == count { break; }
        }
        let endpoint_removed = |end: &crate::ConnectorEnd| {
            elements.contains(&end.role_id)
                || end.port_id.is_some_and(|id| elements.contains(&id))
                || end.property_path.iter().any(|id| elements.contains(id))
        };
        let mut relationships: HashSet<_> = self.relationships.values().filter(|relationship| {
            elements.contains(&relationship.source_id)
                || elements.contains(&relationship.target_id)
                || relationship.owner_id.is_some_and(|id| elements.contains(&id))
                || relationship.association_ends.iter().any(|end| {
                    elements.contains(&end.classifier_id)
                        || end.property_id.is_some_and(|id| elements.contains(&id))
                })
                || relationship.connector.as_ref().is_some_and(|connector| {
                    elements.contains(&connector.context_id)
                        || endpoint_removed(&connector.source) || endpoint_removed(&connector.target)
                })
                || relationship.item_flow.as_ref().is_some_and(|flow| {
                    endpoint_removed(&flow.source) || endpoint_removed(&flow.target)
                        || flow.conveyed_item_ids.iter().any(|id| elements.contains(id))
                })
                || relationship.binding.as_ref().is_some_and(|binding| {
                    elements.contains(&binding.source.role_id)
                        || binding.source.parameter_id.is_some_and(|id| elements.contains(&id))
                        || elements.contains(&binding.target.role_id)
                        || binding.target.parameter_id.is_some_and(|id| elements.contains(&id))
                })
        }).map(|relationship| relationship.id).collect();
        let flows: Vec<_> = self.relationships.values().filter(|relationship| {
            relationship.item_flow.as_ref().is_some_and(|flow| relationships.contains(&flow.connector_id))
        }).map(|relationship| relationship.id).collect();
        relationships.extend(flows);
        let mut project = self.clone();
        project.relationships.retain(|id, _| !relationships.contains(id));
        project.elements.retain(|id, _| !elements.contains(id));
        project.profiles.profile_applications.retain(|_, application| !elements.contains(&application.scope_id));
        project.profiles.stereotype_applications.retain(|_, application| match application.target {
            SemanticTarget::Element(id) => !elements.contains(&id),
            SemanticTarget::Relationship(id) => !relationships.contains(&id),
        });
        // A surviving typed usage or required profile reference may require an
        // explicit retype/edit. Never remove unrelated elements to hide that error.
        project.validate().map_err(|error| format!("Cannot delete while a surviving dependency would be invalid: {error}"))?;
        Ok(ElementDeletion { project, elements, relationships })
    }
}
