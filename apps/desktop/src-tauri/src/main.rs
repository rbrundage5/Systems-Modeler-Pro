mod workspace {
    include!("workspace.rs");
    mod relationship_editing;
    pub use relationship_editing::{
        delete_bdd_relationship, reconnect_bdd_relationship, update_association_end,
    };
}

use serde::Serialize;
use workspace::{
    WorkspaceState, create_bdd, create_bdd_relationship, create_block, create_package,
    delete_bdd_relationship, new_project, open_project_file, place_element_on_bdd,
    reconnect_bdd_relationship, rename_element, save_current_project, save_project_file,
    update_association_end, workspace_snapshot,
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
            save_project_file,
            save_current_project,
            open_project_file,
            create_package,
            create_block,
            rename_element,
            create_bdd,
            place_element_on_bdd,
            create_bdd_relationship,
            update_association_end,
            reconnect_bdd_relationship,
            delete_bdd_relationship
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Systems Modeler Pro");
}
