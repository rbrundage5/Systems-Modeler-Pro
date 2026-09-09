use std::{io::{Read, Write}, net::TcpStream, sync::Arc, time::Duration};
use serde_json::{Value, json};
use systems_modeler_core::Project;
use systems_modeler_persistence::ProjectDatabase;
use systems_modeler_server::{Config, Credential, Grant, Role, Service, MAX_BODY, serve, token_hash};
use uuid::Uuid;

fn fixture() -> (Arc<Service>, Project, String, String) {
    let mut database = ProjectDatabase::open_in_memory().unwrap();
    let project = Project::new("Shared");
    database.save_project(&project).unwrap();
    let editor = "a".repeat(64);
    let viewer = "b".repeat(64);
    let credentials = vec![
        Credential { actor:Uuid::new_v4(), token_sha256:token_hash(&editor), projects:vec![Grant { project:project.id, role:Role::Editor }] },
        Credential { actor:Uuid::new_v4(), token_sha256:token_hash(&viewer), projects:vec![Grant { project:project.id, role:Role::Viewer }] },
    ];
    (Arc::new(Service::new(database, Config { credentials }).unwrap()), project, format!("Bearer {editor}"), format!("Bearer {viewer}"))
}

fn operation(project: &Project) -> Value {
    json!({"operation_id":Uuid::new_v4(),"expected_revision":0,"edit":{"CreateBlock":{"owner":project.root_id,"name":"Engine"}}})
}

#[test]
fn authenticated_edits_retries_conflicts_and_viewer_reads() {
    let (service, project, editor, viewer) = fixture();
    let path = format!("/v1/projects/{}", project.id);
    let edits = format!("{path}/operations");
    let body = serde_json::to_vec(&operation(&project)).unwrap();
    assert_eq!(service.dispatch("GET", &path, None, &[]).0, 401);
    assert_eq!(service.dispatch("POST", &edits, Some(&viewer), &body).0, 403);
    let receipt = service.dispatch("POST", &edits, Some(&editor), &body);
    assert_eq!(receipt.0, 200);
    assert_eq!(service.dispatch("POST", &edits, Some(&editor), &body), receipt);
    let other = serde_json::to_vec(&operation(&project)).unwrap();
    assert_eq!(service.dispatch("POST", &edits, Some(&editor), &other).0, 409);
    let snapshot = service.dispatch("GET", &path, Some(&viewer), &[]);
    assert_eq!(snapshot.1["revision"], 1);
    assert_eq!(snapshot.1["project"]["elements"].as_object().unwrap().len(), 2);
    let unknown = format!("/v1/projects/{}", Uuid::new_v4());
    assert_eq!(service.dispatch("GET", &unknown, Some(&editor), &[]).0, 403);
}

#[test]
fn malformed_oversized_and_forged_identity_requests_are_rejected() {
    let (service, project, editor, _) = fixture();
    let path = format!("/v1/projects/{}/operations", project.id);
    assert_eq!(service.dispatch("POST", &path, Some(&editor), b"garbage").0, 400);
    assert_eq!(service.dispatch("POST", &path, Some(&editor), &vec![0; MAX_BODY+1]).0, 413);
    let mut forged = operation(&project);
    forged["actor"] = json!(Uuid::new_v4());
    assert_eq!(service.dispatch("POST", &path, Some(&editor), &serde_json::to_vec(&forged).unwrap()).0, 400);
    let revision = service.dispatch("GET", &format!("/v1/projects/{}",project.id), Some(&editor), &[]);
    assert_eq!(revision.1["revision"], 0);
}

#[test]
fn real_http_connection_requires_authentication() {
    let (service, _, editor, _) = fixture();
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    let listener = runtime.block_on(tokio::net::TcpListener::bind("127.0.0.1:0")).unwrap();
    let address = listener.local_addr().unwrap();
    let task = runtime.spawn(serve(listener, service));
    for (header, expected) in [(String::new(), "401"), (format!("Authorization: {editor}\r\n"), "200")] {
        let mut stream = TcpStream::connect(address).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        write!(stream, "GET /v1/projects HTTP/1.1\r\nHost: localhost\r\n{header}Connection: close\r\n\r\n").unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.starts_with(&format!("HTTP/1.1 {expected}")), "{response}");
        assert!(response.to_lowercase().contains("cache-control: no-store"));
    }
    task.abort();
}
