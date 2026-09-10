//! Remote project sessions are separate from the offline workspace and database.
//! Only server-accepted operations change the shared model. Credentials never
//! enter snapshots, local storage, project files, or diagnostics.
use reqwest::{Client, Method, Url};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::time::Duration;
use systems_modeler_core::{Project, ProjectId};
use systems_modeler_persistence::collaboration::{EditRequest, SharedBddDiagram, SharedEdit};
use tokio::sync::Mutex;
use uuid::Uuid;

const MAX_RESPONSE: usize = 32 * 1024 * 1024;

#[derive(Default)]
pub struct CollaborationState(Mutex<Option<Session>>);

struct Session {
    client: Client,
    base: Url,
    token: String,
    projects: Vec<Grant>,
    snapshot: Option<Snapshot>,
    pending: Option<EditRequest>,
    needs_refresh: bool,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Grant {
    id: ProjectId,
    role: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Snapshot {
    project: Project,
    #[serde(default)]
    diagrams: Vec<SharedBddDiagram>,
    revision: i64,
}

#[derive(Serialize)]
pub struct View {
    projects: Vec<Grant>,
    snapshot: Option<Snapshot>,
    pending: bool,
    needs_refresh: bool,
}

#[derive(Deserialize)]
struct ProjectList {
    projects: Vec<Grant>,
}

#[derive(Deserialize)]
struct Receipt {
    operation_id: Uuid,
    revision: i64,
}

fn server_url(value: &str) -> Result<Url, String> {
    let mut url = Url::parse(value).map_err(|_| "Enter a valid HTTPS server address.")?;
    let local = matches!(url.host_str(), Some("127.0.0.1" | "[::1]"));
    if !(url.scheme() == "https" || (url.scheme() == "http" && local))
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
    {
        return Err("Use an HTTPS server origin; HTTP is allowed only for a loopback test server. Do not include credentials, paths, or query parameters.".into());
    }
    url.set_path("/");
    Ok(url)
}

fn http_client() -> Result<Client, String> {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .map_err(|_| "Could not initialize HTTPS client.".into())
}

impl Session {
    fn view(&self) -> View {
        View {
            projects: self.projects.clone(),
            snapshot: self.snapshot.clone(),
            pending: self.pending.is_some(),
            needs_refresh: self.needs_refresh,
        }
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&EditRequest>,
    ) -> Result<T, (bool, String)> {
        let url = self
            .base
            .join(path)
            .map_err(|_| (false, "Invalid endpoint.".into()))?;
        let mut request = self.client.request(method, url).bearer_auth(&self.token);
        if let Some(body) = body {
            request = request.json(body);
        }
        let mut response = request.send().await.map_err(|_| {
            (true, "Connection failed. Check the server and certificate. If an edit was sent, use Retry pending edit to recover its result.".into())
        })?;
        let status = response.status().as_u16();
        if status != 200 {
            let message = match status {
                401 => "Authentication failed. Reconnect with a valid access token.",
                403 => "Access denied. Your server permissions do not allow this action.",
                409 => {
                    "Project changed or operation conflicts. Refresh, review the latest model, and submit your intended edit again."
                }
                422 => {
                    "The server rejected this shared edit. Check the selected owner, element, diagram, and geometry."
                }
                _ => {
                    "The server could not complete the request. Retry a pending edit before making another change."
                }
            };
            return Err((
                !matches!(status, 400 | 401 | 403 | 404 | 409 | 413 | 415 | 422),
                message.into(),
            ));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| (true, "Response interrupted. Retry any pending edit.".into()))?
        {
            if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE {
                return Err((
                    true,
                    "Server response exceeds the desktop size limit.".into(),
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| {
            (
                true,
                "Invalid server response. Retry any pending edit.".into(),
            )
        })
    }

    async fn refresh(&mut self, id: ProjectId) -> Result<(), String> {
        let snapshot: Snapshot = self
            .request(Method::GET, &format!("v1/projects/{id}"), None)
            .await
            .map_err(|e| e.1)?;
        if snapshot.project.id != id || snapshot.revision < 0 {
            return Err("Server returned an inconsistent project snapshot.".into());
        }
        self.snapshot = Some(snapshot);
        self.needs_refresh = false;
        Ok(())
    }

    async fn submit_pending(&mut self) -> Result<(), String> {
        let request = self.pending.as_ref().ok_or("No pending edit.")?;
        let id = self
            .snapshot
            .as_ref()
            .ok_or("Open a shared project first.")?
            .project
            .id;
        let result: Result<Receipt, _> = self
            .request(
                Method::POST,
                &format!("v1/projects/{id}/operations"),
                Some(request),
            )
            .await;
        match result {
            Ok(receipt)
                if receipt.operation_id == request.operation_id
                    && receipt.revision == request.expected_revision + 1 =>
            {
                self.pending = None;
                self.needs_refresh = true;
                // Do not allow another edit against a stale snapshot if this GET fails.
                self.refresh(id).await.map_err(|_| {
                    "Edit saved. Refresh the project before editing again.".to_string()
                })
            }
            Ok(_) => {
                Err("Unexpected edit receipt. Retry the pending edit to recover its result.".into())
            }
            Err((uncertain, message)) => {
                if !uncertain {
                    self.pending = None;
                    self.needs_refresh = true;
                }
                Err(message)
            }
        }
    }

    async fn edit(&mut self, expected_revision: i64, edit: SharedEdit) -> Result<View, String> {
        if self.pending.is_some() {
            return Err("Retry the pending edit before submitting another change.".into());
        }
        let snapshot = self
            .snapshot
            .as_ref()
            .ok_or("Open a shared project first.")?;
        let project_id = snapshot.project.id;
        if self.needs_refresh
            || snapshot.revision != expected_revision
            || expected_revision == i64::MAX
        {
            return Err("Refresh and review the project before editing.".into());
        }
        if !self
            .projects
            .iter()
            .any(|g| g.id == project_id && g.role == "editor")
        {
            return Err("This project is view-only.".into());
        }
        self.pending = Some(EditRequest {
            operation_id: Uuid::new_v4(),
            expected_revision,
            edit,
        });
        self.submit_pending().await?;
        Ok(self.view())
    }
}

#[tauri::command]
pub async fn collaboration_connect(
    state: tauri::State<'_, CollaborationState>,
    server: String,
    token: String,
) -> Result<View, String> {
    let mut state = state.0.lock().await;
    if state.as_ref().is_some_and(|s| s.pending.is_some()) {
        return Err("Resolve the pending edit before reconnecting.".into());
    }
    if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Enter the 64-character access token supplied by your administrator.".into());
    }
    let mut session = Session {
        client: http_client()?,
        base: server_url(&server)?,
        token,
        projects: Vec::new(),
        snapshot: None,
        pending: None,
        needs_refresh: false,
    };
    let list: ProjectList = session
        .request(Method::GET, "v1/projects", None)
        .await
        .map_err(|e| e.1)?;
    session.projects = list.projects;
    let view = session.view();
    *state = Some(session);
    Ok(view)
}

#[tauri::command]
pub async fn collaboration_open(
    state: tauri::State<'_, CollaborationState>,
    project: ProjectId,
) -> Result<View, String> {
    let mut guard = state.0.lock().await;
    let session = guard.as_mut().ok_or("Connect to a server first.")?;
    if session.pending.is_some() {
        return Err("Resolve the pending edit before opening or refreshing a project.".into());
    }
    if !session.projects.iter().any(|g| g.id == project) {
        return Err("Project access denied.".into());
    }
    session.refresh(project).await?;
    Ok(session.view())
}

#[tauri::command]
pub async fn collaboration_edit(
    state: tauri::State<'_, CollaborationState>,
    expected_revision: i64,
    edit: SharedEdit,
) -> Result<View, String> {
    let mut guard = state.0.lock().await;
    guard
        .as_mut()
        .ok_or("Connect to a server first.")?
        .edit(expected_revision, edit)
        .await
}

#[tauri::command]
pub async fn collaboration_retry(
    state: tauri::State<'_, CollaborationState>,
) -> Result<View, String> {
    let mut guard = state.0.lock().await;
    let session = guard.as_mut().ok_or("Connect to a server first.")?;
    session.submit_pending().await?;
    Ok(session.view())
}

#[tauri::command]
pub async fn collaboration_disconnect(
    state: tauri::State<'_, CollaborationState>,
) -> Result<(), String> {
    let mut guard = state.0.lock().await;
    if guard.as_ref().is_some_and(|s| s.pending.is_some()) {
        return Err(
            "Retry the pending edit before disconnecting; its result is not yet known.".into(),
        );
    }
    *guard = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_address_boundary() {
        for address in [
            "https://models.example",
            "http://127.0.0.1:4783",
            "http://[::1]:4783",
        ] {
            assert!(server_url(address).is_ok(), "{address}");
        }
        for address in [
            "http://models.example",
            "http://localhost:4783",
            "file:///tmp/model",
            "https://user:secret@models.example",
            "https://models.example/path",
            "https://models.example?token=secret",
            "https://models.example#fragment",
        ] {
            assert!(server_url(address).is_err(), "{address}");
        }
    }
}

#[tauri::command]
pub async fn collaboration_status(
    state: tauri::State<'_, CollaborationState>,
) -> Result<View, String> {
    let guard = state.0.lock().await;
    Ok(guard.as_ref().ok_or("Connect to a server first.")?.view())
}

#[cfg(test)]
mod transport_tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::Arc,
        thread,
    };
    use systems_modeler_core::DiagramId;
    use systems_modeler_persistence::ProjectDatabase;
    use systems_modeler_server::{
        Config, Credential, Grant as ServerGrant, Role, Service, serve, token_hash,
    };

    fn mock(responses: Vec<(u16, String)>) -> (Url, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = server_url(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
        let handle = thread::spawn(move || {
            let mut requests = Vec::new();
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0; 4096];
                loop {
                    let count = stream.read(&mut buffer).unwrap();
                    if count == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&buffer[..count]);
                    let text = String::from_utf8_lossy(&bytes);
                    if let Some(end) = text.find("\r\n\r\n") {
                        let length: usize = text[..end]
                            .lines()
                            .find_map(|line| {
                                line.to_ascii_lowercase()
                                    .strip_prefix("content-length:")
                                    .map(|s| s.trim().parse().unwrap())
                            })
                            .unwrap_or(0);
                        if bytes.len() >= end + 4 + length {
                            break;
                        }
                    }
                }
                requests.push(String::from_utf8(bytes).unwrap());
                write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
            requests
        });
        (base, handle)
    }

    fn session(base: Url, project: Project, request: EditRequest) -> Session {
        Session {
            client: Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .no_proxy()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
            base,
            token: "a".repeat(64),
            projects: Vec::new(),
            snapshot: Some(Snapshot {
                project,
                diagrams: Vec::new(),
                revision: 0,
            }),
            pending: Some(request),
            needs_refresh: false,
        }
    }

    fn blank_session(base: Url, token: String) -> Session {
        Session {
            client: Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .no_proxy()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
            base,
            token,
            projects: Vec::new(),
            snapshot: None,
            pending: None,
            needs_refresh: false,
        }
    }

    async fn connect_test_session(mut session: Session, project: ProjectId) -> Session {
        let list: ProjectList = session
            .request(Method::GET, "v1/projects", None)
            .await
            .unwrap();
        session.projects = list.projects;
        session.refresh(project).await.unwrap();
        session
    }

    #[test]
    fn retry_preserves_exact_operation_and_refreshes_after_receipt() {
        tauri::async_runtime::block_on(async {
            let project = Project::new("Shared");
            let request = EditRequest {
                operation_id: Uuid::new_v4(),
                expected_revision: 0,
                edit: SharedEdit::CreateBlock {
                    owner: project.root_id,
                    name: "Motor".into(),
                },
            };
            let receipt = serde_json::json!({"operation_id": request.operation_id, "revision": 1})
                .to_string();
            let snapshot = serde_json::json!({"project": project, "revision": 1}).to_string();
            let (base, server) = mock(vec![(503, "{}".into()), (200, receipt), (200, snapshot)]);
            let mut session = session(base, project, request);
            assert!(session.submit_pending().await.is_err());
            assert!(session.pending.is_some());
            session.submit_pending().await.unwrap();
            assert!(session.pending.is_none());
            assert_eq!(session.snapshot.as_ref().unwrap().revision, 1);
            let requests = server.join().unwrap();
            assert_eq!(requests[0], requests[1]);
            assert!(
                requests[0]
                    .to_ascii_lowercase()
                    .contains("authorization: bearer ")
            );
            let view = serde_json::to_string(&session.view()).unwrap();
            assert!(!view.contains(&"a".repeat(64)));
        });
    }

    #[test]
    fn conflict_requires_explicit_refresh_and_preserves_snapshot() {
        tauri::async_runtime::block_on(async {
            let project = Project::new("Shared");
            let request = EditRequest {
                operation_id: Uuid::new_v4(),
                expected_revision: 0,
                edit: SharedEdit::RenameElement {
                    element: project.root_id,
                    name: "Changed".into(),
                },
            };
            let (base, server) = mock(vec![(409, "{}".into())]);
            let mut session = session(base, project, request);
            assert!(
                session
                    .submit_pending()
                    .await
                    .err()
                    .expect("stale edit must be rejected")
                    .contains("Refresh")
            );
            assert!(session.pending.is_none());
            assert!(session.needs_refresh);
            assert_eq!(session.snapshot.as_ref().unwrap().project.name, "Shared");
            server.join().unwrap();
        });
    }

    #[test]
    fn accepted_edit_with_failed_refresh_cannot_be_resubmitted() {
        tauri::async_runtime::block_on(async {
            let project = Project::new("Shared");
            let request = EditRequest {
                operation_id: Uuid::new_v4(),
                expected_revision: 0,
                edit: SharedEdit::CreatePackage {
                    owner: project.root_id,
                    name: "Systems".into(),
                },
            };
            let receipt = serde_json::json!({"operation_id": request.operation_id, "revision": 1})
                .to_string();
            let (base, server) = mock(vec![(200, receipt), (503, "{}".into())]);
            let mut session = session(base, project, request);
            assert!(
                session
                    .submit_pending()
                    .await
                    .unwrap_err()
                    .contains("Edit saved")
            );
            assert!(session.pending.is_none());
            assert!(session.needs_refresh);
            server.join().unwrap();
        });
    }

    #[test]
    fn two_independent_clients_converge_after_conflict_and_bdd_edits() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut database = ProjectDatabase::open_in_memory().unwrap();
            let project = Project::new("Shared");
            database.save_project(&project).unwrap();
            let first_token = "c".repeat(64);
            let second_token = "d".repeat(64);
            let credentials = vec![
                Credential {
                    actor: Uuid::new_v4(),
                    token_sha256: token_hash(&first_token),
                    projects: vec![ServerGrant {
                        project: project.id,
                        role: Role::Editor,
                    }],
                },
                Credential {
                    actor: Uuid::new_v4(),
                    token_sha256: token_hash(&second_token),
                    projects: vec![ServerGrant {
                        project: project.id,
                        role: Role::Editor,
                    }],
                },
            ];
            let service = Arc::new(Service::new(database, Config { credentials }).unwrap());
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base = server_url(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
            let server = tokio::spawn(serve(listener, service));

            let mut first =
                connect_test_session(blank_session(base.clone(), first_token), project.id).await;
            let mut second =
                connect_test_session(blank_session(base, second_token), project.id).await;
            assert_eq!(first.snapshot.as_ref().unwrap().revision, 0);
            assert_eq!(second.snapshot.as_ref().unwrap().revision, 0);

            first
                .edit(
                    0,
                    SharedEdit::CreateBlock {
                        owner: project.root_id,
                        name: "Motor".into(),
                    },
                )
                .await
                .unwrap();
            assert!(
                second
                    .edit(
                        0,
                        SharedEdit::CreatePackage {
                            owner: project.root_id,
                            name: "Stale".into(),
                        },
                    )
                    .await
                    .err()
                    .expect("stale edit must be rejected")
                    .contains("Refresh")
            );
            assert!(second.needs_refresh);
            second.refresh(project.id).await.unwrap();
            let block = second
                .snapshot
                .as_ref()
                .unwrap()
                .project
                .elements
                .values()
                .find(|element| element.name == "Motor")
                .unwrap()
                .id;
            assert_eq!(second.snapshot.as_ref().unwrap().revision, 1);

            let diagram = DiagramId::new();
            second
                .edit(
                    1,
                    SharedEdit::CreateBddDiagram {
                        diagram,
                        owner: project.root_id,
                        name: "Structure".into(),
                    },
                )
                .await
                .unwrap();
            let node = Uuid::new_v4();
            second
                .edit(
                    2,
                    SharedEdit::PlaceBddElement {
                        diagram,
                        node,
                        element: block,
                    },
                )
                .await
                .unwrap();
            second
                .edit(
                    3,
                    SharedEdit::UpdateBddNodeGeometry {
                        diagram,
                        node,
                        x: 400.0,
                        y: 260.0,
                        width: 220.0,
                        height: 140.0,
                    },
                )
                .await
                .unwrap();

            first.refresh(project.id).await.unwrap();
            let snapshot = first.snapshot.as_ref().unwrap();
            assert_eq!(snapshot.revision, 4);
            assert_eq!(snapshot.diagrams.len(), 1);
            assert_eq!(snapshot.diagrams[0].id, diagram);
            assert_eq!(snapshot.diagrams[0].nodes.len(), 1);
            assert_eq!(snapshot.diagrams[0].nodes[0].element, block);
            assert_eq!(snapshot.diagrams[0].nodes[0].x, 400.0);
            assert_eq!(snapshot.diagrams[0].nodes[0].height, 140.0);
            server.abort();
        });
    }
}
