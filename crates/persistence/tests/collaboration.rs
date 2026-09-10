use systems_modeler_core::{DiagramId, ElementId, Project};
use systems_modeler_persistence::collaboration::{
    CollaborationError, EditRequest, ProjectRole, SharedEdit,
};
use systems_modeler_persistence::{PersistenceError, ProjectDatabase};
use uuid::Uuid;

fn create(root: ElementId, revision: i64) -> EditRequest {
    EditRequest {
        operation_id: Uuid::new_v4(),
        expected_revision: revision,
        edit: SharedEdit::CreateBlock {
            owner: root,
            name: "Engine".into(),
        },
    }
}

fn edit(revision: i64, edit: SharedEdit) -> EditRequest {
    EditRequest {
        operation_id: Uuid::new_v4(),
        expected_revision: revision,
        edit,
    }
}

#[test]
fn commits_survive_reopen_and_retries_do_not_duplicate() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shared.sqlite");
    let project = Project::new("Shared");
    let actor = Uuid::new_v4();
    let request = create(project.root_id, 0);
    let receipt = {
        let mut db = ProjectDatabase::open(&path).unwrap();
        db.save_project(&project).unwrap();
        db.provision_shared_member(project.id, actor, ProjectRole::Editor)
            .unwrap();
        db.commit_shared_edit(project.id, actor, &request).unwrap()
    };
    let db = ProjectDatabase::open(&path).unwrap();
    assert_eq!(
        db.commit_shared_edit(project.id, actor, &request).unwrap(),
        receipt
    );
    let (loaded, diagrams, revision) = db.shared_snapshot(project.id, actor).unwrap();
    assert_eq!(revision, 1);
    assert!(diagrams.is_empty());
    assert_eq!(loaded.elements.len(), 2);
    assert_eq!(loaded.element(receipt.element).unwrap().name, "Engine");
    let mut altered = request;
    altered.expected_revision = 1;
    assert!(matches!(
        db.commit_shared_edit(project.id, actor, &altered),
        Err(CollaborationError::OperationIdReused)
    ));
}

#[test]
fn stale_clients_cannot_overwrite_and_can_resynchronize() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shared.sqlite");
    let project = Project::new("Shared");
    let actor = Uuid::new_v4();
    let mut first = ProjectDatabase::open(&path).unwrap();
    first.save_project(&project).unwrap();
    first
        .provision_shared_member(project.id, actor, ProjectRole::Editor)
        .unwrap();
    let second = ProjectDatabase::open(&path).unwrap();
    first
        .commit_shared_edit(project.id, actor, &create(project.root_id, 0))
        .unwrap();
    assert!(matches!(
        second.commit_shared_edit(project.id, actor, &create(project.root_id, 0)),
        Err(CollaborationError::Conflict { current: 1, .. })
    ));
    let (_, _, revision) = second.shared_snapshot(project.id, actor).unwrap();
    second
        .commit_shared_edit(project.id, actor, &create(project.root_id, revision))
        .unwrap();
    assert_eq!(first.shared_snapshot(project.id, actor).unwrap().2, 2);
}

#[test]
fn permissions_invalid_edits_and_legacy_save_preserve_state() {
    let project = Project::new("Shared");
    let editor = Uuid::new_v4();
    let viewer = Uuid::new_v4();
    let outsider = Uuid::new_v4();
    let mut db = ProjectDatabase::open_in_memory().unwrap();
    db.save_project(&project).unwrap();
    db.provision_shared_member(project.id, editor, ProjectRole::Editor)
        .unwrap();
    db.provision_shared_member(project.id, viewer, ProjectRole::Viewer)
        .unwrap();
    assert!(db.shared_snapshot(project.id, viewer).is_ok());
    assert!(matches!(
        db.shared_snapshot(project.id, outsider),
        Err(CollaborationError::Forbidden)
    ));
    for actor in [viewer, outsider] {
        assert!(matches!(
            db.commit_shared_edit(project.id, actor, &create(project.root_id, 0)),
            Err(CollaborationError::Forbidden)
        ));
    }
    assert!(
        db.commit_shared_edit(project.id, editor, &create(ElementId::new(), 0))
            .is_err()
    );
    assert!(matches!(
        db.save_project(&project),
        Err(PersistenceError::SharedProject)
    ));
    let (loaded, diagrams, revision) = db.shared_snapshot(project.id, editor).unwrap();
    assert_eq!(revision, 0);
    assert!(diagrams.is_empty());
    assert_eq!(loaded.elements.len(), 1);
    let receipt = db
        .commit_shared_edit(project.id, editor, &create(project.root_id, 0))
        .unwrap();
    let rename = EditRequest {
        operation_id: Uuid::new_v4(),
        expected_revision: 1,
        edit: SharedEdit::RenameElement {
            element: receipt.element,
            name: "Motor".into(),
        },
    };
    db.commit_shared_edit(project.id, editor, &rename).unwrap();
    assert_eq!(
        db.shared_snapshot(project.id, viewer)
            .unwrap()
            .0
            .element(receipt.element)
            .unwrap()
            .name,
        "Motor"
    );
    db.provision_shared_member(project.id, editor, ProjectRole::Viewer)
        .unwrap();
    assert!(matches!(
        db.commit_shared_edit(project.id, editor, &rename),
        Err(CollaborationError::Forbidden)
    ));
}

#[test]
fn shared_bdd_node_edits_share_the_project_revision_and_survive_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shared-bdd.sqlite");
    let project = Project::new("Shared");
    let actor = Uuid::new_v4();
    let mut db = ProjectDatabase::open(&path).unwrap();
    db.save_project(&project).unwrap();
    db.provision_shared_member(project.id, actor, ProjectRole::Editor)
        .unwrap();

    let block = db
        .commit_shared_edit(project.id, actor, &create(project.root_id, 0))
        .unwrap()
        .element;
    let diagram = DiagramId::new();
    db.commit_shared_edit(
        project.id,
        actor,
        &edit(
            1,
            SharedEdit::CreateBddDiagram {
                diagram,
                owner: project.root_id,
                name: "Structure".into(),
            },
        ),
    )
    .unwrap();
    let node = Uuid::new_v4();
    db.commit_shared_edit(
        project.id,
        actor,
        &edit(
            2,
            SharedEdit::PlaceBddElement {
                diagram,
                node,
                element: block,
            },
        ),
    )
    .unwrap();
    db.commit_shared_edit(
        project.id,
        actor,
        &edit(
            3,
            SharedEdit::UpdateBddNodeGeometry {
                diagram,
                node,
                x: 320.0,
                y: 240.0,
                width: 210.0,
                height: 130.0,
            },
        ),
    )
    .unwrap();
    db.commit_shared_edit(
        project.id,
        actor,
        &edit(
            4,
            SharedEdit::RenameBddDiagram {
                diagram,
                name: "Vehicle Structure".into(),
            },
        ),
    )
    .unwrap();
    drop(db);

    let db = ProjectDatabase::open(&path).unwrap();
    let (_, diagrams, revision) = db.shared_snapshot(project.id, actor).unwrap();
    assert_eq!(revision, 5);
    assert_eq!(diagrams.len(), 1);
    assert_eq!(diagrams[0].id, diagram);
    assert_eq!(diagrams[0].name, "Vehicle Structure");
    assert_eq!(diagrams[0].nodes.len(), 1);
    assert_eq!(diagrams[0].nodes[0].id, node);
    assert_eq!(diagrams[0].nodes[0].element, block);
    assert_eq!(diagrams[0].nodes[0].x, 320.0);

    assert!(matches!(
        db.commit_shared_edit(
            project.id,
            actor,
            &edit(
                5,
                SharedEdit::PlaceBddElement {
                    diagram,
                    node: Uuid::new_v4(),
                    element: block,
                },
            ),
        ),
        Err(CollaborationError::InvalidDiagram(_))
    ));
    assert_eq!(db.shared_snapshot(project.id, actor).unwrap().2, 5);
}

#[test]
fn failed_receipt_write_rolls_back_model_and_revision() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shared.sqlite");
    let project = Project::new("Shared");
    let actor = Uuid::new_v4();
    let mut db = ProjectDatabase::open(&path).unwrap();
    db.save_project(&project).unwrap();
    db.provision_shared_member(project.id, actor, ProjectRole::Editor)
        .unwrap();
    let fault = rusqlite::Connection::open(&path).unwrap();
    fault.execute_batch("CREATE TRIGGER fail_receipt BEFORE INSERT ON shared_operations BEGIN SELECT RAISE(ABORT, 'injected receipt failure'); END;").unwrap();
    let request = create(project.root_id, 0);
    assert!(db.commit_shared_edit(project.id, actor, &request).is_err());
    let (loaded, diagrams, revision) = db.shared_snapshot(project.id, actor).unwrap();
    assert_eq!(revision, 0);
    assert!(diagrams.is_empty());
    assert_eq!(loaded.elements.len(), 1);
    fault.execute_batch("DROP TRIGGER fail_receipt").unwrap();
    let receipt = db.commit_shared_edit(project.id, actor, &request).unwrap();
    assert_eq!(receipt.revision, 1);
}
