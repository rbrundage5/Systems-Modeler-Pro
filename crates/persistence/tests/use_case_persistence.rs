use systems_modeler_core::{ElementKind, Project, RelationshipKind};
use systems_modeler_persistence::ProjectDatabase;

#[test]
fn use_case_semantics_round_trip_with_stable_identity_and_direction() {
    let mut project = Project::new("Use Case persistence");
    let package = project
        .create_element(ElementKind::Package, "Operations", project.root_id)
        .unwrap();
    let system = project
        .create_element(ElementKind::Block, "Reservation System", package)
        .unwrap();
    let base = project
        .create_element(ElementKind::UseCase, "Book trip", package)
        .unwrap();
    let extension = project
        .create_element(ElementKind::UseCase, "Apply promotion", package)
        .unwrap();
    project
        .update_use_case(
            base,
            "Book a valid itinerary",
            vec!["discount".into()],
            Some(system),
        )
        .unwrap();
    let extend = project
        .create_relationship(RelationshipKind::Extend, extension, base, Some(package))
        .unwrap();
    project
        .update_extend_relationship(
            extend,
            Some("traveler is eligible".into()),
            Some("discount".into()),
        )
        .unwrap();
    project.validate().unwrap();

    let mut database = ProjectDatabase::open_in_memory().unwrap();
    database.save_project(&project).unwrap();
    let restored = database.load_project(project.id).unwrap();
    restored.validate().unwrap();

    let restored_base = restored.element(base).unwrap();
    assert_eq!(restored_base.represented_classifier_id, Some(system));
    assert_eq!(restored_base.extension_points, ["discount"]);
    assert_eq!(restored_base.use_case_specification, "Book a valid itinerary");
    let restored_extend = restored.relationship(extend).unwrap();
    assert_eq!(restored_extend.source_id, extension);
    assert_eq!(restored_extend.target_id, base);
    assert_eq!(restored_extend.extension_location.as_deref(), Some("discount"));
    assert_eq!(
        restored_extend.extension_condition.as_deref(),
        Some("traveler is eligible")
    );
}
