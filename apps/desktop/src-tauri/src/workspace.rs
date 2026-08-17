use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use systems_modeler_core::{DiagramId, ElementId, ElementKind, Project};
use systems_modeler_persistence::ProjectDatabase;

const BDD_METADATA_KEY: &str = "bdd-diagrams";

#[derive(Debug, Clone, Serialize)]
pub struct ElementSnapshot {
    pub id: String,
    pub external_id: String,
    pub kind: String,
    pub name: String,
    pub owner_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectSnapshot {
    pub id: String,
    pub name: String,
    pub root_id: String,
    pub elements: Vec<ElementSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramNode {
    pub id: String,
    pub element_id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BddDiagram {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub nodes: Vec<DiagramNode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceSnapshot {
    pub project: Option<ProjectSnapshot>,
    pub diagrams: Vec<BddDiagram>,
    pub current_file: Option<String>,
}

pub struct WorkspaceState {
    project: Mutex<Option<Project>>,
    diagrams: Mutex<Vec<BddDiagram>>,
    current_file: Mutex<Option<String>>,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self {
            project: Mutex::new(None),
            diagrams: Mutex::new(Vec::new()),
            current_file: Mutex::new(None),
        }
    }
}

fn parse_element_id(value: &str) -> Result<ElementId, String> {
    uuid::Uuid::parse_str(value)
        .map(ElementId)
        .map_err(|_| format!("invalid element id: {value}"))
}

fn parse_diagram_id(value: &str) -> Result<DiagramId, String> {
    uuid::Uuid::parse_str(value)
        .map(DiagramId)
        .map_err(|_| format!("invalid diagram id: {value}"))
}

fn normalize_project_path(value: &str) -> Result<PathBuf, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("project path is required".into());
    }
    let mut path = PathBuf::from(trimmed);
    if path.extension().is_none() {
        path.set_extension("smproj");
    }
    Ok(path)
}

fn snapshot_project(project: &Project) -> ProjectSnapshot {
    let mut elements: Vec<_> = project
        .elements
        .values()
        .map(|element| ElementSnapshot {
            id: element.id.to_string(),
            external_id: element.external_id.clone(),
            kind: format!("{:?}", element.kind),
            name: element.name.clone(),
            owner_id: element.owner_id.map(|id| id.to_string()),
        })
        .collect();
    elements.sort_by(|a, b| a.name.cmp(&b.name));
    ProjectSnapshot {
        id: project.id.to_string(),
        name: project.name.clone(),
        root_id: project.root_id.to_string(),
        elements,
    }
}

fn validate_loaded_diagrams(project: &Project, diagrams: &[BddDiagram]) -> Result<(), String> {
    let mut diagram_ids = HashSet::new();
    let mut node_ids = HashSet::new();
    for diagram in diagrams {
        parse_diagram_id(&diagram.id)?;
        if !diagram_ids.insert(&diagram.id) {
            return Err(format!("duplicate diagram id: {}", diagram.id));
        }
        let owner_id = parse_element_id(&diagram.owner_id)?;
        let owner = project
            .element(owner_id)
            .map_err(|error| error.to_string())?;
        if !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {
            return Err(format!(
                "BDD owner is not a Model or Package: {}",
                diagram.owner_id
            ));
        }
        for node in &diagram.nodes {
            if uuid::Uuid::parse_str(&node.id).is_err() {
                return Err(format!("invalid diagram node id: {}", node.id));
            }
            if !node_ids.insert(&node.id) {
                return Err(format!("duplicate diagram node id: {}", node.id));
            }
            let element_id = parse_element_id(&node.element_id)?;
            let element = project
                .element(element_id)
                .map_err(|error| error.to_string())?;
            if element.kind != ElementKind::Block {
                return Err(format!(
                    "BDD node does not reference a Block: {}",
                    node.element_id
                ));
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn workspace_snapshot(
    state: tauri::State<'_, WorkspaceState>,
) -> Result<WorkspaceSnapshot, String> {
    let project = state.project.lock().map_err(|_| "project lock poisoned")?;
    let diagrams = state.diagrams.lock().map_err(|_| "diagram lock poisoned")?;
    let current_file = state
        .current_file
        .lock()
        .map_err(|_| "project path lock poisoned")?;
    Ok(WorkspaceSnapshot {
        project: project.as_ref().map(snapshot_project),
        diagrams: diagrams.clone(),
        current_file: current_file.clone(),
    })
}

#[tauri::command]
pub fn new_project(name: String, state: tauri::State<'_, WorkspaceState>) -> Result<(), String> {
    let mut project = state.project.lock().map_err(|_| "project lock poisoned")?;
    let mut diagrams = state.diagrams.lock().map_err(|_| "diagram lock poisoned")?;
    let mut current_file = state
        .current_file
        .lock()
        .map_err(|_| "project path lock poisoned")?;
    *project = Some(Project::new(name));
    diagrams.clear();
    *current_file = None;
    Ok(())
}

#[tauri::command]
pub fn save_project_file(
    path: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let path = normalize_project_path(&path)?;
    let project = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project.as_ref().ok_or("no project open")?;
    let diagrams = state.diagrams.lock().map_err(|_| "diagram lock poisoned")?;

    project
        .validate()
        .map_err(|error| format!("project validation failed: {error}"))?;
    validate_loaded_diagrams(project, &diagrams)?;

    let mut database = ProjectDatabase::open(&path).map_err(|error| error.to_string())?;
    database
        .save_project(project)
        .map_err(|error| error.to_string())?;
    let diagram_payload = serde_json::to_string(&*diagrams).map_err(|error| error.to_string())?;
    database
        .save_metadata(project.id, BDD_METADATA_KEY, &diagram_payload)
        .map_err(|error| error.to_string())?;

    let saved_path = path.to_string_lossy().into_owned();
    *state
        .current_file
        .lock()
        .map_err(|_| "project path lock poisoned")? = Some(saved_path.clone());
    Ok(saved_path)
}

#[tauri::command]
pub fn save_current_project(state: tauri::State<'_, WorkspaceState>) -> Result<String, String> {
    let path = state
        .current_file
        .lock()
        .map_err(|_| "project path lock poisoned")?
        .clone()
        .ok_or("project has not been saved yet; use Save As")?;
    save_project_file(path, state)
}

#[tauri::command]
pub fn open_project_file(
    path: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let path = normalize_project_path(&path)?;
    if !path.exists() {
        return Err(format!("project file does not exist: {}", path.display()));
    }

    let database = ProjectDatabase::open(&path).map_err(|error| error.to_string())?;
    let project = database
        .load_first_project()
        .map_err(|error| error.to_string())?;
    project
        .validate()
        .map_err(|error| format!("saved project validation failed: {error}"))?;
    let diagrams = match database
        .load_metadata(project.id, BDD_METADATA_KEY)
        .map_err(|error| error.to_string())?
    {
        Some(payload) => serde_json::from_str::<Vec<BddDiagram>>(&payload)
            .map_err(|error| format!("invalid saved BDD presentation data: {error}"))?,
        None => Vec::new(),
    };
    validate_loaded_diagrams(&project, &diagrams)?;

    let opened_path = path.to_string_lossy().into_owned();
    *state.project.lock().map_err(|_| "project lock poisoned")? = Some(project);
    *state.diagrams.lock().map_err(|_| "diagram lock poisoned")? = diagrams;
    *state
        .current_file
        .lock()
        .map_err(|_| "project path lock poisoned")? = Some(opened_path.clone());
    Ok(opened_path)
}

#[tauri::command]
pub fn create_package(
    owner_id: String,
    name: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    create_element(ElementKind::Package, owner_id, name, state)
}

#[tauri::command]
pub fn create_block(
    owner_id: String,
    name: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    create_element(ElementKind::Block, owner_id, name, state)
}

fn create_element(
    kind: ElementKind,
    owner_id: String,
    name: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let owner_id = parse_element_id(&owner_id)?;
    let mut project = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project.as_mut().ok_or("no project open")?;
    project
        .create_element(kind, name, owner_id)
        .map(|id| id.to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn rename_element(
    element_id: String,
    name: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let element_id = parse_element_id(&element_id)?;
    let mut project = state.project.lock().map_err(|_| "project lock poisoned")?;
    project
        .as_mut()
        .ok_or("no project open")?
        .rename_element(element_id, name)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_bdd(
    owner_id: String,
    name: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let owner_id = parse_element_id(&owner_id)?;
    let project = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project.as_ref().ok_or("no project open")?;
    let owner = project
        .element(owner_id)
        .map_err(|error| error.to_string())?;
    if !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {
        return Err("BDD owner must be a Model or Package in this workflow".into());
    }
    drop(project);

    let id = DiagramId::new();
    state
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .push(BddDiagram {
            id: id.to_string(),
            name,
            owner_id: owner_id.to_string(),
            nodes: Vec::new(),
        });
    Ok(id.to_string())
}

#[tauri::command]
pub fn place_element_on_bdd(
    diagram_id: String,
    element_id: String,
    x: f64,
    y: f64,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let diagram_id = parse_diagram_id(&diagram_id)?;
    let element_id = parse_element_id(&element_id)?;
    let project = state.project.lock().map_err(|_| "project lock poisoned")?;
    let element = project
        .as_ref()
        .ok_or("no project open")?
        .element(element_id)
        .map_err(|error| error.to_string())?;
    if element.kind != ElementKind::Block {
        return Err("only Blocks can be presented on this first BDD slice".into());
    }
    drop(project);

    let mut diagrams = state.diagrams.lock().map_err(|_| "diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id.to_string())
        .ok_or("diagram not found")?;
    if diagram
        .nodes
        .iter()
        .any(|node| node.element_id == element_id.to_string())
    {
        return Err("this Block is already presented on the BDD".into());
    }
    let node_id = uuid::Uuid::new_v4().to_string();
    diagram.nodes.push(DiagramNode {
        id: node_id.clone(),
        element_id: element_id.to_string(),
        x,
        y,
        width: 180.0,
        height: 105.0,
    });
    Ok(node_id)
}
