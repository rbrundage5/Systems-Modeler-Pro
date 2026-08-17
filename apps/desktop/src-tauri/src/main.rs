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

#[derive(Serialize)]
struct DiagramPaletteItem {
    id: &'static str,
    label: &'static str,
    category: &'static str,
    semantic_kind: Option<&'static str>,
    relationship_kind: Option<&'static str>,
    draggable: bool,
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

#[tauri::command]
fn diagram_palette(diagram_type: String) -> Result<Vec<DiagramPaletteItem>, String> {
    if diagram_type != "BDD" {
        return Err(format!("unsupported diagram palette: {diagram_type}"));
    }

    Ok(vec![
        DiagramPaletteItem {
            id: "block",
            label: "Block",
            category: "element",
            semantic_kind: Some("Block"),
            relationship_kind: None,
            draggable: true,
        },
        DiagramPaletteItem {
            id: "association",
            label: "Association",
            category: "relationship",
            semantic_kind: None,
            relationship_kind: Some("Association"),
            draggable: false,
        },
        DiagramPaletteItem {
            id: "aggregation",
            label: "Aggregation",
            category: "relationship",
            semantic_kind: None,
            relationship_kind: Some("Aggregation"),
            draggable: false,
        },
        DiagramPaletteItem {
            id: "composition",
            label: "Composition",
            category: "relationship",
            semantic_kind: None,
            relationship_kind: Some("Composition"),
            draggable: false,
        },
        DiagramPaletteItem {
            id: "generalization",
            label: "Generalization",
            category: "relationship",
            semantic_kind: None,
            relationship_kind: Some("Generalization"),
            draggable: false,
        },
        DiagramPaletteItem {
            id: "dependency",
            label: "Dependency",
            category: "relationship",
            semantic_kind: None,
            relationship_kind: Some("Dependency"),
            draggable: false,
        },
        DiagramPaletteItem {
            id: "realization",
            label: "Realization",
            category: "relationship",
            semantic_kind: None,
            relationship_kind: Some("Realization"),
            draggable: false,
        },
    ])
}

fn main() {
    tauri::Builder::default()
        .manage(WorkspaceState::default())
        .invoke_handler(tauri::generate_handler![
            engine_status,
            diagram_palette,
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
