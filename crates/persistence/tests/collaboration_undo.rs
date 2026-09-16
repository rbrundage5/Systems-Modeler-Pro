use systems_modeler_core::{DiagramId, ElementKind, Project, ProjectId, RelationshipKind};
use systems_modeler_persistence::{ProjectDatabase, collaboration::{CollaborationError, EditRequest, ProjectRole, SharedEdit}};
use uuid::Uuid;

fn request(revision: i64, edit: SharedEdit) -> EditRequest {
    EditRequest { operation_id: Uuid::new_v4(), expected_revision: revision, edit }
}

fn undo(revision: i64, operation: Uuid) -> EditRequest {
    request(revision, SharedEdit::UndoOperation { operation })
}

fn member(database: &mut ProjectDatabase, project: ProjectId) -> Uuid {
    let actor = Uuid::new_v4();
    database.provision_shared_member(project, actor, ProjectRole::Editor).unwrap();
    actor
}

fn snapshot(database: &ProjectDatabase, project: ProjectId, actor: Uuid) -> serde_json::Value {
    serde_json::to_value(database.shared_snapshot(project, actor).unwrap()).unwrap()
}

#[test]
fn reversal_preserves_other_users_work_and_retries_after_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("shared.sqlite");
    let mut database = ProjectDatabase::open(&path).unwrap();
    let project = Project::new("Shared");
    database.save_project(&project).unwrap();
    let alice = member(&mut database, project.id);
    let bob = member(&mut database, project.id);
    let first = request(0, SharedEdit::CreateBlock { owner: project.root_id, name: "Alice block".into() });
    let alice_block = database.commit_shared_edit(project.id, alice, &first).unwrap().element;
    let bob_block = database.commit_shared_edit(project.id, bob, &request(1, SharedEdit::CreateBlock { owner: project.root_id, name: "Bob block".into() })).unwrap().element;
    let reverse = undo(2, first.operation_id);
    assert_eq!(database.commit_shared_edit(project.id, alice, &reverse).unwrap().revision, 3);
    let (model, _, revision) = database.shared_snapshot(project.id, alice).unwrap();
    assert_eq!(revision, 3);
    assert!(!model.elements.contains_key(&alice_block));
    assert!(model.elements.contains_key(&bob_block));
    let history = database.shared_history(project.id, alice).unwrap();
    assert_eq!(history.len(), 2);
    assert!(history[0].can_undo);
    assert!(!history[1].can_undo);
    let before = snapshot(&database, project.id, alice);
    drop(database);
    let database = ProjectDatabase::open(&path).unwrap();
    assert_eq!(database.commit_shared_edit(project.id, alice, &reverse).unwrap().revision, 3);
    assert_eq!(snapshot(&database, project.id, alice), before);
    database.commit_shared_edit(project.id, alice, &undo(3, reverse.operation_id)).unwrap();
    let (model, _, revision) = database.shared_snapshot(project.id, alice).unwrap();
    assert_eq!(revision, 4);
    assert_eq!(model.elements[&alice_block].name, "Alice block");
    assert_eq!(model.elements[&bob_block].name, "Bob block");
    assert!(matches!(database.commit_shared_edit(project.id, alice, &undo(4, first.operation_id)), Err(CollaborationError::UndoUnavailable)));
    // Operations written before inverse capture remain inspectable, without
    // inventing an unsafe inverse from the current model.
    rusqlite::Connection::open(&path).unwrap().execute("DELETE FROM shared_undo", []).unwrap();
    assert!(database.shared_history(project.id, alice).unwrap().iter().all(|entry| !entry.can_undo));
}

#[test]
fn changed_objects_other_authors_and_viewers_cannot_be_overwritten() {
    let mut database = ProjectDatabase::open_in_memory().unwrap();
    let mut project = Project::new("Shared");
    let block = project.create_element(ElementKind::Block, "Original", project.root_id).unwrap();
    database.save_project(&project).unwrap();
    let alice = member(&mut database, project.id);
    let bob = member(&mut database, project.id);
    let viewer = Uuid::new_v4();
    database.provision_shared_member(project.id, viewer, ProjectRole::Viewer).unwrap();
    let change = request(0, SharedEdit::RenameElement { element: block, name: "Alice name".into() });
    database.commit_shared_edit(project.id, alice, &change).unwrap();
    database.commit_shared_edit(project.id, bob, &request(1, SharedEdit::RenameElement { element: block, name: "Bob name".into() })).unwrap();
    let before = snapshot(&database, project.id, alice);
    assert!(matches!(database.commit_shared_edit(project.id, alice, &undo(2, change.operation_id)), Err(CollaborationError::UndoConflict)));
    assert!(matches!(database.commit_shared_edit(project.id, bob, &undo(2, change.operation_id)), Err(CollaborationError::Forbidden)));
    assert!(matches!(database.commit_shared_edit(project.id, viewer, &undo(2, change.operation_id)), Err(CollaborationError::Forbidden)));
    assert_eq!(snapshot(&database, project.id, alice), before);
    assert_eq!(database.shared_history(project.id, alice).unwrap().len(), 1);
    assert_eq!(database.shared_history(project.id, bob).unwrap().len(), 1);
    assert!(database.shared_history(project.id, viewer).unwrap().is_empty());
}

#[test]
fn new_dependencies_block_reversal_until_the_author_removes_them() {
    let mut database = ProjectDatabase::open_in_memory().unwrap();
    let project = Project::new("Shared");
    database.save_project(&project).unwrap();
    let alice = member(&mut database, project.id);
    let bob = member(&mut database, project.id);
    let package = request(0, SharedEdit::CreatePackage { owner: project.root_id, name: "Design".into() });
    let owner = database.commit_shared_edit(project.id, alice, &package).unwrap().element;
    let child = request(1, SharedEdit::CreateBlock { owner, name: "Component".into() });
    database.commit_shared_edit(project.id, bob, &child).unwrap();
    let before = snapshot(&database, project.id, alice);
    assert!(matches!(database.commit_shared_edit(project.id, alice, &undo(2, package.operation_id)), Err(CollaborationError::UndoConflict)));
    assert_eq!(snapshot(&database, project.id, alice), before);
    database.commit_shared_edit(project.id, bob, &undo(2, child.operation_id)).unwrap();
    database.commit_shared_edit(project.id, alice, &undo(3, package.operation_id)).unwrap();
    assert_eq!(database.shared_snapshot(project.id, alice).unwrap().0.elements.len(), 1);
}

#[test]
fn diagram_node_and_edge_restore_together_without_removing_another_diagram() {
    let mut database = ProjectDatabase::open_in_memory().unwrap();
    let mut project = Project::new("Shared");
    let source = project.create_element(ElementKind::Block, "Source", project.root_id).unwrap();
    let target = project.create_element(ElementKind::Block, "Target", project.root_id).unwrap();
    let relationship = project.create_relationship(RelationshipKind::Dependency, source, target, Some(project.root_id)).unwrap();
    database.save_project(&project).unwrap();
    let alice = member(&mut database, project.id);
    let bob = member(&mut database, project.id);
    let diagram = DiagramId(Uuid::new_v4());
    let source_node = Uuid::new_v4();
    let target_node = Uuid::new_v4();
    let edits = vec![
        SharedEdit::CreateBddDiagram { diagram, owner: project.root_id, name: "Structure".into() },
        SharedEdit::PlaceBddElement { diagram, node: source_node, element: source },
        SharedEdit::PlaceBddElement { diagram, node: target_node, element: target },
        SharedEdit::PresentBddRelationship { diagram, edge: Uuid::new_v4(), relationship },
    ];
    for (revision, edit) in edits.into_iter().enumerate() {
        database.commit_shared_edit(project.id, alice, &request(revision as i64, edit)).unwrap();
    }
    let original = database.shared_snapshot(project.id, alice).unwrap().1.remove(0);
    let removal = request(4, SharedEdit::RemoveBddNode { diagram, node: source_node });
    database.commit_shared_edit(project.id, alice, &removal).unwrap();
    let other = DiagramId(Uuid::new_v4());
    database.commit_shared_edit(project.id, bob, &request(5, SharedEdit::CreateBddDiagram { diagram: other, owner: project.root_id, name: "Bob diagram".into() })).unwrap();
    database.commit_shared_edit(project.id, alice, &undo(6, removal.operation_id)).unwrap();
    let (model, diagrams, revision) = database.shared_snapshot(project.id, alice).unwrap();
    assert_eq!(revision, 7);
    assert_eq!(diagrams.iter().find(|entry| entry.id == diagram), Some(&original));
    assert!(diagrams.iter().any(|entry| entry.id == other));
    assert!(model.relationships.contains_key(&relationship));
}
