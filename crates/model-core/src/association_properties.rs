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
        crate::validate_type_kind(
            &ElementKind::PartProperty,
            &self.element(part_type_id)?.kind,
        )?;
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

    /// Eligible existing usages for an explicit legacy composition link. The caller
    /// chooses an ID; equal names never imply that two records are one Property.
    pub fn composition_property_choices(
        &self,
        relationship_id: RelationshipId,
    ) -> Result<Vec<crate::ElementTypeChoice>, ModelError> {
        let (whole, part_type, _) = self.legacy_composition_context(relationship_id)?;
        let linked: std::collections::HashSet<_> = self
            .relationships
            .values()
            .flat_map(|relationship| {
                relationship
                    .association_ends
                    .iter()
                    .filter_map(|end| end.property_id)
            })
            .collect();
        let mut choices: Vec<_> = self
            .elements
            .values()
            .filter(|property| {
                property.kind == ElementKind::PartProperty
                    && property.owner_id == Some(whole)
                    && property.type_id == Some(part_type)
                    && !linked.contains(&property.id)
            })
            .map(|property| crate::ElementTypeChoice {
                id: property.id,
                external_id: property.external_id.clone(),
                qualified_name: self
                    .qualified_name(property.id)
                    .unwrap_or_else(|_| property.name.clone()),
                kind: property.kind.clone(),
            })
            .collect();
        choices.sort_by(|left, right| {
            left.qualified_name
                .cmp(&right.qualified_name)
                .then(left.external_id.cmp(&right.external_id))
        });
        Ok(choices)
    }

    fn legacy_composition_context(
        &self,
        relationship_id: RelationshipId,
    ) -> Result<(ElementId, ElementId, usize), ModelError> {
        let relationship = self.relationship(relationship_id)?;
        if relationship
            .association_ends
            .iter()
            .any(|end| end.property_id.is_some())
        {
            return Err(ModelError::InvalidAssociationPropertyShape);
        }
        if relationship.kind == crate::RelationshipKind::Composition
            && relationship.association_ends.is_empty()
        {
            return Ok((relationship.source_id, relationship.target_id, 1));
        }
        if !matches!(
            relationship.kind,
            crate::RelationshipKind::Association | crate::RelationshipKind::Composition
        ) || relationship.association_ends.len() != 2
        {
            return Err(ModelError::InvalidAssociationPropertyShape);
        }
        let whole_index = relationship
            .association_ends
            .iter()
            .position(|end| end.aggregation == AggregationKind::Composite)
            .ok_or(ModelError::InvalidAssociationPropertyShape)?;
        Ok((
            relationship.association_ends[whole_index].classifier_id,
            relationship.association_ends[1 - whole_index].classifier_id,
            1 - whole_index,
        ))
    }

    /// Explicitly materialize or attach one part to an old composition. Validate a
    /// staged Project so corrupt legacy data and rejected selections are unchanged.
    pub fn link_composition_property(
        &mut self,
        relationship_id: RelationshipId,
        selected_property: Option<ElementId>,
    ) -> Result<ElementId, ModelError> {
        let original = self.relationship(relationship_id)?;
        if let Some(property_id) = original
            .association_ends
            .iter()
            .find_map(|end| end.property_id)
        {
            if selected_property.is_some_and(|selected| selected != property_id) {
                return Err(ModelError::DuplicateAssociationProperty(property_id));
            }
            self.validate()?;
            return Ok(property_id);
        }
        let (whole, part_type, part_index) = self.legacy_composition_context(relationship_id)?;
        let mut candidate = self.clone();
        if original.association_ends.is_empty() {
            candidate
                .relationships
                .get_mut(&relationship_id)
                .unwrap()
                .association_ends = vec![
                Self::association_end(
                    whole,
                    "",
                    Multiplicity::ONE,
                    true,
                    AggregationKind::Composite,
                ),
                Self::association_end(
                    part_type,
                    "",
                    Multiplicity::ONE,
                    true,
                    AggregationKind::None,
                ),
            ];
        }
        let ends = &candidate.relationship(relationship_id)?.association_ends;
        if ends[1 - part_index]
            .multiplicity
            .upper
            .is_none_or(|upper| upper > 1)
        {
            return Err(ModelError::InvalidCompositeWholeMultiplicity);
        }
        let property_id = if let Some(property_id) = selected_property {
            if !self
                .composition_property_choices(relationship_id)?
                .iter()
                .any(|choice| choice.id == property_id)
            {
                return Err(ModelError::InvalidAssociationProperty(property_id));
            }
            property_id
        } else {
            let part_end = &ends[part_index];
            let base_name = if part_end.role_name.trim().is_empty() {
                "part"
            } else {
                part_end.role_name.trim()
            };
            let names: std::collections::HashSet<_> = candidate
                .owned_features(whole)
                .map(|property| property.name.clone())
                .collect();
            let mut name = base_name.to_owned();
            let mut suffix = 2;
            while names.contains(&name) {
                name = format!("{base_name}{suffix}");
                suffix += 1;
            }
            candidate.create_typed_feature(
                ElementKind::PartProperty,
                name,
                whole,
                part_type,
                part_end.multiplicity,
            )?
        };
        let property = candidate.element(property_id)?.clone();
        let relationship = candidate.relationships.get_mut(&relationship_id).unwrap();
        relationship.kind = crate::RelationshipKind::Association;
        let inverse = &mut relationship.association_ends[1 - part_index];
        inverse.aggregation = AggregationKind::None;
        inverse.navigable = false;
        let end = &mut relationship.association_ends[part_index];
        end.property_id = Some(property_id);
        end.role_name = property.name;
        end.multiplicity = property.multiplicity.unwrap_or(Multiplicity::ONE);
        end.aggregation = AggregationKind::Composite;
        end.navigable = true;
        candidate.validate()?;
        *self = candidate;
        Ok(property_id)
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
