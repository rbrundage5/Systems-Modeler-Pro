from pathlib import Path


def replace_once(path: str, old: str, new: str, label: str) -> None:
    file_path = Path(path)
    payload = file_path.read_text(encoding="utf-8")
    if old not in payload:
        raise SystemExit(f"{path}: missing {label} anchor")
    file_path.write_text(payload.replace(old, new, 1), encoding="utf-8")


workspace = Path("apps/desktop/src-tauri/src/workspace.rs")
workspace_text = workspace.read_text(encoding="utf-8")
if "pub behavior_repository: BehaviorRepository" in workspace_text:
    print("Unified workspace snapshot is already applied")
    raise SystemExit(0)

replace_once(
    str(workspace),
    """pub struct WorkspaceSnapshot {
    pub project: Option<ProjectSnapshot>,
    pub diagrams: Vec<BddDiagram>,
    pub ibd_diagrams: Vec<ibd::IbdDiagram>,
    pub current_file: Option<String>,
}""",
    """pub struct WorkspaceSnapshot {
    pub project: Option<ProjectSnapshot>,
    pub diagrams: Vec<BddDiagram>,
    pub ibd_diagrams: Vec<ibd::IbdDiagram>,
    pub behavior_repository: BehaviorRepository,
    pub behavior_diagrams: Vec<behavior_workspace::BehaviorDiagram>,
    pub current_file: Option<String>,
}""",
    "WorkspaceSnapshot",
)

replace_once(
    str(workspace),
    """    let ibd_diagrams = state.ibd_diagrams.lock().map_err(|_| \"IBD lock poisoned\")?;
    let current_file = state.current_file.lock().map_err(|_| \"project path lock poisoned\")?;
    Ok(WorkspaceSnapshot {
        project: project.as_ref().map(snapshot_project),
        diagrams: diagrams.clone(),
        ibd_diagrams: ibd_diagrams.clone(),
        current_file: current_file.clone(),
    })""",
    """    let ibd_diagrams = state.ibd_diagrams.lock().map_err(|_| \"IBD lock poisoned\")?;
    let behavior_repository = state.behavior.lock().map_err(|_| \"behavior lock poisoned\")?;
    let behavior_diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| \"behavior diagram lock poisoned\")?;
    let current_file = state.current_file.lock().map_err(|_| \"project path lock poisoned\")?;
    Ok(WorkspaceSnapshot {
        project: project.as_ref().map(snapshot_project),
        diagrams: diagrams.clone(),
        ibd_diagrams: ibd_diagrams.clone(),
        behavior_repository: behavior_repository.clone(),
        behavior_diagrams: behavior_diagrams.clone(),
        current_file: current_file.clone(),
    })""",
    "workspace_snapshot body",
)

replace_once(
    "apps/desktop/frontend/app.js",
    """  state.snapshot = await requireInvoke()('workspace_snapshot');
  if (state.snapshot.diagrams.length) state.selectedDiagramId = state.snapshot.diagrams[0].id;
  await loadPalette();
  render();""",
    """  await refresh();
  if (!state.selectedDiagramId && state.snapshot?.diagrams?.length) {
    state.selectedDiagramId = state.snapshot.diagrams[0].id;
    await loadPalette();
    render();
  }""",
    "openProject direct snapshot bypass",
)

behavior_ui = Path("apps/desktop/frontend/behavior-ui.js")
behavior_text = behavior_ui.read_text(encoding="utf-8")
behavior_text = behavior_text.replace(
    """  const MESSAGE_SORTS = new Set([
    'SynchCall', 'AsynchCall', 'AsynchSignal', 'Reply', 'Create', 'Delete', 'Lost', 'Found',
  ]);

  function activeBehaviorDiagram() {""",
    """  const MESSAGE_SORTS = new Set([
    'SynchCall', 'AsynchCall', 'AsynchSignal', 'Reply', 'Create', 'Delete', 'Lost', 'Found',
  ]);

  function syncBehaviorSnapshotFromWorkspace() {
    if (state.snapshot?.behavior_repository && Array.isArray(state.snapshot?.behavior_diagrams)) {
      state.behaviorSnapshot = {
        repository: state.snapshot.behavior_repository,
        diagrams: state.snapshot.behavior_diagrams,
      };
    }
    return state.behaviorSnapshot;
  }

  function activeBehaviorDiagram() {
    syncBehaviorSnapshotFromWorkspace();""",
    1,
)
behavior_text = behavior_text.replace(
    """  async function loadBehaviorSnapshot() {
    try {
      state.behaviorSnapshot = await requireInvoke()('behavior_snapshot');
    } catch (error) {
      console.error('Unable to load Rust behavior workspace', error);
      state.behaviorSnapshot = {
        repository: { state_machines: {}, interactions: {} },
        diagrams: [],
      };
      throw error;
    }
  }""",
    """  async function loadBehaviorSnapshot() {
    if (state.snapshot?.behavior_repository && Array.isArray(state.snapshot?.behavior_diagrams)) {
      return syncBehaviorSnapshotFromWorkspace();
    }
    try {
      state.behaviorSnapshot = await requireInvoke()('behavior_snapshot');
      return state.behaviorSnapshot;
    } catch (error) {
      console.error('Unable to load Rust behavior workspace', error);
      state.behaviorSnapshot = {
        repository: { state_machines: {}, interactions: {} },
        diagrams: [],
      };
      throw error;
    }
  }""",
    1,
)
behavior_text = behavior_text.replace(
    """    await baseRefresh();
    await loadBehaviorSnapshot();""",
    """    await baseRefresh();
    syncBehaviorSnapshotFromWorkspace();""",
    1,
)
behavior_text = behavior_text.replace(
    """  renderDiagramTabs = function renderBehaviorDiagramTabs() {
    baseRenderDiagramTabs();""",
    """  renderDiagramTabs = function renderBehaviorDiagramTabs() {
    syncBehaviorSnapshotFromWorkspace();
    baseRenderDiagramTabs();""",
    1,
)
behavior_text = behavior_text.replace(
    """  renderRepository = function renderBehaviorRepository() {
    baseRenderRepository();""",
    """  renderRepository = function renderBehaviorRepository() {
    syncBehaviorSnapshotFromWorkspace();
    baseRenderRepository();""",
    1,
)
old_startup = """  loadBehaviorSnapshot().then(() => render()).catch((error) => {
    const status = $('status');
    if (status) status.textContent = `Behavior workspace unavailable: ${error?.message || String(error)}`;
  });"""
if old_startup not in behavior_text:
    raise SystemExit("behavior-ui.js: missing startup load anchor")
behavior_text = behavior_text.replace(
    old_startup,
    """  // app.js owns initial loading. Behavior state is taken from the same Rust
  // workspace snapshot so a stale startup request cannot overwrite a project Open.
  syncBehaviorSnapshotFromWorkspace();""",
    1,
)
if "syncBehaviorSnapshotFromWorkspace" not in behavior_text:
    raise SystemExit("behavior-ui.js: unified snapshot transformation failed")
behavior_ui.write_text(behavior_text, encoding="utf-8")

command_authority = Path("apps/desktop/frontend/behavior-command-authority.js")
command_text = command_authority.read_text(encoding="utf-8")
open_command_block = """      if (command === 'open_project_file') {
        return invoke(command, args).then(async (result) => {
          await window.smpLoadBehaviorSnapshot?.();
          return result;
        });
      }
"""
command_text = command_text.replace(open_command_block, "", 1)
legacy_comment = command_text.find("  // app.js historically reloads only the structural workspace snapshot after")
if legacy_comment != -1:
    command_text = command_text[:legacy_comment] + "})();\n"
command_authority.write_text(command_text, encoding="utf-8")

behavior_workspace = Path("apps/desktop/src-tauri/src/workspace/behavior_workspace.rs")
behavior_workspace_text = behavior_workspace.read_text(encoding="utf-8")
if "behavior_metadata_database_round_trip_preserves_stm_and_seq_diagrams" not in behavior_workspace_text:
    behavior_workspace_text += r'''

#[cfg(test)]
mod behavior_metadata_database_tests {
    use super::*;

    #[test]
    fn behavior_metadata_database_round_trip_preserves_stm_and_seq_diagrams() {
        let mut project = Project::new("Behavior Round Trip");
        let package = project
            .create_element(ElementKind::Package, "Behavior", project.root_id)
            .expect("package");
        let block = project
            .create_element(ElementKind::Block, "Controller", package)
            .expect("block");

        let mut repository = BehaviorRepository::default();
        let state_machine_id = repository
            .create_state_machine(&project, block, "Controller States")
            .expect("state machine");
        let interaction_id = repository
            .create_interaction(&project, block, "Controller Sequence")
            .expect("interaction");
        let diagrams = vec![
            BehaviorDiagram {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Controller States".into(),
                owner_id: package.to_string(),
                context_id: block.to_string(),
                kind: BehaviorDiagramKind::StateMachine,
                semantic_id: state_machine_id.to_string(),
                state_nodes: Vec::new(),
                lifelines: Vec::new(),
            },
            BehaviorDiagram {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Controller Sequence".into(),
                owner_id: package.to_string(),
                context_id: block.to_string(),
                kind: BehaviorDiagramKind::Sequence,
                semantic_id: interaction_id.to_string(),
                state_nodes: Vec::new(),
                lifelines: Vec::new(),
            },
        ];

        let path = std::env::temp_dir().join(format!(
            "systems-modeler-behavior-round-trip-{}.smproj",
            uuid::Uuid::new_v4()
        ));
        {
            let mut database = ProjectDatabase::open(&path).expect("open database");
            database.save_project(&project).expect("save project");
            save_behavior_metadata(&mut database, &project, &repository, &diagrams)
                .expect("save behavior metadata");
        }
        {
            let database = ProjectDatabase::open(&path).expect("reopen database");
            let restored_project = database.load_first_project().expect("load project");
            let (restored_repository, restored_diagrams) =
                load_behavior_metadata(&database, &restored_project)
                    .expect("load behavior metadata");
            assert_eq!(restored_repository.state_machines.len(), 1);
            assert_eq!(restored_repository.interactions.len(), 1);
            assert_eq!(restored_diagrams.len(), 2);
            assert!(restored_diagrams.iter().any(|diagram| {
                diagram.kind == BehaviorDiagramKind::StateMachine
                    && diagram.semantic_id == state_machine_id.to_string()
            }));
            assert!(restored_diagrams.iter().any(|diagram| {
                diagram.kind == BehaviorDiagramKind::Sequence
                    && diagram.semantic_id == interaction_id.to_string()
            }));
        }
        let _ = std::fs::remove_file(path);
    }
}
'''
behavior_workspace.write_text(behavior_workspace_text, encoding="utf-8")

validator = Path("scripts/validate_behavior_integration.py")
validator_text = validator.read_text(encoding="utf-8")
if "Open must use unified refresh()" not in validator_text:
    insert = '''
require(
    "apps/desktop/src-tauri/src/workspace.rs",
    "pub behavior_repository: BehaviorRepository",
    "pub behavior_diagrams: Vec<behavior_workspace::BehaviorDiagram>",
    "behavior_repository: behavior_repository.clone()",
    "behavior_diagrams: behavior_diagrams.clone()",
)
require(
    "apps/desktop/frontend/behavior-ui.js",
    "syncBehaviorSnapshotFromWorkspace",
    "state.snapshot.behavior_repository",
    "state.snapshot.behavior_diagrams",
)
app_payload = text("apps/desktop/frontend/app.js")
open_section = app_payload.split("async function openProject()", 1)[1].split("async function saveProjectAs()", 1)[0]
if "await refresh();" not in open_section:
    raise SystemExit("apps/desktop/frontend/app.js: Open must use unified refresh()")
if "state.snapshot = await requireInvoke()('workspace_snapshot')" in open_section:
    raise SystemExit("apps/desktop/frontend/app.js: Open must not bypass unified refresh()")
require(
    "apps/desktop/src-tauri/src/workspace/behavior_workspace.rs",
    "behavior_metadata_database_round_trip_preserves_stm_and_seq_diagrams",
)

'''
    anchor = 'print("PR12 consolidated Rust-authoritative behavior integration is complete")'
    if anchor not in validator_text:
        raise SystemExit("validator print anchor missing")
    validator.write_text(validator_text.replace(anchor, insert + anchor), encoding="utf-8")

print("Applied unified Rust workspace snapshot and behavior save/open regression coverage")
