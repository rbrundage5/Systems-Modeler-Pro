use super::*;
use std::sync::Mutex;
use systems_modeler_core::{ActivityRepository, BehaviorRepository, Project};

const HISTORY_LIMIT: usize = 100;

#[derive(Clone)]
pub(super) struct HistorySnapshot {
    project: Option<Project>,
    diagrams: Vec<BddDiagram>,
    ibd_diagrams: Vec<ibd::IbdDiagram>,
    behavior: BehaviorRepository,
    behavior_diagrams: Vec<behavior_workspace::BehaviorDiagram>,
    activity_repository: ActivityRepository,
    activity_diagrams: Vec<activity_workspace::ActivityDiagram>,
}

pub struct HistoryState {
    undo: Mutex<Vec<HistorySnapshot>>,
    redo: Mutex<Vec<HistorySnapshot>>,
}

impl Default for HistoryState {
    fn default() -> Self {
        Self {
            undo: Mutex::new(Vec::new()),
            redo: Mutex::new(Vec::new()),
        }
    }
}

pub(super) fn capture_states(
    workspace: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
) -> Result<HistorySnapshot, String> {
    Ok(HistorySnapshot {
        project: workspace
            .project
            .lock()
            .map_err(|_| "project lock poisoned")?
            .clone(),
        diagrams: workspace
            .diagrams
            .lock()
            .map_err(|_| "diagram lock poisoned")?
            .clone(),
        ibd_diagrams: workspace
            .ibd_diagrams
            .lock()
            .map_err(|_| "IBD lock poisoned")?
            .clone(),
        behavior: workspace
            .behavior
            .lock()
            .map_err(|_| "behavior lock poisoned")?
            .clone(),
        behavior_diagrams: workspace
            .behavior_diagrams
            .lock()
            .map_err(|_| "behavior diagram lock poisoned")?
            .clone(),
        activity_repository: activity
            .repository
            .lock()
            .map_err(|_| "Activity repository lock poisoned")?
            .clone(),
        activity_diagrams: activity
            .diagrams
            .lock()
            .map_err(|_| "Activity diagram lock poisoned")?
            .clone(),
    })
}

/// Stage one IBD presentation edit and publish geometry plus one history entry.
/// Acquire every fallible lock before changing either authored state or history.
pub(super) fn edit_ibd_geometry(
    workspace: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
    history: &HistoryState,
    diagram_id: &str,
    edit: impl FnOnce(&mut ibd::IbdDiagram) -> Result<(), String>,
) -> Result<(), String> {
    let project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?;
    let diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?;
    let mut ibd_diagrams = workspace
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")?;
    let index = ibd_diagrams
        .iter()
        .position(|diagram| diagram.id == diagram_id)
        .ok_or("IBD not found")?;
    let mut staged = ibd_diagrams[index].clone();
    edit(&mut staged)?;
    if serde_json::to_value(&staged).map_err(|error| error.to_string())?
        == serde_json::to_value(&ibd_diagrams[index]).map_err(|error| error.to_string())?
    {
        return Ok(());
    }
    let behavior = workspace
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    let behavior_diagrams = workspace
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let activity_repository = activity
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?;
    let activity_diagrams = activity
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?;
    let mut undo = history
        .undo
        .lock()
        .map_err(|_| "undo history lock poisoned")?;
    let mut redo = history
        .redo
        .lock()
        .map_err(|_| "redo history lock poisoned")?;
    undo.push(HistorySnapshot {
        project: project.clone(),
        diagrams: diagrams.clone(),
        ibd_diagrams: ibd_diagrams.clone(),
        behavior: behavior.clone(),
        behavior_diagrams: behavior_diagrams.clone(),
        activity_repository: activity_repository.clone(),
        activity_diagrams: activity_diagrams.clone(),
    });
    if undo.len() > HISTORY_LIMIT {
        undo.remove(0);
    }
    redo.clear();
    ibd_diagrams[index] = staged;
    Ok(())
}

pub(super) fn checkpoint_states(
    workspace: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
    history: &HistoryState,
) -> Result<(), String> {
    let snapshot = capture_states(workspace, activity)?;
    commit_snapshot(snapshot, history)
}

pub(super) fn commit_snapshot(
    snapshot: HistorySnapshot,
    history: &HistoryState,
) -> Result<(), String> {
    let mut undo = history
        .undo
        .lock()
        .map_err(|_| "undo history lock poisoned")?;
    undo.push(snapshot);
    if undo.len() > HISTORY_LIMIT {
        undo.remove(0);
    }
    history
        .redo
        .lock()
        .map_err(|_| "redo history lock poisoned")?
        .clear();
    Ok(())
}

#[cfg(test)]
pub(super) fn undo_len(history: &HistoryState) -> usize {
    history.undo.lock().expect("undo history lock").len()
}

fn restore(
    snapshot: HistorySnapshot,
    workspace: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
) -> Result<(), String> {
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = snapshot.project;
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = snapshot.diagrams;
    *workspace
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")? = snapshot.ibd_diagrams;
    *workspace
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")? = snapshot.behavior;
    *workspace
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")? = snapshot.behavior_diagrams;
    *activity
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")? = snapshot.activity_repository;
    *activity
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")? = snapshot.activity_diagrams;
    Ok(())
}

#[tauri::command]
pub fn history_checkpoint(
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
) -> Result<(), String> {
    checkpoint_states(&workspace, &activity, &history)
}

#[tauri::command]
pub fn history_undo(
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
) -> Result<bool, String> {
    undo_states(&workspace, &activity, &history)
}

pub(super) fn undo_states(
    workspace: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
    history: &HistoryState,
) -> Result<bool, String> {
    let target = {
        let mut undo = history
            .undo
            .lock()
            .map_err(|_| "undo history lock poisoned")?;
        undo.pop()
    };
    let Some(target) = target else {
        return Ok(false);
    };
    let current = capture_states(workspace, activity)?;
    restore(target, workspace, activity)?;
    let mut redo = history
        .redo
        .lock()
        .map_err(|_| "redo history lock poisoned")?;
    redo.push(current);
    if redo.len() > HISTORY_LIMIT {
        redo.remove(0);
    }
    Ok(true)
}

#[tauri::command]
pub fn history_redo(
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
) -> Result<bool, String> {
    redo_states(&workspace, &activity, &history)
}

pub(super) fn redo_states(
    workspace: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
    history: &HistoryState,
) -> Result<bool, String> {
    let target = {
        let mut redo = history
            .redo
            .lock()
            .map_err(|_| "redo history lock poisoned")?;
        redo.pop()
    };
    let Some(target) = target else {
        return Ok(false);
    };
    let current = capture_states(workspace, activity)?;
    restore(target, workspace, activity)?;
    let mut undo = history
        .undo
        .lock()
        .map_err(|_| "undo history lock poisoned")?;
    undo.push(current);
    if undo.len() > HISTORY_LIMIT {
        undo.remove(0);
    }
    Ok(true)
}

#[tauri::command]
pub fn history_reset(history: tauri::State<'_, HistoryState>) -> Result<(), String> {
    history
        .undo
        .lock()
        .map_err(|_| "undo history lock poisoned")?
        .clear();
    history
        .redo
        .lock()
        .map_err(|_| "redo history lock poisoned")?
        .clear();
    Ok(())
}
