mod workspace {
    include!("workspace.rs");
    mod bdd_elements;
    mod feature_editing;
    mod relationship_editing;
    pub use bdd_elements::{
        create_bdd_element, create_bdd_feature, create_bdd_relationship_complete,
        open_project_file_complete, place_bdd_element, save_current_project_complete,
        save_project_file_complete, update_bdd_element_details, workspace_snapshot_complete,
    };
    pub use feature_editing::update_bdd_feature_semantics;
    pub use relationship_editing::{
        delete_bdd_relationship, reconnect_bdd_relationship, update_association_end,
    };
}

use serde::Serialize;
use workspace::{
    WorkspaceState, create_bdd, create_bdd_element, create_bdd_feature, create_bdd_relationship,
    create_bdd_relationship_complete, create_block, create_package, delete_bdd_relationship,
    new_project, open_project_file, open_project_file_complete, place_bdd_element,
    place_element_on_bdd, reconnect_bdd_relationship, rename_element, save_current_project,
    save_current_project_complete, save_project_file, save_project_file_complete,
    update_association_end, update_bdd_element_details, update_bdd_feature_semantics,
    workspace_snapshot, workspace_snapshot_complete,
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

fn element_item(
    id: &'static str,
    label: &'static str,
    semantic_kind: &'static str,
) -> DiagramPaletteItem {
    DiagramPaletteItem {
        id,
        label,
        category: "element",
        semantic_kind: Some(semantic_kind),
        relationship_kind: None,
        draggable: true,
    }
}

fn feature_item(
    id: &'static str,
    label: &'static str,
    semantic_kind: &'static str,
) -> DiagramPaletteItem {
    DiagramPaletteItem {
        id,
        label,
        category: "feature",
        semantic_kind: Some(semantic_kind),
        relationship_kind: None,
        draggable: false,
    }
}

fn relationship_item(
    id: &'static str,
    label: &'static str,
    kind: &'static str,
) -> DiagramPaletteItem {
    DiagramPaletteItem {
        id,
        label,
        category: "relationship",
        semantic_kind: None,
        relationship_kind: Some(kind),
        draggable: false,
    }
}

#[tauri::command]
fn diagram_palette(diagram_type: String) -> Result<Vec<DiagramPaletteItem>, String> {
    if diagram_type != "BDD" {
        return Err(format!("unsupported diagram palette: {diagram_type}"));
    }

    Ok(vec![
        element_item("block", "Block", "Block"),
        element_item("interface-block", "Interface Block", "InterfaceBlock"),
        element_item("value-type", "Value Type", "ValueType"),
        element_item("data-type", "Data Type", "DataType"),
        element_item("enumeration", "Enumeration", "Enumeration"),
        element_item("constraint-block", "Constraint Block", "ConstraintBlock"),
        feature_item("part-property", "Part Property", "PartProperty"),
        feature_item(
            "reference-property",
            "Reference Property",
            "ReferenceProperty",
        ),
        feature_item("value-property", "Value Property", "ValueProperty"),
        feature_item(
            "constraint-property",
            "Constraint Property",
            "ConstraintProperty",
        ),
        feature_item("proxy-port", "Proxy Port", "ProxyPort"),
        feature_item("full-port", "Full Port", "FullPort"),
        feature_item("operation", "Operation", "Operation"),
        feature_item("reception", "Reception", "Reception"),
        feature_item("parameter", "Parameter", "Parameter"),
        feature_item(
            "enumeration-literal",
            "Enumeration Literal",
            "EnumerationLiteral",
        ),
        relationship_item("association", "Association", "Association"),
        relationship_item("aggregation", "Aggregation", "Aggregation"),
        relationship_item("composition", "Composition", "Composition"),
        relationship_item("generalization", "Generalization", "Generalization"),
        relationship_item("dependency", "Dependency", "Dependency"),
        relationship_item("realization", "Realization", "Realization"),
    ])
}

fn main() {
    tauri::Builder::default()
        .manage(WorkspaceState::default())
        .invoke_handler(tauri::generate_handler![
            engine_status,
            diagram_palette,
            workspace_snapshot,
            workspace_snapshot_complete,
            new_project,
            save_project_file,
            save_project_file_complete,
            save_current_project,
            save_current_project_complete,
            open_project_file,
            open_project_file_complete,
            create_package,
            create_block,
            create_bdd_element,
            create_bdd_feature,
            update_bdd_element_details,
            update_bdd_feature_semantics,
            rename_element,
            create_bdd,
            place_element_on_bdd,
            place_bdd_element,
            create_bdd_relationship,
            create_bdd_relationship_complete,
            update_association_end,
            reconnect_bdd_relationship,
            delete_bdd_relationship
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Systems Modeler Pro");
}
