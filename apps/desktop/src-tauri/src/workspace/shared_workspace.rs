use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Mutex;
use systems_modeler_core::{
    DiagramFamilyDescriptor, DiagramFamilyId, GeometryRect, PanelPreference, ViewportPreference,
    fit_viewport, supported_diagram_families, zoom_viewport_at,
};

use super::presentation_theme::{ResolvedDiagramCommand, resolve_diagram_commands};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveDiagramContext {
    pub diagram_id: String,
    pub family: DiagramFamilyDescriptor,
    pub name: String,
    pub semantic_context_id: String,
}

#[derive(Default)]
pub struct SharedWorkspaceState {
    active: Mutex<Option<ActiveDiagramContext>>,
    viewports: Mutex<BTreeMap<String, ViewportPreference>>,
    panels: Mutex<PanelPreference>,
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
    *state
        .active
        .lock()
        .map_err(|_| "active diagram context lock poisoned")? = Some(context.clone());
    Ok(context)
}

#[tauri::command]
pub fn get_viewport_preference(
    state: tauri::State<'_, SharedWorkspaceState>,
    diagram_id: String,
) -> Result<ViewportPreference, String> {
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
    state: tauri::State<'_, SharedWorkspaceState>,
    diagram_id: String,
    preference: ViewportPreference,
) -> Result<(), String> {
    if uuid::Uuid::parse_str(&diagram_id).is_err() {
        return Err("viewport diagram id is invalid".into());
    }
    preference.validate()?;
    state
        .viewports
        .lock()
        .map_err(|_| "viewport preference lock poisoned")?
        .insert(diagram_id, preference);
    Ok(())
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
    state: tauri::State<'_, SharedWorkspaceState>,
) -> Result<PanelPreference, String> {
    Ok(state
        .panels
        .lock()
        .map_err(|_| "panel preference lock poisoned")?
        .clone())
}

#[tauri::command]
pub fn set_panel_preferences(
    state: tauri::State<'_, SharedWorkspaceState>,
    preference: PanelPreference,
) -> Result<(), String> {
    preference.validate()?;
    *state
        .panels
        .lock()
        .map_err(|_| "panel preference lock poisoned")? = preference;
    Ok(())
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
}
