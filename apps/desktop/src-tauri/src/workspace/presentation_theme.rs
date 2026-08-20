//! Rust-owned presentation and command metadata shared by every diagram renderer.
//! Colors supplement authoritative notation; they never encode semantic identity.

use serde::Serialize;

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationStyle {
    pub category: &'static str,
    pub fill: &'static str,
    pub header: &'static str,
    pub border: &'static str,
    pub text: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticPresentation {
    pub semantic_kind: &'static str,
    #[serde(flatten)]
    pub style: PresentationStyle,
}

const STRUCTURAL: PresentationStyle = PresentationStyle {
    category: "structural",
    fill: "#f2ead5",
    header: "#e2d4b4",
    border: "#59645c",
    text: "#17201b",
};
const INTERFACE: PresentationStyle = PresentationStyle {
    category: "interface",
    fill: "#e3f0e6",
    header: "#bed9c5",
    border: "#476454",
    text: "#152019",
};
const ACTIVITY: PresentationStyle = PresentationStyle {
    category: "activity",
    fill: "#dfeee9",
    header: "#c5dfd6",
    border: "#49645d",
    text: "#15201d",
};
const STATE: PresentationStyle = PresentationStyle {
    category: "state",
    fill: "#e2edf6",
    header: "#c8dceb",
    border: "#4d6171",
    text: "#172029",
};
const CONTROL: PresentationStyle = PresentationStyle {
    category: "control",
    fill: "#ffffff",
    header: "#ffffff",
    border: "#171717",
    text: "#111111",
};
const REQUIREMENT: PresentationStyle = PresentationStyle {
    category: "requirement",
    fill: "#f3e2e5",
    header: "#e6c6cc",
    border: "#71575c",
    text: "#291b1e",
};
const CONSTRAINT: PresentationStyle = PresentationStyle {
    category: "constraint",
    fill: "#ebe5f3",
    header: "#d8cce7",
    border: "#625873",
    text: "#211b29",
};
const DATA: PresentationStyle = PresentationStyle {
    category: "data",
    fill: "#f4efcf",
    header: "#e7dda8",
    border: "#6b6547",
    text: "#272414",
};
const EVENT: PresentationStyle = PresentationStyle {
    category: "event",
    fill: "#f4e4d6",
    header: "#e8cdb6",
    border: "#705d4d",
    text: "#2b2119",
};
const VERIFY: PresentationStyle = PresentationStyle {
    category: "verification",
    fill: "#dff0f1",
    header: "#c2dfe2",
    border: "#4d696c",
    text: "#162426",
};
const NOTE: PresentationStyle = PresentationStyle {
    category: "annotation",
    fill: "#f7f1d8",
    header: "#eee3b9",
    border: "#706a50",
    text: "#292616",
};
const FRAME: PresentationStyle = PresentationStyle {
    category: "frame",
    fill: "#f4f6f7",
    header: "#dce3e8",
    border: "#63717c",
    text: "#192128",
};

const PRESENTATIONS: &[(&str, PresentationStyle)] = &[
    ("Model", FRAME),
    ("Package", FRAME),
    ("Block", STRUCTURAL),
    ("AssociationBlock", STRUCTURAL),
    ("InstanceSpecification", STRUCTURAL),
    ("Slot", STRUCTURAL),
    ("PartProperty", STRUCTURAL),
    ("ReferenceProperty", STRUCTURAL),
    ("InterfaceBlock", INTERFACE),
    ("ProxyPort", INTERFACE),
    ("FullPort", INTERFACE),
    ("FlowProperty", INTERFACE),
    ("Activity", ACTIVITY),
    ("OpaqueAction", ACTIVITY),
    ("CallBehaviorAction", ACTIVITY),
    ("CallOperationAction", ACTIVITY),
    ("AcceptEventAction", ACTIVITY),
    ("SendSignalAction", ACTIVITY),
    ("State", STATE),
    ("FinalState", CONTROL),
    ("Pseudostate", CONTROL),
    ("Decision", CONTROL),
    ("Merge", CONTROL),
    ("Fork", CONTROL),
    ("Join", CONTROL),
    ("InitialNode", CONTROL),
    ("ActivityFinalNode", CONTROL),
    ("FlowFinalNode", CONTROL),
    ("Requirement", REQUIREMENT),
    ("ConstraintBlock", CONSTRAINT),
    ("ConstraintProperty", CONSTRAINT),
    ("ValueType", DATA),
    ("DataType", DATA),
    ("PrimitiveType", DATA),
    ("Enumeration", DATA),
    ("EnumerationLiteral", DATA),
    ("Unit", DATA),
    ("QuantityKind", DATA),
    ("ValueProperty", DATA),
    ("Parameter", DATA),
    ("Signal", EVENT),
    ("Reception", EVENT),
    ("ChangeEvent", EVENT),
    ("TimeEvent", EVENT),
    ("TestCase", VERIFY),
    ("Comment", NOTE),
    ("Rationale", NOTE),
    ("Operation", STRUCTURAL),
    ("ActivityPartition", FRAME),
    ("StructuredActivityNode", FRAME),
    ("CompositeState", FRAME),
    ("Lifeline", STRUCTURAL),
    ("CombinedFragment", FRAME),
];

#[tauri::command]
pub fn semantic_presentation_manifest() -> Vec<SemanticPresentation> {
    PRESENTATIONS
        .iter()
        .map(|(semantic_kind, style)| SemanticPresentation {
            semantic_kind,
            style: *style,
        })
        .collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramCommandCapability {
    pub id: &'static str,
    pub label: &'static str,
    pub shortcut: Option<&'static str>,
    pub supported_diagrams: &'static [&'static str],
    pub rust_adapter: Option<&'static str>,
    pub unavailable_reason: Option<&'static str>,
}

#[tauri::command]
pub fn diagram_command_manifest() -> Vec<DiagramCommandCapability> {
    const ALL: &[&str] = &["BDD", "IBD", "StateMachine", "Sequence", "Activity"];
    let mut commands = vec![
        DiagramCommandCapability {
            id: "select",
            label: "Select",
            shortcut: Some("V"),
            supported_diagrams: ALL,
            rust_adapter: None,
            unavailable_reason: None,
        },
        DiagramCommandCapability {
            id: "clearSelection",
            label: "Clear Selection",
            shortcut: Some("Escape"),
            supported_diagrams: ALL,
            rust_adapter: None,
            unavailable_reason: None,
        },
        DiagramCommandCapability {
            id: "delete",
            label: "Delete",
            shortcut: Some("Delete"),
            supported_diagrams: ALL,
            rust_adapter: Some("delete_active_selection"),
            unavailable_reason: None,
        },
        DiagramCommandCapability {
            id: "undo",
            label: "Undo",
            shortcut: Some("Ctrl+Z"),
            supported_diagrams: ALL,
            rust_adapter: Some("history_undo"),
            unavailable_reason: None,
        },
        DiagramCommandCapability {
            id: "redo",
            label: "Redo",
            shortcut: Some("Ctrl+Y"),
            supported_diagrams: ALL,
            rust_adapter: Some("history_redo"),
            unavailable_reason: None,
        },
        DiagramCommandCapability {
            id: "copy",
            label: "Copy",
            shortcut: Some("Ctrl+C"),
            supported_diagrams: ALL,
            rust_adapter: Some("copy_selection"),
            unavailable_reason: None,
        },
        DiagramCommandCapability {
            id: "paste",
            label: "Paste",
            shortcut: Some("Ctrl+V"),
            supported_diagrams: ALL,
            rust_adapter: Some("paste_selection"),
            unavailable_reason: None,
        },
        DiagramCommandCapability {
            id: "duplicate",
            label: "Duplicate",
            shortcut: Some("Ctrl+D"),
            supported_diagrams: ALL,
            rust_adapter: Some("duplicate_selection"),
            unavailable_reason: None,
        },
    ];
    commands.extend(viewport_commands());
    commands.extend([
        DiagramCommandCapability {
            id: "route",
            label: "Route",
            shortcut: None,
            supported_diagrams: &["BDD", "IBD", "Activity"],
            rust_adapter: Some("active_diagram_router"),
            unavailable_reason: Some("Routing is not applicable to this diagram type."),
        },
        DiagramCommandCapability {
            id: "cleanLayout",
            label: "Clean Layout",
            shortcut: None,
            supported_diagrams: &["BDD", "IBD", "Activity"],
            rust_adapter: Some("active_diagram_layout"),
            unavailable_reason: Some("Automatic layout is not available for this diagram type."),
        },
    ]);
    commands
}

fn viewport_commands() -> [DiagramCommandCapability; 9] {
    [
        DiagramCommandCapability {
            id: "zoomIn",
            label: "Zoom In",
            shortcut: Some("Ctrl++"),
            supported_diagrams: &["BDD", "IBD", "StateMachine", "Sequence", "Activity"],
            rust_adapter: None,
            unavailable_reason: None,
        },
        DiagramCommandCapability {
            id: "zoomOut",
            label: "Zoom Out",
            shortcut: Some("Ctrl+-"),
            supported_diagrams: &["BDD", "IBD", "StateMachine", "Sequence", "Activity"],
            rust_adapter: None,
            unavailable_reason: None,
        },
        DiagramCommandCapability {
            id: "actualSize",
            label: "100%",
            shortcut: Some("Ctrl+0"),
            supported_diagrams: &["BDD", "IBD", "StateMachine", "Sequence", "Activity"],
            rust_adapter: None,
            unavailable_reason: None,
        },
        DiagramCommandCapability {
            id: "fitDiagram",
            label: "Fit Diagram",
            shortcut: Some("Ctrl+9"),
            supported_diagrams: &["BDD", "IBD", "StateMachine", "Sequence", "Activity"],
            rust_adapter: None,
            unavailable_reason: None,
        },
        DiagramCommandCapability {
            id: "pan",
            label: "Pan",
            shortcut: Some("Space"),
            supported_diagrams: &["BDD", "IBD", "StateMachine", "Sequence", "Activity"],
            rust_adapter: None,
            unavailable_reason: None,
        },
        DiagramCommandCapability {
            id: "toggleGrid",
            label: "Show/Hide Grid",
            shortcut: None,
            supported_diagrams: &["BDD", "IBD", "StateMachine", "Sequence", "Activity"],
            rust_adapter: None,
            unavailable_reason: None,
        },
        DiagramCommandCapability {
            id: "snapGrid",
            label: "Snap to Grid",
            shortcut: None,
            supported_diagrams: &["BDD", "IBD", "StateMachine", "Sequence", "Activity"],
            rust_adapter: None,
            unavailable_reason: None,
        },
        DiagramCommandCapability {
            id: "showRepository",
            label: "Show/Hide Repository",
            shortcut: None,
            supported_diagrams: &["BDD", "IBD", "StateMachine", "Sequence", "Activity"],
            rust_adapter: None,
            unavailable_reason: None,
        },
        DiagramCommandCapability {
            id: "showProperties",
            label: "Show/Hide Properties",
            shortcut: None,
            supported_diagrams: &["BDD", "IBD", "StateMachine", "Sequence", "Activity"],
            rust_adapter: None,
            unavailable_reason: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_core_element_kind_has_a_presentation() {
        let required = [
            "Model",
            "Package",
            "Block",
            "AssociationBlock",
            "InterfaceBlock",
            "ConstraintBlock",
            "ValueType",
            "DataType",
            "PrimitiveType",
            "Enumeration",
            "EnumerationLiteral",
            "Signal",
            "Unit",
            "QuantityKind",
            "InstanceSpecification",
            "Slot",
            "PartProperty",
            "ReferenceProperty",
            "ValueProperty",
            "FlowProperty",
            "ConstraintProperty",
            "ProxyPort",
            "FullPort",
            "Operation",
            "Parameter",
            "Reception",
            "Comment",
        ];
        for kind in required {
            assert!(
                PRESENTATIONS.iter().any(|entry| entry.0 == kind),
                "missing {kind}"
            );
        }
    }
    #[test]
    fn global_view_commands_cover_all_diagrams() {
        for command in diagram_command_manifest().into_iter().filter(|c| {
            matches!(
                c.id,
                "zoomIn" | "zoomOut" | "actualSize" | "fitDiagram" | "pan"
            )
        }) {
            assert_eq!(command.supported_diagrams.len(), 5);
        }
    }
}
