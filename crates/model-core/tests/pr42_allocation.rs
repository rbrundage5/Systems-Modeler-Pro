use systems_modeler_core::{ElementKind, ModelError, Project, Relationship, RelationshipKind};

#[test]
fn pr42_allocate_is_native_directional_serializable_and_has_no_requirement_side_effects() {
    let mut project = Project::new("Allocation");
    let package = project
        .create_element(ElementKind::Package, "Architecture", project.root_id)
        .unwrap();
    let controller = project
        .create_element(ElementKind::Block, "Controller", package)
        .unwrap();
    let requirement = project
        .create_element(ElementKind::Requirement, "Control Requirement", package)
        .unwrap();
    project
        .update_requirement(requirement, "REQ-42", "Controller shall provide control")
        .unwrap();

    let relationship_id = project
        .create_relationship(
            RelationshipKind::Allocate,
            controller,
            requirement,
            Some(package),
        )
        .unwrap();
    {
        let relationship = project.relationships.get_mut(&relationship_id).unwrap();
        relationship.external_id = "catia:pr42::ALLOC-1".into();
        relationship.name = "ControllerAllocation".into();
        relationship.documentation = "Explicit allocation".into();
    }

    let relationship = project.relationship(relationship_id).unwrap();
    assert_eq!(relationship.kind, RelationshipKind::Allocate);
    assert_eq!(relationship.source_id, controller);
    assert_eq!(relationship.target_id, requirement);
    assert_eq!(relationship.owner_id, Some(package));
    let encoded = serde_json::to_string(relationship).unwrap();
    let decoded: Relationship = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.kind, RelationshipKind::Allocate);
    assert_eq!(decoded.id, relationship_id);
    assert_eq!(decoded.external_id, "catia:pr42::ALLOC-1");
    assert_eq!(decoded.source_id, controller);
    assert_eq!(decoded.target_id, requirement);

    let requirement = project.element(requirement).unwrap();
    assert_eq!(requirement.requirement_id.as_deref(), Some("REQ-42"));
    assert_eq!(
        requirement.requirement_text.as_deref(),
        Some("Controller shall provide control")
    );
    project.validate().unwrap();
}

#[test]
fn pr42_allocate_rejects_self_duplicate_admin_endpoint_and_illegal_owner() {
    let mut project = Project::new("Allocation Validation");
    let package = project
        .create_element(ElementKind::Package, "Architecture", project.root_id)
        .unwrap();
    let a = project
        .create_element(ElementKind::Block, "A", package)
        .unwrap();
    let b = project
        .create_element(ElementKind::Block, "B", package)
        .unwrap();
    let note = project
        .create_element(ElementKind::Comment, "Administrative Note", package)
        .unwrap();

    assert!(matches!(
        project.create_relationship(RelationshipKind::Allocate, a, a, Some(package)),
        Err(ModelError::AllocationSelfReference)
    ));
    assert!(matches!(
        project.create_relationship(RelationshipKind::Allocate, note, b, Some(package)),
        Err(ModelError::InvalidAllocationEndpoints { .. })
    ));
    assert!(matches!(
        project.create_relationship(RelationshipKind::Allocate, a, b, None),
        Err(ModelError::MissingAllocationOwner)
    ));
    assert!(matches!(
        project.create_relationship(RelationshipKind::Allocate, a, b, Some(a)),
        Err(ModelError::InvalidAllocationOwner(id)) if id == a
    ));

    project
        .create_relationship(RelationshipKind::Allocate, a, b, Some(package))
        .unwrap();
    assert!(matches!(
        project.create_relationship(RelationshipKind::Allocate, a, b, Some(package)),
        Err(ModelError::DuplicateAllocationRelationship { source_id, target_id })
            if source_id == a && target_id == b
    ));
}
