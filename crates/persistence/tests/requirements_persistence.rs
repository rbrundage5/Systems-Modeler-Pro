use systems_modeler_core::{ElementKind, Project, RelationshipKind};
use systems_modeler_persistence::ProjectDatabase;

#[test]
fn requirements_and_traceability_round_trip_without_identity_changes() {
    let mut project = Project::new("Requirements");
    let package = project
        .create_element(ElementKind::Package, "Requirements", project.root_id)
        .unwrap();
    let requirement = project
        .create_requirement("Range", "REQ-001", "Range shall exceed 100 km.", package)
        .unwrap();
    let block = project
        .create_element(ElementKind::Block, "Vehicle", package)
        .unwrap();
    let relationship = project
        .create_relationship(RelationshipKind::Satisfy, block, requirement, Some(package))
        .unwrap();
    let external_id = project.element(requirement).unwrap().external_id.clone();

    let mut database = ProjectDatabase::open_in_memory().unwrap();
    database.save_project(&project).unwrap();
    let restored = database.load_project(project.id).unwrap();

    let restored_requirement = restored.element(requirement).unwrap();
    assert_eq!(restored_requirement.external_id, external_id);
    assert_eq!(
        restored_requirement.requirement_id.as_deref(),
        Some("REQ-001")
    );
    assert_eq!(
        restored_requirement.requirement_text.as_deref(),
        Some("Range shall exceed 100 km.")
    );
    assert_eq!(
        restored.relationship(relationship).unwrap().kind,
        RelationshipKind::Satisfy
    );
    restored.validate().unwrap();
}
