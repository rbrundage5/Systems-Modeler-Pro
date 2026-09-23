use systems_modeler_core::{
    AggregationKind, ElementId, ElementKind, ModelError, Multiplicity, Project, RelationshipKind,
    notation::{EndDecoration, relationship_notation},
};

fn fixture() -> (Project, ElementId, ElementId) {
    let mut project = Project::new("Composition");
    let whole = project
        .create_element(ElementKind::Block, "Vehicle", project.root_id)
        .unwrap();
    let part = project
        .create_element(ElementKind::Block, "Wheel", project.root_id)
        .unwrap();
    (project, whole, part)
}

#[test]
fn composition_creates_distinct_inherited_usages_with_canonical_member_identity() {
    let (mut project, whole, part) = fixture();
    let mut usages = Vec::new();
    for name in ["front", "rear"] {
        let (relation, property) = project
            .create_composition(whole, part, name, Multiplicity::ONE, Some(project.root_id))
            .unwrap();
        let usage = project.element(property).unwrap();
        assert_eq!(usage.owner_id, Some(whole));
        assert_eq!(usage.type_id, Some(part));
        assert_eq!(usage.kind, ElementKind::PartProperty);
        let relationship = project.relationship(relation).unwrap();
        assert_eq!(relationship.association_ends[1].property_id, Some(property));
        assert_eq!(relationship.association_ends[1].role_name, name);
        assert_eq!(
            relationship.association_ends[1].aggregation,
            AggregationKind::Composite
        );
        assert_eq!(
            relationship_notation(relationship).source_decoration,
            EndDecoration::FilledDiamond
        );
        usages.push(property);
    }
    assert_ne!(usages[0], usages[1]);
    let derived = project
        .create_element(ElementKind::Block, "ElectricVehicle", project.root_id)
        .unwrap();
    project
        .create_relationship(
            RelationshipKind::Generalization,
            derived,
            whole,
            Some(project.root_id),
        )
        .unwrap();
    let inherited = project.inherited_features(derived).unwrap();
    for usage in usages {
        assert!(inherited.iter().any(|feature| feature.id == usage));
    }
    project.validate().unwrap();
}

#[test]
fn property_edits_project_by_id_without_changing_member_end_identity() {
    let (mut project, whole, part) = fixture();
    let (relation, property) = project
        .create_composition(
            whole,
            part,
            "wheel",
            Multiplicity::ONE,
            Some(project.root_id),
        )
        .unwrap();
    let end_id = project.relationship(relation).unwrap().association_ends[1].id;
    let replacement = project
        .create_element(ElementKind::Block, "NewWheel", project.root_id)
        .unwrap();
    project.rename_element(property, "front").unwrap();
    project
        .set_multiplicity(property, Multiplicity::new(2, Some(4)).unwrap())
        .unwrap();
    project.set_element_type(property, replacement).unwrap();
    let relationship = project.relationship(relation).unwrap();
    let end = &relationship.association_ends[1];
    assert_eq!(end.id, end_id);
    assert_eq!(end.property_id, Some(property));
    assert_eq!(end.role_name, "front");
    assert_eq!(end.multiplicity.notation(), "2..4");
    assert_eq!(relationship.target_id, replacement);
    project.validate().unwrap();
    assert!(matches!(
        project.delete_element(property),
        Err(ModelError::ElementStillReferenced(id)) if id == property
    ));
    project.relationships.remove(&relation);
    project.delete_element(property).unwrap();
    assert!(project.element(part).is_ok());
    assert!(project.element(replacement).is_ok());
}

#[test]
fn failed_composition_creation_leaves_no_orphan_property() {
    let (mut project, whole, part) = fixture();
    let before = serde_json::to_value(&project).unwrap();
    assert!(
        project
            .create_composition(whole, project.root_id, "invalid", Multiplicity::ONE, None)
            .is_err()
    );
    assert_eq!(serde_json::to_value(&project).unwrap(), before);
    for (owner, multiplicity) in [
        (Some(ElementId::new()), Multiplicity::ONE),
        (
            Some(project.root_id),
            Multiplicity {
                lower: 4,
                upper: Some(2),
            },
        ),
    ] {
        assert!(
            project
                .create_composition(whole, part, "wheel", multiplicity, owner)
                .is_err()
        );
        assert_eq!(serde_json::to_value(&project).unwrap(), before);
    }
}

#[test]
fn malformed_property_links_and_duplicate_membership_are_rejected() {
    let (mut project, whole, part) = fixture();
    let (relation, property) = project
        .create_composition(
            whole,
            part,
            "wheel",
            Multiplicity::ONE,
            Some(project.root_id),
        )
        .unwrap();
    let before = serde_json::to_value(&project).unwrap();
    assert_eq!(
        project.create_property_association(property, Some(project.root_id)),
        Err(ModelError::DuplicateAssociationProperty(property))
    );
    assert_eq!(serde_json::to_value(&project).unwrap(), before);
    for case in 0..7 {
        let mut invalid = project.clone();
        let ends = &mut invalid
            .relationships
            .get_mut(&relation)
            .unwrap()
            .association_ends;
        match case {
            0 => ends[1].property_id = Some(ElementId::new()),
            1 => ends[1].role_name = "stale".into(),
            2 => ends[1].multiplicity = Multiplicity::new(3, Some(3)).unwrap(),
            3 => ends[1].navigable = false,
            4 => ends[0].multiplicity = Multiplicity::new(0, None).unwrap(),
            5 => ends[0].property_id = Some(property),
            _ => ends[0].navigable = true,
        }
        assert!(invalid.validate().is_err(), "case {case}");
    }
    let mut invalid = project.clone();
    invalid.element_mut(property).unwrap().owner_id = Some(part);
    assert!(invalid.validate().is_err());
}

#[test]
fn legacy_end_payloads_keep_their_notation_and_do_not_infer_properties() {
    let (mut project, whole, part) = fixture();
    let relation = project
        .create_association(
            Some(project.root_id),
            vec![
                Project::association_end(
                    whole,
                    "",
                    Multiplicity::ONE,
                    true,
                    AggregationKind::Composite,
                ),
                Project::association_end(
                    part,
                    "wheel",
                    Multiplicity::ONE,
                    true,
                    AggregationKind::None,
                ),
            ],
        )
        .unwrap();
    let encoded = serde_json::to_string(&project).unwrap();
    assert!(!encoded.contains("property_id"));
    let decoded: Project = serde_json::from_str(&encoded).unwrap();
    decoded.validate().unwrap();
    let relationship = decoded.relationship(relation).unwrap();
    assert_eq!(
        relationship_notation(relationship).source_decoration,
        EndDecoration::FilledDiamond
    );
    assert_eq!(decoded.owned_features(whole).count(), 0);
}

#[test]
fn explicit_legacy_link_preserves_both_member_ids_and_diamond_orientation() {
    for whole_index in 0..2 {
        let (mut project, whole, part) = fixture();
        let whole_end = Project::association_end(whole, "", Multiplicity::ONE, true, AggregationKind::Composite);
        let part_end = Project::association_end(part, "wheel", Multiplicity::new(2, Some(2)).unwrap(), true, AggregationKind::None);
        let ends = if whole_index == 0 { vec![whole_end, part_end] } else { vec![part_end, whole_end] };
        let ids: Vec<_> = ends.iter().map(|end| end.id).collect();
        let relation = project.create_association(Some(project.root_id), ends).unwrap();
        let property = project.link_composition_property(relation, None).unwrap();
        let relationship = project.relationship(relation).unwrap();
        assert_eq!(relationship.association_ends.iter().map(|end| end.id).collect::<Vec<_>>(), ids);
        assert_eq!(relationship.association_ends[1 - whole_index].property_id, Some(property));
        assert_eq!(project.element(property).unwrap().owner_id, Some(whole));
        assert_eq!(project.element(property).unwrap().type_id, Some(part));
        assert_eq!(project.element(property).unwrap().multiplicity.unwrap().notation(), "2");
        let notation = relationship_notation(relationship);
        let diamond = if whole_index == 0 { notation.source_decoration } else { notation.target_decoration };
        assert_eq!(diamond, EndDecoration::FilledDiamond);
        let before = serde_json::to_value(&project).unwrap();
        assert_eq!(project.link_composition_property(relation, Some(property)).unwrap(), property);
        assert_eq!(serde_json::to_value(&project).unwrap(), before);
        project.validate().unwrap();
    }
}

#[test]
fn legacy_foundation_composition_can_reuse_only_an_explicit_eligible_part() {
    let (mut project, whole, part) = fixture();
    let property = project.create_typed_feature(ElementKind::PartProperty, "front", whole, part, Multiplicity::ONE).unwrap();
    let relation = project.create_relationship(RelationshipKind::Composition, whole, part, Some(project.root_id)).unwrap();
    let before = serde_json::to_value(&project).unwrap();
    assert_eq!(project.composition_property_choices(relation).unwrap().len(), 1);
    assert!(project.link_composition_property(relation, Some(part)).is_err());
    assert_eq!(serde_json::to_value(&project).unwrap(), before);
    let count = project.elements.len();
    assert_eq!(project.link_composition_property(relation, Some(property)).unwrap(), property);
    assert_eq!(project.elements.len(), count);
    let second = project.create_relationship(RelationshipKind::Composition, whole, part, Some(project.root_id)).unwrap();
    assert!(project.composition_property_choices(second).unwrap().is_empty());
    let before = serde_json::to_value(&project).unwrap();
    assert!(project.link_composition_property(second, Some(property)).is_err());
    assert_eq!(serde_json::to_value(&project).unwrap(), before);
}

#[test]
fn legacy_link_rejects_invalid_whole_multiplicity_without_creating_a_part() {
    let (mut project, whole, part) = fixture();
    let relation = project.create_association(Some(project.root_id), vec![
        Project::association_end(whole, "", Multiplicity::new(0, None).unwrap(), true, AggregationKind::Composite),
        Project::association_end(part, "wheel", Multiplicity::ONE, true, AggregationKind::None),
    ]).unwrap();
    let before = serde_json::to_value(&project).unwrap();
    assert_eq!(project.link_composition_property(relation, None), Err(ModelError::InvalidCompositeWholeMultiplicity));
    assert_eq!(serde_json::to_value(&project).unwrap(), before);
}
