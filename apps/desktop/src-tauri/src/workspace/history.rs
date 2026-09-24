use super::*;
use std::sync::{Mutex, MutexGuard};
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

// Acquire every authored guard before publishing a field or consuming a checkpoint.
// Legacy diagram commands do not all use one lock order. Do not wait while holding
// a partial snapshot: contention rejects this operation and releases its guards.
struct AuthoredStateGuards<'a> {
    project: MutexGuard<'a, Option<Project>>,
    diagrams: MutexGuard<'a, Vec<BddDiagram>>,
    ibd_diagrams: MutexGuard<'a, Vec<ibd::IbdDiagram>>,
    behavior: MutexGuard<'a, BehaviorRepository>,
    behavior_diagrams: MutexGuard<'a, Vec<behavior_workspace::BehaviorDiagram>>,
    activity_repository: MutexGuard<'a, ActivityRepository>,
    activity_diagrams: MutexGuard<'a, Vec<activity_workspace::ActivityDiagram>>,
}

impl<'a> AuthoredStateGuards<'a> {
    fn lock(
        workspace: &'a WorkspaceState,
        activity: &'a activity_workspace::ActivityWorkspaceState,
    ) -> Result<Self, String> {
        fn acquire<'a, T>(mutex: &'a Mutex<T>, name: &str) -> Result<MutexGuard<'a, T>, String> {
            mutex.try_lock().map_err(|error| match error {
                std::sync::TryLockError::Poisoned(_) => format!("{name} lock poisoned"),
                std::sync::TryLockError::WouldBlock => {
                    format!(
                        "workspace is busy ({name}); retry after the current operation completes"
                    )
                }
            })
        }
        Ok(Self {
            project: acquire(&workspace.project, "project")?,
            diagrams: acquire(&workspace.diagrams, "diagram")?,
            ibd_diagrams: acquire(&workspace.ibd_diagrams, "IBD")?,
            behavior: acquire(&workspace.behavior, "behavior")?,
            behavior_diagrams: acquire(&workspace.behavior_diagrams, "behavior diagram")?,
            activity_repository: acquire(&activity.repository, "Activity repository")?,
            activity_diagrams: acquire(&activity.diagrams, "Activity diagram")?,
        })
    }

    fn capture(&self) -> HistorySnapshot {
        HistorySnapshot {
            project: self.project.clone(),
            diagrams: self.diagrams.clone(),
            ibd_diagrams: self.ibd_diagrams.clone(),
            behavior: self.behavior.clone(),
            behavior_diagrams: self.behavior_diagrams.clone(),
            activity_repository: self.activity_repository.clone(),
            activity_diagrams: self.activity_diagrams.clone(),
        }
    }

    fn replace(&mut self, snapshot: HistorySnapshot) -> HistorySnapshot {
        // Move the previous authored state into the opposite history stack; undo
        // and redo need no additional full-project clone.
        HistorySnapshot {
            project: std::mem::replace(&mut *self.project, snapshot.project),
            diagrams: std::mem::replace(&mut *self.diagrams, snapshot.diagrams),
            ibd_diagrams: std::mem::replace(&mut *self.ibd_diagrams, snapshot.ibd_diagrams),
            behavior: std::mem::replace(&mut *self.behavior, snapshot.behavior),
            behavior_diagrams: std::mem::replace(
                &mut *self.behavior_diagrams,
                snapshot.behavior_diagrams,
            ),
            activity_repository: std::mem::replace(
                &mut *self.activity_repository,
                snapshot.activity_repository,
            ),
            activity_diagrams: std::mem::replace(
                &mut *self.activity_diagrams,
                snapshot.activity_diagrams,
            ),
        }
    }
}

pub(super) fn capture_states(
    workspace: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
) -> Result<HistorySnapshot, String> {
    Ok(AuthoredStateGuards::lock(workspace, activity)?.capture())
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
    let authored = AuthoredStateGuards::lock(workspace, activity)?;
    commit_snapshot(authored.capture(), history)
}

pub(super) fn commit_snapshot(
    snapshot: HistorySnapshot,
    history: &HistoryState,
) -> Result<(), String> {
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
    let mut diagrams = workspace
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
    let candidate_diagrams = super::relationship_editing::stage_relationship_presentations(
        current, &candidate, &diagrams, None,
    )?;
    super::validate_loaded_diagrams(&candidate, &candidate_diagrams)?;
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
    *diagrams = candidate_diagrams;
    *ibd_diagrams = candidate_ibds;
    Ok(true)
}

#[cfg(test)]
pub(super) fn undo_len(history: &HistoryState) -> usize {
    history.undo.lock().expect("undo history lock").len()
}

fn transfer_history(
    workspace: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
    history: &HistoryState,
    undoing: bool,
) -> Result<bool, String> {
    let mut authored = AuthoredStateGuards::lock(workspace, activity)?;
    let mut undo = history
        .undo
        .lock()
        .map_err(|_| "undo history lock poisoned")?;
    let mut redo = history
        .redo
        .lock()
        .map_err(|_| "redo history lock poisoned")?;
    let (source, destination) = if undoing {
        (&mut *undo, &mut *redo)
    } else {
        (&mut *redo, &mut *undo)
    };
    let Some(target) = source.pop() else {
        return Ok(false);
    };
    destination.push(authored.replace(target));
    if destination.len() > HISTORY_LIMIT {
        destination.remove(0);
    }
    Ok(true)
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
    transfer_history(workspace, activity, history, true)
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
    transfer_history(workspace, activity, history, false)
}

#[tauri::command]
pub fn history_reset(history: tauri::State<'_, HistoryState>) -> Result<(), String> {
    reset_states(&history)
}

pub(super) fn reset_states(history: &HistoryState) -> Result<(), String> {
    let mut undo = history
        .undo
        .lock()
        .map_err(|_| "undo history lock poisoned")?;
    let mut redo = history
        .redo
        .lock()
        .map_err(|_| "redo history lock poisoned")?;
    undo.clear();
    redo.clear();
    Ok(())
}

#[cfg(test)]
mod open_history_tests {
    use super::*;
    use systems_modeler_persistence::ProjectDatabase;

    fn fixture() -> (WorkspaceState, activity_workspace::ActivityWorkspaceState, HistoryState) {
        let workspace = WorkspaceState::default();
        let activity = activity_workspace::ActivityWorkspaceState::default();
        let history = HistoryState::default();
        let project = Project::new("Old session");
        activity.repository.lock().unwrap()
            .create_activity(&project, project.root_id, None, "Old activity").unwrap();
        *workspace.project.lock().unwrap() = Some(project);
        *workspace.current_file.lock().unwrap() = Some("old.smproj".into());
        checkpoint_states(&workspace, &activity, &history).unwrap();
        history.redo.lock().unwrap().push(capture_states(&workspace, &activity).unwrap());
        (workspace, activity, history)
    }

    fn before_value(workspace: &WorkspaceState, activity: &activity_workspace::ActivityWorkspaceState) -> serde_json::Value {
        serde_json::json!([
            &*workspace.project.lock().unwrap(),
            &*activity.repository.lock().unwrap(),
            &*workspace.current_file.lock().unwrap_or_else(|error| error.into_inner()),
        ])
    }

    fn lengths(history: &HistoryState) -> (usize, usize) {
        (
            history.undo.lock().unwrap_or_else(|error| error.into_inner()).len(),
            history.redo.lock().unwrap_or_else(|error| error.into_inner()).len(),
        )
    }

    #[test]
    fn complete_open_history_failure_cannot_publish_new_authored_state() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("new.smproj");
        ProjectDatabase::open(&path).unwrap().save_project(&Project::new("New")).unwrap();
        for fail_history in [true, false] {
            let (workspace, activity, history) = fixture();
            let before = before_value(&workspace, &activity);
            let stacks = lengths(&history);
            let poisoned = std::panic::catch_unwind(|| {
                if fail_history {
                    let _held = history.redo.lock().unwrap();
                    panic!("injected redo failure");
                } else {
                    let _held = workspace.current_file.lock().unwrap();
                    panic!("injected path failure");
                }
            });
            assert!(poisoned.is_err());
            assert!(super::super::bdd_elements::open_project_file_in_state(
                path.to_string_lossy().into_owned(), &workspace, &activity, &history,
            ).is_err());
            assert_eq!(before_value(&workspace, &activity), before);
            assert_eq!(lengths(&history), stacks);
        }
    }

    #[test]
    fn complete_open_retires_old_history_before_returning_to_frontend() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("new.smproj");
        let replacement = Project::new("New session");
        ProjectDatabase::open(&path).unwrap().save_project(&replacement).unwrap();
        let (workspace, activity, history) = fixture();
        super::super::bdd_elements::open_project_file_in_state(
            path.to_string_lossy().into_owned(), &workspace, &activity, &history,
        ).unwrap();
        assert_eq!(workspace.project.lock().unwrap().as_ref().unwrap().id, replacement.id);
        assert!(activity.repository.lock().unwrap().activities.is_empty());
        assert_eq!(lengths(&history), (0, 0));
        assert!(!undo_states(&workspace, &activity, &history).unwrap());
        assert!(!redo_states(&workspace, &activity, &history).unwrap());
    }

    #[test]
    fn invalid_complete_open_preserves_both_old_history_stacks() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("invalid.smproj");
        std::fs::write(&path, b"invalid sqlite input").unwrap();
        let (workspace, activity, history) = fixture();
        let before = before_value(&workspace, &activity);
        let stacks = lengths(&history);
        assert!(super::super::bdd_elements::open_project_file_in_state(
            path.to_string_lossy().into_owned(), &workspace, &activity, &history,
        ).is_err());
        assert_eq!(before_value(&workspace, &activity), before);
        assert_eq!(lengths(&history), stacks);
    }
}

#[cfg(test)]
mod atomic_history_tests {
    use super::*;

    fn poison<T: Send>(mutex: &Mutex<T>) {
        std::thread::scope(|scope| {
            assert!(
                scope
                    .spawn(|| {
                        let _guard = mutex.lock().unwrap();
                        panic!("injected history lock failure");
                    })
                    .join()
                    .is_err()
            );
        });
    }

    fn fixture() -> (
        WorkspaceState,
        activity_workspace::ActivityWorkspaceState,
        HistoryState,
    ) {
        let workspace = WorkspaceState::default();
        let activity = activity_workspace::ActivityWorkspaceState::default();
        let history = HistoryState::default();
        *workspace.project.lock().unwrap() = Some(Project::new("Before"));
        checkpoint_states(&workspace, &activity, &history).unwrap();
        *workspace.project.lock().unwrap() = Some(Project::new("After"));
        (workspace, activity, history)
    }

    fn project_value(workspace: &WorkspaceState) -> serde_json::Value {
        serde_json::to_value(&*workspace.project.lock().unwrap()).unwrap()
    }

    fn stack_lengths(history: &HistoryState) -> (usize, usize) {
        (
            history
                .undo
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .len(),
            history
                .redo
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .len(),
        )
    }

    #[test]
    fn failed_undo_and_redo_preserve_authored_state_and_both_stacks() {
        for undoing in [true, false] {
            for failed_lock in ["late_workspace", "undo", "redo"] {
                let (workspace, activity, history) = fixture();
                if !undoing {
                    undo_states(&workspace, &activity, &history).unwrap();
                }
                let before = project_value(&workspace);
                let lengths = stack_lengths(&history);
                match failed_lock {
                    "late_workspace" => poison(&activity.diagrams),
                    "undo" => poison(&history.undo),
                    _ => poison(&history.redo),
                }
                assert!(transfer_history(&workspace, &activity, &history, undoing).is_err());
                assert_eq!(project_value(&workspace), before);
                assert_eq!(stack_lengths(&history), lengths);
            }
        }
    }

    #[test]
    fn busy_workspace_rejects_history_operations_without_waiting_or_consuming_state() {
        for operation in ["capture", "checkpoint", "undo", "redo"] {
            let (workspace, activity, history) = fixture();
            if operation == "redo" {
                undo_states(&workspace, &activity, &history).unwrap();
            }
            let before = project_value(&workspace);
            let lengths = stack_lengths(&history);
            std::thread::scope(|scope| {
                // Model an Activity command holding diagrams before it needs
                // the repository. History must not retain that repository while
                // waiting for this guard, or both operations would deadlock.
                let held = activity.diagrams.lock().unwrap();
                let (sender, receiver) = std::sync::mpsc::channel();
                let workspace_ref = &workspace;
                let activity_ref = &activity;
                let history_ref = &history;
                let worker = scope.spawn(move || {
                    let result = match operation {
                        "capture" => capture_states(workspace_ref, activity_ref).map(|_| ()),
                        "checkpoint" => checkpoint_states(workspace_ref, activity_ref, history_ref),
                        "undo" => undo_states(workspace_ref, activity_ref, history_ref).map(|_| ()),
                        _ => redo_states(workspace_ref, activity_ref, history_ref).map(|_| ()),
                    };
                    sender.send(result).unwrap();
                });
                let received = receiver.recv_timeout(std::time::Duration::from_secs(2));
                // Always release the guard before asserting, so a regressed
                // blocking implementation fails instead of hanging the suite.
                drop(held);
                worker.join().unwrap();
                let error = received
                    .expect("history waited on a busy workspace")
                    .unwrap_err();
                assert!(error.contains("workspace is busy"), "{error}");
                assert!(activity.repository.try_lock().is_ok());
                assert!(workspace.project.try_lock().is_ok());
            });
            assert_eq!(project_value(&workspace), before);
            assert_eq!(stack_lengths(&history), lengths);
            // Contention is temporary and does not poison a later operation.
            assert!(capture_states(&workspace, &activity).is_ok());
            assert!(
                transfer_history(&workspace, &activity, &history, operation != "redo").unwrap()
            );
        }
    }

    #[test]
    fn failed_checkpoint_and_reset_do_not_change_either_stack() {
        let (workspace, activity, history) = fixture();
        undo_states(&workspace, &activity, &history).unwrap();
        checkpoint_states(&workspace, &activity, &history).unwrap();
        // Retain both kinds of history to detect partial clearing/publication.
        history
            .redo
            .lock()
            .unwrap()
            .push(capture_states(&workspace, &activity).unwrap());
        let lengths = stack_lengths(&history);
        let before = project_value(&workspace);
        poison(&history.redo);
        assert!(checkpoint_states(&workspace, &activity, &history).is_err());
        assert_eq!(stack_lengths(&history), lengths);
        assert!(reset_states(&history).is_err());
        assert_eq!(stack_lengths(&history), lengths);
        assert_eq!(project_value(&workspace), before);
    }

    #[test]
    fn successful_undo_redo_moves_state_and_preserves_identity() {
        let (workspace, activity, history) = fixture();
        let after = project_value(&workspace);
        let allocation = workspace
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .name
            .as_ptr();
        assert!(undo_states(&workspace, &activity, &history).unwrap());
        assert_eq!(
            workspace.project.lock().unwrap().as_ref().unwrap().name,
            "Before"
        );
        assert_eq!(stack_lengths(&history), (0, 1));
        assert!(redo_states(&workspace, &activity, &history).unwrap());
        assert_eq!(project_value(&workspace), after);
        assert_eq!(
            workspace
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .name
                .as_ptr(),
            allocation
        );
        assert_eq!(stack_lengths(&history), (1, 0));
        assert!(!redo_states(&workspace, &activity, &history).unwrap());
        reset_states(&history).unwrap();
        assert!(!undo_states(&workspace, &activity, &history).unwrap());
    }
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
    fn linked_property_retype_updates_bdd_and_undo_redo_restores_the_whole_edit() {
        use crate::workspace::{BddDiagram, DiagramEdge, DiagramNode};
        let (workspace, activity, history, feature, next) = fixture();
        {
            let mut guard = workspace.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            let property = project.element(feature).unwrap().clone();
            let relationship = project
                .create_property_association(feature, Some(project.root_id))
                .unwrap();
            let nodes: Vec<_> = [property.owner_id.unwrap(), property.type_id.unwrap()]
                .iter()
                .enumerate()
                .map(|(index, id)| DiagramNode {
                    id: uuid::Uuid::new_v4().to_string(),
                    element_id: id.to_string(),
                    x: 100.0 + index as f64 * 300.0,
                    y: 100.0,
                    width: 180.0,
                    height: 100.0,
                    actor_notation: None,
                    parameter_presentations: Vec::new(),
                })
                .collect();
            let edge = DiagramEdge {
                id: uuid::Uuid::new_v4().to_string(),
                relationship_id: relationship.to_string(),
                source_node_id: nodes[0].id.clone(),
                target_node_id: nodes[1].id.clone(),
                points: super::super::route_relationship(&nodes[0], &nodes[1], &nodes).unwrap(),
                label_anchor: None,
            };
            workspace.diagrams.lock().unwrap().push(BddDiagram {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Parts".into(),
                owner_id: project.root_id.to_string(),
                family: "bdd".into(),
                semantic_context_id: None,
                subject_boundary: None,
                nodes,
                edges: vec![edge],
            });
        }
        let before = (
            project_value(&workspace),
            serde_json::to_value(&*workspace.diagrams.lock().unwrap()).unwrap(),
        );
        apply_element_specification(
            &workspace,
            &activity,
            &history,
            feature,
            &ElementSpecificationEdit {
                name: "renamed".into(),
                type_id: Some(next),
                ..Default::default()
            },
        )
        .unwrap();
        let after = (
            project_value(&workspace),
            serde_json::to_value(&*workspace.diagrams.lock().unwrap()).unwrap(),
        );
        assert_ne!(after, before);
        assert_eq!(workspace.diagrams.lock().unwrap()[0].nodes.len(), 3);
        assert_eq!(undo_len(&history), 1);
        undo_states(&workspace, &activity, &history).unwrap();
        assert_eq!(
            (
                project_value(&workspace),
                serde_json::to_value(&*workspace.diagrams.lock().unwrap()).unwrap()
            ),
            before
        );
        redo_states(&workspace, &activity, &history).unwrap();
        assert_eq!(
            (
                project_value(&workspace),
                serde_json::to_value(&*workspace.diagrams.lock().unwrap()).unwrap()
            ),
            after
        );
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
