mod workspace {
    include!("workspace.rs");
    mod activity_editing;
    mod activity_mutation;
    mod activity_workspace;
    mod bdd_elements;
    mod behavior_completion;
    mod behavior_creation;
    mod behavior_workspace;
    mod feature_editing;
    mod history;
    mod ibd;
    mod item_flow_notation;
    mod layout;
    mod package_diagrams;
    mod parametrics;
    mod presentation_interaction;
    mod presentation_theme;
    mod relationship_editing;
    mod repository_editing;
    mod requirements;
    mod routing;
    mod shared_workspace;
    #[rustfmt::skip]
    mod standard_editing;
    mod standard_editing_bridge;
    mod use_cases;
    pub use activity_editing::{
        add_activity_action, add_activity_parameter_node, add_activity_partition,
        add_structured_activity_node, assign_activity_node_partition,
        assign_activity_node_structured_parent, update_activity_node_semantics,
    };
    pub use activity_mutation::{
        delete_activity_item, reconnect_activity_edge, route_activity_diagram,
    };
    pub use activity_workspace::{
        ActivityWorkspaceState, activity_snapshot, add_activity_edge, add_activity_node,
        create_activity_diagram, load_activity_workspace, reset_activity_workspace,
        save_activity_workspace,
    };
    pub use bdd_elements::{
        create_bdd_element, create_bdd_feature, create_bdd_relationship_complete,
        open_project_file_complete, place_bdd_element, save_current_project_complete,
        save_project_file_complete, update_bdd_element_details, workspace_snapshot_complete,
    };
    pub use behavior_completion::{
        add_combined_fragment_operand, add_composite_state, add_state_transition_complete,
        add_submachine_state, delete_behavior_item, reconnect_sequence_message,
        update_combined_fragment_operand, update_execution_specification, update_sequence_message,
        update_sequence_message_complete, update_state_invariant, update_state_transition,
    };
    pub use behavior_creation::{
        create_sequence_diagram_staged, create_state_machine_diagram_staged,
    };
    pub use behavior_workspace::{
        add_combined_fragment, add_execution_specification, add_sequence_lifeline,
        add_sequence_message, add_state_invariant, add_state_region, add_state_transition,
        add_state_vertex, behavior_lifeline_candidates, behavior_snapshot, create_sequence_diagram,
        create_state_machine_diagram, move_sequence_lifeline, move_state_vertex,
        resize_sequence_lifeline_timeline, route_behavior_diagram, update_state_behaviors,
    };
    pub use feature_editing::update_bdd_feature_semantics;
    pub use history::{
        HistoryState, history_checkpoint, history_redo, history_reset, history_undo,
    };
    pub use ibd::{
        add_item_flow_to_connector, add_nested_port_to_ibd, create_ibd, create_ibd_connector,
        populate_ibd_from_context, route_ibd,
    };
    pub use item_flow_notation::ibd_item_flow_notation;
    pub use package_diagrams::{create_package_diagram, place_on_package_diagram};
    pub use parametrics::{
        create_binding_connector, create_constraint_parameter,
        create_parametric_constraint_property, create_parametric_diagram,
        create_parametric_value_property, delete_binding_connector, evaluate_parametric_diagram,
        place_on_parametric_diagram, reconnect_binding_connector, update_constraint_block_details,
        update_constraint_parameter, update_constraint_parameter_presentation,
        update_parametric_constraint_property, update_parametric_presentation_geometry,
        update_parametric_value_property, update_quantity_kind_details, update_unit_details,
        update_value_type_details,
    };
    pub use presentation_interaction::{
        update_activity_presentation_geometry, update_bdd_presentation_geometry,
        update_ibd_port_geometry, update_ibd_property_geometry, update_state_presentation_geometry,
    };
    pub use presentation_theme::{
        diagram_command_manifest, semantic_presentation_manifest, semantic_presentation_stylesheet,
    };
    pub use relationship_editing::{
        delete_bdd_relationship, reconnect_bdd_relationship, update_association_end,
    };
    pub use repository_editing::{
        delete_model_element, delete_repository_diagram, move_repository_diagram,
        move_repository_element,
    };
    pub use requirements::{
        create_requirement, create_requirement_diagram, create_test_case,
        create_traceability_relationship, place_on_requirement_diagram,
        reconnect_traceability_relationship, update_requirement,
    };
    pub use routing::route_diagram_geometry;
    pub use shared_workspace::{
        SharedWorkspaceState, activate_diagram, active_diagram_command_manifest,
        active_diagram_layout, active_diagram_router, clear_workspace_interaction,
        diagram_family_registry, fit_diagram_viewport, get_diagram_frame_preference,
        get_panel_preferences, get_viewport_preference, rename_active_diagram_header,
        set_diagram_frame_preference, set_panel_preferences, set_viewport_preference,
        set_workspace_interaction, workspace_interaction_snapshot, zoom_diagram_viewport,
    };
    pub use standard_editing::StandardEditingState;
    pub use standard_editing_bridge::{
        copy_selection, delete_active_selection, duplicate_selection, move_active_selection,
        paste_selection,
    };
    pub use use_cases::{
        create_use_case_diagram, create_use_case_element, create_use_case_relationship,
        delete_use_case_relationship, place_on_use_case_diagram, reconnect_use_case_relationship,
        update_actor_details, update_extend_specification, update_use_case_actor_notation,
        update_use_case_diagram_subject, update_use_case_specification,
        update_use_case_subject_boundary_geometry,
    };
}

use serde::Serialize;
use workspace::{
    ActivityWorkspaceState, HistoryState, SharedWorkspaceState, StandardEditingState,
    WorkspaceState, activate_diagram, active_diagram_command_manifest, active_diagram_layout,
    active_diagram_router, activity_snapshot, add_activity_action, add_activity_edge,
    add_activity_node, add_activity_parameter_node, add_activity_partition, add_combined_fragment,
    add_combined_fragment_operand, add_composite_state, add_execution_specification,
    add_item_flow_to_connector, add_nested_port_to_ibd, add_sequence_lifeline,
    add_sequence_message, add_state_invariant, add_state_region, add_state_transition,
    add_state_transition_complete, add_state_vertex, add_structured_activity_node,
    add_submachine_state, assign_activity_node_partition, assign_activity_node_structured_parent,
    behavior_lifeline_candidates, behavior_snapshot, clear_workspace_interaction, copy_selection,
    create_activity_diagram, create_bdd, create_bdd_element, create_bdd_feature,
    create_bdd_relationship, create_bdd_relationship_complete, create_binding_connector,
    create_block, create_constraint_parameter, create_ibd, create_ibd_connector, create_package,
    create_package_diagram, create_parametric_constraint_property, create_parametric_diagram,
    create_parametric_value_property, create_requirement, create_requirement_diagram,
    create_sequence_diagram, create_sequence_diagram_staged, create_state_machine_diagram,
    create_state_machine_diagram_staged, create_test_case, create_traceability_relationship,
    create_use_case_diagram, create_use_case_element, create_use_case_relationship,
    delete_active_selection, delete_activity_item, delete_bdd_relationship, delete_behavior_item,
    delete_binding_connector, delete_model_element, delete_repository_diagram,
    delete_use_case_relationship, diagram_command_manifest, diagram_family_registry,
    duplicate_selection, evaluate_parametric_diagram, fit_diagram_viewport,
    get_diagram_frame_preference, get_panel_preferences, get_viewport_preference,
    history_checkpoint, history_redo, history_reset, history_undo, ibd_item_flow_notation,
    load_activity_workspace, move_active_selection, move_repository_diagram,
    move_repository_element, move_sequence_lifeline, move_state_vertex, new_project,
    open_project_file, open_project_file_complete, paste_selection, place_bdd_element,
    place_element_on_bdd, place_on_package_diagram, place_on_parametric_diagram,
    place_on_requirement_diagram, place_on_use_case_diagram, populate_ibd_from_context,
    reconnect_activity_edge, reconnect_bdd_relationship, reconnect_binding_connector,
    reconnect_sequence_message, reconnect_traceability_relationship, reconnect_use_case_relationship,
    rename_active_diagram_header, rename_element, reset_activity_workspace,
    resize_sequence_lifeline_timeline, route_activity_diagram, route_behavior_diagram,
    route_diagram_geometry, route_ibd, save_activity_workspace, save_current_project,
    save_current_project_complete, save_project_file, save_project_file_complete,
    semantic_presentation_manifest, semantic_presentation_stylesheet, set_diagram_frame_preference,
    set_panel_preferences, set_viewport_preference, set_workspace_interaction,
    update_activity_node_semantics, update_activity_presentation_geometry, update_actor_details,
    update_association_end, update_bdd_element_details, update_bdd_feature_semantics,
    update_bdd_presentation_geometry, update_combined_fragment_operand,
    update_constraint_block_details, update_constraint_parameter,
    update_constraint_parameter_presentation, update_execution_specification,
    update_extend_specification, update_ibd_port_geometry, update_ibd_property_geometry,
    update_parametric_constraint_property, update_parametric_presentation_geometry,
    update_parametric_value_property, update_quantity_kind_details, update_requirement,
    update_sequence_message, update_sequence_message_complete, update_state_behaviors,
    update_state_invariant, update_state_presentation_geometry, update_state_transition,
    update_unit_details, update_use_case_actor_notation, update_use_case_diagram_subject,
    update_use_case_specification, update_use_case_subject_boundary_geometry,
    update_value_type_details, workspace_interaction_snapshot, workspace_snapshot,
    workspace_snapshot_complete, zoom_diagram_viewport,
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
            feature_item(
                "constraint-parameter",
                "Constraint Parameter",
                "ConstraintParameter",
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
            element_item("SubmachineState", "Submachine State", "SubmachineState"),
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
        "Activity" => Ok(vec![
            element_item("Initial", "Initial Node", "Initial"),
            element_item("ActivityFinal", "Activity Final", "ActivityFinal"),
            element_item("FlowFinal", "Flow Final", "FlowFinal"),
            element_item("OpaqueAction", "Opaque Action", "OpaqueAction"),
            element_item(
                "CallBehaviorAction",
                "Call Behavior Action",
                "CallBehaviorAction",
            ),
            element_item(
                "CallOperationAction",
                "Call Operation Action",
                "CallOperationAction",
            ),
            element_item("SendSignalAction", "Send Signal Action", "SendSignalAction"),
            element_item(
                "AcceptEventAction",
                "Accept Event Action",
                "AcceptEventAction",
            ),
            element_item(
                "AcceptTimeEventAction",
                "Accept Time Event",
                "AcceptTimeEventAction",
            ),
            element_item(
                "ActivityParameterNode",
                "Activity Parameter",
                "ActivityParameterNode",
            ),
            element_item("Decision", "Decision", "Decision"),
            element_item("Merge", "Merge", "Merge"),
            element_item("Fork", "Fork", "Fork"),
            element_item("Join", "Join", "Join"),
            element_item("ObjectNode", "Object Node", "ObjectNode"),
            element_item("CentralBufferNode", "Central Buffer", "CentralBufferNode"),
            element_item("DataStoreNode", "Data Store", "DataStoreNode"),
            element_item(
                "ActivityPartition",
                "Activity Partition",
                "ActivityPartition",
            ),
            element_item(
                "StructuredActivityNode",
                "Structured Activity Node",
                "StructuredActivityNode",
            ),
            element_item("ConditionalNode", "Conditional Node", "ConditionalNode"),
            element_item("LoopNode", "Loop Node", "LoopNode"),
            element_item("SequenceNode", "Sequence Node", "SequenceNode"),
            element_item("ExpansionRegion", "Expansion Region", "ExpansionRegion"),
            element_item(
                "InterruptibleActivityRegion",
                "Interruptible Region",
                "InterruptibleActivityRegion",
            ),
            relationship_item("ControlFlow", "Control Flow", "ControlFlow"),
            relationship_item("ObjectFlow", "Object Flow", "ObjectFlow"),
        ]),
        "Requirement" => Ok(vec![
            element_item("requirement", "Requirement", "Requirement"),
            element_item("test-case", "Test Case", "TestCase"),
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
            relationship_item("derive-reqt", "Derive Requirement", "DeriveRequirement"),
            relationship_item("satisfy", "Satisfy", "Satisfy"),
            relationship_item("verify", "Verify", "Verify"),
            relationship_item("refine", "Refine", "Refine"),
            relationship_item("trace", "Trace", "Trace"),
            relationship_item("copy", "Copy", "Copy"),
        ]),
        "UseCase" => Ok(vec![
            element_item("actor", "Actor", "Actor"),
            element_item("use-case", "Use Case", "UseCase"),
            relationship_item("association", "Association", "Association"),
            relationship_item("include", "Include", "Include"),
            relationship_item("extend", "Extend", "Extend"),
            relationship_item("generalization", "Generalization", "Generalization"),
        ]),
        "Parametric" => Ok(vec![
            element_item(
                "constraint-property",
                "Constraint Property",
                "ConstraintProperty",
            ),
            element_item("value-property", "Value Property", "ValueProperty"),
            relationship_item("binding-connector", "Binding Connector", "BindingConnector"),
        ]),
        _ => Err(format!("unsupported diagram palette: {diagram_type}")),
    }
}

fn main() {
    tauri::Builder::default()
        .manage(WorkspaceState::default())
        .manage(ActivityWorkspaceState::default())
        .manage(HistoryState::default())
        .manage(SharedWorkspaceState::default())
        .manage(StandardEditingState::default())
        .invoke_handler(tauri::generate_handler![
            engine_status,
            semantic_presentation_manifest,
            semantic_presentation_stylesheet,
            diagram_command_manifest,
            active_diagram_command_manifest,
            active_diagram_layout,
            active_diagram_router,
            diagram_family_registry,
            activate_diagram,
            workspace_interaction_snapshot,
            set_workspace_interaction,
            clear_workspace_interaction,
            fit_diagram_viewport,
            zoom_diagram_viewport,
            get_viewport_preference,
            set_viewport_preference,
            get_diagram_frame_preference,
            set_diagram_frame_preference,
            get_panel_preferences,
            set_panel_preferences,
            copy_selection,
            paste_selection,
            duplicate_selection,
            delete_active_selection,
            move_active_selection,
            diagram_palette,
            create_package_diagram,
            place_on_package_diagram,
            create_parametric_diagram,
            place_on_parametric_diagram,
            create_parametric_constraint_property,
            create_parametric_value_property,
            update_constraint_block_details,
            create_constraint_parameter,
            update_constraint_parameter,
            update_parametric_constraint_property,
            update_parametric_value_property,
            update_value_type_details,
            update_quantity_kind_details,
            update_unit_details,
            create_binding_connector,
            reconnect_binding_connector,
            delete_binding_connector,
            update_parametric_presentation_geometry,
            update_constraint_parameter_presentation,
            evaluate_parametric_diagram,
            create_use_case_diagram,
            create_use_case_element,
            update_use_case_specification,
            update_use_case_diagram_subject,
            update_use_case_subject_boundary_geometry,
            update_use_case_actor_notation,
            place_on_use_case_diagram,
            create_use_case_relationship,
            update_extend_specification,
            reconnect_use_case_relationship,
            delete_use_case_relationship,
            update_actor_details,
            create_requirement_diagram,
            create_requirement,
            create_test_case,
            update_requirement,
            place_on_requirement_diagram,
            create_traceability_relationship,
            reconnect_traceability_relationship,
            history_checkpoint,
            history_undo,
            history_redo,
            history_reset,
            move_repository_element,
            delete_model_element,
            move_repository_diagram,
            delete_repository_diagram,
            workspace_snapshot,
            workspace_snapshot_complete,
            activity_snapshot,
            reset_activity_workspace,
            create_activity_diagram,
            add_activity_node,
            add_activity_edge,
            add_activity_action,
            add_activity_parameter_node,
            add_activity_partition,
            assign_activity_node_partition,
            add_structured_activity_node,
            assign_activity_node_structured_parent,
            update_activity_node_semantics,
            update_activity_presentation_geometry,
            delete_activity_item,
            reconnect_activity_edge,
            route_activity_diagram,
            save_activity_workspace,
            load_activity_workspace,
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
            update_bdd_presentation_geometry,
            rename_element,
            rename_active_diagram_header,
            create_bdd,
            create_ibd,
            populate_ibd_from_context,
            add_nested_port_to_ibd,
            update_ibd_property_geometry,
            update_ibd_port_geometry,
            create_ibd_connector,
            add_item_flow_to_connector,
            ibd_item_flow_notation,
            route_ibd,
            route_diagram_geometry,
            route_behavior_diagram,
            behavior_snapshot,
            create_state_machine_diagram,
            create_sequence_diagram,
            create_state_machine_diagram_staged,
            create_sequence_diagram_staged,
            add_state_vertex,
            add_composite_state,
            add_submachine_state,
            add_state_region,
            update_state_behaviors,
            add_state_transition,
            add_state_transition_complete,
            update_state_transition,
            move_state_vertex,
            update_state_presentation_geometry,
            behavior_lifeline_candidates,
            add_sequence_lifeline,
            move_sequence_lifeline,
            resize_sequence_lifeline_timeline,
            add_sequence_message,
            update_sequence_message,
            update_sequence_message_complete,
            reconnect_sequence_message,
            add_execution_specification,
            update_execution_specification,
            add_combined_fragment,
            add_combined_fragment_operand,
            update_combined_fragment_operand,
            add_state_invariant,
            update_state_invariant,
            delete_behavior_item,
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
