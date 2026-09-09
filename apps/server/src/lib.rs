//! Authenticated HTTP adapter over the existing shared-project authority.
use bytes::Bytes;
use http_body_util::{BodyExt, Full, Limited};
use hyper::{Method, Request, Response, StatusCode, body::Incoming, service::service_fn};
use hyper_util::rt::TokioIo;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    convert::Infallible,
    io,
    sync::{Arc, Mutex},
    time::Duration,
};
use systems_modeler_core::ProjectId;
use systems_modeler_persistence::{
    ProjectDatabase,
    collaboration::{CollaborationError, EditRequest, ProjectRole},
};
use tokio::{net::TcpListener, sync::Semaphore};
use uuid::Uuid;

pub const MAX_BODY: usize = 16 * 1024;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Viewer,
    Editor,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    pub project: ProjectId,
    pub role: Role,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Credential {
    pub actor: Uuid,
    pub token_sha256: String,
    pub projects: Vec<Grant>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub credentials: Vec<Credential>,
}

pub fn token_hash(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

pub struct Service {
    database: Mutex<ProjectDatabase>,
    credentials: Vec<Credential>,
}

impl Service {
    /// Configuration is trusted local administration, never an HTTP request.
    pub fn new(mut database: ProjectDatabase, config: Config) -> Result<Self, String> {
        if config.credentials.is_empty() {
            return Err("at least one credential is required".into());
        }
        let mut actors = HashSet::new();
        let mut hashes = HashSet::new();
        for credential in &config.credentials {
            if credential.actor.is_nil()
                || !actors.insert(credential.actor)
                || credential.token_sha256.len() != 64
                || !credential
                    .token_sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                || !hashes.insert(credential.token_sha256.clone())
            {
                return Err("invalid or duplicate credential configuration".into());
            }
            let mut projects = HashSet::new();
            for grant in &credential.projects {
                if !projects.insert(grant.project) {
                    return Err("duplicate project grant".into());
                }
                database
                    .load_project(grant.project)
                    .map_err(|_| "configured project does not exist")?;
            }
        }
        for credential in &config.credentials {
            for grant in &credential.projects {
                let role = match grant.role {
                    Role::Viewer => ProjectRole::Viewer,
                    Role::Editor => ProjectRole::Editor,
                };
                database
                    .provision_shared_member(grant.project, credential.actor, role)
                    .map_err(|_| "could not provision project membership")?;
            }
        }
        Ok(Self {
            database: Mutex::new(database),
            credentials: config.credentials,
        })
    }

    fn authenticate(&self, authorization: Option<&str>) -> Option<&Credential> {
        let token = authorization?.strip_prefix("Bearer ")?;
        if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let digest = token_hash(token);
        self.credentials
            .iter()
            .find(|c| c.token_sha256 == digest)
    }

    /// Method/path/body adapter shared by the network service and contract tests.
    pub fn dispatch(
        &self,
        method: &str,
        path: &str,
        authorization: Option<&str>,
        body: &[u8],
    ) -> (u16, Value) {
        let Some(credential) = self.authenticate(authorization) else {
            return (401, json!({"error":"unauthorized"}));
        };
        if body.len() > MAX_BODY {
            return (413, json!({"error":"request_too_large"}));
        }
        if method == "GET" && path == "/v1/projects" {
            return (
                200,
                json!({"projects": credential.projects.iter().map(|g| json!({"id":g.project, "role":match g.role { Role::Viewer => "viewer", Role::Editor => "editor" }})).collect::<Vec<_>>() }),
            );
        }
        let segments: Vec<_> = path.split('/').collect();
        if segments.len() < 4
            || !segments[0].is_empty()
            || segments[1] != "v1"
            || segments[2] != "projects"
        {
            return (404, json!({"error":"not_found"}));
        }
        let Ok(id) = Uuid::parse_str(segments[3]) else {
            return (400, json!({"error":"invalid_project_id"}));
        };
        let project = ProjectId(id);
        let Some(grant) = credential.projects.iter().find(|g| g.project == project) else {
            return (403, json!({"error":"forbidden"}));
        };
        let Ok(database) = self.database.lock() else {
            return (503, json!({"error":"unavailable"}));
        };
        if method == "GET" && segments.len() == 4 {
            return match database.shared_snapshot(project, credential.actor) {
                Ok((model, diagrams, revision)) => (
                    200,
                    json!({"project":model,"diagrams":diagrams,"revision":revision}),
                ),
                Err(error) => failure(error),
            };
        }
        if method == "POST" && segments.len() == 5 && segments[4] == "operations" {
            if !matches!(grant.role, Role::Editor) {
                return (403, json!({"error":"forbidden"}));
            }
            let Ok(request) = serde_json::from_slice::<EditRequest>(body) else {
                return (400, json!({"error":"invalid_request"}));
            };
            return match database.commit_shared_edit(project, credential.actor, &request) {
                Ok(receipt) => (
                    200,
                    json!({"revision":receipt.revision,"element":receipt.element,"operation_id":request.operation_id}),
                ),
                Err(error) => failure(error),
            };
        }
        (404, json!({"error":"not_found"}))
    }
}

fn failure(error: CollaborationError) -> (u16, Value) {
    match error {
        CollaborationError::Forbidden => (403, json!({"error":"forbidden"})),
        CollaborationError::Conflict { expected, current } => (
            409,
            json!({"error":"revision_conflict","expected":expected,"current":current}),
        ),
        CollaborationError::OperationIdReused => {
            (409, json!({"error":"operation_id_reused"}))
        }
        CollaborationError::Model(_)
        | CollaborationError::InvalidName
        | CollaborationError::DiagramNotFound
        | CollaborationError::InvalidDiagram(_) => {
            (422, json!({"error":"invalid_model_edit"}))
        }
        _ => (500, json!({"error":"storage_failure"})),
    }
}

fn response(status: u16, body: Value) -> Response<Full<Bytes>> {
    let mut response = Response::new(Full::new(Bytes::from(body.to_string())));
    *response.status_mut() =
        StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    response
        .headers_mut()
        .insert("content-type", "application/json".parse().unwrap());
    response
        .headers_mut()
        .insert("cache-control", "no-store".parse().unwrap());
    response
        .headers_mut()
        .insert("x-content-type-options", "nosniff".parse().unwrap());
    response
}

async fn handle(
    service: Arc<Service>,
    request: Request<Incoming>,
    permit: Arc<tokio::sync::OwnedSemaphorePermit>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    if request.headers().contains_key("origin") {
        return Ok(response(
            403,
            json!({"error":"browser_origin_not_allowed"}),
        ));
    }
    let authorization = request
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .map(str::to_owned);
    if request.headers().get_all("authorization").iter().count() != 1
        || service.authenticate(authorization.as_deref()).is_none()
    {
        return Ok(response(401, json!({"error":"unauthorized"})));
    }
    if request.uri().query().is_some() {
        return Ok(response(400, json!({"error":"query_not_supported"})));
    }
    if request.method() == Method::POST
        && request
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            != Some("application/json")
    {
        return Ok(response(415, json!({"error":"expected_json"})));
    }
    let method = request.method().to_string();
    let path = request.uri().path().to_owned();
    let collected = tokio::time::timeout(
        Duration::from_secs(5),
        Limited::new(request.into_body(), MAX_BODY).collect(),
    )
    .await;
    let body = match collected {
        Ok(Ok(body)) => body.to_bytes(),
        Ok(Err(_)) => {
            return Ok(response(
                413,
                json!({"error":"invalid_or_oversized_body"}),
            ));
        }
        Err(_) => return Ok(response(408, json!({"error":"request_timeout"}))),
    };
    let result = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        service.dispatch(&method, &path, authorization.as_deref(), &body)
    })
    .await;
    let (status, body) = result.unwrap_or((503, json!({"error":"unavailable"})));
    Ok(response(status, body))
}

/// Loopback only. Remote deployment requires an HTTPS reverse proxy on this host.
pub async fn serve(listener: TcpListener, service: Arc<Service>) -> io::Result<()> {
    if !listener.local_addr()?.ip().is_loopback() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "only loopback listeners are permitted",
        ));
    }
    let slots = Arc::new(Semaphore::new(32));
    loop {
        let permit = slots
            .clone()
            .acquire_owned()
            .await
            .map_err(io::Error::other)?;
        let (stream, _) = listener.accept().await?;
        let service = service.clone();
        tokio::spawn(async move {
            let permit = Arc::new(permit);
            let handler =
                service_fn(move |request| handle(service.clone(), request, permit.clone()));
            let mut builder = hyper::server::conn::http1::Builder::new();
            builder.keep_alive(false).max_buf_size(32 * 1024);
            let connection = builder.serve_connection(TokioIo::new(stream), handler);
            let _ = tokio::time::timeout(Duration::from_secs(15), connection).await;
        });
    }
}
