use systems_modeler_core::*;

fn fixture() -> (Project, ElementId, ElementId) {
    let mut project = Project::new("PR61 copied Requirement reimport");
    let package = project
        .create_element(ElementKind::Package, "Requirements", project.root_id)
        .unwrap();
    let master = project
        .create_requirement(
            "MasterRequirement",
            "REQ-MASTER",
            "The vehicle shall provide controlled propulsion.",
            package,
        )
        .unwrap();
    let copy = project
        .create_requirement(
            "VerificationCopy",
            "REQ-COPY",
            "The vehicle shall provide controlled propulsion.",
            package,
        )
        .unwrap();
    project
        .create_relationship(RelationshipKind::Copy, copy, master, Some(package))
        .unwrap();
    project.validate().unwrap();
    (project, master, copy)
}

#[test]
fn identical_reassertion_of_copied_requirement_is_idempotent() {
    let (mut project, _master, copy) = fixture();
    project
        .update_requirement(
            copy,
            "REQ-COPY",
            "The vehicle shall provide controlled propulsion.",
        )
        .unwrap();
    project.validate().unwrap();
    let copied = project.element(copy).unwrap();
    assert_eq!(copied.requirement_id.as_deref(), Some("REQ-COPY"));
    assert_eq!(
        copied.requirement_text.as_deref(),
        Some("The vehicle shall provide controlled propulsion.")
    );
}

#[test]
fn master_update_then_copy_row_reassertion_is_safe() {
    let (mut project, master, copy) = fixture();
    project
        .update_requirement(
            master,
            "REQ-MASTER",
            "The vehicle shall provide controlled propulsion in all commanded modes.",
        )
        .unwrap();
    assert_eq!(
        project.element(copy).unwrap().requirement_text.as_deref(),
        Some("The vehicle shall provide controlled propulsion in all commanded modes.")
    );
    project
        .update_requirement(
            copy,
            "REQ-COPY",
            "The vehicle shall provide controlled propulsion in all commanded modes.",
        )
        .unwrap();
    project.validate().unwrap();
}

#[test]
fn copied_requirement_still_rejects_real_semantic_change() {
    let (mut project, _master, copy) = fixture();
    let error = project
        .update_requirement(copy, "REQ-COPY", "Unauthorized divergent copy text")
        .unwrap_err();
    assert!(matches!(
        error,
        ModelError::CopiedRequirementIsReadOnly(id) if id == copy
    ));
}

#[test]
fn master_updates_reach_transitive_copies_and_keep_local_identity() {
    let (mut project, master, copy) = fixture();
    let package = project.element(master).unwrap().owner_id.unwrap();
    let leaf = project
        .create_requirement("Site copy", "REQ-SITE", "Placeholder", package)
        .unwrap();
    let branch = project
        .create_requirement("Other copy", "REQ-OTHER", "Placeholder", package)
        .unwrap();
    project
        .create_relationship(RelationshipKind::Copy, leaf, copy, Some(package))
        .unwrap();
    project
        .create_relationship(RelationshipKind::Copy, branch, master, Some(package))
        .unwrap();
    project.element_mut(leaf).unwrap().documentation = "Local verification context".into();
    let identity = project.element(leaf).unwrap().clone();
    project
        .update_requirement(master, "REQ-MASTER", "Revised supplier text")
        .unwrap();
    for id in [master, copy, leaf, branch] {
        assert_eq!(
            project.element(id).unwrap().requirement_text.as_deref(),
            Some("Revised supplier text")
        );
    }
    let updated = project.element(leaf).unwrap();
    assert_eq!(updated.external_id, identity.external_id);
    assert_eq!(updated.name, identity.name);
    assert_eq!(updated.owner_id, identity.owner_id);
    assert_eq!(updated.requirement_id, identity.requirement_id);
    assert_eq!(updated.documentation, identity.documentation);
    project
        .update_requirement(leaf, "REQ-SITE", "Revised supplier text")
        .unwrap();
    project.validate().unwrap();
}

#[test]
fn attaching_an_existing_copy_subtree_synchronizes_every_descendant() {
    let (mut project, master, _) = fixture();
    let package = project.element(master).unwrap().owner_id.unwrap();
    let local = project
        .create_requirement("Local", "REQ-LOCAL", "Old local text", package)
        .unwrap();
    let leaf = project
        .create_requirement("Leaf", "REQ-LEAF", "Placeholder", package)
        .unwrap();
    project
        .create_relationship(RelationshipKind::Copy, leaf, local, Some(package))
        .unwrap();
    project
        .create_relationship(RelationshipKind::Copy, local, master, Some(package))
        .unwrap();
    assert_eq!(
        project.element(leaf).unwrap().requirement_text,
        project.element(master).unwrap().requirement_text
    );
    let reopened: Project =
        serde_json::from_value(serde_json::to_value(&project).unwrap()).unwrap();
    reopened.validate().unwrap();
    assert_eq!(
        reopened.element(leaf).unwrap().requirement_text,
        reopened.element(master).unwrap().requirement_text
    );
}

#[test]
fn conflicting_copy_suppliers_reject_creation_and_master_edits_atomically() {
    let (mut project, master, copy) = fixture();
    let package = project.element(master).unwrap().owner_id.unwrap();
    let other = project
        .create_requirement("Other supplier", "REQ-OTHER", "Different text", package)
        .unwrap();
    let before = serde_json::to_value(&project).unwrap();
    assert_eq!(
        project.create_relationship(RelationshipKind::Copy, copy, other, Some(package)),
        Err(ModelError::ConflictingRequirementCopyText(copy))
    );
    assert_eq!(serde_json::to_value(&project).unwrap(), before);
    let text = project
        .element(master)
        .unwrap()
        .requirement_text
        .clone()
        .unwrap();
    project
        .update_requirement(other, "REQ-OTHER", text)
        .unwrap();
    project
        .create_relationship(RelationshipKind::Copy, copy, other, Some(package))
        .unwrap();
    let before = serde_json::to_value(&project).unwrap();
    assert_eq!(
        project.update_requirement(master, "REQ-MASTER", "Divergent revision"),
        Err(ModelError::ConflictingRequirementCopyText(copy))
    );
    assert_eq!(serde_json::to_value(&project).unwrap(), before);
}

#[test]
fn missing_downstream_copy_rejects_before_mutating_master_text() {
    let (mut project, master, copy) = fixture();
    project.elements.remove(&copy);
    let before = serde_json::to_value(&project).unwrap();
    assert!(
        project
            .update_requirement(master, "REQ-MASTER", "Must not be partially applied")
            .is_err()
    );
    assert_eq!(serde_json::to_value(&project).unwrap(), before);
}
