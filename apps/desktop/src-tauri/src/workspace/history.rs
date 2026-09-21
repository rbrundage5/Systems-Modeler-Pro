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

/// Commit one specification against the current authored state. Keep the existing
/// history representation and lock order; reject before touching either stack.
pub(super) fn apply_element_specification(
    workspace: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
    history: &HistoryState,
    element_id: systems_modeler_core::ElementId,
    edit: &systems_modeler_core::ElementSpecificationEdit,
) -> Result<bool, String> {
    apply_structural_specification(workspace, activity, history, |current, diagrams| {
        Ok((
            current.stage_element_specification(element_id, edit)?,
            diagrams.to_vec(),
        ))
    })
}

/// Shared structural specification transaction, including affected IBD views.
pub(super) fn apply_structural_specification(
    workspace: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
    history: &HistoryState,
    edit: impl FnOnce(&Project, &[ibd::IbdDiagram]) -> Result<(Project, Vec<ibd::IbdDiagram>), String>,
) -> Result<bool, String> {
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?;
    let current = project.as_ref().ok_or("no project open")?;
    let diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?;
    let mut ibd_diagrams = workspace
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")?;
    let (candidate, candidate_ibds) = edit(current, &ibd_diagrams)?;
    if serde_json::to_value(current).map_err(|error| error.to_string())?
        == serde_json::to_value(&candidate).map_err(|error| error.to_string())?
        && serde_json::to_value(&*ibd_diagrams).map_err(|error| error.to_string())?
            == serde_json::to_value(&candidate_ibds).map_err(|error| error.to_string())?
    {
        return Ok(false);
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
    super::validate_loaded_diagrams(&candidate, &diagrams)?;
    super::ibd::validate_ibd_diagrams(&candidate, &candidate_ibds)?;
    super::behavior_workspace::validate_behavior_workspace(
        &candidate,
        &behavior,
        &behavior_diagrams,
    )?;
    activity_repository
        .validate(&candidate)
        .map_err(|error| error.to_string())?;
    let snapshot = HistorySnapshot {
        project: project.clone(),
        diagrams: diagrams.clone(),
        ibd_diagrams: ibd_diagrams.clone(),
        behavior: behavior.clone(),
        behavior_diagrams: behavior_diagrams.clone(),
        activity_repository: activity_repository.clone(),
        activity_diagrams: activity_diagrams.clone(),
    };
    // Acquire both stacks before any mutation. A failed lock cannot consume redo
    // or append a checkpoint for a rejected edit.
    let mut undo = history
        .undo
        .lock()
        .map_err(|_| "undo history lock poisoned")?;
    let mut redo = history
        .redo
        .lock()
        .map_err(|_| "redo history lock poisoned")?;
    undo.push(snapshot);
    if undo.len() > HISTORY_LIMIT {
        undo.remove(0);
    }
    redo.clear();
    *project = Some(candidate);
    *ibd_diagrams = candidate_ibds;
    Ok(true)
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

#[cfg(test)]
mod specification_tests {
    use super::*;
    use systems_modeler_core::{ElementId, ElementKind, ElementSpecificationEdit, Multiplicity};
    use systems_modeler_persistence::ProjectDatabase;

    fn fixture() -> (
        WorkspaceState,
        activity_workspace::ActivityWorkspaceState,
        HistoryState,
        ElementId,
        ElementId,
    ) {
        let workspace = WorkspaceState::default();
        let mut project = Project::new("Edit history");
        let owner = project
            .create_element(ElementKind::Block, "System", project.root_id)
            .unwrap();
        let first = project
            .create_element(ElementKind::Block, "Unit", project.root_id)
            .unwrap();
        let next = project
            .create_element(ElementKind::Block, "NewUnit", project.root_id)
            .unwrap();
        let feature = project
            .create_typed_feature(
                ElementKind::PartProperty,
                "unit",
                owner,
                first,
                Multiplicity::ONE,
            )
            .unwrap();
        *workspace.project.lock().unwrap() = Some(project);
        (
            workspace,
            activity_workspace::ActivityWorkspaceState::default(),
            HistoryState::default(),
            feature,
            next,
        )
    }

    fn project_value(workspace: &WorkspaceState) -> serde_json::Value {
        serde_json::to_value(workspace.project.lock().unwrap().as_ref().unwrap()).unwrap()
    }

    #[test]
    fn specification_one_step_undo_redo_noop_and_rejected_edit_preserve_history() {
        let (workspace, activity, history, feature, next) = fixture();
        let before = project_value(&workspace);
        let edit = ElementSpecificationEdit {
            name: "renamed".into(),
            type_id: Some(next),
            documentation: Some("notes".into()),
            multiplicity: Some("0..*".into()),
            ..Default::default()
        };
        assert!(
            apply_element_specification(&workspace, &activity, &history, feature, &edit).unwrap()
        );
        let after = project_value(&workspace);
        assert_ne!(after, before);
        assert_eq!(undo_len(&history), 1);
        assert!(
            !apply_element_specification(&workspace, &activity, &history, feature, &edit).unwrap()
        );
        assert_eq!(undo_len(&history), 1);
        assert!(undo_states(&workspace, &activity, &history).unwrap());
        assert_eq!(project_value(&workspace), before);
        let invalid = ElementSpecificationEdit {
            multiplicity: Some("8..2".into()),
            ..edit.clone()
        };
        assert!(
            apply_element_specification(&workspace, &activity, &history, feature, &invalid)
                .is_err()
        );
        assert_eq!(project_value(&workspace), before);
        assert_eq!(undo_len(&history), 0);
        assert!(redo_states(&workspace, &activity, &history).unwrap());
        assert_eq!(project_value(&workspace), after);
    }

    #[test]
    fn specification_retype_rejects_orphaned_ibd_port_presentation_atomically() {
        let (workspace, activity, history, feature, next) = fixture();
        let mut project = workspace.project.lock().unwrap();
        let model = project.as_mut().unwrap();
        let property = model.element(feature).unwrap().clone();
        let original_type = property.type_id.unwrap();
        let port = model
            .create_typed_feature(
                ElementKind::FullPort,
                "port",
                original_type,
                next,
                Multiplicity::ONE,
            )
            .unwrap();
        // Deserialize the legacy presentation shape so optional schema additions
        // do not couple this semantic rollback regression to another PR.
        let diagram: ibd::IbdDiagram = serde_json::from_value(serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "name": "System internals",
            "context_block_id": property.owner_id.unwrap().to_string(),
            "owner_id": model.root_id.to_string(),
            "properties": vec![ibd::IbdPropertyPresentation {
                id: uuid::Uuid::new_v4().to_string(),
                element_id: feature.to_string(),
                property_path: vec![feature.to_string()],
                x: 100.0,
                y: 100.0,
                width: 180.0,
                height: 100.0,
                ports: vec![ibd::IbdPortPresentation {
                    id: uuid::Uuid::new_v4().to_string(),
                    element_id: port.to_string(),
                    property_path: vec![feature.to_string()],
                    x: 280.0,
                    y: 150.0,
                    size: 12.0,
                }],
            }],
            "boundary_ports": [],
            "connectors": [],
        }))
        .unwrap();
        ibd::validate_ibd_diagrams(model, std::slice::from_ref(&diagram)).unwrap();
        drop(project);
        *workspace.ibd_diagrams.lock().unwrap() = vec![diagram];
        let before = project_value(&workspace);
        let before_diagrams =
            serde_json::to_value(&*workspace.ibd_diagrams.lock().unwrap()).unwrap();
        let edit = ElementSpecificationEdit {
            name: "draft".into(),
            type_id: Some(next),
            ..Default::default()
        };
        assert!(
            apply_element_specification(&workspace, &activity, &history, feature, &edit).is_err()
        );
        assert_eq!(project_value(&workspace), before);
        assert_eq!(
            serde_json::to_value(&*workspace.ibd_diagrams.lock().unwrap()).unwrap(),
            before_diagrams
        );
        assert_eq!(undo_len(&history), 0);
    }

    #[test]
    fn specification_save_reopen_preserves_changed_fields_and_stable_identity() {
        let (workspace, activity, history, feature, next) = fixture();
        let edit = ElementSpecificationEdit {
            name: "renamed".into(),
            type_id: Some(next),
            documentation: Some("Saved documentation".into()),
            multiplicity: Some("0..*".into()),
            default_value: Some("initial".into()),
            ..Default::default()
        };
        apply_element_specification(&workspace, &activity, &history, feature, &edit).unwrap();
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("specification.smproj");
        let project = workspace.project.lock().unwrap().clone().unwrap();
        {
            let mut database = ProjectDatabase::open(&path).unwrap();
            database.save_project(&project).unwrap();
        }
        let database = ProjectDatabase::open(&path).unwrap();
        let reopened = database.load_project(project.id).unwrap();
        assert_eq!(
            serde_json::to_value(reopened).unwrap(),
            serde_json::to_value(project).unwrap()
        );
    }
}
