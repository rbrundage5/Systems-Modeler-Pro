use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpStream,
    sync::Arc,
    time::Duration,
};
use systems_modeler_core::{Project, RelationshipKind};
use systems_modeler_persistence::ProjectDatabase;
use systems_modeler_server::{
    Config, Credential, Grant, MAX_BODY, Role, Service, serve, token_hash,
};
use uuid::Uuid;

fn fixture() -> (Arc<Service>, Project, String, String) {
    let mut database = ProjectDatabase::open_in_memory().unwrap();
    let project = Project::new("Shared");
    database.save_project(&project).unwrap();
    let editor = "a".repeat(64);
    let viewer = "b".repeat(64);
    let credentials = vec![
        Credential {
            actor: Uuid::new_v4(),
            token_sha256: token_hash(&editor),
            projects: vec![Grant {
                project: project.id,
                role: Role::Editor,
            }],
        },
        Credential {
            actor: Uuid::new_v4(),
            token_sha256: token_hash(&viewer),
            projects: vec![Grant {
                project: project.id,
                role: Role::Viewer,
            }],
        },
    ];
    (
        Arc::new(Service::new(database, Config { credentials }).unwrap()),
        project,
        format!("Bearer {editor}"),
        format!("Bearer {viewer}"),
    )
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
    assert_eq!(
        service.dispatch("POST", &edits, Some(&viewer), &body).0,
        403
    );
    let receipt = service.dispatch("POST", &edits, Some(&editor), &body);
    assert_eq!(receipt.0, 200);
    assert_eq!(
        service.dispatch("POST", &edits, Some(&editor), &body),
        receipt
    );
    let other = serde_json::to_vec(&operation(&project)).unwrap();
    assert_eq!(
        service.dispatch("POST", &edits, Some(&editor), &other).0,
        409
    );
    let snapshot = service.dispatch("GET", &path, Some(&viewer), &[]);
    assert_eq!(snapshot.1["revision"], 1);
    assert_eq!(
        snapshot.1["project"]["elements"].as_object().unwrap().len(),
        2
    );
    let unknown = format!("/v1/projects/{}", Uuid::new_v4());
    assert_eq!(service.dispatch("GET", &unknown, Some(&editor), &[]).0, 403);
}

#[test]
fn malformed_oversized_and_forged_identity_requests_are_rejected() {
    let (service, project, editor, _) = fixture();
    let path = format!("/v1/projects/{}/operations", project.id);
    assert_eq!(
        service.dispatch("POST", &path, Some(&editor), b"garbage").0,
        400
    );
    assert_eq!(
        service
            .dispatch("POST", &path, Some(&editor), &vec![0; MAX_BODY + 1])
            .0,
        413
    );
    let mut forged = operation(&project);
    forged["actor"] = json!(Uuid::new_v4());
    assert_eq!(
        service
            .dispatch(
                "POST",
                &path,
                Some(&editor),
                &serde_json::to_vec(&forged).unwrap()
            )
            .0,
        400
    );
    let revision = service.dispatch(
        "GET",
        &format!("/v1/projects/{}", project.id),
        Some(&editor),
        &[],
    );
    assert_eq!(revision.1["revision"], 0);
}

#[test]
fn real_http_connection_requires_authentication() {
    let (service, _, editor, _) = fixture();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let address = listener.local_addr().unwrap();
    let task = runtime.spawn(serve(listener, service));
    for (header, expected) in [
        (String::new(), "401"),
        (format!("Authorization: {editor}\r\n"), "200"),
    ] {
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        write!(
            stream,
            "GET /v1/projects HTTP/1.1\r\nHost: localhost\r\n{header}Connection: close\r\n\r\n"
        )
        .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(
            response.starts_with(&format!("HTTP/1.1 {expected}")),
            "{response}"
        );
        assert!(response.to_lowercase().contains("cache-control: no-store"));
    }
    task.abort();
}

#[test]
fn removed_grants_are_denied_after_restart() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("server.sqlite");
    let project = Project::new("Shared");
    let actor = Uuid::new_v4();
    let token = "c".repeat(64);
    let authorization = format!("Bearer {token}");
    let mut database = ProjectDatabase::open(&path).unwrap();
    database.save_project(&project).unwrap();
    let credential = Credential {
        actor,
        token_sha256: token_hash(&token),
        projects: vec![Grant {
            project: project.id,
            role: Role::Editor,
        }],
    };
    let service = Service::new(
        database,
        Config {
            credentials: vec![credential.clone()],
        },
    )
    .unwrap();
    let route = format!("/v1/projects/{}", project.id);
    assert_eq!(
        service.dispatch("GET", &route, Some(&authorization), &[]).0,
        200
    );
    drop(service);
    let mut revoked = credential;
    revoked.projects.clear();
    let service = Service::new(
        ProjectDatabase::open(&path).unwrap(),
        Config {
            credentials: vec![revoked],
        },
    )
    .unwrap();
    assert_eq!(
        service.dispatch("GET", &route, Some(&authorization), &[]).0,
        403
    );
}

#[test]
fn non_loopback_listener_is_rejected() {
    let (service, _, _, _) = fixture();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind("0.0.0.0:0"))
        .unwrap();
    let error = runtime.block_on(serve(listener, service)).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
}

#[test]
fn relationship_edits_use_the_authenticated_revisioned_http_contract() {
    let (service, project, editor, viewer) = fixture();
    let path = format!("/v1/projects/{}", project.id);
    let edits = format!("{path}/operations");
    let first = json!({"operation_id":Uuid::new_v4(),"expected_revision":0,"edit":{"CreateBlock":{"owner":project.root_id,"name":"Base"}}});
    let second = json!({"operation_id":Uuid::new_v4(),"expected_revision":1,"edit":{"CreateBlock":{"owner":project.root_id,"name":"Derived"}}});
    assert_eq!(
        service
            .dispatch(
                "POST",
                &edits,
                Some(&editor),
                &serde_json::to_vec(&first).unwrap()
            )
            .0,
        200
    );
    assert_eq!(
        service
            .dispatch(
                "POST",
                &edits,
                Some(&editor),
                &serde_json::to_vec(&second).unwrap()
            )
            .0,
        200
    );
    let snapshot = service.dispatch("GET", &path, Some(&viewer), &[]).1;
    let elements = snapshot["project"]["elements"].as_object().unwrap();
    let id_for = |name: &str| {
        elements
            .values()
            .find(|element| element["name"] == name)
            .unwrap()["id"]
            .clone()
    };
    let relationship = json!({
        "operation_id": Uuid::new_v4(),
        "expected_revision": 2,
        "edit": {"CreateRelationship": {
            "kind": "Generalization",
            "source": id_for("Derived"),
            "target": id_for("Base"),
            "owner": project.root_id
        }}
    });
    assert_eq!(
        service
            .dispatch(
                "POST",
                &edits,
                Some(&viewer),
                &serde_json::to_vec(&relationship).unwrap()
            )
            .0,
        403
    );
    let receipt = service.dispatch(
        "POST",
        &edits,
        Some(&editor),
        &serde_json::to_vec(&relationship).unwrap(),
    );
    assert_eq!(receipt.0, 200);
    let snapshot = service.dispatch("GET", &path, Some(&viewer), &[]).1;
    assert_eq!(snapshot["revision"], 3);
    let relationships = snapshot["project"]["relationships"].as_object().unwrap();
    assert_eq!(relationships.len(), 1);
    assert_eq!(
        relationships.values().next().unwrap()["kind"],
        serde_json::to_value(RelationshipKind::Generalization).unwrap()
    );
}

#[test]
fn bdd_relationship_presentations_use_authenticated_server_routing() {
    let (service, project, editor, viewer) = fixture();
    let path = format!("/v1/projects/{}", project.id);
    let edits = format!("{path}/operations");
    let post = |authorization: &str, body: &Value| {
        service.dispatch(
            "POST",
            &edits,
            Some(authorization),
            &serde_json::to_vec(body).unwrap(),
        )
    };
    for (revision, name) in [(0, "Source"), (1, "Target")] {
        assert_eq!(
            post(
                &editor,
                &json!({
                    "operation_id": Uuid::new_v4(),
                    "expected_revision": revision,
                    "edit": {"CreateBlock": {"owner": project.root_id, "name": name}}
                }),
            )
            .0,
            200
        );
    }
    let snapshot = service.dispatch("GET", &path, Some(&editor), &[]).1;
    let elements = snapshot["project"]["elements"].as_object().unwrap();
    let id_for = |name: &str| {
        elements
            .values()
            .find(|element| element["name"] == name)
            .unwrap()["id"]
            .clone()
    };
    assert_eq!(
        post(
            &editor,
            &json!({
                "operation_id": Uuid::new_v4(),
                "expected_revision": 2,
                "edit": {"CreateRelationship": {
                    "kind": "Dependency",
                    "source": id_for("Source"),
                    "target": id_for("Target"),
                    "owner": project.root_id
                }}
            }),
        )
        .0,
        200
    );
    let snapshot = service.dispatch("GET", &path, Some(&editor), &[]).1;
    let relationship = snapshot["project"]["relationships"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap()["id"]
        .clone();
    let diagram = Uuid::new_v4();
    assert_eq!(
        post(
            &editor,
            &json!({
                "operation_id": Uuid::new_v4(),
                "expected_revision": 3,
                "edit": {"CreateBddDiagram": {
                    "diagram": diagram,
                    "owner": project.root_id,
                    "name": "Structure"
                }}
            }),
        )
        .0,
        200
    );
    for (revision, node, element) in [
        (4, Uuid::new_v4(), id_for("Source")),
        (5, Uuid::new_v4(), id_for("Target")),
    ] {
        assert_eq!(
            post(
                &editor,
                &json!({
                    "operation_id": Uuid::new_v4(),
                    "expected_revision": revision,
                    "edit": {"PlaceBddElement": {
                        "diagram": diagram,
                        "node": node,
                        "element": element
                    }}
                }),
            )
            .0,
            200
        );
    }
    let edge = json!({
        "operation_id": Uuid::new_v4(),
        "expected_revision": 6,
        "edit": {"PresentBddRelationship": {
            "diagram": diagram,
            "edge": Uuid::new_v4(),
            "relationship": relationship
        }}
    });
    assert_eq!(post(&viewer, &edge).0, 403);
    assert_eq!(post(&editor, &edge).0, 200);
    let snapshot = service.dispatch("GET", &path, Some(&viewer), &[]).1;
    assert_eq!(snapshot["revision"], 7);
    let presented = &snapshot["diagrams"][0]["edges"][0];
    assert_eq!(presented["relationship"], relationship);
    assert!(presented["points"].as_array().unwrap().len() >= 2);
    assert_eq!(
        post(
            &editor,
            &json!({
                "operation_id": Uuid::new_v4(),
                "expected_revision": 7,
                "edit": {"DeleteRelationship": {"relationship": relationship}}
            }),
        )
        .0,
        200
    );
    let snapshot = service.dispatch("GET", &path, Some(&viewer), &[]).1;
    assert_eq!(snapshot["revision"], 8);
    assert!(
        snapshot["project"]["relationships"]
            .as_object()
            .unwrap()
            .is_empty()
    );
    assert!(
        snapshot["diagrams"][0]["edges"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
