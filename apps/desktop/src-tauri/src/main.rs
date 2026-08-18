mod workspace {
    include!("workspace.rs");
    mod bdd_elements;
    mod behavior_completion;
    mod behavior_creation;
    mod behavior_workspace;
    mod feature_editing;
    mod ibd;
    mod item_flow_notation;
    mod relationship_editing;
    mod routing;
    pub use bdd_elements::{
        create_bdd_element, create_bdd_feature, create_bdd_relationship_complete,
        open_project_file_complete, place_bdd_element, save_current_project_complete,
        save_project_file_complete, update_bdd_element_details, workspace_snapshot_complete,
    };
    pub use behavior_completion::{
        add_combined_fragment_operand, add_composite_state, reconnect_sequence_message,
        update_combined_fragment_operand, update_execution_specification, update_sequence_message,
        update_state_invariant, update_state_transition,
    };
    pub use behavior_creation::{
        create_sequence_diagram_staged, create_state_machine_diagram_staged,
    };
    pub use behavior_workspace::{
        add_combined_fragment, add_execution_specification, add_sequence_lifeline,
        add_sequence_message, add_state_invariant, add_state_region, add_state_transition,
        add_state_vertex, behavior_lifeline_candidates, behavior_snapshot, create_sequence_diagram,
        create_state_machine_diagram, move_sequence_lifeline, move_state_vertex,
        update_state_behaviors,
    };
    pub use feature_editing::update_bdd_feature_semantics;
    pub use ibd::{
        add_item_flow_to_connector, add_nested_port_to_ibd, create_ibd, create_ibd_connector,
        populate_ibd_from_context, route_ibd,
    };
    pub use item_flow_notation::ibd_item_flow_notation;
    pub use relationship_editing::{
        delete_bdd_relationship, reconnect_bdd_relationship, update_association_end,
    };
}

use serde::Serialize;
use workspace::{
    WorkspaceState, add_combined_fragment, add_combined_fragment_operand, add_composite_state,
    add_execution_specification, add_item_flow_to_connector, add_nested_port_to_ibd,
    add_sequence_lifeline, add_sequence_message, add_state_invariant, add_state_region,
    add_state_transition, add_state_vertex, behavior_lifeline_candidates, behavior_snapshot,
    create_bdd, create_bdd_element, create_bdd_feature, create_bdd_relationship,
    create_bdd_relationship_complete, create_block, create_ibd, create_ibd_connector,
    create_package, create_sequence_diagram, create_sequence_diagram_staged,
    create_state_machine_diagram, create_state_machine_diagram_staged, delete_bdd_relationship,
    ibd_item_flow_notation, move_sequence_lifeline, move_state_vertex, new_project,
    open_project_file, open_project_file_complete, place_bdd_element, place_element_on_bdd,
    populate_ibd_from_context, reconnect_bdd_relationship, reconnect_sequence_message,
    rename_element, route_ibd, save_current_project, save_current_project_complete,
    save_project_file, save_project_file_complete, update_association_end,
    update_bdd_element_details, update_bdd_feature_semantics, update_combined_fragment_operand,
    update_execution_specification, update_sequence_message, update_state_behaviors,
    update_state_invariant, update_state_transition, workspace_snapshot,
    workspace_snapshot_complete,
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
    match diagram_type.as_str() {
        "BDD" => Ok(vec![
            element_item("block", "Block", "Block"),
            element_item("association-block", "Association Block", "AssociationBlock"),
            element_item("interface-block", "Interface Block", "InterfaceBlock"),
            element_item("constraint-block", "Constraint Block", "ConstraintBlock"),
            element_item("value-type", "Value Type", "ValueType"),
            element_item("data-type", "Data Type", "DataType"),
            element_item("primitive-type", "Primitive Type", "PrimitiveType"),
            element_item("enumeration", "Enumeration", "Enumeration"),
            element_item("signal", "Signal", "Signal"),
            element_item("unit", "Unit", "Unit"),
            element_item("quantity-kind", "Quantity Kind", "QuantityKind"),
            element_item(
                "instance-specification",
                "Instance Specification",
                "InstanceSpecification",
            ),
            element_item("comment", "Comment", "Comment"),
            feature_item("part-property", "Part Property", "PartProperty"),
            feature_item(
                "reference-property",
                "Reference Property",
                "ReferenceProperty",
            ),
            feature_item("value-property", "Value Property", "ValueProperty"),
            feature_item("flow-property", "Flow Property", "FlowProperty"),
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
            feature_item("slot", "Slot", "Slot"),
            relationship_item("association", "Association", "Association"),
            relationship_item("aggregation", "Aggregation", "Aggregation"),
            relationship_item("composition", "Composition", "Composition"),
            relationship_item("generalization", "Generalization", "Generalization"),
            relationship_item("dependency", "Dependency", "Dependency"),
            relationship_item("realization", "Realization", "Realization"),
        ]),
        "IBD" => Ok(vec![
            feature_item("part-property", "Part Property", "PartProperty"),
            feature_item(
                "reference-property",
                "Reference Property",
                "ReferenceProperty",
            ),
            feature_item("proxy-port", "Proxy Port", "ProxyPort"),
            feature_item("full-port", "Full Port", "FullPort"),
            relationship_item("assembly-connector", "Assembly Connector", "Assembly"),
            relationship_item("delegation-connector", "Delegation Connector", "Delegation"),
            relationship_item("item-flow", "Item Flow", "ItemFlow"),
        ]),
        "StateMachine" => Ok(vec![
            element_item("State", "State", "State"),
            element_item("CompositeState", "Composite State", "CompositeState"),
            element_item("OrthogonalState", "Orthogonal State", "OrthogonalState"),
            element_item("Initial", "Initial", "InitialPseudostate"),
            element_item("FinalState", "Final State", "FinalState"),
            element_item("Choice", "Choice", "Choice"),
            element_item("Junction", "Junction", "Junction"),
            element_item("Fork", "Fork", "Fork"),
            element_item("Join", "Join", "Join"),
            element_item("ShallowHistory", "Shallow History", "ShallowHistory"),
            element_item("DeepHistory", "Deep History", "DeepHistory"),
            element_item("EntryPoint", "Entry Point", "EntryPoint"),
            element_item("ExitPoint", "Exit Point", "ExitPoint"),
            element_item("Terminate", "Terminate", "Terminate"),
            relationship_item("Transition", "Transition", "Transition"),
        ]),
        "Sequence" => Ok(vec![
            element_item("Lifeline", "Lifeline", "Lifeline"),
            relationship_item("SynchCall", "Synchronous Call", "SynchCall"),
            relationship_item("AsynchCall", "Asynchronous Call", "AsynchCall"),
            relationship_item("AsynchSignal", "Asynchronous Signal", "AsynchSignal"),
            relationship_item("Reply", "Reply", "Reply"),
            relationship_item("Create", "Create Message", "Create"),
            relationship_item("Delete", "Delete Message", "Delete"),
            relationship_item("Lost", "Lost Message", "Lost"),
            relationship_item("Found", "Found Message", "Found"),
            element_item(
                "Execution",
                "Execution Specification",
                "ExecutionSpecification",
            ),
            element_item("alt", "alt Fragment", "CombinedFragment"),
            element_item("opt", "opt Fragment", "CombinedFragment"),
            element_item("loop", "loop Fragment", "CombinedFragment"),
            element_item("break", "break Fragment", "CombinedFragment"),
            element_item("par", "par Fragment", "CombinedFragment"),
            element_item("critical", "critical Fragment", "CombinedFragment"),
            element_item("neg", "neg Fragment", "CombinedFragment"),
            element_item("assert", "assert Fragment", "CombinedFragment"),
            element_item("strict", "strict Fragment", "CombinedFragment"),
            element_item("seq", "seq Fragment", "CombinedFragment"),
            element_item("ignore", "ignore Fragment", "CombinedFragment"),
            element_item("consider", "consider Fragment", "CombinedFragment"),
            element_item("Invariant", "State Invariant", "StateInvariant"),
        ]),
        _ => Err(format!("unsupported diagram palette: {diagram_type}")),
    }
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
            create_ibd,
            populate_ibd_from_context,
            add_nested_port_to_ibd,
            create_ibd_connector,
            add_item_flow_to_connector,
            ibd_item_flow_notation,
            route_ibd,
            behavior_snapshot,
            create_state_machine_diagram,
            create_sequence_diagram,
            create_state_machine_diagram_staged,
            create_sequence_diagram_staged,
            add_state_vertex,
            add_composite_state,
            add_state_region,
            update_state_behaviors,
            add_state_transition,
            update_state_transition,
            move_state_vertex,
            behavior_lifeline_candidates,
            add_sequence_lifeline,
            move_sequence_lifeline,
            add_sequence_message,
            update_sequence_message,
            reconnect_sequence_message,
            add_execution_specification,
            update_execution_specification,
            add_combined_fragment,
            add_combined_fragment_operand,
            update_combined_fragment_operand,
            add_state_invariant,
            update_state_invariant,
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
