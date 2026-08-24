use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use systems_modeler_core::{
    DiagramFamilyDescriptor, DiagramFamilyId, GeometryRect, PanelPreference, ViewportPreference,
    fit_viewport, supported_diagram_families, zoom_viewport_at,
};

use super::presentation_theme::{ResolvedDiagramCommand, resolve_diagram_commands};
use tauri::Manager;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedWorkspacePreferences {
    viewports: BTreeMap<String, ViewportPreference>,
    #[serde(default)]
    frames: BTreeMap<String, DiagramFramePreference>,
    panels: PanelPreference,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramFramePreference {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub manually_sized: bool,
}

impl DiagramFramePreference {
    fn validate(&self) -> Result<(), String> {
        if ![self.x, self.y, self.width, self.height]
            .iter()
            .all(|value| value.is_finite())
            || self.width < 320.0
            || self.height < 240.0
            || self.width > 100_000.0
            || self.height > 100_000.0
        {
            return Err("diagram frame preference contains invalid geometry".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveDiagramContext {
    pub diagram_id: String,
    pub family: DiagramFamilyDescriptor,
    pub name: String,
    pub model_element_name: String,
    pub frame_label: String,
    pub semantic_context_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveWorkspaceSnapshot {
    pub context: ActiveDiagramContext,
    pub interaction: WorkspaceInteractionSnapshot,
    pub commands: Vec<ResolvedDiagramCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSelection {
    pub kind: String,
    pub id: String,
}

impl WorkspaceSelection {
    fn validate(&self) -> Result<(), String> {
        if self.kind.trim().is_empty() || self.id.trim().is_empty() {
            return Err("workspace selection kind and id are required".into());
        }
        if self.kind.len() > 64 || self.id.len() > 256 {
            return Err("workspace selection kind or id exceeds the supported length".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInteractionSnapshot {
    pub diagram_id: Option<String>,
    pub selections: Vec<WorkspaceSelection>,
    pub active_tool: Option<String>,
    pub revision: u64,
}

impl WorkspaceInteractionSnapshot {
    fn validate_for(&self, active_diagram_id: &str) -> Result<(), String> {
        if self.diagram_id.as_deref() != Some(active_diagram_id) {
            return Err("workspace interaction does not target the active diagram".into());
        }
        if self.selections.len() > 1024 {
            return Err("workspace selection exceeds the supported size".into());
        }
        for selection in &self.selections {
            selection.validate()?;
        }
        if self
            .active_tool
            .as_ref()
            .is_some_and(|tool| tool.trim().is_empty() || tool.len() > 128)
        {
            return Err("workspace active tool is invalid".into());
        }
        Ok(())
    }

    fn require_revision(&self, expected_revision: Option<u64>) -> Result<(), String> {
        if expected_revision.is_some_and(|revision| revision != self.revision) {
            return Err(format!(
                "workspace interaction revision conflict: expected {}, current {}",
                expected_revision.unwrap_or_default(),
                self.revision
            ));
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct SharedWorkspaceState {
    active: Mutex<Option<ActiveDiagramContext>>,
    interaction: Mutex<WorkspaceInteractionSnapshot>,
    viewports: Mutex<BTreeMap<String, ViewportPreference>>,
    frames: Mutex<BTreeMap<String, DiagramFramePreference>>,
    panels: Mutex<PanelPreference>,
    preferences_loaded: Mutex<bool>,
}

impl SharedWorkspaceState {
    pub(super) fn forget_diagram(&self, diagram_id: &str) -> Result<(), String> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| "active diagram context lock poisoned")?;
        if active
            .as_ref()
            .is_some_and(|context| context.diagram_id == diagram_id)
        {
            *active = None;
        }
        drop(active);

        let mut interaction = self
            .interaction
            .lock()
            .map_err(|_| "workspace interaction lock poisoned")?;
        if interaction.diagram_id.as_deref() == Some(diagram_id) {
            interaction.diagram_id = None;
            interaction.selections.clear();
            interaction.active_tool = None;
            interaction.revision = interaction.revision.saturating_add(1);
        }
        drop(interaction);

        self.viewports
            .lock()
            .map_err(|_| "viewport preference lock poisoned")?
            .remove(diagram_id);
        self.frames
            .lock()
            .map_err(|_| "diagram frame preference lock poisoned")?
            .remove(diagram_id);
        Ok(())
    }
}

fn preference_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join("workspace-preferences.json"))
        .map_err(|error| format!("workspace preference directory is unavailable: {error}"))
}

fn ensure_preferences_loaded(
    app: &tauri::AppHandle,
    state: &SharedWorkspaceState,
) -> Result<(), String> {
    let mut loaded = state
        .preferences_loaded
        .lock()
        .map_err(|_| "workspace preference initialization lock poisoned")?;
    if *loaded {
        return Ok(());
    }
    let path = preference_path(app)?;
    if path.exists() {
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("failed to read workspace preferences: {error}"))?;
        let preferences: PersistedWorkspacePreferences = serde_json::from_slice(&bytes)
            .map_err(|error| format!("workspace preferences are invalid: {error}"))?;
        preferences.panels.validate()?;
        for preference in preferences.viewports.values() {
            preference.validate()?;
        }
        for preference in preferences.frames.values() {
            preference.validate()?;
        }
        *state
            .viewports
            .lock()
            .map_err(|_| "viewport preference lock poisoned")? = preferences.viewports;
        *state
            .frames
            .lock()
            .map_err(|_| "diagram frame preference lock poisoned")? = preferences.frames;
        *state
            .panels
            .lock()
            .map_err(|_| "panel preference lock poisoned")? = preferences.panels;
    }
    *loaded = true;
    Ok(())
}

fn save_preferences(app: &tauri::AppHandle, state: &SharedWorkspaceState) -> Result<(), String> {
    let path = preference_path(app)?;
    let directory = path
        .parent()
        .ok_or("workspace preference path has no parent directory")?;
    std::fs::create_dir_all(directory)
        .map_err(|error| format!("failed to create workspace preference directory: {error}"))?;
    let preferences = PersistedWorkspacePreferences {
        viewports: state
            .viewports
            .lock()
            .map_err(|_| "viewport preference lock poisoned")?
            .clone(),
        frames: state
            .frames
            .lock()
            .map_err(|_| "diagram frame preference lock poisoned")?
            .clone(),
        panels: state
            .panels
            .lock()
            .map_err(|_| "panel preference lock poisoned")?
            .clone(),
    };
    let bytes = serde_json::to_vec_pretty(&preferences)
        .map_err(|error| format!("failed to serialize workspace preferences: {error}"))?;
    std::fs::write(path, bytes)
        .map_err(|error| format!("failed to persist workspace preferences: {error}"))
}

#[tauri::command]
pub fn diagram_family_registry() -> Vec<DiagramFamilyDescriptor> {
    supported_diagram_families().descriptors()
}

#[tauri::command]
pub fn active_diagram_command_manifest(
    state: tauri::State<'_, SharedWorkspaceState>,
) -> Result<Vec<ResolvedDiagramCommand>, String> {
    let active = state
        .active
        .lock()
        .map_err(|_| "active diagram context lock poisoned")?;
    Ok(resolve_diagram_commands(
        active.as_ref().map(|context| &context.family),
    ))
}

fn routing_bounds(
    shared: &SharedWorkspaceState,
    diagram_id: &str,
    supplied: Option<DiagramFramePreference>,
) -> Result<Option<super::routing::RouteRect>, String> {
    let preference = if let Some(preference) = supplied {
        preference.validate()?;
        Some(preference)
    } else {
        shared
            .frames
            .lock()
            .map_err(|_| "diagram frame preference lock poisoned")?
            .get(diagram_id)
            .cloned()
    };
    // An automatic frame follows content and is recalculated after rendering, so it
    // must not act as a hard routing boundary. Only a frame the user explicitly
    // sized represents a committed boundary that Route/Clean Layout must respect.
    Ok(preference
        .filter(|frame| frame.manually_sized)
        .map(|frame| super::routing::RouteRect {
            x: frame.x,
            y: frame.y + 42.0,
            width: frame.width,
            height: (frame.height - 42.0).max(1.0),
        }))
}

fn dispatch_route(
    family: &str,
    diagram_id: &str,
    workspace: &super::WorkspaceState,
    activity: &super::activity_workspace::ActivityWorkspaceState,
    bounds: Option<super::routing::RouteRect>,
) -> Result<bool, String> {
    match family {
        "bdd" | "requirement" | "use-case" => {
            super::route_bdd_with_bounds(diagram_id, workspace, bounds)
        }
        "ibd" => super::ibd::route_ibd_with_bounds(diagram_id, workspace, bounds),
        "state-machine" | "sequence" => {
            super::behavior_workspace::route_behavior_with_bounds(diagram_id, workspace, bounds)
        }
        "activity" => {
            super::activity_mutation::route_activity_with_bounds(diagram_id, activity, bounds)
        }
        family => Err(format!(
            "shared routing geometry is not implemented for {family} yet"
        )),
    }
}

fn dispatch_layout(
    family: &str,
    diagram_id: &str,
    workspace: &super::WorkspaceState,
    activity: &super::activity_workspace::ActivityWorkspaceState,
    bounds: Option<super::routing::RouteRect>,
) -> Result<bool, String> {
    match family {
        "bdd" | "requirement" | "use-case" => {
            super::layout_bdd_with_bounds(diagram_id, workspace, bounds)
        }
        "ibd" => super::ibd::layout_ibd_with_bounds(diagram_id, workspace, bounds),
        "state-machine" | "sequence" => {
            super::behavior_workspace::layout_behavior_with_bounds(diagram_id, workspace, bounds)
        }
        "activity" => {
            super::activity_mutation::layout_activity_with_bounds(diagram_id, activity, bounds)
        }
        family => Err(format!(
            "shared Clean Layout is not implemented for {family}"
        )),
    }
}

fn record_presentation_change(
    workspace: &super::WorkspaceState,
    activity: &super::activity_workspace::ActivityWorkspaceState,
    history: &super::history::HistoryState,
    operation: impl FnOnce() -> Result<bool, String>,
) -> Result<(), String> {
    let before = super::history::capture_states(workspace, activity)?;
    if operation()? {
        super::history::commit_snapshot(before, history)?;
    }
    Ok(())
}

#[tauri::command]
pub fn active_diagram_router(
    diagram_id: String,
    frame_preference: Option<DiagramFramePreference>,
    shared: tauri::State<'_, SharedWorkspaceState>,
    workspace: tauri::State<'_, super::WorkspaceState>,
    activity: tauri::State<'_, super::activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, super::history::HistoryState>,
) -> Result<(), String> {
    let active = shared
        .active
        .lock()
        .map_err(|_| "active diagram context lock poisoned")?
        .clone()
        .ok_or("no active diagram")?;
    if active.diagram_id != diagram_id {
        return Err("routing request does not match the active diagram".into());
    }
    let bounds = routing_bounds(&shared, &diagram_id, frame_preference)?;
    record_presentation_change(&workspace, &activity, &history, || {
        dispatch_route(
            active.family.id.0.as_str(),
            &diagram_id,
            &workspace,
            &activity,
            bounds,
        )
    })
}

#[tauri::command]
pub fn active_diagram_layout(
    diagram_id: String,
    frame_preference: Option<DiagramFramePreference>,
    shared: tauri::State<'_, SharedWorkspaceState>,
    workspace: tauri::State<'_, super::WorkspaceState>,
    activity: tauri::State<'_, super::activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, super::history::HistoryState>,
) -> Result<(), String> {
    let active = shared
        .active
        .lock()
        .map_err(|_| "active diagram context lock poisoned")?
        .clone()
        .ok_or("no active diagram")?;
    if active.diagram_id != diagram_id {
        return Err("layout request does not match the active diagram".into());
    }
    let bounds = routing_bounds(&shared, &diagram_id, frame_preference)?;
    record_presentation_change(&workspace, &activity, &history, || {
        dispatch_layout(
            active.family.id.0.as_str(),
            &diagram_id,
            &workspace,
            &activity,
            bounds,
        )
    })
}

#[tauri::command]
pub fn activate_diagram(
    state: tauri::State<'_, SharedWorkspaceState>,
    diagram_id: String,
    family_id: String,
    name: String,
    model_element_name: Option<String>,
    semantic_context_id: String,
) -> Result<ActiveWorkspaceSnapshot, String> {
    if uuid::Uuid::parse_str(&diagram_id).is_err() {
        return Err("active diagram id is invalid".into());
    }
    let family_id = DiagramFamilyId::new(family_id)?;
    let family = supported_diagram_families()
        .get(&family_id)
        .cloned()
        .ok_or_else(|| format!("unregistered diagram family: {}", family_id.0))?;
    let model_element_name = model_element_name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| name.clone());
    let frame_label = format!(
        "{} [{}] {} [{}]",
        family.frame_abbreviation, family.frame_model_element_type, model_element_name, name
    );
    let context = ActiveDiagramContext {
        diagram_id,
        family,
        name,
        model_element_name,
        frame_label,
        semantic_context_id,
    };
    let changed_diagram = state
        .active
        .lock()
        .map_err(|_| "active diagram context lock poisoned")?
        .as_ref()
        .is_none_or(|active| active.diagram_id != context.diagram_id);
    *state
        .active
        .lock()
        .map_err(|_| "active diagram context lock poisoned")? = Some(context.clone());
    let interaction = if changed_diagram {
        let mut interaction = state
            .interaction
            .lock()
            .map_err(|_| "workspace interaction lock poisoned")?;
        interaction.diagram_id = Some(context.diagram_id.clone());
        interaction.selections.clear();
        interaction.active_tool = None;
        interaction.revision = interaction.revision.saturating_add(1);
        interaction.clone()
    } else {
        state
            .interaction
            .lock()
            .map_err(|_| "workspace interaction lock poisoned")?
            .clone()
    };
    let commands = resolve_diagram_commands(Some(&context.family));
    Ok(ActiveWorkspaceSnapshot {
        context,
        interaction,
        commands,
    })
}

#[tauri::command]
pub fn rename_active_diagram_header(
    diagram_id: String,
    model_element_name: String,
    diagram_name: String,
    state: tauri::State<'_, SharedWorkspaceState>,
    workspace: tauri::State<'_, super::WorkspaceState>,
    activity: tauri::State<'_, super::activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, super::history::HistoryState>,
) -> Result<ActiveDiagramContext, String> {
    let model_element_name = model_element_name.trim();
    let diagram_name = diagram_name.trim();
    if model_element_name.is_empty() || diagram_name.is_empty() {
        return Err("diagram header names cannot be empty".into());
    }
    if model_element_name.chars().count() > 256 || diagram_name.chars().count() > 256 {
        return Err("diagram header names cannot exceed 256 characters".into());
    }
    let active = state
        .active
        .lock()
        .map_err(|_| "active diagram context lock poisoned")?
        .clone()
        .ok_or("no active diagram")?;
    if active.diagram_id != diagram_id {
        return Err("rename request does not match the active diagram".into());
    }
    super::history::checkpoint_states(&workspace, &activity, &history)?;
    match active.family.id.0.as_str() {
        "bdd" => {
            let owner_id = workspace
                .diagrams
                .lock()
                .map_err(|_| "diagram lock poisoned")?
                .iter()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("BDD not found")?
                .owner_id
                .clone();
            workspace
                .project
                .lock()
                .map_err(|_| "project lock poisoned")?
                .as_mut()
                .ok_or("no project open")?
                .rename_element(super::parse_element_id(&owner_id)?, model_element_name)
                .map_err(|error| error.to_string())?;
            workspace
                .diagrams
                .lock()
                .map_err(|_| "diagram lock poisoned")?
                .iter_mut()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("BDD not found")?
                .name = diagram_name.into();
        }
        "use-case" => {
            let context_id = workspace
                .diagrams
                .lock()
                .map_err(|_| "diagram lock poisoned")?
                .iter()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("Use Case Diagram not found")?
                .semantic_context_id
                .clone();
            if let Some(context_id) = context_id {
                workspace
                    .project
                    .lock()
                    .map_err(|_| "project lock poisoned")?
                    .as_mut()
                    .ok_or("no project open")?
                    .rename_element(super::parse_element_id(&context_id)?, model_element_name)
                    .map_err(|error| error.to_string())?;
            }
            workspace
                .diagrams
                .lock()
                .map_err(|_| "diagram lock poisoned")?
                .iter_mut()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("Use Case Diagram not found")?
                .name = diagram_name.into();
        }
        "ibd" => {
            let context_id = workspace
                .ibd_diagrams
                .lock()
                .map_err(|_| "IBD lock poisoned")?
                .iter()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("IBD not found")?
                .context_block_id
                .clone();
            workspace
                .project
                .lock()
                .map_err(|_| "project lock poisoned")?
                .as_mut()
                .ok_or("no project open")?
                .rename_element(super::parse_element_id(&context_id)?, model_element_name)
                .map_err(|error| error.to_string())?;
            workspace
                .ibd_diagrams
                .lock()
                .map_err(|_| "IBD lock poisoned")?
                .iter_mut()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("IBD not found")?
                .name = diagram_name.into();
        }
        "state-machine" | "sequence" => {
            let mut diagrams = workspace
                .behavior_diagrams
                .lock()
                .map_err(|_| "behavior diagram lock poisoned")?;
            let diagram = diagrams
                .iter_mut()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("behavior diagram not found")?;
            let semantic_id = uuid::Uuid::parse_str(&diagram.semantic_id)
                .map_err(|_| "behavior semantic id is invalid")?;
            let mut repository = workspace
                .behavior
                .lock()
                .map_err(|_| "behavior repository lock poisoned")?;
            if active.family.id.0 == "state-machine" {
                repository
                    .state_machines
                    .get_mut(&systems_modeler_core::behavior::StateMachineId(semantic_id))
                    .ok_or("StateMachine not found")?
                    .name = model_element_name.into();
            } else {
                repository
                    .interactions
                    .get_mut(&systems_modeler_core::behavior::InteractionId(semantic_id))
                    .ok_or("Interaction not found")?
                    .name = model_element_name.into();
            }
            diagram.name = diagram_name.into();
        }
        "activity" => {
            let mut diagrams = activity
                .diagrams
                .lock()
                .map_err(|_| "Activity diagram lock poisoned")?;
            let diagram = diagrams
                .iter_mut()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("Activity diagram not found")?;
            let activity_id = super::activity_workspace::parse_activity_id(&diagram.activity_id)?;
            activity
                .repository
                .lock()
                .map_err(|_| "Activity repository lock poisoned")?
                .activities
                .get_mut(&activity_id)
                .ok_or("Activity not found")?
                .name = model_element_name.into();
            diagram.name = diagram_name.into();
        }
        family => {
            return Err(format!(
                "diagram header editing is unavailable for {family}"
            ));
        }
    }
    let mut updated = active;
    updated.name = diagram_name.into();
    updated.model_element_name = model_element_name.into();
    updated.frame_label = format!(
        "{} [{}] {} [{}]",
        updated.family.frame_abbreviation,
        updated.family.frame_model_element_type,
        updated.model_element_name,
        updated.name
    );
    *state
        .active
        .lock()
        .map_err(|_| "active diagram context lock poisoned")? = Some(updated.clone());
    Ok(updated)
}

#[tauri::command]
pub fn workspace_interaction_snapshot(
    state: tauri::State<'_, SharedWorkspaceState>,
) -> Result<WorkspaceInteractionSnapshot, String> {
    state
        .interaction
        .lock()
        .map_err(|_| "workspace interaction lock poisoned".to_string())
        .map(|interaction| interaction.clone())
}

#[tauri::command]
pub fn set_workspace_interaction(
    state: tauri::State<'_, SharedWorkspaceState>,
    diagram_id: String,
    selections: Vec<WorkspaceSelection>,
    active_tool: Option<String>,
    expected_revision: Option<u64>,
) -> Result<WorkspaceInteractionSnapshot, String> {
    let active_diagram_id = state
        .active
        .lock()
        .map_err(|_| "active diagram context lock poisoned")?
        .as_ref()
        .map(|active| active.diagram_id.clone())
        .ok_or("no active diagram")?;
    let next = WorkspaceInteractionSnapshot {
        diagram_id: Some(diagram_id),
        selections,
        active_tool,
        revision: 0,
    };
    next.validate_for(&active_diagram_id)?;
    let mut interaction = state
        .interaction
        .lock()
        .map_err(|_| "workspace interaction lock poisoned")?;
    interaction.require_revision(expected_revision)?;
    interaction.diagram_id = next.diagram_id;
    interaction.selections = next.selections;
    interaction.active_tool = next.active_tool;
    interaction.revision = interaction.revision.saturating_add(1);
    Ok(interaction.clone())
}

#[tauri::command]
pub fn clear_workspace_interaction(
    state: tauri::State<'_, SharedWorkspaceState>,
    diagram_id: String,
    cancel_tool: bool,
    expected_revision: Option<u64>,
) -> Result<WorkspaceInteractionSnapshot, String> {
    let active_diagram_id = state
        .active
        .lock()
        .map_err(|_| "active diagram context lock poisoned")?
        .as_ref()
        .map(|active| active.diagram_id.clone())
        .ok_or("no active diagram")?;
    if diagram_id != active_diagram_id {
        return Err("workspace interaction does not target the active diagram".into());
    }
    let mut interaction = state
        .interaction
        .lock()
        .map_err(|_| "workspace interaction lock poisoned")?;
    interaction.require_revision(expected_revision)?;
    interaction.diagram_id = Some(diagram_id);
    interaction.selections.clear();
    if cancel_tool {
        interaction.active_tool = None;
    }
    interaction.revision = interaction.revision.saturating_add(1);
    Ok(interaction.clone())
}

#[tauri::command]
pub fn get_viewport_preference(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedWorkspaceState>,
    diagram_id: String,
) -> Result<ViewportPreference, String> {
    ensure_preferences_loaded(&app, &state)?;
    Ok(state
        .viewports
        .lock()
        .map_err(|_| "viewport preference lock poisoned")?
        .get(&diagram_id)
        .cloned()
        .unwrap_or_default())
}

#[tauri::command]
pub fn set_viewport_preference(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedWorkspaceState>,
    diagram_id: String,
    preference: ViewportPreference,
) -> Result<(), String> {
    ensure_preferences_loaded(&app, &state)?;
    if uuid::Uuid::parse_str(&diagram_id).is_err() {
        return Err("viewport diagram id is invalid".into());
    }
    preference.validate()?;
    state
        .viewports
        .lock()
        .map_err(|_| "viewport preference lock poisoned")?
        .insert(diagram_id, preference);
    save_preferences(&app, &state)
}

#[tauri::command]
pub fn get_diagram_frame_preference(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedWorkspaceState>,
    diagram_id: String,
) -> Result<Option<DiagramFramePreference>, String> {
    ensure_preferences_loaded(&app, &state)?;
    Ok(state
        .frames
        .lock()
        .map_err(|_| "diagram frame preference lock poisoned")?
        .get(&diagram_id)
        .cloned())
}

#[tauri::command]
pub fn set_diagram_frame_preference(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedWorkspaceState>,
    diagram_id: String,
    preference: DiagramFramePreference,
) -> Result<(), String> {
    ensure_preferences_loaded(&app, &state)?;
    if uuid::Uuid::parse_str(&diagram_id).is_err() {
        return Err("diagram frame id is invalid".into());
    }
    preference.validate()?;
    state
        .frames
        .lock()
        .map_err(|_| "diagram frame preference lock poisoned")?
        .insert(diagram_id, preference);
    save_preferences(&app, &state)
}

#[tauri::command]
pub fn fit_diagram_viewport(
    bounds: GeometryRect,
    viewport_width: f64,
    viewport_height: f64,
    padding: f64,
    current: ViewportPreference,
) -> Result<ViewportPreference, String> {
    fit_viewport(bounds, viewport_width, viewport_height, padding, &current)
}

#[tauri::command]
pub fn zoom_diagram_viewport(
    current: ViewportPreference,
    requested_zoom: f64,
    pointer_x: f64,
    pointer_y: f64,
) -> Result<ViewportPreference, String> {
    zoom_viewport_at(&current, requested_zoom, pointer_x, pointer_y)
}

#[tauri::command]
pub fn get_panel_preferences(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedWorkspaceState>,
) -> Result<PanelPreference, String> {
    ensure_preferences_loaded(&app, &state)?;
    Ok(state
        .panels
        .lock()
        .map_err(|_| "panel preference lock poisoned")?
        .clone())
}

#[tauri::command]
pub fn set_panel_preferences(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedWorkspaceState>,
    preference: PanelPreference,
) -> Result<(), String> {
    ensure_preferences_loaded(&app, &state)?;
    preference.validate()?;
    *state
        .panels
        .lock()
        .map_err(|_| "panel preference lock poisoned")? = preference;
    save_preferences(&app, &state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_exposes_renderer_contract_for_every_current_family() {
        let families = diagram_family_registry();
        assert_eq!(families.len(), 7);
        let requirement = families
            .iter()
            .find(|family| family.id.0 == "requirement")
            .expect("Requirement Diagram must be registered in the shared workspace");
        assert_eq!(requirement.renderer_id, "requirement");
        assert_eq!(requirement.frame_abbreviation, "req");
        let use_case = families
            .iter()
            .find(|family| family.id.0 == "use-case")
            .expect("Use Case Diagram must be registered in the shared workspace");
        assert_eq!(use_case.renderer_id, "use-case");
        assert_eq!(use_case.frame_abbreviation, "uc");
        assert!(families.iter().all(|family| !family.renderer_id.is_empty()));
        assert!(
            families
                .iter()
                .all(|family| !family.empty_message.is_empty())
        );
        let commands = resolve_diagram_commands(Some(requirement));
        assert!(
            commands
                .iter()
                .any(|command| command.command.id == "route" && command.enabled)
        );
        assert!(
            commands
                .iter()
                .any(|command| command.command.id == "cleanLayout" && command.enabled)
        );
    }

    #[test]
    fn requirement_dispatcher_routes_and_lays_out_real_geometry() {
        let workspace = super::super::WorkspaceState::default();
        let activity = super::super::activity_workspace::ActivityWorkspaceState::default();
        let diagram_id = uuid::Uuid::new_v4().to_string();
        let node = |id: &str, x: f64, y: f64, width: f64, height: f64| super::super::DiagramNode {
            id: id.into(),
            element_id: uuid::Uuid::new_v4().to_string(),
            x,
            y,
            width,
            height,
        };
        workspace
            .diagrams
            .lock()
            .expect("diagram lock")
            .push(super::super::BddDiagram {
                id: diagram_id.clone(),
                name: "Traceability".into(),
                owner_id: uuid::Uuid::new_v4().to_string(),
                family: "requirement".into(),
                semantic_context_id: None,
                nodes: vec![
                    node("source", 80.0, 140.0, 150.0, 80.0),
                    node("obstacle", 340.0, 120.0, 150.0, 120.0),
                    node("target", 620.0, 140.0, 150.0, 80.0),
                ],
                edges: vec![super::super::DiagramEdge {
                    id: "presentation-edge".into(),
                    relationship_id: uuid::Uuid::new_v4().to_string(),
                    source_node_id: "source".into(),
                    target_node_id: "target".into(),
                    points: vec![
                        super::super::DiagramPoint { x: 230.0, y: 180.0 },
                        super::super::DiagramPoint { x: 620.0, y: 180.0 },
                    ],
                    label_anchor: None,
                }],
            });
        let bounds = Some(super::super::routing::RouteRect {
            x: 0.0,
            y: 42.0,
            width: 960.0,
            height: 640.0,
        });

        assert!(
            dispatch_route("requirement", &diagram_id, &workspace, &activity, bounds)
                .expect("Requirement Route must dispatch")
        );
        {
            let diagrams = workspace.diagrams.lock().expect("diagram lock");
            let route = &diagrams[0].edges[0].points;
            assert!(super::super::routing::route_is_clear(
                route,
                &[super::super::routing::RouteRect {
                    x: 340.0,
                    y: 120.0,
                    width: 150.0,
                    height: 120.0,
                }]
            ));
            assert!(diagrams[0].edges[0].label_anchor.is_some());
        }
        assert!(
            dispatch_layout("requirement", &diagram_id, &workspace, &activity, bounds)
                .expect("Requirement Clean Layout must dispatch")
        );
        let diagrams = workspace.diagrams.lock().expect("diagram lock");
        assert_eq!(diagrams[0].family, "requirement");
        assert_eq!(diagrams[0].edges[0].relationship_id.len(), 36);
    }

    #[test]
    fn use_case_dispatcher_reuses_routing_and_registered_left_to_right_layout() {
        let workspace = super::super::WorkspaceState::default();
        let activity = super::super::activity_workspace::ActivityWorkspaceState::default();
        let mut project = systems_modeler_core::Project::new("Use Cases");
        let package = project
            .create_element(
                systems_modeler_core::ElementKind::Package,
                "Operations",
                project.root_id,
            )
            .unwrap();
        let actor = project
            .create_element(systems_modeler_core::ElementKind::Actor, "Operator", package)
            .unwrap();
        let use_case = project
            .create_element(
                systems_modeler_core::ElementKind::UseCase,
                "Operate system",
                package,
            )
            .unwrap();
        *workspace.project.lock().expect("project lock") = Some(project);
        let diagram_id = uuid::Uuid::new_v4().to_string();
        workspace
            .diagrams
            .lock()
            .expect("diagram lock")
            .push(super::super::BddDiagram {
                id: diagram_id.clone(),
                name: "Operations".into(),
                owner_id: package.to_string(),
                family: "use-case".into(),
                semantic_context_id: None,
                nodes: vec![
                    super::super::DiagramNode {
                        id: "actor".into(),
                        element_id: actor.to_string(),
                        x: 760.0,
                        y: 420.0,
                        width: 110.0,
                        height: 150.0,
                    },
                    super::super::DiagramNode {
                        id: "use-case".into(),
                        element_id: use_case.to_string(),
                        x: 90.0,
                        y: 100.0,
                        width: 210.0,
                        height: 115.0,
                    },
                ],
                edges: vec![super::super::DiagramEdge {
                    id: "association-presentation".into(),
                    relationship_id: uuid::Uuid::new_v4().to_string(),
                    source_node_id: "use-case".into(),
                    target_node_id: "actor".into(),
                    points: vec![
                        super::super::DiagramPoint { x: 90.0, y: 155.0 },
                        super::super::DiagramPoint { x: 760.0, y: 495.0 },
                    ],
                    label_anchor: None,
                }],
            });

        dispatch_route("use-case", &diagram_id, &workspace, &activity, None)
            .expect("Use Case Route must use the shared dispatcher");
        dispatch_layout("use-case", &diagram_id, &workspace, &activity, None)
            .expect("Use Case Clean Layout must use the shared dispatcher");
        let diagrams = workspace.diagrams.lock().expect("diagram lock");
        let actor = diagrams[0]
            .nodes
            .iter()
            .find(|node| node.id == "actor")
            .unwrap();
        let use_case = diagrams[0]
            .nodes
            .iter()
            .find(|node| node.id == "use-case")
            .unwrap();
        assert!(actor.x < use_case.x);
        assert!(diagrams[0].edges[0].points.len() >= 2);
    }

    #[test]
    fn failed_or_noop_presentation_commands_do_not_checkpoint_history() {
        let workspace = super::super::WorkspaceState::default();
        let activity = super::super::activity_workspace::ActivityWorkspaceState::default();
        let history = super::super::history::HistoryState::default();
        let error = record_presentation_change(&workspace, &activity, &history, || {
            Err::<bool, String>("qualification failure".into())
        });
        assert!(error.is_err());
        assert_eq!(super::super::history::undo_len(&history), 0);
        record_presentation_change(&workspace, &activity, &history, || Ok(false))
            .expect("no-op command succeeds");
        assert_eq!(super::super::history::undo_len(&history), 0);
    }

    #[test]
    fn only_manually_sized_frames_constrain_routing_and_layout() {
        let shared = SharedWorkspaceState::default();
        let diagram_id = uuid::Uuid::new_v4().to_string();
        let automatic = DiagramFramePreference {
            x: 480.0,
            y: 320.0,
            width: 720.0,
            height: 520.0,
            manually_sized: false,
        };
        assert_eq!(
            routing_bounds(&shared, &diagram_id, Some(automatic)),
            Ok(None)
        );

        let manual = DiagramFramePreference {
            x: 480.0,
            y: 320.0,
            width: 720.0,
            height: 520.0,
            manually_sized: true,
        };
        assert_eq!(
            routing_bounds(&shared, &diagram_id, Some(manual)),
            Ok(Some(super::super::routing::RouteRect {
                x: 480.0,
                y: 362.0,
                width: 720.0,
                height: 478.0,
            }))
        );
    }

    #[test]
    fn workspace_preferences_round_trip_all_diagram_viewports_and_panels() {
        let diagram_id = uuid::Uuid::new_v4().to_string();
        let mut preferences = PersistedWorkspacePreferences::default();
        preferences.viewports.insert(
            diagram_id.clone(),
            ViewportPreference {
                zoom: 1.5,
                pan_x: 24.0,
                pan_y: -18.0,
                grid_visible: false,
                snap_to_grid: true,
            },
        );
        preferences.panels.repository_width = 310;
        preferences.panels.properties_visible = false;
        preferences.frames.insert(
            diagram_id.clone(),
            DiagramFramePreference {
                x: 12.0,
                y: 18.0,
                width: 960.0,
                height: 640.0,
                manually_sized: true,
            },
        );
        let json = serde_json::to_vec(&preferences).expect("preferences serialize");
        let restored: PersistedWorkspacePreferences =
            serde_json::from_slice(&json).expect("preferences deserialize");
        assert_eq!(restored.viewports[&diagram_id].zoom, 1.5);
        assert_eq!(restored.frames[&diagram_id].width, 960.0);
        assert!(restored.frames[&diagram_id].manually_sized);
        assert_eq!(restored.panels.repository_width, 310);
        assert!(!restored.panels.properties_visible);
    }

    #[test]
    fn interaction_validation_rejects_stale_diagrams_and_invalid_selections() {
        let active = uuid::Uuid::new_v4().to_string();
        let stale = WorkspaceInteractionSnapshot {
            diagram_id: Some(uuid::Uuid::new_v4().to_string()),
            selections: Vec::new(),
            active_tool: None,
            revision: 0,
        };
        assert!(stale.validate_for(&active).is_err());

        let invalid = WorkspaceInteractionSnapshot {
            diagram_id: Some(active.clone()),
            selections: vec![WorkspaceSelection {
                kind: String::new(),
                id: "id".into(),
            }],
            active_tool: None,
            revision: 0,
        };
        assert!(invalid.validate_for(&active).is_err());
    }

    #[test]
    fn interaction_revision_rejects_stale_writers() {
        let snapshot = WorkspaceInteractionSnapshot {
            revision: 7,
            ..WorkspaceInteractionSnapshot::default()
        };
        assert!(snapshot.require_revision(Some(6)).is_err());
        assert!(snapshot.require_revision(Some(7)).is_ok());
        assert!(snapshot.require_revision(None).is_ok());
    }

    #[test]
    fn active_workspace_snapshot_serializes_one_host_contract() {
        let family = diagram_family_registry().remove(0);
        let context = ActiveDiagramContext {
            diagram_id: uuid::Uuid::new_v4().to_string(),
            family: family.clone(),
            name: "System Structure".into(),
            model_element_name: "Vehicle".into(),
            frame_label: "bdd [Package] Vehicle [System Structure]".into(),
            semantic_context_id: "model".into(),
        };
        let snapshot = ActiveWorkspaceSnapshot {
            interaction: WorkspaceInteractionSnapshot {
                diagram_id: Some(context.diagram_id.clone()),
                ..WorkspaceInteractionSnapshot::default()
            },
            commands: resolve_diagram_commands(Some(&family)),
            context,
        };
        let value = serde_json::to_value(snapshot).expect("workspace host snapshot serializes");
        assert!(value["context"]["diagramId"].is_string());
        assert_eq!(
            value["context"]["frameLabel"],
            "bdd [Package] Vehicle [System Structure]"
        );
        assert!(value["interaction"]["revision"].is_number());
        assert!(value["commands"].is_array());
    }
}
