use systems_modeler_core::notation::{EndDecoration, LineStyle, relationship_notation};
use systems_modeler_core::{
    AggregationKind, ElementKind, ModelError, Multiplicity, Project, RelationshipKind,
};

fn use_case_fixture() -> (Project, systems_modeler_core::ElementId) {
    let mut project = Project::new("Use Case Qualification");
    let package = project
        .create_element(ElementKind::Package, "Operations", project.root_id)
        .unwrap();
    (project, package)
}

#[test]
fn actors_and_use_cases_are_repository_semantics_with_specs_and_subjects() {
    let (mut project, package) = use_case_fixture();
    let system = project
        .create_element(ElementKind::Block, "Reservation System", package)
        .unwrap();
    let actor = project
        .create_element(ElementKind::Actor, "Traveler", package)
        .unwrap();
    let use_case = project
        .create_element(ElementKind::UseCase, "Book trip", package)
        .unwrap();

    project
        .update_use_case(
            use_case,
            "The traveler books a valid itinerary.",
            vec!["payment".into(), "confirmation".into(), "payment".into()],
            Some(system),
        )
        .unwrap();
    project.element_mut(use_case).unwrap().documentation = "Booking goal".into();
    project.validate().unwrap();

    assert!(project.element(actor).unwrap().is_classifier());
    let stored = project.element(use_case).unwrap();
    assert_eq!(stored.extension_points, ["payment", "confirmation"]);
    assert_eq!(stored.represented_classifier_id, Some(system));
    assert_eq!(stored.documentation, "Booking goal");
    assert!(stored.use_case_specification.contains("itinerary"));
}

#[test]
fn association_include_extend_and_generalization_enforce_sysml_endpoints_and_direction() {
    let (mut project, package) = use_case_fixture();
    let actor = project
        .create_element(ElementKind::Actor, "Traveler", package)
        .unwrap();
    let specialized_actor = project
        .create_element(ElementKind::Actor, "Member", package)
        .unwrap();
    let base = project
        .create_element(ElementKind::UseCase, "Book trip", package)
        .unwrap();
    let included = project
        .create_element(ElementKind::UseCase, "Authenticate", package)
        .unwrap();
    let extending = project
        .create_element(ElementKind::UseCase, "Apply promotion", package)
        .unwrap();
    project
        .update_use_case(base, "", vec!["discount".into()], None)
        .unwrap();

    project
        .create_association(
            Some(package),
            vec![
                Project::association_end(
                    actor,
                    "",
                    Multiplicity::ONE,
                    false,
                    AggregationKind::None,
                ),
                Project::association_end(base, "", Multiplicity::ONE, false, AggregationKind::None),
            ],
        )
        .unwrap();
    let include = project
        .create_relationship(RelationshipKind::Include, base, included, Some(package))
        .unwrap();
    let extend = project
        .create_relationship(RelationshipKind::Extend, extending, base, Some(package))
        .unwrap();
    project
        .update_extend_relationship(
            extend,
            Some("member has promotion".into()),
            Some("discount".into()),
        )
        .unwrap();
    project
        .create_relationship(
            RelationshipKind::Generalization,
            specialized_actor,
            actor,
            Some(package),
        )
        .unwrap();
    project.validate().unwrap();

    assert_eq!(project.relationship(include).unwrap().source_id, base);
    assert_eq!(project.relationship(include).unwrap().target_id, included);
    assert_eq!(project.relationship(extend).unwrap().source_id, extending);
    assert_eq!(project.relationship(extend).unwrap().target_id, base);
    let notation = relationship_notation(project.relationship(extend).unwrap());
    assert_eq!(notation.line, LineStyle::Dashed);
    assert_eq!(notation.target_decoration, EndDecoration::OpenArrow);
}

#[test]
fn invalid_use_case_relationships_fail_without_entering_the_model() {
    let (mut project, package) = use_case_fixture();
    let actor = project
        .create_element(ElementKind::Actor, "Traveler", package)
        .unwrap();
    let use_case = project
        .create_element(ElementKind::UseCase, "Book trip", package)
        .unwrap();
    let block = project
        .create_element(ElementKind::Block, "System", package)
        .unwrap();

    assert!(matches!(
        project.create_relationship(RelationshipKind::Include, actor, use_case, Some(package)),
        Err(ModelError::InvalidUseCaseRelationshipEndpoints { .. })
    ));
    assert!(matches!(
        project.create_relationship(RelationshipKind::Extend, use_case, use_case, Some(package)),
        Err(ModelError::SelfUseCaseRelationship)
    ));
    assert!(matches!(
        project.create_relationship(RelationshipKind::Association, actor, block, Some(package)),
        Err(ModelError::InvalidUseCaseRelationshipEndpoints { .. })
    ));
    assert!(matches!(
        project.create_relationship(
            RelationshipKind::Generalization,
            actor,
            use_case,
            Some(package)
        ),
        Err(ModelError::InvalidUseCaseGeneralization)
    ));
    assert!(project.relationships.is_empty());
}

#[test]
fn extend_location_must_resolve_on_the_extended_target_use_case() {
    let (mut project, package) = use_case_fixture();
    let base = project
        .create_element(ElementKind::UseCase, "Book trip", package)
        .unwrap();
    let extension = project
        .create_element(ElementKind::UseCase, "Apply promotion", package)
        .unwrap();
    let relationship = project
        .create_relationship(RelationshipKind::Extend, extension, base, Some(package))
        .unwrap();

    assert!(matches!(
        project.update_extend_relationship(
            relationship,
            Some("eligible".into()),
            Some("discount".into())
        ),
        Err(ModelError::ExtensionPointNotFound { .. })
    ));
    assert!(
        project
            .relationship(relationship)
            .unwrap()
            .extension_location
            .is_none()
    );
}
