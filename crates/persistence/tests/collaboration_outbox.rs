use systems_modeler_core::Project;
use systems_modeler_persistence::{
    collaboration::{EditRequest, SharedEdit},
    collaboration_outbox::{CollaborationOutbox, PendingSharedEdit},
};
use uuid::Uuid;

fn pending(project: &Project, name: &str) -> PendingSharedEdit {
    PendingSharedEdit {
        project: project.id,
        request: EditRequest {
            operation_id: Uuid::new_v4(),
            expected_revision: 7,
            edit: SharedEdit::CreateBlock { owner: project.root_id, name: name.into() },
        },
    }
}

#[test]
fn pending_operation_survives_reopen_and_is_scoped_to_actor_and_server() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("outbox.sqlite");
    let actor = Uuid::new_v4();
    let original = pending(&Project::new("Shared"), "Actuator");
    let server = "https://models.example/";
    let mut outbox = CollaborationOutbox::open(&path).unwrap();
    outbox.reserve(server, actor, &original).unwrap();
    drop(outbox);
    let mut recovered = CollaborationOutbox::open(&path).unwrap();
    assert_eq!(recovered.load(server, actor).unwrap(), Some(original.clone()));
    assert!(recovered.load(server, Uuid::new_v4()).unwrap().is_none());
    assert!(recovered.load("https://other.example/", actor).unwrap().is_none());
    recovered.complete(server, actor, &original).unwrap();
    drop(recovered);
    assert!(CollaborationOutbox::open(&path).unwrap().load(server, actor).unwrap().is_none());
}

#[test]
fn another_application_cannot_overwrite_or_clear_a_different_operation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("outbox.sqlite");
    let actor = Uuid::new_v4();
    let project = Project::new("Shared");
    let original = pending(&project, "Original");
    let other = pending(&project, "Competing edit");
    let mut first = CollaborationOutbox::open(&path).unwrap();
    let mut second = CollaborationOutbox::open(&path).unwrap();
    first.reserve("https://models.example/", actor, &original).unwrap();
    assert!(second.reserve("https://models.example/", actor, &other).is_err());
    assert!(second.complete("https://models.example/", actor, &other).is_err());
    assert_eq!(first.load("https://models.example/", actor).unwrap(), Some(original.clone()));
    second.reserve("https://models.example/", actor, &original).unwrap();
    first.complete("https://models.example/", actor, &original).unwrap();
    second.complete("https://models.example/", actor, &original).unwrap();
    second.reserve("https://models.example/", actor, &other).unwrap();
    assert!(first.complete("https://models.example/", actor, &original).is_err());
    assert_eq!(second.load("https://models.example/", actor).unwrap(), Some(other));
}

#[test]
fn malformed_record_is_preserved_and_reported_instead_of_replaced() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("outbox.sqlite");
    let actor = Uuid::new_v4();
    let request = pending(&Project::new("Shared"), "Example");
    let mut outbox = CollaborationOutbox::open(&path).unwrap();
    outbox.reserve("https://models.example/", actor, &request).unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute("UPDATE pending_shared_edits SET request_json='invalid json'", []).unwrap();
    assert!(outbox.load("https://models.example/", actor).is_err());
    assert!(outbox.reserve("https://models.example/", actor, &request).is_err());
    let saved: String = connection.query_row("SELECT request_json FROM pending_shared_edits", [], |row| row.get(0)).unwrap();
    assert_eq!(saved, "invalid json");
}
