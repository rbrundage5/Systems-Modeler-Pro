//! Rust-owned presentation and command metadata shared by every diagram renderer.
//! Colors supplement authoritative notation; they never encode semantic identity.

use serde::Serialize;
use systems_modeler_core::DiagramCapability;

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationStyle {
    pub category: &'static str,
    pub fill: &'static str,
    pub header: &'static str,
    pub border: &'static str,
    pub text: &'static str,
}

#[derive(Clone, Serialize)]
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
const STRUCTURAL_ASSOCIATION: PresentationStyle = PresentationStyle {
    category: "structural",
    fill: "#e8dfc7",
    header: "#d4c49f",
    border: "#526058",
    text: "#17201b",
};
const STRUCTURAL_INSTANCE: PresentationStyle = PresentationStyle {
    category: "structural",
    fill: "#f7f1e3",
    header: "#e7dbc0",
    border: "#626b63",
    text: "#1c241f",
};
const STRUCTURAL_PART: PresentationStyle = PresentationStyle {
    category: "structural",
    fill: "#eadfbe",
    header: "#d7c696",
    border: "#536058",
    text: "#17201b",
};
const STRUCTURAL_REFERENCE: PresentationStyle = PresentationStyle {
    category: "structural",
    fill: "#f8f2df",
    header: "#e9ddba",
    border: "#687168",
    text: "#1c241f",
};
const INTERFACE_PORT: PresentationStyle = PresentationStyle {
    category: "interface",
    fill: "#d2e7d8",
    header: "#acd0b5",
    border: "#3f5e4b",
    text: "#132019",
};
const INTERFACE: PresentationStyle = PresentationStyle {
    category: "interface",
    fill: "#e3f0e6",
    header: "#bed9c5",
    border: "#476454",
    text: "#152019",
};
const INTERFACE_FULL_PORT: PresentationStyle = PresentationStyle {
    category: "interface",
    fill: "#315443",
    header: "#315443",
    border: "#1f382c",
    text: "#ffffff",
};
const ACTIVITY: PresentationStyle = PresentationStyle {
    category: "activity",
    fill: "#dfeee9",
    header: "#c5dfd6",
    border: "#49645d",
    text: "#15201d",
};
const ACTIVITY_CALL: PresentationStyle = PresentationStyle {
    category: "activity",
    fill: "#bfdccf",
    header: "#9fc8b6",
    border: "#3f6658",
    text: "#10231c",
};
const ACTIVITY_OPERATION: PresentationStyle = PresentationStyle {
    category: "activity",
    fill: "#cee5d9",
    header: "#afd3c2",
    border: "#456b5d",
    text: "#12241d",
};
const ACTIVITY_OBJECT: PresentationStyle = PresentationStyle {
    category: "activity",
    fill: "#edf6f1",
    header: "#d5e9df",
    border: "#55766a",
    text: "#17251f",
};
const ACTIVITY_EVENT: PresentationStyle = PresentationStyle {
    category: "activity",
    fill: "#d9eadc",
    header: "#bad6c0",
    border: "#536f58",
    text: "#18251a",
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
const SEQUENCE_EXECUTION: PresentationStyle = PresentationStyle {
    category: "sequence",
    fill: "#d7e5f0",
    header: "#bfd3e3",
    border: "#536a7a",
    text: "#17232b",
};
const SEQUENCE_INVARIANT: PresentationStyle = PresentationStyle {
    category: "sequence",
    fill: "#f5efd7",
    header: "#e7dcad",
    border: "#6c664d",
    text: "#272414",
};

const PRESENTATIONS: &[(&str, PresentationStyle)] = &[
    ("Model", FRAME),
    ("Package", FRAME),
    ("Block", STRUCTURAL),
    ("AssociationBlock", STRUCTURAL_ASSOCIATION),
    ("InstanceSpecification", STRUCTURAL_INSTANCE),
    ("Slot", STRUCTURAL),
    ("PartProperty", STRUCTURAL_PART),
    ("ReferenceProperty", STRUCTURAL_REFERENCE),
    ("InterfaceBlock", INTERFACE),
    ("ProxyPort", INTERFACE_PORT),
    ("FullPort", INTERFACE_FULL_PORT),
    ("FlowProperty", INTERFACE),
    ("Activity", ACTIVITY),
    ("OpaqueAction", ACTIVITY),
    ("CallBehaviorAction", ACTIVITY_CALL),
    ("CallOperationAction", ACTIVITY_OPERATION),
    ("AcceptEventAction", ACTIVITY_EVENT),
    ("AcceptTimeEventAction", ACTIVITY_EVENT),
    ("SendSignalAction", ACTIVITY_EVENT),
    ("ObjectNode", ACTIVITY_OBJECT),
    ("CentralBufferNode", ACTIVITY_OBJECT),
    ("DataStoreNode", ACTIVITY_OBJECT),
    ("ActivityParameterNode", ACTIVITY_OBJECT),
    ("State", STATE),
    ("FinalState", CONTROL),
    ("Pseudostate", CONTROL),
    ("Decision", CONTROL),
    ("Merge", CONTROL),
    ("Fork", CONTROL),
    ("Join", CONTROL),
    ("InitialNode", CONTROL),
    ("Initial", CONTROL),
    ("ActivityFinalNode", CONTROL),
    ("ActivityFinal", CONTROL),
    ("FlowFinalNode", CONTROL),
    ("FlowFinal", CONTROL),
    ("Choice", CONTROL),
    ("Junction", CONTROL),
    ("EntryPoint", CONTROL),
    ("ExitPoint", CONTROL),
    ("ShallowHistory", CONTROL),
    ("DeepHistory", CONTROL),
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
    ("ExecutionSpecification", SEQUENCE_EXECUTION),
    ("StateInvariant", SEQUENCE_INVARIANT),
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

#[tauri::command]
pub fn semantic_presentation_stylesheet() -> String {
    let mut stylesheet = String::from(
        "/* Rust-generated semantic presentation tokens. */\n[data-semantic-kind]:not([data-semantic-kind=\"Lifeline\"]){background:var(--semantic-fill)!important;border-color:var(--semantic-border)!important;color:var(--semantic-text)!important}\n[data-semantic-kind=\"Lifeline\"]{background:transparent!important}\n[data-semantic-kind] :is(.diagram-header,.lifeline-head,.classifier-header,.block-name){background:var(--semantic-header)!important;color:var(--semantic-text)!important}\n[data-semantic-kind] .classifier-header .block-name{background:transparent!important}\nsvg [data-semantic-kind] :is(rect,polygon,ellipse){fill:var(--semantic-fill)!important;stroke:var(--semantic-border)!important}\nsvg [data-semantic-kind] text{fill:var(--semantic-text)!important}\n",
    );
    for (kind, style) in PRESENTATIONS {
        stylesheet.push_str(&format!(
            "[data-semantic-kind=\"{kind}\"]{{--semantic-fill:{};--semantic-header:{};--semantic-border:{};--semantic-text:{}}}\n",
            style.fill, style.header, style.border, style.text
        ));
    }
    stylesheet
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
    pub required_capability: Option<DiagramCapability>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedDiagramCommand {
    #[serde(flatten)]
    pub command: DiagramCommandCapability,
    pub enabled: bool,
    pub disabled_reason: Option<&'static str>,
}

pub fn resolve_diagram_commands(
    family: Option<&systems_modeler_core::DiagramFamilyDescriptor>,
) -> Vec<ResolvedDiagramCommand> {
    diagram_command_manifest()
        .into_iter()
        .map(|command| {
            let enabled = command
                .required_capability
                .is_none_or(|capability| family.is_some_and(|active| active.supports(capability)));
            let disabled_reason = (!enabled).then_some(
                command
                    .unavailable_reason
                    .unwrap_or("This command is unavailable in the active diagram."),
            );
            ResolvedDiagramCommand {
                command,
                enabled,
                disabled_reason,
            }
        })
        .collect()
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
            required_capability: None,
        },
        DiagramCommandCapability {
            id: "clearSelection",
            label: "Clear Selection",
            shortcut: Some("Escape"),
            supported_diagrams: ALL,
            rust_adapter: None,
            unavailable_reason: None,
            required_capability: None,
        },
        DiagramCommandCapability {
            id: "delete",
            label: "Delete",
            shortcut: Some("Delete"),
            supported_diagrams: ALL,
            rust_adapter: Some("delete_active_selection"),
            unavailable_reason: None,
            required_capability: Some(DiagramCapability::Delete),
        },
        DiagramCommandCapability {
            id: "undo",
            label: "Undo",
            shortcut: Some("Ctrl+Z"),
            supported_diagrams: ALL,
            rust_adapter: Some("history_undo"),
            unavailable_reason: None,
            required_capability: None,
        },
        DiagramCommandCapability {
            id: "redo",
            label: "Redo",
            shortcut: Some("Ctrl+Y"),
            supported_diagrams: ALL,
            rust_adapter: Some("history_redo"),
            unavailable_reason: None,
            required_capability: None,
        },
        DiagramCommandCapability {
            id: "copy",
            label: "Copy",
            shortcut: Some("Ctrl+C"),
            supported_diagrams: ALL,
            rust_adapter: Some("copy_selection"),
            unavailable_reason: None,
            required_capability: Some(DiagramCapability::Clipboard),
        },
        DiagramCommandCapability {
            id: "paste",
            label: "Paste",
            shortcut: Some("Ctrl+V"),
            supported_diagrams: ALL,
            rust_adapter: Some("paste_selection"),
            unavailable_reason: None,
            required_capability: Some(DiagramCapability::Clipboard),
        },
        DiagramCommandCapability {
            id: "duplicate",
            label: "Duplicate",
            shortcut: Some("Ctrl+D"),
            supported_diagrams: ALL,
            rust_adapter: Some("duplicate_selection"),
            unavailable_reason: None,
            required_capability: Some(DiagramCapability::Clipboard),
        },
    ];
    commands.extend(viewport_commands());
    commands.extend([
        DiagramCommandCapability {
            id: "route",
            label: "Route",
            shortcut: None,
            supported_diagrams: &[
                "BDD",
                "IBD",
                "Requirement",
                "StateMachine",
                "Sequence",
                "Activity",
            ],
            rust_adapter: Some("active_diagram_router"),
            unavailable_reason: Some("Routing is not applicable to this diagram type."),
            required_capability: Some(DiagramCapability::Routing),
        },
        DiagramCommandCapability {
            id: "cleanLayout",
            label: "Clean Layout",
            shortcut: None,
            supported_diagrams: &[
                "BDD",
                "IBD",
                "Requirement",
                "StateMachine",
                "Sequence",
                "Activity",
            ],
            rust_adapter: Some("active_diagram_layout"),
            unavailable_reason: Some("Automatic layout is not available for this diagram type."),
            required_capability: Some(DiagramCapability::CleanLayout),
        },
    ]);
    commands
}

fn viewport_commands() -> Vec<DiagramCommandCapability> {
    vec![
        DiagramCommandCapability {
            id: "zoomIn",
            label: "Zoom In",
            shortcut: Some("Ctrl++"),
            supported_diagrams: &["BDD", "IBD", "StateMachine", "Sequence", "Activity"],
            rust_adapter: None,
            unavailable_reason: None,
            required_capability: None,
        },
        DiagramCommandCapability {
            id: "zoomOut",
            label: "Zoom Out",
            shortcut: Some("Ctrl+-"),
            supported_diagrams: &["BDD", "IBD", "StateMachine", "Sequence", "Activity"],
            rust_adapter: None,
            unavailable_reason: None,
            required_capability: None,
        },
        DiagramCommandCapability {
            id: "actualSize",
            label: "100%",
            shortcut: Some("Ctrl+0"),
            supported_diagrams: &["BDD", "IBD", "StateMachine", "Sequence", "Activity"],
            rust_adapter: None,
            unavailable_reason: None,
            required_capability: None,
        },
        DiagramCommandCapability {
            id: "fitDiagram",
            label: "Fit Diagram",
            shortcut: Some("Ctrl+9"),
            supported_diagrams: &["BDD", "IBD", "StateMachine", "Sequence", "Activity"],
            rust_adapter: None,
            unavailable_reason: None,
            required_capability: None,
        },
        DiagramCommandCapability {
            id: "pan",
            label: "Pan",
            shortcut: Some("Space"),
            supported_diagrams: &["BDD", "IBD", "StateMachine", "Sequence", "Activity"],
            rust_adapter: None,
            unavailable_reason: None,
            required_capability: None,
        },
        DiagramCommandCapability {
            id: "toggleGrid",
            label: "Show/Hide Grid",
            shortcut: None,
            supported_diagrams: &["BDD", "IBD", "StateMachine", "Sequence", "Activity"],
            rust_adapter: None,
            unavailable_reason: None,
            required_capability: None,
        },
        DiagramCommandCapability {
            id: "snapGrid",
            label: "Snap to Grid",
            shortcut: None,
            supported_diagrams: &["BDD", "IBD", "StateMachine", "Sequence", "Activity"],
            rust_adapter: None,
            unavailable_reason: None,
            required_capability: None,
        },
        DiagramCommandCapability {
            id: "showRepository",
            label: "Show/Hide Repository",
            shortcut: None,
            supported_diagrams: &["BDD", "IBD", "StateMachine", "Sequence", "Activity"],
            rust_adapter: None,
            unavailable_reason: None,
            required_capability: None,
        },
        DiagramCommandCapability {
            id: "showElements",
            label: "Show/Hide Elements",
            shortcut: None,
            supported_diagrams: &["BDD", "IBD", "StateMachine", "Sequence", "Activity"],
            rust_adapter: None,
            unavailable_reason: None,
            required_capability: None,
        },
        DiagramCommandCapability {
            id: "showProperties",
            label: "Show/Hide Properties",
            shortcut: None,
            supported_diagrams: &["BDD", "IBD", "StateMachine", "Sequence", "Activity"],
            rust_adapter: None,
            unavailable_reason: None,
            required_capability: None,
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

    #[test]
    fn every_workspace_panel_has_a_registered_toggle() {
        let commands = diagram_command_manifest();
        for id in ["showRepository", "showElements", "showProperties"] {
            assert!(
                commands.iter().any(|command| command.id == id),
                "missing {id}"
            );
        }
    }

    #[test]
    fn command_eligibility_is_resolved_from_registered_family_capabilities() {
        let registry = systems_modeler_core::supported_diagram_families();
        let sequence = registry
            .get(&systems_modeler_core::DiagramFamilyId("sequence".into()))
            .expect("sequence is registered");
        let commands = resolve_diagram_commands(Some(sequence));
        let route = commands
            .iter()
            .find(|item| item.command.id == "route")
            .expect("Route is registered");
        assert!(route.enabled);
        assert!(route.disabled_reason.is_none());
        let layout = commands
            .iter()
            .find(|item| item.command.id == "cleanLayout")
            .expect("Clean Layout is registered");
        assert!(layout.enabled);
        assert!(layout.disabled_reason.is_none());
    }

    #[test]
    fn generated_stylesheet_covers_every_registered_presentation() {
        let stylesheet = semantic_presentation_stylesheet();
        for (kind, _) in PRESENTATIONS {
            assert!(stylesheet.contains(&format!("data-semantic-kind=\"{kind}\"")));
        }
        assert!(!stylesheet.contains("undefined"));
        assert!(stylesheet.contains("[data-semantic-kind=\"Lifeline\"]{background:transparent"));
        let manifest = semantic_presentation_manifest();
        let opaque = manifest
            .iter()
            .find(|item| item.semantic_kind == "OpaqueAction")
            .unwrap();
        let call = manifest
            .iter()
            .find(|item| item.semantic_kind == "CallBehaviorAction")
            .unwrap();
        assert_eq!(opaque.style.category, "activity");
        assert_eq!(call.style.category, "activity");
        assert_ne!(opaque.style.fill, call.style.fill);
    }
}
