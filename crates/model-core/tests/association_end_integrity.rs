use systems_modeler_core::{
    AggregationKind, AssociationEnd, ElementId, ElementKind, ModelError, Multiplicity, Project,
    RelationshipId, RelationshipKind,
};

fn fixture() -> (Project, [ElementId; 3]) {
    let mut project = Project::new("Association integrity");
    let ids = ["A", "B", "C"].map(|name| {
        project
            .create_element(ElementKind::Block, name, project.root_id)
            .unwrap()
    });
    (project, ids)
}

fn end(classifier: ElementId) -> AssociationEnd {
    Project::association_end(
        classifier,
        "role",
        Multiplicity::ONE,
        true,
        AggregationKind::None,
    )
}

#[test]
fn invalid_end_payloads_are_rejected_before_relationship_publication() {
    let (mut project, [a, b, c]) = fixture();
    let mut duplicate = vec![end(a), end(b)];
    duplicate[1].id = duplicate[0].id;
    let mut multiplicity = vec![end(a), end(b)];
    multiplicity[1].multiplicity = Multiplicity {
        lower: 3,
        upper: Some(1),
    };
    let mut both_aggregated = vec![end(a), end(b)];
    both_aggregated[0].aggregation = AggregationKind::Composite;
    both_aggregated[1].aggregation = AggregationKind::Shared;
    let mut nonbinary_composition = vec![end(a), end(b), end(c)];
    nonbinary_composition[0].aggregation = AggregationKind::Composite;
    for ends in [
        vec![end(a)],
        vec![end(a), end(project.root_id)],
        duplicate,
        multiplicity,
        both_aggregated,
        nonbinary_composition,
    ] {
        let before = serde_json::to_value(&project).unwrap();
        assert!(
            project
                .create_association(Some(project.root_id), ends)
                .is_err()
        );
        assert_eq!(serde_json::to_value(&project).unwrap(), before);
    }
}

#[test]
fn reopened_associations_cannot_disagree_with_their_endpoints_or_end_types() {
    let (mut project, [a, b, c]) = fixture();
    let id = project
        .create_association(Some(project.root_id), vec![end(a), end(b)])
        .unwrap();
    let saved = serde_json::to_value(&project).unwrap();
    for invalid in 0..5 {
        let mut loaded: Project = serde_json::from_value(saved.clone()).unwrap();
        let association = loaded.relationships.get_mut(&id).unwrap();
        match invalid {
            0 => association.target_id = c,
            1 => association.association_ends.truncate(1),
            2 => association.association_ends[1].classifier_id = project.root_id,
            3 => {
                association.association_ends[0].multiplicity = Multiplicity {
                    lower: 2,
                    upper: Some(0),
                }
            }
            _ => association.kind = RelationshipKind::Realization,
        }
        let before = serde_json::to_value(&loaded).unwrap();
        assert!(loaded.validate().is_err(), "malformed variant {invalid}");
        assert_eq!(serde_json::to_value(&loaded).unwrap(), before);
    }
}

#[test]
fn member_end_identity_is_unique_across_distinct_associations() {
    let (mut project, [a, b, c]) = fixture();
    let ends = vec![end(a), end(b)];
    let id = project
        .create_association(Some(project.root_id), ends.clone())
        .unwrap();
    let before = serde_json::to_value(&project).unwrap();
    assert!(matches!(
        project.create_association(Some(project.root_id), vec![ends[0].clone(), end(c)]),
        Err(ModelError::DuplicateAssociationEndId(_))
    ));
    assert_eq!(serde_json::to_value(&project).unwrap(), before);
    let mut duplicate = project.relationship(id).unwrap().clone();
    duplicate.id = RelationshipId::new();
    duplicate.external_id = duplicate.id.to_string();
    project.relationships.insert(duplicate.id, duplicate);
    assert!(matches!(
        project.validate(),
        Err(ModelError::DuplicateAssociationEndId(_))
    ));
}

#[test]
fn parallel_reflexive_nary_and_legacy_associations_remain_readable() {
    let (mut project, [a, b, c]) = fixture();
    for ends in [
        vec![end(a), end(b)],
        vec![end(a), end(b)],
        vec![end(a), end(a)],
        vec![end(a), end(b), end(c)],
    ] {
        project
            .create_association(Some(project.root_id), ends)
            .unwrap();
    }
    for aggregation in [AggregationKind::Shared, AggregationKind::Composite] {
        let mut ends = vec![end(a), end(b)];
        ends[0].aggregation = aggregation;
        project
            .create_association(Some(project.root_id), ends)
            .unwrap();
    }
    project
        .create_relationship(RelationshipKind::Association, a, b, Some(project.root_id))
        .unwrap();
    project
        .create_relationship(RelationshipKind::Composition, a, b, Some(project.root_id))
        .unwrap();
    project.validate().unwrap();
    let reopened: Project =
        serde_json::from_value(serde_json::to_value(&project).unwrap()).unwrap();
    reopened.validate().unwrap();
    assert_eq!(reopened.relationships.len(), project.relationships.len());
}
