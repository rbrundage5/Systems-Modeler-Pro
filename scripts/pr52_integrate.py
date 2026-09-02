from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if new in text:
        return text
    if old not in text:
        raise SystemExit(f"{label}: source fragment not found")
    return text.replace(old, new, 1)


# Persist the detected/explicit mappings with the source provenance and emit a
# standards-shaped header/XHTML namespace on deterministic export.
path = Path("apps/desktop/src-tauri/src/workspace/reqif_interchange.rs")
text = path.read_text(encoding="utf-8")
text = replace_once(
    text,
    '''pub struct ReqifSourceState {\n    pub document: ReqifDocument,\n    #[serde(default)]\n    pub element_bindings: BTreeMap<String, String>,\n''',
    '''pub struct ReqifSourceState {\n    pub document: ReqifDocument,\n    #[serde(default)]\n    pub configuration: Option<ReqifImportConfiguration>,\n    #[serde(default)]\n    pub element_bindings: BTreeMap<String, String>,\n''',
    "ReqIF source mapping persistence",
)
text = replace_once(
    text,
    '''    let title = header.map(node_long_name).unwrap_or_default();\n''',
    '''    let title = header\n        .and_then(|item| child_element(item, "TITLE"))\n        .and_then(|item| item.text())\n        .map(str::trim)\n        .filter(|value| !value.is_empty())\n        .map(ToOwned::to_owned)\n        .or_else(|| header.map(node_long_name).filter(|value| !value.is_empty()))\n        .unwrap_or_default();\n''',
    "ReqIF standard TITLE parsing",
)
text = replace_once(
    text,
    '''        "<?xml version=\\\"1.0\\\" encoding=\\\"UTF-8\\\"?>\\n<REQ-IF xmlns=\\\"http://www.omg.org/spec/ReqIF/20110401/reqif.xsd\\\">\\n",\n''',
    '''        "<?xml version=\\\"1.0\\\" encoding=\\\"UTF-8\\\"?>\\n<REQ-IF xmlns=\\\"http://www.omg.org/spec/ReqIF/20110401/reqif.xsd\\\" xmlns:xhtml=\\\"http://www.w3.org/1999/xhtml\\\">\\n",\n''',
    "ReqIF XHTML namespace",
)
text = replace_once(
    text,
    '''    out.push_str("  <THE-HEADER><REQ-IF-HEADER");\n    out.push_str(&format!(\n        " IDENTIFIER=\\\"{}\\\" LONG-NAME=\\\"{}\\\">",\n        escape_xml(&document.header_identifier),\n        escape_xml(&document.title)\n    ));\n    if let Some(time) = &document.creation_time {\n        out.push_str(&format!("<CREATION-TIME>{}</CREATION-TIME>", escape_xml(time)));\n    }\n    out.push_str("</REQ-IF-HEADER></THE-HEADER>\\n  <CORE-CONTENT><REQ-IF-CONTENT>\\n");\n''',
    '''    out.push_str(&format!(\n        "  <THE-HEADER><REQ-IF-HEADER IDENTIFIER=\\\"{}\\\">",\n        escape_xml(&document.header_identifier)\n    ));\n    if let Some(time) = &document.creation_time {\n        out.push_str(&format!("<CREATION-TIME>{}</CREATION-TIME>", escape_xml(time)));\n    }\n    out.push_str("<REQ-IF-TOOL-ID>Systems-Modeler-Pro</REQ-IF-TOOL-ID>");\n    out.push_str("<REQ-IF-VERSION>1.0</REQ-IF-VERSION>");\n    out.push_str("<SOURCE-TOOL-ID>Systems-Modeler-Pro</SOURCE-TOOL-ID>");\n    out.push_str(&format!("<TITLE>{}</TITLE>", escape_xml(&document.title)));\n    out.push_str("</REQ-IF-HEADER></THE-HEADER>\\n  <CORE-CONTENT><REQ-IF-CONTENT>\\n");\n''',
    "ReqIF standards-shaped header serialization",
)
text = text.replace(
    '    } else if name.contains("requirement") || name.contains("requirement") || name.contains("spec object") {',
    '    } else if name.contains("requirement") || name.contains("spec object") {',
)
path.write_text(text, encoding="utf-8")


# Store the import configuration together with the normalized source document.
path = Path("apps/desktop/src-tauri/src/workspace/reqif_runtime.rs")
text = path.read_text(encoding="utf-8")
text = replace_once(
    text,
    '''            ReqifSourceState {\n                document,\n                element_bindings,\n                relationship_bindings,\n            },\n''',
    '''            ReqifSourceState {\n                document,\n                configuration: Some(configuration.clone()),\n                element_bindings,\n                relationship_bindings,\n            },\n''',
    "ReqIF source configuration binding",
)
path.write_text(text, encoding="utf-8")


# Make ReqIF exchange metadata part of ordinary workspace lifecycle/persistence.
path = Path("apps/desktop/src-tauri/src/workspace.rs")
text = path.read_text(encoding="utf-8")
text = replace_once(
    text,
    '''    behavior_diagrams: Mutex<Vec<behavior_workspace::BehaviorDiagram>>,\n    current_file: Mutex<Option<String>>,\n}\n''',
    '''    behavior_diagrams: Mutex<Vec<behavior_workspace::BehaviorDiagram>>,\n    current_file: Mutex<Option<String>>,\n    reqif_exchange: Mutex<reqif_interchange::ReqifExchangeState>,\n}\n''',
    "Workspace ReqIF state field",
)
text = replace_once(
    text,
    '''            behavior_diagrams: Mutex::new(Vec::new()),\n            current_file: Mutex::new(None),\n        }\n''',
    '''            behavior_diagrams: Mutex::new(Vec::new()),\n            current_file: Mutex::new(None),\n            reqif_exchange: Mutex::new(reqif_interchange::ReqifExchangeState::default()),\n        }\n''',
    "Workspace ReqIF default",
)
text = replace_once(
    text,
    '''    state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")?.clear();\n    *state.current_file.lock().map_err(|_| "project path lock poisoned")? = None;\n''',
    '''    state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")?.clear();\n    *state.reqif_exchange.lock().map_err(|_| "ReqIF exchange lock poisoned")? = reqif_interchange::ReqifExchangeState::default();\n    *state.current_file.lock().map_err(|_| "project path lock poisoned")? = None;\n''',
    "New project ReqIF reset",
)
text = replace_once(
    text,
    '''    behavior_workspace::save_behavior_metadata(&mut database, project, &behavior, &behavior_diagrams)?;\n    let saved_path = path.to_string_lossy().into_owned();\n''',
    '''    behavior_workspace::save_behavior_metadata(&mut database, project, &behavior, &behavior_diagrams)?;\n    let reqif_exchange = state.reqif_exchange.lock().map_err(|_| "ReqIF exchange lock poisoned")?;\n    reqif_runtime::save_reqif_metadata(&mut database, project, &reqif_exchange)?;\n    let saved_path = path.to_string_lossy().into_owned();\n''',
    "Save ReqIF metadata",
)
text = replace_once(
    text,
    '''    let (behavior, behavior_diagrams) = behavior_workspace::load_behavior_metadata(&database, &project)?;\n    let opened_path = path.to_string_lossy().into_owned();\n''',
    '''    let (behavior, behavior_diagrams) = behavior_workspace::load_behavior_metadata(&database, &project)?;\n    let reqif_exchange = reqif_runtime::load_reqif_metadata(&database, &project)?;\n    let opened_path = path.to_string_lossy().into_owned();\n''',
    "Load ReqIF metadata",
)
text = replace_once(
    text,
    '''    *state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")? = behavior_diagrams;\n    *state.current_file.lock().map_err(|_| "project path lock poisoned")? = Some(opened_path.clone());\n''',
    '''    *state.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")? = behavior_diagrams;\n    *state.reqif_exchange.lock().map_err(|_| "ReqIF exchange lock poisoned")? = reqif_exchange;\n    *state.current_file.lock().map_err(|_| "project path lock poisoned")? = Some(opened_path.clone());\n''',
    "Commit loaded ReqIF metadata",
)
path.write_text(text, encoding="utf-8")


# Existing model-script candidates must preserve the new exchange metadata too.
path = Path("apps/desktop/src-tauri/src/workspace/model_script.rs")
text = path.read_text(encoding="utf-8")
text = replace_once(
    text,
    '''    let current_file = workspace\n        .current_file\n        .lock()\n        .map_err(|_| "project path lock poisoned")?\n        .clone();\n    let activity_repository = activity\n''',
    '''    let current_file = workspace\n        .current_file\n        .lock()\n        .map_err(|_| "project path lock poisoned")?\n        .clone();\n    let reqif_exchange = workspace\n        .reqif_exchange\n        .lock()\n        .map_err(|_| "ReqIF exchange lock poisoned")?\n        .clone();\n    let activity_repository = activity\n''',
    "Model-script clone ReqIF metadata",
)
text = replace_once(
    text,
    '''            behavior_diagrams: std::sync::Mutex::new(behavior_diagrams),\n            current_file: std::sync::Mutex::new(current_file),\n        },\n''',
    '''            behavior_diagrams: std::sync::Mutex::new(behavior_diagrams),\n            current_file: std::sync::Mutex::new(current_file),\n            reqif_exchange: std::sync::Mutex::new(reqif_exchange),\n        },\n''',
    "Model-script candidate ReqIF field",
)
path.write_text(text, encoding="utf-8")


# Register ReqIF modules and commands without disturbing existing command surfaces.
path = Path("apps/desktop/src-tauri/src/main.rs")
text = path.read_text(encoding="utf-8")
text = replace_once(
    text,
    '''    mod repository_editing;\n    mod requirements;\n    mod routing;\n''',
    '''    mod repository_editing;\n    mod reqif_interchange;\n    mod reqif_runtime;\n    mod requirements;\n    mod routing;\n''',
    "ReqIF module registration",
)
text = replace_once(
    text,
    '''    pub use repository_editing::{\n        delete_model_element, delete_repository_diagram, move_repository_diagram,\n        move_repository_element,\n    };\n    pub use requirements::{\n''',
    '''    pub use repository_editing::{\n        delete_model_element, delete_repository_diagram, move_repository_diagram,\n        move_repository_element,\n    };\n    pub use reqif_runtime::{apply_reqif_import, export_reqif, preview_reqif_import};\n    pub use requirements::{\n''',
    "ReqIF command exports",
)
text = replace_once(
    text,
    '''    add_submachine_state, apply_model_script, apply_spreadsheet_import,\n    apply_spreadsheet_workbook_import, assign_activity_node_partition,\n''',
    '''    add_submachine_state, apply_model_script, apply_reqif_import, apply_spreadsheet_import,\n    apply_spreadsheet_workbook_import, assign_activity_node_partition,\n''',
    "ReqIF apply command import",
)
text = replace_once(
    text,
    '''    evaluate_parametric_diagram, export_portable_project_json, export_spreadsheet_workbook,\n''',
    '''    evaluate_parametric_diagram, export_portable_project_json, export_reqif, export_spreadsheet_workbook,\n''',
    "ReqIF export command import",
)
text = replace_once(
    text,
    '''    preview_activity_execution_runtime, preview_model_script, preview_sequence_execution_runtime,\n    preview_spreadsheet_import, preview_spreadsheet_workbook_import,\n''',
    '''    preview_activity_execution_runtime, preview_model_script, preview_reqif_import, preview_sequence_execution_runtime,\n    preview_spreadsheet_import, preview_spreadsheet_workbook_import,\n''',
    "ReqIF preview command import",
)
text = replace_once(
    text,
    '''            export_portable_project_json,\n            import_portable_project_json,\n            preview_model_script,\n''',
    '''            export_portable_project_json,\n            import_portable_project_json,\n            preview_reqif_import,\n            apply_reqif_import,\n            export_reqif,\n            preview_model_script,\n''',
    "ReqIF Tauri handler registration",
)
path.write_text(text, encoding="utf-8")
