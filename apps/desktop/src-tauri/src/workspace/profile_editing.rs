//! Authoritative desktop editing bridge for first-class profiles and stereotype applications.

use super::{WorkspaceState, parse_element_id};
use systems_modeler_core::{
    ProfileId, SemanticTarget, StereotypeApplicationId, StereotypeId, StereotypeTargetKind,
    TagDefinitionId, TagValue, TagValueType,
};
use uuid::Uuid;

fn parse_uuid(value: &str, label: &str) -> Result<Uuid, String> {
    Uuid::parse_str(value).map_err(|_| format!("invalid {label}: {value}"))
}

#[tauri::command]
pub fn create_profile_definition(
    external_id: String,
    name: String,
    uri: Option<String>,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let mut guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = guard.as_mut().ok_or("no project open")?;
    project
        .create_profile(external_id, name, uri)
        .map(|id| id.to_string())
}

#[tauri::command]
pub fn create_stereotype_definition(
    profile_id: String,
    external_id: String,
    name: String,
    extends: Vec<StereotypeTargetKind>,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let profile_id = ProfileId(parse_uuid(&profile_id, "profile id")?);
    let mut guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = guard.as_mut().ok_or("no project open")?;
    project
        .create_stereotype(profile_id, external_id, name, extends)
        .map(|id| id.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_tag_definition(
    stereotype_id: String,
    external_id: String,
    name: String,
    value_type: TagValueType,
    lower: u32,
    upper: Option<u32>,
    default: Option<TagValue>,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let stereotype_id = StereotypeId(parse_uuid(&stereotype_id, "stereotype id")?);
    let mut guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = guard.as_mut().ok_or("no project open")?;
    project
        .create_tag_definition(
            stereotype_id,
            external_id,
            name,
            value_type,
            (lower, upper),
            default,
        )
        .map(|id| id.to_string())
}

#[tauri::command]
pub fn apply_profile_definition(
    profile_id: String,
    scope_id: String,
    external_id: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let profile_id = ProfileId(parse_uuid(&profile_id, "profile id")?);
    let scope_id = parse_element_id(&scope_id)?;
    let mut guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = guard.as_mut().ok_or("no project open")?;
    project
        .apply_profile(profile_id, scope_id, external_id)
        .map(|id| id.to_string())
}

#[tauri::command]
pub fn apply_stereotype_definition(
    stereotype_id: String,
    target: SemanticTarget,
    external_id: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let stereotype_id = StereotypeId(parse_uuid(&stereotype_id, "stereotype id")?);
    let mut guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = guard.as_mut().ok_or("no project open")?;
    project
        .apply_stereotype(stereotype_id, target, external_id)
        .map(|id| id.to_string())
}

#[tauri::command]
pub fn set_stereotype_tag_values(
    application_id: String,
    definition_id: String,
    values: Vec<TagValue>,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let application_id =
        StereotypeApplicationId(parse_uuid(&application_id, "stereotype application id")?);
    let definition_id = TagDefinitionId(parse_uuid(&definition_id, "tag definition id")?);
    let mut guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = guard.as_mut().ok_or("no project open")?;
    project.set_tagged_values(application_id, definition_id, values)
}

#[tauri::command]
pub fn remove_stereotype_application(
    application_id: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let application_id =
        StereotypeApplicationId(parse_uuid(&application_id, "stereotype application id")?);
    let mut guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = guard.as_mut().ok_or("no project open")?;
    project.remove_stereotype_application(application_id)
}
