//! Real client/session -> HTTP server -> SQLite qualification.
//! These tests do not launch WebView windows or claim physical-device UI coverage.
use super::*;
use std::{path::Path, sync::Arc};
use systems_modeler_core::{DiagramId, ElementId};
use systems_modeler_persistence::{ProjectDatabase, collaboration::SharedRelationshipKind};
use systems_modeler_server::{
    Config, Credential, Grant as ServerGrant, Role, Service, serve, token_hash,
};

fn credentials(project: ProjectId) -> Vec<Credential> {
    ['a', 'b', 'c']
        .into_iter()
        .map(|letter| Credential {
            actor: Uuid::new_v4(),
            token_sha256: token_hash(&letter.to_string().repeat(64)),
            projects: vec![ServerGrant {
                project,
                role: if letter == 'c' {
                    Role::Viewer
                } else {
                    Role::Editor
                },
            }],
        })
        .collect()
}

async fn start(
    path: &Path,
    credentials: Vec<Credential>,
) -> (String, tokio::task::JoinHandle<std::io::Result<()>>) {
    let service =
        Service::new(ProjectDatabase::open(path).unwrap(), Config { credentials }).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(serve(listener, Arc::new(service)));
    (url, task)
}

fn seed(path: &Path) -> Project {
    let model = Project::new("Shared engineering");
    ProjectDatabase::open(path)
        .unwrap()
        .save_project(&model)
        .unwrap();
    model
}

fn block(owner: ElementId, name: &str) -> SharedEdit {
    SharedEdit::CreateBlock {
        owner,
        name: name.into(),
    }
}

fn named(session: &Session, name: &str) -> ElementId {
    session
        .snapshot
        .as_ref()
        .unwrap()
        .project
        .elements
        .values()
        .find(|element| element.name == name)
        .unwrap()
        .id
}

#[test]
fn two_clients_reverse_only_their_own_changes_and_preserve_unrelated_work() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("shared.sqlite");
        let project = seed(&path);
        let (url, task) = start(&path, credentials(project.id)).await;
        let mut alice = Session::connect(&url, "a".repeat(64)).await.unwrap();
        alice
            .attach_outbox(&directory.path().join("alice-outbox.sqlite"))
            .await
            .unwrap();
        let mut bob = Session::connect(&url, "b".repeat(64)).await.unwrap();
        alice.open(project.id).await.unwrap();
        alice
            .edit(0, block(project.root_id, "Alice component"))
            .await
            .unwrap();
        let history: SharedHistory = alice
            .request(
                Method::GET,
                &format!("v1/projects/{}/history", project.id),
                None,
            )
            .await
            .unwrap();
        let original = history.operations[0].operation_id;
        bob.open(project.id).await.unwrap();
        bob.edit(1, block(project.root_id, "Bob component"))
            .await
            .unwrap();
        alice.open(project.id).await.unwrap();
        alice
            .edit(
                2,
                SharedEdit::UndoOperation {
                    operation: original,
                },
            )
            .await
            .unwrap();
        bob.open(project.id).await.unwrap();
        let model = &bob.snapshot.as_ref().unwrap().project;
        assert!(
            model
                .elements
                .values()
                .any(|element| element.name == "Bob component")
        );
        assert!(
            !model
                .elements
                .values()
                .any(|element| element.name == "Alice component")
        );
        assert!(
            bob.edit(
                3,
                SharedEdit::UndoOperation {
                    operation: original
                }
            )
            .await
            .unwrap_err()
            .contains("Access denied")
        );
        let history: SharedHistory = alice
            .request(
                Method::GET,
                &format!("v1/projects/{}/history", project.id),
                None,
            )
            .await
            .unwrap();
        assert_eq!(history.operations.len(), 2);
        assert!(!history.operations[1].can_undo);
        alice
            .edit(
                3,
                SharedEdit::UndoOperation {
                    operation: history.operations[0].operation_id,
                },
            )
            .await
            .unwrap();
        assert!(
            alice
                .snapshot
                .as_ref()
                .unwrap()
                .project
                .elements
                .values()
                .any(|element| element.name == "Alice component")
        );
        assert!(
            alice
                .outbox
                .as_ref()
                .unwrap()
                .load(alice.base.as_str(), alice.actor)
                .unwrap()
                .is_none()
        );
        task.abort();
    });
}

#[test]
fn editor_and_viewer_presence_converges_over_http_without_mutating_the_project() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("shared.sqlite");
        let project = seed(&path);
        let (url, task) = start(&path, credentials(project.id)).await;
        let mut editor = Session::connect(&url, "a".repeat(64)).await.unwrap();
        let mut viewer = Session::connect(&url, "c".repeat(64)).await.unwrap();
        editor.display_name = "Engineer".into();
        viewer.display_name = "Reviewer".into();
        editor.open(project.id).await.unwrap();
        viewer.open(project.id).await.unwrap();
        assert_eq!(
            editor
                .presence_request(project.id, Method::POST)
                .await
                .unwrap()
                .participants
                .len(),
            1
        );
        let participants = viewer
            .presence_request(project.id, Method::POST)
            .await
            .unwrap();
        assert_eq!(participants.participants.len(), 2);
        assert!(
            participants
                .participants
                .iter()
                .any(|person| person.actor == viewer.actor
                    && person.role == "viewer"
                    && person.name == "Reviewer")
        );
        assert_eq!(
            editor
                .presence_request(project.id, Method::POST)
                .await
                .unwrap()
                .participants
                .len(),
            2
        );
        editor
            .presence_request(project.id, Method::DELETE)
            .await
            .unwrap();
        assert_eq!(
            viewer
                .presence_request(project.id, Method::POST)
                .await
                .unwrap()
                .participants
                .len(),
            1
        );
        viewer.open(project.id).await.unwrap();
        assert_eq!(viewer.snapshot.as_ref().unwrap().revision, 0);
        assert!(viewer.pending.is_none());
        assert!(!viewer.needs_refresh);
        task.abort();
    });
}

#[test]
fn restarted_client_recovers_committed_operation_and_does_not_duplicate_it() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("shared.sqlite");
        let journal = directory.path().join("outbox.sqlite");
        let project = seed(&path);
        let (url, task) = start(&path, credentials(project.id)).await;
        let mut first = Session::connect(&url, "a".repeat(64)).await.unwrap();
        first.attach_outbox(&journal).await.unwrap();
        first.open(project.id).await.unwrap();
        let request = EditRequest {
            operation_id: Uuid::new_v4(),
            expected_revision: 0,
            edit: block(project.root_id, "Recovered actuator"),
        };
        first
            .outbox
            .as_mut()
            .unwrap()
            .reserve(
                first.base.as_str(),
                first.actor,
                &PendingSharedEdit {
                    project: project.id,
                    request: request.clone(),
                },
            )
            .unwrap();
        // Commit via real HTTP, then terminate the session before receipt handling.
        let receipt: Receipt = first
            .request(
                Method::POST,
                &format!("v1/projects/{}/operations", project.id),
                Some(&request),
            )
            .await
            .unwrap();
        assert_eq!(receipt.revision, 1);
        drop(first);
        let mut other = Session::connect(&url, "b".repeat(64)).await.unwrap();
        other.attach_outbox(&journal).await.unwrap();
        assert!(other.pending.is_none());
        let mut recovered = Session::connect(&url, "a".repeat(64)).await.unwrap();
        recovered.attach_outbox(&journal).await.unwrap();
        assert_eq!(recovered.pending.as_ref(), Some(&request));
        assert_eq!(recovered.snapshot.as_ref().unwrap().revision, 1);
        recovered.submit_pending().await.unwrap();
        assert!(recovered.pending.is_none());
        assert_eq!(recovered.snapshot.as_ref().unwrap().revision, 1);
        assert_eq!(
            recovered
                .snapshot
                .as_ref()
                .unwrap()
                .project
                .elements
                .values()
                .filter(|element| element.name == "Recovered actuator")
                .count(),
            1
        );
        assert!(
            recovered
                .outbox
                .as_ref()
                .unwrap()
                .load(recovered.base.as_str(), recovered.actor)
                .unwrap()
                .is_none()
        );
        drop(recovered);
        let bytes = std::fs::read(&journal).unwrap();
        assert!(
            !bytes
                .windows(64)
                .any(|window| window == "a".repeat(64).as_bytes())
        );
        task.abort();
    });
}

#[test]
fn durable_reservation_blocks_competing_edits_and_rejected_retry_clears_it() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("shared.sqlite");
        let journal = directory.path().join("outbox.sqlite");
        let project = seed(&path);
        let (url, task) = start(&path, credentials(project.id)).await;
        let mut first = Session::connect(&url, "a".repeat(64)).await.unwrap();
        first.attach_outbox(&journal).await.unwrap();
        first.open(project.id).await.unwrap();
        let saved = PendingSharedEdit {
            project: project.id,
            request: EditRequest {
                operation_id: Uuid::new_v4(),
                expected_revision: 0,
                edit: block(project.root_id, "Unsent edit"),
            },
        };
        first
            .outbox
            .as_mut()
            .unwrap()
            .reserve(first.base.as_str(), first.actor, &saved)
            .unwrap();
        assert!(
            first
                .edit(0, block(project.root_id, "Must not transmit"))
                .await
                .unwrap_err()
                .contains("No edit was sent")
        );
        assert!(first.pending.is_none());
        let mut other = Session::connect(&url, "b".repeat(64)).await.unwrap();
        other.open(project.id).await.unwrap();
        assert_eq!(other.snapshot.as_ref().unwrap().revision, 0);
        other
            .edit(0, block(project.root_id, "Other actor"))
            .await
            .unwrap();
        drop(first);
        let mut recovered = Session::connect(&url, "a".repeat(64)).await.unwrap();
        recovered.attach_outbox(&journal).await.unwrap();
        assert_eq!(recovered.pending.as_ref(), Some(&saved.request));
        assert!(
            recovered
                .submit_pending()
                .await
                .unwrap_err()
                .contains("Refresh")
        );
        assert!(recovered.pending.is_none());
        assert!(recovered.needs_refresh);
        assert!(
            recovered
                .outbox
                .as_ref()
                .unwrap()
                .load(recovered.base.as_str(), recovered.actor)
                .unwrap()
                .is_none()
        );
        task.abort();
    });
}

#[test]
fn authenticated_actor_recovers_unsent_operation_after_token_rotation() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("shared.sqlite");
        let journal = directory.path().join("outbox.sqlite");
        let project = seed(&path);
        let mut identities = credentials(project.id);
        let actor = identities[0].actor;
        identities[0].token_sha256 = token_hash(&"d".repeat(64));
        let (url, task) = start(&path, identities).await;
        let server = server_url(&url).unwrap();
        let pending = PendingSharedEdit {
            project: project.id,
            request: EditRequest {
                operation_id: Uuid::new_v4(),
                expected_revision: 0,
                edit: block(project.root_id, "Restored unsent edit"),
            },
        };
        // The journal created before a crash has no dependency on the old token.
        CollaborationOutbox::open(&journal)
            .unwrap()
            .reserve(server.as_str(), actor, &pending)
            .unwrap();
        assert!(Session::connect(&url, "a".repeat(64)).await.is_err());
        let mut recovered = Session::connect(&url, "d".repeat(64)).await.unwrap();
        recovered.attach_outbox(&journal).await.unwrap();
        assert_eq!(recovered.actor, actor);
        assert_eq!(recovered.pending.as_ref(), Some(&pending.request));
        assert_eq!(recovered.snapshot.as_ref().unwrap().revision, 0);
        recovered.submit_pending().await.unwrap();
        assert_eq!(recovered.snapshot.as_ref().unwrap().revision, 1);
        assert!(recovered.pending.is_none());
        assert!(
            recovered
                .snapshot
                .as_ref()
                .unwrap()
                .project
                .elements
                .values()
                .any(|element| element.name == "Restored unsent edit")
        );
        task.abort();
    });
}

#[test]
fn two_clients_share_edits_and_recover_from_stale_revision() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("shared.sqlite");
        let project = seed(&path);
        let (url, task) = start(&path, credentials(project.id)).await;
        let mut first = Session::connect(&url, "a".repeat(64)).await.unwrap();
        let mut second = Session::connect(&url, "b".repeat(64)).await.unwrap();
        first.open(project.id).await.unwrap();
        second.open(project.id).await.unwrap();
        first
            .edit(
                0,
                SharedEdit::CreatePackage {
                    owner: project.root_id,
                    name: "Architecture".into(),
                },
            )
            .await
            .unwrap();
        assert!(
            second
                .edit(0, block(project.root_id, "Stale edit"))
                .await
                .unwrap_err()
                .contains("Refresh")
        );
        assert!(second.needs_refresh);
        assert!(second.pending.is_none());
        assert!(
            second
                .edit(0, block(project.root_id, "No blind retry"))
                .await
                .is_err()
        );
        second.open(project.id).await.unwrap();
        let owner = named(&second, "Architecture");
        second.edit(1, block(owner, "Motor")).await.unwrap();
        first.open(project.id).await.unwrap();
        let motor = named(&first, "Motor");
        assert_eq!(
            first.snapshot.as_ref().unwrap().project.elements[&motor].owner_id,
            Some(owner)
        );
        first
            .edit(
                2,
                SharedEdit::RenameElement {
                    element: motor,
                    name: "Traction motor".into(),
                },
            )
            .await
            .unwrap();
        second.open(project.id).await.unwrap();
        assert_eq!(named(&second, "Traction motor"), motor);
        assert_eq!(second.snapshot.as_ref().unwrap().revision, 3);
        assert_eq!(second.snapshot.as_ref().unwrap().project.elements.len(), 3);
        assert_eq!(
            serde_json::to_value(&first.snapshot).unwrap(),
            serde_json::to_value(&second.snapshot).unwrap()
        );
        task.abort();
        let _ = task.await;
    });
}

#[test]
fn concurrent_clients_accept_only_one_edit_at_a_shared_revision() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("shared.sqlite");
        let project = seed(&path);
        let (url, server) = start(&path, credentials(project.id)).await;
        let mut first = Session::connect(&url, "a".repeat(64)).await.unwrap();
        let mut second = Session::connect(&url, "b".repeat(64)).await.unwrap();
        first.open(project.id).await.unwrap();
        second.open(project.id).await.unwrap();
        let root = project.root_id;
        let barrier = Arc::new(tokio::sync::Barrier::new(2));
        let first_barrier = barrier.clone();
        let a = tokio::spawn(async move {
            first_barrier.wait().await;
            let result = first.edit(0, block(root, "First")).await;
            (first, result)
        });
        let b = tokio::spawn(async move {
            barrier.wait().await;
            let result = second.edit(0, block(root, "Second")).await;
            (second, result)
        });
        let (mut first, first_result) = a.await.unwrap();
        let (mut second, second_result) = b.await.unwrap();
        assert_ne!(first_result.is_ok(), second_result.is_ok());
        let loser = if first_result.is_err() {
            &mut first
        } else {
            &mut second
        };
        assert!(loser.needs_refresh);
        loser.open(project.id).await.unwrap();
        assert_eq!(loser.snapshot.as_ref().unwrap().revision, 1);
        assert_eq!(loser.snapshot.as_ref().unwrap().project.elements.len(), 2);
        loser.edit(1, block(root, "Resolved edit")).await.unwrap();
        first.open(project.id).await.unwrap();
        second.open(project.id).await.unwrap();
        assert_eq!(first.snapshot.as_ref().unwrap().revision, 2);
        assert_eq!(first.snapshot.as_ref().unwrap().project.elements.len(), 3);
        assert_eq!(
            serde_json::to_value(&first.snapshot).unwrap(),
            serde_json::to_value(&second.snapshot).unwrap()
        );
        server.abort();
        let _ = server.await;
    });
}

#[test]
fn viewer_and_invalid_credentials_cannot_mutate_shared_projects() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("shared.sqlite");
        let project = seed(&path);
        let (url, task) = start(&path, credentials(project.id)).await;
        assert!(Session::connect(&url, "d".repeat(64)).await.is_err());
        let mut viewer = Session::connect(&url, "c".repeat(64)).await.unwrap();
        viewer.open(project.id).await.unwrap();
        assert_eq!(
            viewer
                .edit(0, block(project.root_id, "Forbidden"))
                .await
                .unwrap_err(),
            "This project is view-only."
        );
        assert!(viewer.pending.is_none());
        // Bypass the client-side permission check: the actual HTTP server must deny it too.
        let request = EditRequest {
            operation_id: Uuid::new_v4(),
            expected_revision: 0,
            edit: block(project.root_id, "Forged"),
        };
        let result: Result<Receipt, _> = viewer
            .request(
                Method::POST,
                &format!("v1/projects/{}/operations", project.id),
                Some(&request),
            )
            .await;
        assert!(result.err().unwrap().1.contains("Access denied"));
        let unknown = ProjectId::new();
        assert!(viewer.open(unknown).await.is_err());
        let result: Result<Snapshot, _> = viewer
            .request(Method::GET, &format!("v1/projects/{unknown}"), None)
            .await;
        assert!(result.err().unwrap().1.contains("Access denied"));
        viewer.open(project.id).await.unwrap();
        assert_eq!(viewer.snapshot.as_ref().unwrap().revision, 0);
        assert_eq!(viewer.snapshot.as_ref().unwrap().project.elements.len(), 1);
        task.abort();
        let _ = task.await;
    });
}

#[test]
fn unacknowledged_edit_retries_once_and_survives_server_restart() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("shared.sqlite");
        let project = seed(&path);
        let grants = credentials(project.id);
        let (url, task) = start(&path, grants.clone()).await;
        let mut first = Session::connect(&url, "a".repeat(64)).await.unwrap();
        first.open(project.id).await.unwrap();
        let pending = EditRequest {
            operation_id: Uuid::new_v4(),
            expected_revision: 0,
            edit: block(project.root_id, "Exactly once"),
        };
        // Commit through real HTTP, deliberately discard the result before updating
        // client state, then exercise the production pending-retry path.
        let _: Receipt = first
            .request(
                Method::POST,
                &format!("v1/projects/{}/operations", project.id),
                Some(&pending),
            )
            .await
            .unwrap();
        first.pending = Some(pending.clone());
        assert!(first.open(project.id).await.is_err());
        assert!(
            first
                .edit(0, block(project.root_id, "Must wait"))
                .await
                .is_err()
        );
        first.submit_pending().await.unwrap();
        let stable_id = named(&first, "Exactly once");
        assert_eq!(first.snapshot.as_ref().unwrap().revision, 1);
        assert_eq!(first.snapshot.as_ref().unwrap().project.elements.len(), 2);
        drop(first);
        task.abort();
        let _ = task.await;
        let (url, task) = start(&path, grants).await;
        let mut reconnected = Session::connect(&url, "a".repeat(64)).await.unwrap();
        reconnected.open(project.id).await.unwrap();
        assert_eq!(named(&reconnected, "Exactly once"), stable_id);
        reconnected.pending = Some(pending);
        reconnected.submit_pending().await.unwrap();
        assert_eq!(reconnected.snapshot.as_ref().unwrap().revision, 1);
        assert_eq!(
            reconnected
                .snapshot
                .as_ref()
                .unwrap()
                .project
                .elements
                .len(),
            2
        );
        reconnected
            .snapshot
            .as_ref()
            .unwrap()
            .project
            .validate()
            .unwrap();
        task.abort();
        let _ = task.await;
    });
}

#[test]
fn two_clients_converge_after_relationship_create_and_delete() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("shared.sqlite");
        let project = seed(&path);
        let (url, task) = start(&path, credentials(project.id)).await;
        let mut first = Session::connect(&url, "a".repeat(64)).await.unwrap();
        let mut second = Session::connect(&url, "b".repeat(64)).await.unwrap();
        first.open(project.id).await.unwrap();
        second.open(project.id).await.unwrap();

        first.edit(0, block(project.root_id, "Base")).await.unwrap();
        second.open(project.id).await.unwrap();
        second
            .edit(1, block(project.root_id, "Derived"))
            .await
            .unwrap();
        first.open(project.id).await.unwrap();
        let base = named(&first, "Base");
        let derived = named(&first, "Derived");
        first
            .edit(
                2,
                SharedEdit::CreateRelationship {
                    kind: SharedRelationshipKind::Generalization,
                    source: derived,
                    target: base,
                    owner: project.root_id,
                },
            )
            .await
            .unwrap();

        second.open(project.id).await.unwrap();
        let relationship = second
            .snapshot
            .as_ref()
            .unwrap()
            .project
            .relationships
            .values()
            .next()
            .unwrap();
        assert_eq!(
            relationship.kind,
            systems_modeler_core::RelationshipKind::Generalization
        );
        assert_eq!(relationship.source_id, derived);
        assert_eq!(relationship.target_id, base);
        let relationship_id = relationship.id;

        second
            .edit(
                3,
                SharedEdit::DeleteRelationship {
                    relationship: relationship_id,
                },
            )
            .await
            .unwrap();
        first.open(project.id).await.unwrap();
        assert_eq!(first.snapshot.as_ref().unwrap().revision, 4);
        assert!(
            first
                .snapshot
                .as_ref()
                .unwrap()
                .project
                .relationships
                .is_empty()
        );
        assert_eq!(
            serde_json::to_value(&first.snapshot).unwrap(),
            serde_json::to_value(&second.snapshot).unwrap()
        );
        task.abort();
        let _ = task.await;
    });
}

#[test]
fn two_clients_converge_on_server_routed_bdd_relationship_presentations() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("shared-bdd-edges.sqlite");
        let project = seed(&path);
        let (url, task) = start(&path, credentials(project.id)).await;
        let mut first = Session::connect(&url, "a".repeat(64)).await.unwrap();
        let mut second = Session::connect(&url, "b".repeat(64)).await.unwrap();
        first.open(project.id).await.unwrap();
        second.open(project.id).await.unwrap();

        first
            .edit(0, block(project.root_id, "Source"))
            .await
            .unwrap();
        second.open(project.id).await.unwrap();
        second
            .edit(1, block(project.root_id, "Target"))
            .await
            .unwrap();
        first.open(project.id).await.unwrap();
        let source = named(&first, "Source");
        let target = named(&first, "Target");
        first
            .edit(
                2,
                SharedEdit::CreateRelationship {
                    kind: SharedRelationshipKind::Dependency,
                    source,
                    target,
                    owner: project.root_id,
                },
            )
            .await
            .unwrap();
        let relationship = first
            .snapshot
            .as_ref()
            .unwrap()
            .project
            .relationships
            .values()
            .next()
            .unwrap()
            .id;
        let diagram = DiagramId::new();
        first
            .edit(
                3,
                SharedEdit::CreateBddDiagram {
                    diagram,
                    owner: project.root_id,
                    name: "Structure".into(),
                },
            )
            .await
            .unwrap();
        let source_node = Uuid::new_v4();
        let target_node = Uuid::new_v4();
        first
            .edit(
                4,
                SharedEdit::PlaceBddElement {
                    diagram,
                    node: source_node,
                    element: source,
                },
            )
            .await
            .unwrap();
        first
            .edit(
                5,
                SharedEdit::PlaceBddElement {
                    diagram,
                    node: target_node,
                    element: target,
                },
            )
            .await
            .unwrap();
        let edge = Uuid::new_v4();
        first
            .edit(
                6,
                SharedEdit::PresentBddRelationship {
                    diagram,
                    edge,
                    relationship,
                },
            )
            .await
            .unwrap();

        second.open(project.id).await.unwrap();
        let shared = &second.snapshot.as_ref().unwrap().diagrams[0];
        assert_eq!(second.snapshot.as_ref().unwrap().revision, 7);
        assert_eq!(shared.edges.len(), 1);
        assert_eq!(shared.edges[0].relationship, relationship);
        let before_move = shared.edges[0].points.clone();
        second
            .edit(
                7,
                SharedEdit::UpdateBddNodeGeometry {
                    diagram,
                    node: source_node,
                    x: 120.0,
                    y: 360.0,
                    width: 190.0,
                    height: 115.0,
                },
            )
            .await
            .unwrap();
        assert_ne!(
            second.snapshot.as_ref().unwrap().diagrams[0].edges[0].points,
            before_move
        );

        first.open(project.id).await.unwrap();
        assert_eq!(first.snapshot.as_ref().unwrap().revision, 8);
        assert_eq!(
            serde_json::to_value(&first.snapshot).unwrap(),
            serde_json::to_value(&second.snapshot).unwrap()
        );
        first
            .edit(8, SharedEdit::RemoveBddEdge { diagram, edge })
            .await
            .unwrap();
        second.open(project.id).await.unwrap();
        assert_eq!(second.snapshot.as_ref().unwrap().revision, 9);
        assert!(
            second.snapshot.as_ref().unwrap().diagrams[0]
                .edges
                .is_empty()
        );
        assert_eq!(
            serde_json::to_value(&first.snapshot).unwrap(),
            serde_json::to_value(&second.snapshot).unwrap()
        );
        task.abort();
        let _ = task.await;
    });
}

#[test]
fn two_clients_author_requirements_and_verify_without_losing_stale_edits() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("shared-requirements.sqlite");
        let project = seed(&path);
        let (url, task) = start(&path, credentials(project.id)).await;
        let mut first = Session::connect(&url, "a".repeat(64)).await.unwrap();
        let mut second = Session::connect(&url, "b".repeat(64)).await.unwrap();
        first.open(project.id).await.unwrap();
        first
            .edit(
                0,
                SharedEdit::CreateRequirement {
                    owner: project.root_id,
                    name: "Response".into(),
                    requirement_id: "REQ-1".into(),
                    text: "Respond within 50 ms.".into(),
                },
            )
            .await
            .unwrap();
        second.open(project.id).await.unwrap();
        let requirement = named(&second, "Response");
        second
            .edit(
                1,
                SharedEdit::CreateTestCase {
                    owner: project.root_id,
                    name: "Timing test".into(),
                },
            )
            .await
            .unwrap();
        first.open(project.id).await.unwrap();
        first
            .edit(2, block(project.root_id, "Controller"))
            .await
            .unwrap();
        let test_case = named(&first, "Timing test");
        let controller = named(&first, "Controller");
        for (revision, kind, source) in [
            (3, SharedRelationshipKind::Verify, test_case),
            (4, SharedRelationshipKind::Satisfy, controller),
        ] {
            first
                .edit(
                    revision,
                    SharedEdit::CreateRelationship {
                        kind,
                        source,
                        target: requirement,
                        owner: project.root_id,
                    },
                )
                .await
                .unwrap();
        }
        second.open(project.id).await.unwrap();
        let revised = SharedEdit::UpdateRequirement {
            element: requirement,
            name: "Response budget".into(),
            requirement_id: "REQ-1A".into(),
            text: "Respond within 40 ms.\nMeasured at the interface.".into(),
        };
        first.edit(5, revised).await.unwrap();
        assert!(
            second
                .edit(
                    5,
                    SharedEdit::UpdateRequirement {
                        element: requirement,
                        name: "Stale response".into(),
                        requirement_id: "REQ-1".into(),
                        text: "Stale text must not overwrite the committed revision.".into(),
                    }
                )
                .await
                .is_err()
        );
        assert!(second.needs_refresh);
        second.open(project.id).await.unwrap();
        assert_eq!(
            serde_json::to_value(&first.snapshot).unwrap(),
            serde_json::to_value(&second.snapshot).unwrap()
        );
        let snapshot = second.snapshot.as_ref().unwrap();
        assert_eq!(snapshot.revision, 6);
        assert_eq!(
            snapshot
                .project
                .element(requirement)
                .unwrap()
                .requirement_id
                .as_deref(),
            Some("REQ-1A")
        );
        assert_eq!(snapshot.project.relationships.len(), 2);
        assert!(
            snapshot
                .project
                .relationships
                .values()
                .all(|relationship| relationship.target_id == requirement)
        );
        task.abort();
        let _ = task.await;
        let (url, restarted) = start(&path, credentials(project.id)).await;
        let mut reopened = Session::connect(&url, "a".repeat(64)).await.unwrap();
        reopened.open(project.id).await.unwrap();
        assert_eq!(
            serde_json::to_value(&first.snapshot).unwrap(),
            serde_json::to_value(&reopened.snapshot).unwrap()
        );
        restarted.abort();
        let _ = restarted.await;
    });
}

#[test]
fn two_clients_create_and_present_shared_interface_blocks_through_native_semantics() {
    use systems_modeler_core::structural_presentation::creation::{
        BddElementKind, CreateBddElement,
    };
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("shared-interface.sqlite");
        let project = seed(&path);
        let (url, task) = start(&path, credentials(project.id)).await;
        let mut alice = Session::connect(&url, "a".repeat(64)).await.unwrap();
        let mut bob = Session::connect(&url, "b".repeat(64)).await.unwrap();
        alice.open(project.id).await.unwrap();
        bob.open(project.id).await.unwrap();
        let command = SharedEdit::CreateBddElement(CreateBddElement {
            kind: BddElementKind::InterfaceBlock,
            owner: project.root_id,
            name: "Control interface".into(),
        });
        alice.edit(0, command.clone()).await.unwrap();
        assert!(bob.edit(0, command).await.is_err());
        assert!(bob.needs_refresh);
        bob.open(project.id).await.unwrap();
        let element = named(&bob, "Control interface");
        assert_eq!(
            bob.snapshot
                .as_ref()
                .unwrap()
                .project
                .element(element)
                .unwrap()
                .kind,
            systems_modeler_core::ElementKind::InterfaceBlock
        );
        let diagram = DiagramId::new();
        bob.edit(
            1,
            SharedEdit::CreateBddDiagram {
                diagram,
                owner: project.root_id,
                name: "Interfaces".into(),
            },
        )
        .await
        .unwrap();
        bob.edit(
            2,
            SharedEdit::PlaceBddElement {
                diagram,
                node: Uuid::new_v4(),
                element,
            },
        )
        .await
        .unwrap();
        alice.open(project.id).await.unwrap();
        assert_eq!(
            serde_json::to_value(&alice.snapshot).unwrap(),
            serde_json::to_value(&bob.snapshot).unwrap()
        );
        assert_eq!(
            alice.snapshot.as_ref().unwrap().diagrams[0].nodes[0].element,
            element
        );
        task.abort();
        let _ = task.await;
    });
}
