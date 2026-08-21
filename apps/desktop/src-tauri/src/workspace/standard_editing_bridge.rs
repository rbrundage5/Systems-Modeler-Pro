use super::activity_workspace::ActivityWorkspaceState;
use super::history::HistoryState;
use super::shared_workspace::{SharedWorkspaceState, WorkspaceSelection, workspace_interaction_snapshot};
use super::standard_editing::{self, StandardEditingResult, StandardEditingState};
use super::WorkspaceState;

fn active_selections(
    shared: tauri::State<'_, SharedWorkspaceState>,
) -> Result<Vec<WorkspaceSelection>, String> {
    Ok(workspace_interaction_snapshot(shared)?.selections)
}

/// Standard editing commands deliberately resolve selection from the Rust-owned
/// workspace interaction state. Callers only identify the active diagram; this
/// keeps keyboard, ribbon, context-menu, and Properties actions converged.
#[tauri::command]
pub fn copy_selection(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    editing: tauri::State<'_, StandardEditingState>,
    shared: tauri::State<'_, SharedWorkspaceState>,
) -> Result<StandardEditingResult, String> {
    let selections = active_selections(shared)?;
    standard_editing::copy_selection(diagram_id, selections, workspace, activity, editing)
}

#[tauri::command]
pub fn paste_selection(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
    editing: tauri::State<'_, StandardEditingState>,
    shared: tauri::State<'_, SharedWorkspaceState>,
) -> Result<StandardEditingResult, String> {
    let selections = active_selections(shared)?;
    standard_editing::paste_selection(
        diagram_id, selections, workspace, activity, history, editing,
    )
}

#[tauri::command]
pub fn duplicate_selection(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
    shared: tauri::State<'_, SharedWorkspaceState>,
) -> Result<StandardEditingResult, String> {
    let selections = active_selections(shared)?;
    standard_editing::duplicate_selection(diagram_id, selections, workspace, activity, history)
}

#[tauri::command]
pub fn delete_active_selection(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
    shared: tauri::State<'_, SharedWorkspaceState>,
) -> Result<StandardEditingResult, String> {
    let selections = active_selections(shared)?;
    standard_editing::delete_active_selection(diagram_id, selections, workspace, activity, history)
}

#[tauri::command]
pub fn move_active_selection(
    diagram_id: String,
    dx: f64,
    dy: f64,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
    shared: tauri::State<'_, SharedWorkspaceState>,
) -> Result<StandardEditingResult, String> {
    let selections = active_selections(shared)?;
    standard_editing::move_active_selection(
        diagram_id, selections, dx, dy, workspace, activity, history,
    )
}
