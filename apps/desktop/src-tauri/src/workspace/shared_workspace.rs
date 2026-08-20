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
    panels: PanelPreference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveDiagramContext {
    pub diagram_id: String,
    pub family: DiagramFamilyDescriptor,
    pub name: String,
    pub semantic_context_id: String,
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
}

#[derive(Default)]
pub struct SharedWorkspaceState {
    active: Mutex<Option<ActiveDiagramContext>>,
    interaction: Mutex<WorkspaceInteractionSnapshot>,
    viewports: Mutex<BTreeMap<String, ViewportPreference>>,
    panels: Mutex<PanelPreference>,
    preferences_loaded: Mutex<bool>,
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
        *state
            .viewports
            .lock()
            .map_err(|_| "viewport preference lock poisoned")? = preferences.viewports;
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

#[tauri::command]
pub fn activate_diagram(
    state: tauri::State<'_, SharedWorkspaceState>,
    diagram_id: String,
    family_id: String,
    name: String,
    semantic_context_id: String,
) -> Result<ActiveDiagramContext, String> {
    if uuid::Uuid::parse_str(&diagram_id).is_err() {
        return Err("active diagram id is invalid".into());
    }
    let family_id = DiagramFamilyId::new(family_id)?;
    let family = supported_diagram_families()
        .get(&family_id)
        .cloned()
        .ok_or_else(|| format!("unregistered diagram family: {}", family_id.0))?;
    let context = ActiveDiagramContext {
        diagram_id,
        family,
        name,
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
    if changed_diagram {
        let mut interaction = state
            .interaction
            .lock()
            .map_err(|_| "workspace interaction lock poisoned")?;
        interaction.diagram_id = Some(context.diagram_id.clone());
        interaction.selections.clear();
        interaction.active_tool = None;
        interaction.revision = interaction.revision.saturating_add(1);
    }
    Ok(context)
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
        assert_eq!(families.len(), 5);
        assert!(families.iter().all(|family| !family.renderer_id.is_empty()));
        assert!(
            families
                .iter()
                .all(|family| !family.empty_message.is_empty())
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
        let json = serde_json::to_vec(&preferences).expect("preferences serialize");
        let restored: PersistedWorkspacePreferences =
            serde_json::from_slice(&json).expect("preferences deserialize");
        assert_eq!(restored.viewports[&diagram_id].zoom, 1.5);
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
}
