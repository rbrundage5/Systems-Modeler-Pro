use super::WorkspaceState;
use super::activity_workspace::ActivityWorkspaceState;
use super::history::{self, HistoryState};
use super::shared_workspace::{
    SharedWorkspaceState, WorkspaceSelection, workspace_interaction_snapshot,
};
use super::standard_editing::{self, StandardEditingResult, StandardEditingState};
use systems_modeler_core::RelationshipId;

fn active_selections(
    shared: tauri::State<'_, SharedWorkspaceState>,
) -> Result<Vec<WorkspaceSelection>, String> {
    Ok(workspace_interaction_snapshot(shared)?.selections)
}

fn parse_relationship_id(value: &str) -> Result<RelationshipId, String> {
    uuid::Uuid::parse_str(value)
        .map(RelationshipId)
        .map_err(|_| format!("invalid relationship id: {value}"))
}

fn selected_relationship_id(
    diagram_id: &str,
    selection: &WorkspaceSelection,
    workspace: &WorkspaceState,
) -> Result<RelationshipId, String> {
    let project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;

    if let Some(edge) = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .and_then(|diagram| {
            diagram
                .edges
                .iter()
                .find(|edge| edge.id == selection.id || edge.relationship_id == selection.id)
        })
    {
        return parse_relationship_id(&edge.relationship_id);
    }

    if let Some(edge) = workspace
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")?
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .and_then(|diagram| {
            diagram
                .connectors
                .iter()
                .find(|edge| edge.id == selection.id || edge.relationship_id == selection.id)
        })
    {
        return parse_relationship_id(&edge.relationship_id);
    }

    let candidate = parse_relationship_id(&selection.id)?;
    project
        .relationships
        .contains_key(&candidate)
        .then_some(candidate)
        .ok_or_else(|| "selected presentation does not resolve to a model relationship".into())
}

fn delete_selected_relationship_from_model(
    diagram_id: &str,
    selection: &WorkspaceSelection,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
) -> Result<StandardEditingResult, String> {
    let relationship_id = selected_relationship_id(diagram_id, selection, &workspace)?;
    let relationship_key = relationship_id.to_string();

    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    let mut bdd_diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    let mut ibd_diagrams = workspace
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")?
        .clone();
    let behavior = workspace
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?
        .clone();
    let behavior_diagrams = workspace
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?
        .clone();
    let activity_repository = activity
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?
        .clone();
    let activity_diagrams = activity
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?
        .clone();

    if project.relationships.remove(&relationship_id).is_none() {
        return Err("relationship not found".into());
    }
    for diagram in &mut bdd_diagrams {
        diagram
            .edges
            .retain(|edge| edge.relationship_id != relationship_key);
    }
    for diagram in &mut ibd_diagrams {
        diagram
            .connectors
            .retain(|edge| edge.relationship_id != relationship_key);
    }

    project
        .validate()
        .map_err(|error| format!("Delete from Model rejected: {error}"))?;
    super::validate_loaded_diagrams(&project, &bdd_diagrams)?;
    super::ibd::validate_ibd_diagrams(&project, &ibd_diagrams)?;
    super::behavior_workspace::validate_behavior_workspace(
        &project,
        &behavior,
        &behavior_diagrams,
    )?;
    activity_repository
        .validate(&project)
        .map_err(|error| error.to_string())?;
    for owner_id in activity_diagrams
        .iter()
        .map(|diagram| diagram.owner_id.as_str())
    {
        project
            .element(super::parse_element_id(owner_id)?)
            .map_err(|error| error.to_string())?;
    }

    history::checkpoint_states(&workspace, &activity, &history)?;
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(project);
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = bdd_diagrams;
    *workspace
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")? = ibd_diagrams;

    Ok(StandardEditingResult {
        changed: 1,
        selections: Vec::new(),
    })
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
    from_model: Option<bool>,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
    shared: tauri::State<'_, SharedWorkspaceState>,
) -> Result<StandardEditingResult, String> {
    let selections = active_selections(shared)?;
    if from_model.unwrap_or(false) {
        if selections.len() != 1 {
            return Err("Delete from Model requires exactly one selected relationship".into());
        }
        return delete_selected_relationship_from_model(
            &diagram_id,
            &selections[0],
            workspace,
            activity,
            history,
        );
    }
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
