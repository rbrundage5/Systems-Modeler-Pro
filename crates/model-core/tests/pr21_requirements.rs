use systems_modeler_core::{ElementKind, ModelError, Project, RelationshipKind};

fn requirement_project() -> (Project, systems_modeler_core::ElementId) {
    let mut project = Project::new("Vehicle Requirements");
    let package = project
        .create_element(ElementKind::Package, "Requirements", project.root_id)
        .unwrap();
    (project, package)
}

#[test]
fn requirement_identity_id_and_text_are_distinct() {
    let (mut project, package) = requirement_project();
    let requirement = project
        .create_requirement("Brake response", "REQ-BRK-001", "The vehicle shall stop.", package)
        .unwrap();
    let semantic_uuid = requirement;
    let external_id = project.element(requirement).unwrap().external_id.clone();
    project
        .update_requirement(requirement, "REQ-BRK-001A", "The vehicle shall stop safely.")
        .unwrap();
    let updated = project.element(requirement).unwrap();
    assert_eq!(updated.id, semantic_uuid);
    assert_eq!(updated.external_id, external_id);
    assert_eq!(updated.requirement_id.as_deref(), Some("REQ-BRK-001A"));
    assert_eq!(updated.requirement_text.as_deref(), Some("The vehicle shall stop safely."));
}

#[test]
fn traceability_endpoint_rules_are_enforced() {
    let (mut project, package) = requirement_project();
    let requirement = project
        .create_requirement("Stopping", "REQ-001", "Stop safely", package)
        .unwrap();
    let block = project.create_element(ElementKind::Block, "Brake", package).unwrap();
    let test_case = project.create_element(ElementKind::TestCase, "Brake test", package).unwrap();
    assert!(project.create_relationship(RelationshipKind::Satisfy, block, requirement, Some(package)).is_ok());
    assert!(project.create_relationship(RelationshipKind::Verify, test_case, requirement, Some(package)).is_ok());
    assert!(matches!(
        project.create_relationship(RelationshipKind::Verify, block, requirement, Some(package)),
        Err(ModelError::InvalidTraceabilityEndpoints { .. })
    ));
}

#[test]
fn duplicate_human_readable_requirement_ids_are_rejected() {
    let (mut project, package) = requirement_project();
    project.create_requirement("First", "REQ-001", "One", package).unwrap();
    assert_eq!(
        project.create_requirement("Second", "REQ-001", "Two", package),
        Err(ModelError::DuplicateRequirementId("REQ-001".into()))
    );
}
