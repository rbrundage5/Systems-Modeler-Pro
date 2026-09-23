//! Explicit identity between a classifier-owned Property and an association end.
use crate::{
    AggregationKind, AssociationEnd, ElementId, ElementKind, ModelError, Multiplicity, Project,
    RelationshipId,
};

impl Project {
    /// Link an existing usage by ID; names and classifier pairs never infer identity.
    /// The Property remains classifier-owned if the association is later removed.
    pub fn create_property_association(
        &mut self,
        property_id: ElementId,
        owner_id: Option<ElementId>,
    ) -> Result<RelationshipId, ModelError> {
        self.validate_element(property_id)?;
        let property = self.element(property_id)?;
        if !matches!(
            property.kind,
            ElementKind::PartProperty | ElementKind::ReferenceProperty
        ) {
            return Err(ModelError::InvalidAssociationProperty(property_id));
        }
        let owner = property
            .owner_id
            .ok_or(ModelError::InvalidAssociationProperty(property_id))?;
        let type_id = property
            .type_id
            .ok_or(ModelError::InvalidAssociationProperty(property_id))?;
        let inverse = Self::association_end(
            owner,
            "",
            Multiplicity::new(
                0,
                if property.aggregation == AggregationKind::Composite {
                    Some(1)
                } else {
                    None
                },
            )?,
            false,
            AggregationKind::None,
        );
        let mut end = Self::association_end(
            type_id,
            property.name.clone(),
            property.multiplicity.unwrap_or(Multiplicity::ONE),
            true,
            property.aggregation,
        );
        end.property_id = Some(property_id);
        self.create_association(owner_id, vec![inverse, end])
    }

    /// Create one typed part usage and its association atomically. Failure removes
    /// only the newly staged Property; reusable Block definitions are untouched.
    pub fn create_composition(
        &mut self,
        whole_id: ElementId,
        part_type_id: ElementId,
        role_name: impl Into<String>,
        multiplicity: Multiplicity,
        owner_id: Option<ElementId>,
    ) -> Result<(RelationshipId, ElementId), ModelError> {
        Multiplicity::new(multiplicity.lower, multiplicity.upper)?;
        crate::validate_owner_kind(&ElementKind::PartProperty, &self.element(whole_id)?.kind)?;
        crate::validate_type_kind(&ElementKind::PartProperty, &self.element(part_type_id)?.kind)?;
        let property_id = self.create_typed_feature(
            ElementKind::PartProperty,
            role_name,
            whole_id,
            part_type_id,
            multiplicity,
        )?;
        match self.create_property_association(property_id, owner_id) {
            Ok(relationship_id) => Ok((relationship_id, property_id)),
            Err(error) => {
                self.elements.remove(&property_id);
                Err(error)
            }
        }
    }

    pub(crate) fn validate_association_property_ends(
        &self,
        ends: &[AssociationEnd],
    ) -> Result<(), ModelError> {
        let linked: Vec<_> = ends
            .iter()
            .enumerate()
            .filter(|(_, end)| end.property_id.is_some())
            .collect();
        if linked.is_empty() {
            return Ok(());
        }
        if ends.len() != 2 || linked.len() != 1 {
            return Err(ModelError::InvalidAssociationPropertyShape);
        }
        let (index, end) = linked[0];
        let property_id = end.property_id.unwrap();
        let property = self
            .elements
            .get(&property_id)
            .ok_or(ModelError::InvalidAssociationProperty(property_id))?;
        let inverse = &ends[1 - index];
        if !matches!(
            property.kind,
            ElementKind::PartProperty | ElementKind::ReferenceProperty
        ) || property.owner_id != Some(inverse.classifier_id)
            || property.type_id != Some(end.classifier_id)
            || property.name != end.role_name
            || property.multiplicity.unwrap_or(Multiplicity::ONE) != end.multiplicity
            || property.aggregation != end.aggregation
            || !end.navigable
            || inverse.navigable
            || inverse.aggregation != AggregationKind::None
        {
            return Err(ModelError::InvalidAssociationProperty(property_id));
        }
        if end.aggregation == AggregationKind::Composite
            && inverse.multiplicity.upper.is_none_or(|upper| upper > 1)
        {
            return Err(ModelError::InvalidCompositeWholeMultiplicity);
        }
        Ok(())
    }

    /// Project Property edits into their member ends. Direct deserialized changes
    /// are not repaired: whole-project validation must reject inconsistent files.
    pub(crate) fn refresh_association_property(&mut self, property_id: ElementId) {
        let Some(property) = self.elements.get(&property_id) else {
            return;
        };
        let (Some(owner_id), Some(type_id)) = (property.owner_id, property.type_id) else {
            return;
        };
        for relationship in self.relationships.values_mut() {
            if relationship.association_ends.len() != 2 {
                continue;
            }
            let Some(index) = relationship
                .association_ends
                .iter()
                .position(|end| end.property_id == Some(property_id))
            else {
                continue;
            };
            let end = &mut relationship.association_ends[index];
            end.classifier_id = type_id;
            end.role_name = property.name.clone();
            end.multiplicity = property.multiplicity.unwrap_or(Multiplicity::ONE);
            end.aggregation = property.aggregation;
            relationship.association_ends[1 - index].classifier_id = owner_id;
            relationship.source_id = relationship.association_ends[0].classifier_id;
            relationship.target_id = relationship.association_ends[1].classifier_id;
        }
    }
}
