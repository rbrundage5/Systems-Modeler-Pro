//! Real client/session -> HTTP server -> SQLite qualification.
//! These tests do not launch WebView windows or claim physical-device UI coverage.
use super::*;
use std::{path::Path, sync::Arc};
use systems_modeler_core::ElementId;
use systems_modeler_persistence::ProjectDatabase;
use systems_modeler_server::{Config, Credential, Grant as ServerGrant, Role, Service, serve, token_hash};

fn credentials(project: ProjectId) -> Vec<Credential> {
    ['a', 'b', 'c'].into_iter().map(|letter| Credential {
        actor: Uuid::new_v4(),
        token_sha256: token_hash(&letter.to_string().repeat(64)),
        projects: vec![ServerGrant {
            project,
            role: if letter == 'c' { Role::Viewer } else { Role::Editor },
        }],
    }).collect()
}

async fn start(path: &Path, credentials: Vec<Credential>) -> (String, tokio::task::JoinHandle<std::io::Result<()>>) {
    let service = Service::new(ProjectDatabase::open(path).unwrap(), Config { credentials }).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(serve(listener, Arc::new(service)));
    (url, task)
}

fn seed(path: &Path) -> Project {
    let model = Project::new("Shared engineering");
    ProjectDatabase::open(path).unwrap().save_project(&model).unwrap();
    model
}

fn block(owner: ElementId, name: &str) -> SharedEdit {
    SharedEdit::CreateBlock { owner, name: name.into() }
}

fn named(session: &Session, name: &str) -> ElementId {
    session.snapshot.as_ref().unwrap().project.elements.values()
        .find(|element| element.name == name).unwrap().id
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
        first.edit(0, SharedEdit::CreatePackage { owner: project.root_id, name: "Architecture".into() }).await.unwrap();
        assert!(second.edit(0, block(project.root_id, "Stale edit")).await.unwrap_err().contains("Refresh"));
        assert!(second.needs_refresh);
        assert!(second.pending.is_none());
        assert!(second.edit(0, block(project.root_id, "No blind retry")).await.is_err());
        second.open(project.id).await.unwrap();
        let owner = named(&second, "Architecture");
        second.edit(1, block(owner, "Motor")).await.unwrap();
        first.open(project.id).await.unwrap();
        let motor = named(&first, "Motor");
        assert_eq!(first.snapshot.as_ref().unwrap().project.elements[&motor].owner_id, Some(owner));
        first.edit(2, SharedEdit::RenameElement { element: motor, name: "Traction motor".into() }).await.unwrap();
        second.open(project.id).await.unwrap();
        assert_eq!(named(&second, "Traction motor"), motor);
        assert_eq!(second.snapshot.as_ref().unwrap().revision, 3);
        assert_eq!(second.snapshot.as_ref().unwrap().project.elements.len(), 3);
        assert_eq!(serde_json::to_value(&first.snapshot).unwrap(), serde_json::to_value(&second.snapshot).unwrap());
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
        let loser = if first_result.is_err() { &mut first } else { &mut second };
        assert!(loser.needs_refresh);
        loser.open(project.id).await.unwrap();
        assert_eq!(loser.snapshot.as_ref().unwrap().revision, 1);
        assert_eq!(loser.snapshot.as_ref().unwrap().project.elements.len(), 2);
        loser.edit(1, block(root, "Resolved edit")).await.unwrap();
        first.open(project.id).await.unwrap();
        second.open(project.id).await.unwrap();
        assert_eq!(first.snapshot.as_ref().unwrap().revision, 2);
        assert_eq!(first.snapshot.as_ref().unwrap().project.elements.len(), 3);
        assert_eq!(serde_json::to_value(&first.snapshot).unwrap(), serde_json::to_value(&second.snapshot).unwrap());
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
        assert_eq!(viewer.edit(0, block(project.root_id, "Forbidden")).await.unwrap_err(), "This project is view-only.");
        assert!(viewer.pending.is_none());
        // Bypass the client-side permission check: the actual HTTP server must deny it too.
        let request = EditRequest { operation_id: Uuid::new_v4(), expected_revision: 0, edit: block(project.root_id, "Forged") };
        let result: Result<Receipt, _> = viewer.request(Method::POST, &format!("v1/projects/{}/operations", project.id), Some(&request)).await;
        assert!(result.err().unwrap().1.contains("Access denied"));
        let unknown = ProjectId::new();
        assert!(viewer.open(unknown).await.is_err());
        let result: Result<Snapshot, _> = viewer.request(Method::GET, &format!("v1/projects/{unknown}"), None).await;
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
        let pending = EditRequest { operation_id: Uuid::new_v4(), expected_revision: 0, edit: block(project.root_id, "Exactly once") };
        // Commit through real HTTP, deliberately discard the result before updating
        // client state, then exercise the production pending-retry path.
        let _: Receipt = first.request(Method::POST, &format!("v1/projects/{}/operations", project.id), Some(&pending)).await.unwrap();
        first.pending = Some(pending.clone());
        assert!(first.open(project.id).await.is_err());
        assert!(first.edit(0, block(project.root_id, "Must wait")).await.is_err());
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
        assert_eq!(reconnected.snapshot.as_ref().unwrap().project.elements.len(), 2);
        reconnected.snapshot.as_ref().unwrap().project.validate().unwrap();
        task.abort();
        let _ = task.await;
    });
}
