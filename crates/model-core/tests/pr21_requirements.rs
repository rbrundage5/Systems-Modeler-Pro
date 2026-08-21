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
        .create_requirement(
            "Brake response",
            "REQ-BRK-001",
            "The vehicle shall stop.",
            package,
        )
        .unwrap();
    let semantic_uuid = requirement;
    let external_id = project.element(requirement).unwrap().external_id.clone();
    project
        .update_requirement(
            requirement,
            "REQ-BRK-001A",
            "The vehicle shall stop safely.",
        )
        .unwrap();
    let updated = project.element(requirement).unwrap();
    assert_eq!(updated.id, semantic_uuid);
    assert_eq!(updated.external_id, external_id);
    assert_eq!(updated.requirement_id.as_deref(), Some("REQ-BRK-001A"));
    assert_eq!(
        updated.requirement_text.as_deref(),
        Some("The vehicle shall stop safely.")
    );
}

#[test]
fn traceability_endpoint_rules_are_enforced() {
    let (mut project, package) = requirement_project();
    let requirement = project
        .create_requirement("Stopping", "REQ-001", "Stop safely", package)
        .unwrap();
    let block = project
        .create_element(ElementKind::Block, "Brake", package)
        .unwrap();
    let test_case = project
        .create_element(ElementKind::TestCase, "Brake test", package)
        .unwrap();
    assert!(
        project
            .create_relationship(RelationshipKind::Satisfy, block, requirement, Some(package))
            .is_ok()
    );
    assert!(
        project
            .create_relationship(
                RelationshipKind::Verify,
                test_case,
                requirement,
                Some(package)
            )
            .is_ok()
    );
    assert!(matches!(
        project.create_relationship(RelationshipKind::Verify, block, requirement, Some(package)),
        Err(ModelError::InvalidTraceabilityEndpoints { .. })
    ));
}

#[test]
fn duplicate_human_readable_requirement_ids_are_rejected() {
    let (mut project, package) = requirement_project();
    project
        .create_requirement("First", "REQ-001", "One", package)
        .unwrap();
    assert_eq!(
        project.create_requirement("Second", "REQ-001", "Two", package),
        Err(ModelError::DuplicateRequirementId("REQ-001".into()))
    );
}

#[test]
fn copy_relationship_synchronizes_and_protects_slave_text() {
    let (mut project, package) = requirement_project();
    let master = project
        .create_requirement("Master", "REQ-MASTER", "Authoritative text", package)
        .unwrap();
    let slave = project
        .create_requirement("Copy", "REQ-COPY", "Stale text", package)
        .unwrap();

    project
        .create_relationship(RelationshipKind::Copy, slave, master, Some(package))
        .unwrap();
    assert_eq!(
        project.element(slave).unwrap().requirement_text.as_deref(),
        Some("Authoritative text")
    );
    assert_eq!(
        project.update_requirement(slave, "REQ-COPY", "Unauthorized edit"),
        Err(ModelError::CopiedRequirementIsReadOnly(slave))
    );

    project
        .update_requirement(master, "REQ-MASTER", "Revised authoritative text")
        .unwrap();
    assert_eq!(
        project.element(slave).unwrap().requirement_text.as_deref(),
        Some("Revised authoritative text")
    );
}

#[test]
fn every_requirement_relationship_enforces_endpoints_and_package_ownership() {
    let (mut project, package) = requirement_project();
    let requirement = project
        .create_requirement("Stopping", "REQ-001", "Stop safely", package)
        .unwrap();
    let derived = project
        .create_requirement("Derived", "REQ-002", "Derived stopping rule", package)
        .unwrap();
    let copied = project
        .create_requirement("Copy", "REQ-003", "Copy placeholder", package)
        .unwrap();
    let block = project
        .create_element(ElementKind::Block, "Brake", package)
        .unwrap();
    let test_case = project
        .create_element(ElementKind::TestCase, "Brake test", package)
        .unwrap();

    let relationships = [
        project
            .create_relationship(
                RelationshipKind::DeriveRequirement,
                derived,
                requirement,
                Some(package),
            )
            .unwrap(),
        project
            .create_relationship(RelationshipKind::Satisfy, block, requirement, Some(package))
            .unwrap(),
        project
            .create_relationship(
                RelationshipKind::Verify,
                test_case,
                requirement,
                Some(package),
            )
            .unwrap(),
        project
            .create_relationship(RelationshipKind::Refine, block, requirement, Some(package))
            .unwrap(),
        project
            .create_relationship(RelationshipKind::Refine, requirement, block, Some(package))
            .unwrap(),
        project
            .create_relationship(RelationshipKind::Trace, block, test_case, Some(package))
            .unwrap(),
        project
            .create_relationship(RelationshipKind::Copy, copied, requirement, Some(package))
            .unwrap(),
    ];
    assert!(relationships.iter().all(|id| {
        project.relationship(*id).unwrap().owner_id == Some(package)
    }));

    assert_eq!(
        project.create_relationship(RelationshipKind::Satisfy, block, requirement, Some(package)),
        Err(ModelError::DuplicateTraceabilityRelationship {
            relationship: RelationshipKind::Satisfy,
            source: block,
            target: requirement,
        })
    );
    assert_eq!(
        project.create_relationship(RelationshipKind::Trace, requirement, requirement, Some(package)),
        Err(ModelError::SelfTraceabilityRelationship)
    );
    assert_eq!(
        project.create_relationship(RelationshipKind::Trace, test_case, block, Some(block)),
        Err(ModelError::InvalidTraceabilityOwner(block))
    );
    assert!(matches!(
        project.create_relationship(RelationshipKind::Verify, block, requirement, Some(package)),
        Err(ModelError::InvalidTraceabilityEndpoints { .. })
    ));
    project.validate().unwrap();
}
