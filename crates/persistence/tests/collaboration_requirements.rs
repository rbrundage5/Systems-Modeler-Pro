use systems_modeler_core::{ElementId, ElementKind, Project, RelationshipKind};
use systems_modeler_persistence::ProjectDatabase;
use systems_modeler_persistence::collaboration::{
    CollaborationError, EditRequest, ProjectRole, SharedEdit, SharedRelationshipKind,
};
use uuid::Uuid;

fn request(revision: i64, edit: SharedEdit) -> EditRequest {
    EditRequest {
        operation_id: Uuid::new_v4(),
        expected_revision: revision,
        edit,
    }
}

fn requirement(owner: ElementId, id: &str) -> SharedEdit {
    SharedEdit::CreateRequirement {
        owner,
        name: "Response time".into(),
        requirement_id: id.into(),
        text: "The controller shall respond within 50 ms.".into(),
    }
}

fn update(element: ElementId, id: &str, text: &str) -> SharedEdit {
    SharedEdit::UpdateRequirement {
        element,
        name: "Updated requirement".into(),
        requirement_id: id.into(),
        text: text.into(),
    }
}

#[test]
fn shared_requirements_keep_identity_traceability_and_retry_receipts_after_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("requirements.sqlite");
    let mut db = ProjectDatabase::open(&path).unwrap();
    let project = Project::new("Shared requirements");
    let actor = Uuid::new_v4();
    db.save_project(&project).unwrap();
    db.provision_shared_member(project.id, actor, ProjectRole::Editor)
        .unwrap();
    let create = request(0, requirement(project.root_id, "REQ-1"));
    let created = db.commit_shared_edit(project.id, actor, &create).unwrap();
    assert_eq!(
        db.commit_shared_edit(project.id, actor, &create).unwrap(),
        created
    );
    let test = db
        .commit_shared_edit(
            project.id,
            actor,
            &request(
                1,
                SharedEdit::CreateTestCase {
                    owner: project.root_id,
                    name: "Response timing test".into(),
                },
            ),
        )
        .unwrap();
    db.commit_shared_edit(
        project.id,
        actor,
        &request(
            2,
            SharedEdit::CreateRelationship {
                kind: SharedRelationshipKind::Verify,
                source: test.element,
                target: created.element,
                owner: project.root_id,
            },
        ),
    )
    .unwrap();
    let change = request(
        3,
        update(created.element, "REQ-1A", "Respond within 40 ms."),
    );
    let updated = db.commit_shared_edit(project.id, actor, &change).unwrap();
    assert_eq!(updated.element, created.element);
    drop(db);

    let db = ProjectDatabase::open(&path).unwrap();
    assert_eq!(
        db.commit_shared_edit(project.id, actor, &change).unwrap(),
        updated
    );
    let (model, _, revision) = db.shared_snapshot(project.id, actor).unwrap();
    assert_eq!(revision, 4);
    assert_eq!(model.elements.len(), 3);
    let saved = model.element(created.element).unwrap();
    assert_eq!(saved.name, "Updated requirement");
    assert_eq!(saved.requirement_id.as_deref(), Some("REQ-1A"));
    assert_eq!(
        saved.requirement_text.as_deref(),
        Some("Respond within 40 ms.")
    );
    let verification = model.relationships.values().next().unwrap();
    assert_eq!(verification.kind, RelationshipKind::Verify);
    assert_eq!(verification.source_id, test.element);
    assert_eq!(verification.target_id, created.element);
    model.validate().unwrap();
}

#[test]
fn invalid_requirement_changes_preserve_model_revision_and_operation_identity() {
    let mut db = ProjectDatabase::open_in_memory().unwrap();
    let mut project = Project::new("Shared requirements");
    let block = project
        .create_element(ElementKind::Block, "Invalid owner", project.root_id)
        .unwrap();
    let actor = Uuid::new_v4();
    let viewer = Uuid::new_v4();
    db.save_project(&project).unwrap();
    db.provision_shared_member(project.id, actor, ProjectRole::Editor)
        .unwrap();
    db.provision_shared_member(project.id, viewer, ProjectRole::Viewer)
        .unwrap();
    let first = db
        .commit_shared_edit(
            project.id,
            actor,
            &request(0, requirement(project.root_id, "REQ-1")),
        )
        .unwrap();
    db.commit_shared_edit(
        project.id,
        actor,
        &request(1, requirement(project.root_id, "REQ-2")),
    )
    .unwrap();
    let before = serde_json::to_value(db.shared_snapshot(project.id, actor).unwrap()).unwrap();
    let invalid = [
        requirement(project.root_id, "REQ-1"),
        requirement(project.root_id, "   "),
        requirement(block, "REQ-3"),
        update(first.element, "REQ-2", "Must not replace original text."),
        update(project.root_id, "REQ-3", "Must not change a Model."),
        SharedEdit::UpdateRequirement {
            element: first.element,
            name: " ".into(),
            requirement_id: "REQ-1A".into(),
            text: "Must not change the ID or text.".into(),
        },
    ];
    for edit in invalid {
        assert!(
            db.commit_shared_edit(project.id, actor, &request(2, edit))
                .is_err()
        );
        assert_eq!(
            serde_json::to_value(db.shared_snapshot(project.id, actor).unwrap()).unwrap(),
            before
        );
    }
    let valid = request(2, update(first.element, "REQ-1A", "Approved revision."));
    assert!(matches!(
        db.commit_shared_edit(project.id, viewer, &valid),
        Err(CollaborationError::Forbidden)
    ));
    let mut stale = valid.clone();
    stale.expected_revision = 1;
    assert!(matches!(
        db.commit_shared_edit(project.id, actor, &stale),
        Err(CollaborationError::Conflict { .. })
    ));
    assert_eq!(
        serde_json::to_value(db.shared_snapshot(project.id, actor).unwrap()).unwrap(),
        before
    );
    // Rejection must not consume the operation ID or leave a partial update.
    let mut rejected = valid.clone();
    rejected.edit = update(first.element, "REQ-2", "Rejected duplicate.");
    assert!(db.commit_shared_edit(project.id, actor, &rejected).is_err());
    assert_eq!(
        db.commit_shared_edit(project.id, actor, &valid)
            .unwrap()
            .revision,
        3
    );
}

#[test]
fn nested_shared_creation_preserves_parent_identity_and_retry_receipts() {
    let mut db = ProjectDatabase::open_in_memory().unwrap();
    let project = Project::new("Nested shared requirements");
    let actor = Uuid::new_v4();
    db.save_project(&project).unwrap();
    db.provision_shared_member(project.id, actor, ProjectRole::Editor)
        .unwrap();
    let parent = db
        .commit_shared_edit(
            project.id,
            actor,
            &request(0, requirement(project.root_id, "REQ-1")),
        )
        .unwrap();
    let create = request(1, requirement(parent.element, "REQ-1.1"));
    let child = db.commit_shared_edit(project.id, actor, &create).unwrap();
    assert_eq!(
        db.commit_shared_edit(project.id, actor, &create).unwrap(),
        child
    );
    let (model, _, revision) = db.shared_snapshot(project.id, actor).unwrap();
    assert_eq!(revision, 2);
    assert_eq!(
        model.element(child.element).unwrap().owner_id,
        Some(parent.element)
    );
    model.validate().unwrap();
}

#[test]
fn shared_copy_protects_slave_text_and_propagates_supplier_updates_atomically() {
    let mut db = ProjectDatabase::open_in_memory().unwrap();
    let project = Project::new("Shared requirements");
    let actor = Uuid::new_v4();
    db.save_project(&project).unwrap();
    db.provision_shared_member(project.id, actor, ProjectRole::Editor)
        .unwrap();
    let master = db
        .commit_shared_edit(
            project.id,
            actor,
            &request(0, requirement(project.root_id, "REQ-M")),
        )
        .unwrap()
        .element;
    let slave = db
        .commit_shared_edit(
            project.id,
            actor,
            &request(1, requirement(project.root_id, "REQ-S")),
        )
        .unwrap()
        .element;
    db.commit_shared_edit(
        project.id,
        actor,
        &request(
            2,
            SharedEdit::CreateRelationship {
                kind: SharedRelationshipKind::Copy,
                source: slave,
                target: master,
                owner: project.root_id,
            },
        ),
    )
    .unwrap();
    let before = serde_json::to_value(db.shared_snapshot(project.id, actor).unwrap()).unwrap();
    assert!(
        db.commit_shared_edit(
            project.id,
            actor,
            &request(3, update(slave, "REQ-S", "Forbidden."))
        )
        .is_err()
    );
    assert_eq!(
        serde_json::to_value(db.shared_snapshot(project.id, actor).unwrap()).unwrap(),
        before
    );
    db.commit_shared_edit(
        project.id,
        actor,
        &request(3, update(master, "REQ-M", "Updated supplier text.")),
    )
    .unwrap();
    let (model, _, revision) = db.shared_snapshot(project.id, actor).unwrap();
    assert_eq!(revision, 4);
    assert_eq!(
        model.element(slave).unwrap().requirement_text.as_deref(),
        Some("Updated supplier text.")
    );
}
