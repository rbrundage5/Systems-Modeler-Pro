use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Mutex;
use systems_modeler_core::{
    DiagramFamilyDescriptor, DiagramFamilyId, PanelPreference, ViewportPreference,
    supported_diagram_families,
};

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
