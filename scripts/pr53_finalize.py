from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if new in text:
        return text
    if old not in text:
        raise SystemExit(f"{label}: source fragment not found")
    return text.replace(old, new, 1)


# Complete the native desktop upload surface and add permanent qualification tests.
path = Path("apps/desktop/src-tauri/src/workspace/reqif_runtime.rs")
text = path.read_text(encoding="utf-8")
if "pub fn stage_reqif_upload(" not in text:
    marker = "\nfn generated_attribute(\n"
    if marker not in text:
        raise SystemExit("ReqIF staging insertion point not found")
    staging = r'''
#[tauri::command]
pub fn stage_reqif_upload(file_name: String, bytes: Vec<u8>) -> Result<String, String> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_REQIF_BYTES {
        return Err("ReqIF upload must be between 1 byte and 64 MiB".into());
    }
    let extension = Path::new(&file_name)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or("ReqIF file extension is required")?;
    if !matches!(extension.as_str(), "reqif" | "reqifz") {
        return Err("only .reqif and .reqifz uploads are supported".into());
    }
    let path = std::env::temp_dir().join(format!(
        "systems-modeler-reqif-{}.{}",
        uuid::Uuid::new_v4(),
        extension
    ));
    fs::write(&path, bytes).map_err(|error| error.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn discard_staged_reqif(path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("invalid staged ReqIF path")?;
    if path.parent() != Some(std::env::temp_dir().as_path())
        || !file_name.starts_with("systems-modeler-reqif-")
    {
        return Err("only staged ReqIF uploads can be discarded".into());
    }
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}
'''
    text = text.replace(marker, "\n" + staging + marker, 1)

if "mod pr52_qualification" not in text:
    text += r'''

#[cfg(test)]
mod pr52_qualification {
    use super::*;

    fn fixture() -> &'static str {
        include_str!("../../../../../examples/reqif/external-representative.reqif")
    }

    fn states() -> (WorkspaceState, ActivityWorkspaceState) {
        let workspace = WorkspaceState::default();
        *workspace.project.lock().unwrap() = Some(Project::new("ReqIF Qualification"));
        (workspace, ActivityWorkspaceState::default())
    }

    fn configuration(project: &Project, policy: ReqifSynchronizationPolicy) -> ReqifImportConfiguration {
        ReqifImportConfiguration {
            source_namespace: "reqif:qualification-source".into(),
            target_scope: project.root_id.to_string(),
            synchronization: policy,
            object_type_mappings: BTreeMap::new(),
            relation_type_mappings: BTreeMap::new(),
            attribute_mappings: BTreeMap::new(),
        }
    }

    fn prune_hierarchy(nodes: &mut Vec<ReqifHierarchyNode>, identifier: &str) {
        for node in nodes.iter_mut() {
            prune_hierarchy(&mut node.children, identifier);
        }
        nodes.retain(|node| node.object_identifier != identifier);
    }

    #[test]
    fn authoritative_sync_removes_only_missing_bound_reqif_records() {
        let (workspace, activity) = states();
        let project = workspace.project.lock().unwrap().clone().unwrap();
        let additive = configuration(&project, ReqifSynchronizationPolicy::Additive);
        let applied = apply_reqif_xml(fixture(), None, additive, &workspace, &activity, None);
        assert!(applied.applied, "{:#?}", applied.diagnostics);

        let manual_id = {
            let mut guard = workspace.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            let root = project.root_id;
            let id = project
                .create_element(ElementKind::Requirement, "Manual Requirement", root)
                .unwrap();
            project.set_external_id(id, "MANUAL-REQIF-UNBOUND").unwrap();
            let element = project.element_mut(id).unwrap();
            element.requirement_id = Some("MAN-001".into());
            element.requirement_text = Some("Manual content must survive supplier synchronization.".into());
            id
        };

        let mut document = parse_reqif(fixture(), Some("external.reqif")).unwrap();
        document.spec_objects.retain(|object| object.identifier != "REQ-3");
        for specification in &mut document.specifications {
            prune_hierarchy(&mut specification.children, "REQ-3");
        }
        let reduced = serialize_reqif(&document);
        let current = workspace.project.lock().unwrap().clone().unwrap();
        let authoritative = configuration(
            &current,
            ReqifSynchronizationPolicy::AuthoritativeReqifScope,
        );
        let preview = preview_reqif_xml(
            &reduced,
            Some("external.reqif"),
            authoritative.clone(),
            &workspace,
            &activity,
        );
        assert!(preview.is_valid(), "{:#?}", preview.diagnostics);
        assert!(preview.items.iter().any(|item| {
            item.action == ReqifAction::Remove && item.identifier == "REQ-3"
        }));
        assert!(!preview.items.iter().any(|item| {
            item.action == ReqifAction::Remove && item.identifier == "MANUAL-REQIF-UNBOUND"
        }));

        let applied = apply_reqif_xml(
            &reduced,
            Some("external.reqif"),
            authoritative,
            &workspace,
            &activity,
            None,
        );
        assert!(applied.applied, "{:#?}", applied.diagnostics);
        let project = workspace.project.lock().unwrap();
        let project = project.as_ref().unwrap();
        assert!(project.elements.contains_key(&manual_id));
        assert!(!project.elements.values().any(|element| {
            element.external_id == external_key("reqif:qualification-source", "REQ-3")
        }));
    }

    #[test]
    fn reqif_exchange_metadata_round_trips_through_smproj() {
        let (workspace, activity) = states();
        let project = workspace.project.lock().unwrap().clone().unwrap();
        let config = configuration(&project, ReqifSynchronizationPolicy::Additive);
        let applied = apply_reqif_xml(fixture(), None, config, &workspace, &activity, None);
        assert!(applied.applied, "{:#?}", applied.diagnostics);

        let project = workspace.project.lock().unwrap().clone().unwrap();
        let exchange = workspace.reqif_exchange.lock().unwrap().clone();
        let path = std::env::temp_dir().join(format!("pr52-{}.smproj", uuid::Uuid::new_v4()));
        {
            let mut database = ProjectDatabase::open(&path).unwrap();
            database.save_project(&project).unwrap();
            save_reqif_metadata(&mut database, &project, &exchange).unwrap();
        }
        {
            let database = ProjectDatabase::open(&path).unwrap();
            let reopened = database.load_first_project().unwrap();
            let loaded = load_reqif_metadata(&database, &reopened).unwrap();
            let source = loaded.sources.get("reqif:qualification-source").unwrap();
            assert_eq!(
                source.configuration.as_ref().unwrap().source_namespace,
                "reqif:qualification-source"
            );
            let requirement = source
                .document
                .spec_objects
                .iter()
                .find(|object| object.identifier == "REQ-1")
                .unwrap();
            assert!(requirement.values.iter().any(|value| {
                value.definition_identifier == "A-CUSTOM"
                    && matches!(&value.value, ReqifValue::String(text) if text == "CUSTOM-PRESERVE-1")
            }));
            assert!(requirement.values.iter().any(|value| {
                value.definition_identifier == "A-REQ-TEXT"
                    && matches!(&value.value, ReqifValue::Xhtml { original_xml, .. } if original_xml.contains("xhtml:b"))
            }));
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn staged_reqif_and_reqifz_uploads_use_the_same_parser_and_are_discardable() {
        let staged = stage_reqif_upload("supplier.reqif".into(), fixture().as_bytes().to_vec()).unwrap();
        let payload = read_reqif_file(Path::new(&staged)).unwrap();
        assert_eq!(payload.xml, fixture());
        discard_staged_reqif(staged.clone()).unwrap();
        assert!(!Path::new(&staged).exists());

        let archive = std::env::temp_dir().join(format!("pr52-source-{}.reqifz", uuid::Uuid::new_v4()));
        write_reqif_file(&archive, fixture()).unwrap();
        let bytes = fs::read(&archive).unwrap();
        let staged_zip = stage_reqif_upload("supplier.reqifz".into(), bytes).unwrap();
        let payload = read_reqif_file(Path::new(&staged_zip)).unwrap();
        assert!(payload.xml.contains("Independent Vehicle Requirements"));
        discard_staged_reqif(staged_zip.clone()).unwrap();
        assert!(!Path::new(&staged_zip).exists());
        let _ = fs::remove_file(archive);
    }

    #[test]
    fn export_preserves_unmapped_supplier_values_and_xhtml_when_native_text_is_unchanged() {
        let (workspace, activity) = states();
        let project = workspace.project.lock().unwrap().clone().unwrap();
        let config = configuration(&project, ReqifSynchronizationPolicy::Additive);
        let applied = apply_reqif_xml(fixture(), None, config, &workspace, &activity, None);
        assert!(applied.applied, "{:#?}", applied.diagnostics);
        let scope = workspace.project.lock().unwrap().as_ref().unwrap().root_id;
        let exported = export_reqif_xml(scope, &workspace).unwrap();
        assert!(exported.contains("CUSTOM-PRESERVE-1"));
        assert!(exported.contains("xhtml:b"));
        assert!(exported.contains("Supplier Priority"));
    }
}
'''
path.write_text(text, encoding="utf-8")


# Register upload staging commands in the existing Tauri command surface.
path = Path("apps/desktop/src-tauri/src/main.rs")
text = path.read_text(encoding="utf-8")
text = replace_once(
    text,
    "    pub use reqif_runtime::{apply_reqif_import, export_reqif, preview_reqif_import};\n",
    "    pub use reqif_runtime::{\n        apply_reqif_import, discard_staged_reqif, export_reqif, preview_reqif_import,\n        stage_reqif_upload,\n    };\n",
    "ReqIF command exports",
)
text = replace_once(
    text,
    "    diagram_family_registry, discard_staged_spreadsheet, duplicate_selection,\n",
    "    diagram_family_registry, discard_staged_reqif, discard_staged_spreadsheet, duplicate_selection,\n",
    "ReqIF discard command import",
)
text = replace_once(
    text,
    "    set_workspace_interaction, stage_spreadsheet_upload, state_machine_execution_runtime_selection,\n",
    "    set_workspace_interaction, stage_reqif_upload, stage_spreadsheet_upload,\n    state_machine_execution_runtime_selection,\n",
    "ReqIF staging command import",
)
text = replace_once(
    text,
    "            export_reqif,\n            preview_model_script,\n",
    "            export_reqif,\n            stage_reqif_upload,\n            discard_staged_reqif,\n            preview_model_script,\n",
    "ReqIF Tauri handler staging registration",
)
path.write_text(text, encoding="utf-8")


# Thin frontend adapter. All parse, mapping, validation, synchronization, and mutation remain Rust-owned.
reqif_ui = r'''(() => {
  const invoke = window.__TAURI__?.core?.invoke;
  const ribbon = document.querySelector('.ribbon');

  function notify(message, type = 'info') {
    if (window.smpDialogs?.notify) window.smpDialogs.notify(message, type);
    else if (typeof renderStatus === 'function') renderStatus(message);
  }

  function chooseFile() {
    return new Promise((resolve) => {
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = '.reqif,.reqifz';
      input.onchange = () => resolve(input.files?.[0] || null);
      input.click();
    });
  }

  function projectState() {
    return typeof state !== 'undefined' ? state.snapshot?.project : null;
  }

  function targetScopeId() {
    const project = projectState();
    if (!project) throw new Error('Create or open a project before ReqIF import/export.');
    const selectedId = typeof state !== 'undefined'
      ? (state.selectedElementId || state.selectedPackageId)
      : null;
    const selected = project.elements?.find((element) => element.id === selectedId);
    return selected && ['Model', 'Package'].includes(selected.kind) ? selected.id : project.root_id;
  }

  function summarize(preview) {
    const totals = preview?.totals || {};
    return `CREATE ${totals.create || 0} · UPDATE ${totals.update || 0} · NO_CHANGE ${totals.no_change || 0} · REMOVE ${totals.remove || 0} · BLOCKED ${totals.blocked || 0}`;
  }

  function previewText(preview) {
    const rows = (preview?.items || []).slice(0, 250).map((item) =>
      `${item.action}: ${item.identifier} [${item.kind}]${item.detail ? ` — ${item.detail}` : ''}`
    );
    const diagnostics = (preview?.diagnostics || []).map((item) =>
      `${item.severity} ${item.code}${item.identifier ? ` ${item.identifier}` : ''}: ${item.reason}`
    );
    return [summarize(preview), ...rows, ...diagnostics.map((item) => `DIAGNOSTIC: ${item}`)].join('\n');
  }

  function validPreview(preview) {
    return (preview?.totals?.blocked || 0) === 0
      && !(preview?.diagnostics || []).some((item) => item.severity === 'ERROR');
  }

  async function inspectPreview(preview, confirmLabel) {
    return window.smpDialogs?.edit({
      title: 'ReqIF import preview',
      description: summarize(preview),
      fields: [{ id: 'preview', label: 'Actions and diagnostics', value: previewText(preview), multiline: true, readonly: true }],
      confirmLabel,
    });
  }

  function defaultNamespace(fileName) {
    const stem = fileName.replace(/\.(reqifz?|REQIFZ?)$/i, '').replace(/[^A-Za-z0-9._-]+/g, '-');
    return `reqif:${stem || 'supplier'}`;
  }

  async function configureImport(file) {
    const mapping = await window.smpDialogs?.edit({
      title: 'Configure ReqIF import',
      description: 'Source namespace is durable reimport identity. Keep the same value for later versions of this supplier exchange. Empty mapping objects use standards/name detection; add explicit type or attribute mappings when needed.',
      fields: [
        { id: 'sourceNamespace', label: 'Stable source namespace', value: defaultNamespace(file.name), required: true },
        { id: 'mappingJson', label: 'Mapping overrides (JSON)', value: '{\n  "object_type_mappings": {},\n  "relation_type_mappings": {},\n  "attribute_mappings": {}\n}', multiline: true },
      ],
      confirmLabel: 'Continue',
    });
    if (!mapping) return null;
    const policy = await window.smpDialogs?.choose({
      title: 'ReqIF synchronization policy',
      description: 'Additive mode never removes native content. Authoritative ReqIF scope removes only records proven to belong to this source namespace and missing from the new exchange.',
      candidates: [
        { id: 'additive', label: 'Additive / update-only' },
        { id: 'authoritative-reqif-scope', label: 'Authoritative ReqIF scope' },
      ],
      confirmLabel: 'Preview',
    });
    if (!policy) return null;
    const overrides = JSON.parse(mapping.values.mappingJson || '{}');
    return {
      source_namespace: mapping.values.sourceNamespace.trim(),
      target_scope: targetScopeId(),
      synchronization: policy.selectedId || 'additive',
      object_type_mappings: overrides.object_type_mappings || {},
      relation_type_mappings: overrides.relation_type_mappings || {},
      attribute_mappings: overrides.attribute_mappings || {},
    };
  }

  async function openImport() {
    let stagedPath = null;
    try {
      if (!invoke) throw new Error('ReqIF import is available in the desktop application.');
      const file = await chooseFile();
      if (!file) return;
      const configuration = await configureImport(file);
      if (!configuration) return;
      const bytes = [...new Uint8Array(await file.arrayBuffer())];
      stagedPath = await invoke('stage_reqif_upload', { fileName: file.name, bytes });
      const preview = await invoke('preview_reqif_import', { path: stagedPath, configuration });
      const inspected = await inspectPreview(preview, validPreview(preview) ? 'Apply' : 'Close');
      if (!inspected || !validPreview(preview)) return;
      const applied = await invoke('apply_reqif_import', { path: stagedPath, configuration });
      if (!applied.applied) throw new Error(previewText(applied));
      if (typeof refresh === 'function') await refresh();
      notify(`ReqIF applied atomically. ${summarize(applied)}`);
    } catch (error) {
      notify(String(error), 'error');
    } finally {
      if (stagedPath && invoke) {
        try { await invoke('discard_staged_reqif', { path: stagedPath }); } catch (_) { /* cleanup only */ }
      }
    }
  }

  async function openExport() {
    try {
      if (!invoke) throw new Error('ReqIF export is available in the desktop application.');
      const project = projectState();
      if (!project) throw new Error('Create or open a project before ReqIF export.');
      const result = await window.smpDialogs?.edit({
        title: 'Export ReqIF',
        description: 'Exports Requirement/TestCase content and supported traceability in the selected Model/Package scope. Use .reqifz for a compressed exchange container.',
        fields: [
          { id: 'path', label: 'Destination path (.reqif or .reqifz)', value: `${project.name || 'requirements'}.reqif`, required: true },
        ],
        confirmLabel: 'Export',
      });
      if (!result) return;
      const output = await invoke('export_reqif', { path: result.values.path, scopeId: targetScopeId() });
      notify(`ReqIF exported to ${output}.`);
    } catch (error) {
      notify(String(error), 'error');
    }
  }

  function installRibbonGroup() {
    if (!ribbon || ribbon.querySelector('[data-reqif-interchange]')) return;
    const fileActive = [...document.querySelectorAll('.workspace-tab')]
      .some((tab) => tab.classList.contains('active') && tab.textContent.trim() === 'File');
    if (!fileActive) return;
    const group = document.createElement('section');
    group.className = 'ribbon-group';
    group.dataset.reqifInterchange = 'true';
    group.innerHTML = '<div class="ribbon-actions ribbon-large-actions"><button class="ribbon-command" data-reqif-import><span class="command-icon">⇩</span><span>Import<br>ReqIF</span></button><button class="ribbon-command" data-reqif-export><span class="command-icon">⇧</span><span>Export<br>ReqIF</span></button></div><div class="ribbon-label">Requirements Interchange</div>';
    group.querySelector('[data-reqif-import]').addEventListener('click', openImport);
    group.querySelector('[data-reqif-export]').addEventListener('click', openExport);
    ribbon.appendChild(group);
  }

  if (ribbon) new MutationObserver(installRibbonGroup).observe(ribbon, { childList: true });
  document.querySelectorAll('.workspace-tab').forEach((tab) => {
    tab.addEventListener('click', () => queueMicrotask(installRibbonGroup));
  });
  installRibbonGroup();
  window.smpReqifInterchange = Object.freeze({ openImport, openExport });
})();
'''
Path("apps/desktop/frontend/reqif-ui.js").write_text(reqif_ui, encoding="utf-8")

path = Path("apps/desktop/frontend/index.html")
text = path.read_text(encoding="utf-8")
text = replace_once(
    text,
    '  <script src="model-script-ui.js"></script>\n',
    '  <script src="model-script-ui.js"></script>\n  <script src="reqif-ui.js"></script>\n',
    "ReqIF frontend registration",
)
path.write_text(text, encoding="utf-8")


# Permanent integration contract so future changes cannot silently disconnect ReqIF.
validator = r'''from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def text(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def require(path: str, needles: list[str]) -> None:
    source = text(path)
    missing = [needle for needle in needles if needle not in source]
    if missing:
        raise SystemExit(f"{path} missing ReqIF contract markers: {missing}")


require(
    "apps/desktop/src-tauri/src/workspace/reqif_interchange.rs",
    [
        "ReqifImportConfiguration",
        "ReqifSynchronizationPolicy",
        "AuthoritativeReqifScope",
        "ReqifAction",
        "ReqifValue::Xhtml",
        "validate_references",
        "serialize_reqif",
    ],
)
require(
    "apps/desktop/src-tauri/src/workspace/reqif_runtime.rs",
    [
        "ModelBuildPlan",
        "apply_unified_model_build",
        "preview_reqif_import",
        "apply_reqif_import",
        "export_reqif",
        "stage_reqif_upload",
        "discard_staged_reqif",
        "portable_from_states",
        "save_reqif_metadata",
        "load_reqif_metadata",
        "AuthoritativeReqifScope",
    ],
)
require(
    "apps/desktop/src-tauri/src/workspace.rs",
    ["reqif_exchange", "save_reqif_metadata", "load_reqif_metadata"],
)
require(
    "apps/desktop/src-tauri/src/workspace/model_script.rs",
    ["reqif_exchange", "ReqIF exchange lock poisoned"],
)
require(
    "apps/desktop/src-tauri/src/main.rs",
    [
        "preview_reqif_import",
        "apply_reqif_import",
        "export_reqif",
        "stage_reqif_upload",
        "discard_staged_reqif",
    ],
)
require(
    "apps/desktop/frontend/reqif-ui.js",
    [
        ".reqif,.reqifz",
        "source_namespace",
        "target_scope",
        "preview_reqif_import",
        "apply_reqif_import",
        "export_reqif",
        "stage_reqif_upload",
        "discard_staged_reqif",
    ],
)
require("apps/desktop/frontend/index.html", ['<script src="reqif-ui.js"></script>'])

rust_reqif = text("apps/desktop/src-tauri/src/workspace/reqif_runtime.rs") + text(
    "apps/desktop/src-tauri/src/workspace/reqif_interchange.rs"
)
for forbidden in ["mod xmi", "XmiImporter", "ProfileRepository"]:
    if forbidden in rust_reqif:
        raise SystemExit(f"PR52 scope violation detected in ReqIF Rust: {forbidden}")

print("ReqIF integration contract passed")
'''
Path("scripts/validate_reqif_interchange.py").write_text(validator, encoding="utf-8")


# Make the permanent CI aware of the new thin UI and integration contract.
path = Path(".github/workflows/ci.yml")
text = path.read_text(encoding="utf-8")
text = replace_once(
    text,
    "            apps/desktop/frontend/ui-shell.js \\\n",
    "            apps/desktop/frontend/ui-shell.js \\\n            apps/desktop/frontend/reqif-ui.js \\\n",
    "ReqIF frontend syntax CI",
)
spreadsheet_contract = "      - name: Spreadsheet interchange integration contract\n        run: python scripts/validate_spreadsheet_interchange.py\n"
reqif_contract = spreadsheet_contract + "      - name: ReqIF interchange integration contract\n        run: python scripts/validate_reqif_interchange.py\n"
if text.count(spreadsheet_contract) != 2:
    raise SystemExit("expected two spreadsheet integration CI blocks")
text = text.replace(spreadsheet_contract, reqif_contract)
path.write_text(text, encoding="utf-8")


# Correct the public format status now that PR51 is merged and PR52 is being qualified.
path = Path("README.md")
text = path.read_text(encoding="utf-8")
text = text.replace(
    "Bulk and interchange adapters converge on the Rust-owned `ModelBuildPlan` construction path. The current spreadsheet importer maps business-facing CSV/XLSX data into semantic operations, resolves stable and plan-local references, validates a complete native candidate, provides non-mutating preview classifications, and applies a valid MapGroup atomically. Stable source namespace + External ID identity is the preferred reimport identity; display names and spreadsheet positions are not permanent identity.",
    "Bulk, scripting, and interchange adapters converge on the Rust-owned `ModelBuildPlan` construction path. CSV/XLSX mapping, bounded Groovy-compatible model scripting, and ReqIF all construct ordinary native model content through the existing semantic authority. ReqIF uses stable source namespace + ReqIF `IDENTIFIER` identity, non-mutating CREATE/UPDATE/NO_CHANGE/REMOVE/BLOCKED preview, atomic apply, preserved exchange metadata/XHTML, and deterministic `.reqif`/`.reqifz` export.",
)
text = text.replace(
    "| Groovy / model script | PLANNED / NOT YET IMPLEMENTED |\n| ReqIF | PLANNED / NOT YET IMPLEMENTED |",
    "| Groovy / model script | QUALIFIED bounded native model construction / automation |\n| ReqIF | QUALIFIED Requirement/TestCase + supported traceability import, stable reimport, `.reqif`/`.reqifz` export |",
)
path.write_text(text, encoding="utf-8")

path = Path("docs/IMPORT_RULES_AND_QUALIFICATION.txt")
text = path.read_text(encoding="utf-8")
text = text.replace(
    "Applies to: main at c0bcd0af825476f613374bd51d05302f8a1d6487",
    "Applies to: logical PR52 ReqIF milestone based on main 7764fdb78329dd893319330a038275bdaa63de63",
)
text = text.replace(
    "- how future adapters such as Groovy/model scripting, XMI, ReqIF, and SysML v2\n  interchange must join the existing Rust construction architecture.",
    "- how current Groovy/model scripting and ReqIF adapters join the shared Rust\n  construction architecture, and how future XMI/SysML v2 adapters must do the same.",
)
text = text.replace(
    "Groovy / model-script import               PLANNED / NOT YET IMPLEMENTED\nReqIF                                      PLANNED / NOT YET IMPLEMENTED",
    "Groovy / model-script import               QUALIFIED bounded native construction / automation\nReqIF                                      QUALIFIED Requirement/TestCase + traceability interchange",
)
if "REQIF QUALIFIED CONTRACT (PR52)" not in text:
    text += r'''

===============================================================================
REQIF QUALIFIED CONTRACT (PR52)
===============================================================================

ReqIF is a requirements interchange adapter, not a second model authority.

Supported exchange boundary:

- `.reqif` XML and compressed `.reqifz` containers;
- ReqIF header, datatypes, SpecTypes, SpecObjects, Specifications/hierarchy,
  SpecRelations, and typed attribute values;
- STRING, XHTML, BOOLEAN, INTEGER, REAL, DATE, and ENUMERATION values;
- configurable SpecObject -> native Requirement/TestCase mapping;
- configurable SpecRelation -> supported native traceability relationship mapping;
- configurable attribute -> Requirement ID, Requirement Text, Name, and
  Documentation mapping;
- unmapped ReqIF attributes and original XHTML retained in ReqIF exchange
  metadata so mapped native content does not silently destroy supplier data;
- stable semantic identity is `(source namespace, ReqIF IDENTIFIER)`;
- identical reimport resolves to NO_CHANGE rather than duplication;
- additive synchronization never removes native content;
- authoritative ReqIF synchronization may remove only records proven to belong
  to that same ReqIF source namespace and absent from the new exchange;
- preview reports CREATE / UPDATE / NO_CHANGE / REMOVE / BLOCKED before apply;
- parse, reference, ownership, mapping, and semantic errors block the complete
  candidate; valid application is atomic;
- ReqIF source configuration and exchange fidelity metadata survive `.smproj`;
- deterministic `.reqif` export and `.reqifz` packaging are supported for a
  selected Model/Package scope;
- imported Requirements/TestCases are ordinary native model elements and
  imported traceability is ordinary native relationships, so existing
  requirements diagrams, editing, persistence, spreadsheet/Groovy paths, and
  execution architecture remain authoritative.

ReqIF does not imply XMI, SysML v2, CATIA native-file compatibility, or a
profile/stereotype framework. Those remain separate future scopes.
'''
path.write_text(text, encoding="utf-8")
