mod workspace;

use serde::Serialize;
use workspace::{
    WorkspaceState, create_bdd, create_block, create_package, new_project, place_element_on_bdd,
    rename_element, workspace_snapshot,
};

#[derive(Serialize)]
struct EngineStatus {
    product: &'static str,
    architecture: &'static str,
    storage: &'static str,
    cloud_required: bool,
}

#[tauri::command]
fn engine_status() -> EngineStatus {
    EngineStatus {
        product: "Systems Modeler Pro",
        architecture: "Rust model engine + SQLite + Tauri web frontend",
        storage: "SQLite",
        cloud_required: false,
    }
}

fn main() {
    tauri::Builder::default()
        .manage(WorkspaceState::default())
        .invoke_handler(tauri::generate_handler![
            engine_status,
            workspace_snapshot,
            new_project,
            create_package,
            create_block,
            rename_element,
            create_bdd,
            place_element_on_bdd
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Systems Modeler Pro");
}
