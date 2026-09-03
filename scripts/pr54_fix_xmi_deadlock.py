from pathlib import Path

# Temporary qualification patch: remove after the repaired Rust commit is published.

runtime_path = Path("apps/desktop/src-tauri/src/workspace/xmi_runtime.rs")
runtime = runtime_path.read_text(encoding="utf-8")

old = """    if let Some(portable) = &embedded {
        let current = portable_from_states(workspace, activity).ok();
        let incoming = semantic_only(portable.clone());
"""
new = """    if let Some(portable) = &embedded {
        // `portable_from_states` snapshots `workspace.project` itself. Release the
        // validation guard first: std::sync::Mutex is non-reentrant, so retaining
        // this guard here would self-deadlock on native authored-state XMI.
        drop(project_guard);
        let current = portable_from_states(workspace, activity).ok();
        let incoming = semantic_only(portable.clone());
"""
if old in runtime:
    runtime = runtime.replace(old, new, 1)
elif new not in runtime:
    raise SystemExit("embedded XMI lock pattern not found")

old_tail = """        preview.recount();
        drop(project_guard);
        return PreparedXmiImport {
"""
new_tail = """        preview.recount();
        return PreparedXmiImport {
"""
if old_tail in runtime:
    runtime = runtime.replace(old_tail, new_tail, 1)
elif new_tail not in runtime:
    raise SystemExit("embedded XMI trailing drop pattern not found")

old_apply = """    let apply_result = if let Some(portable) = prepared.embedded.take() {
        replace_embedded_semantics(portable, &candidate, &candidate_activity)
    } else {
"""
new_apply = """    let used_embedded = prepared.embedded.is_some();
    let apply_result = if let Some(portable) = prepared.embedded.take() {
        replace_embedded_semantics(portable, &candidate, &candidate_activity)
    } else {
"""
if old_apply in runtime:
    runtime = runtime.replace(old_apply, new_apply, 1)
elif new_apply not in runtime:
    raise SystemExit("embedded apply marker not found")

old_profiles = """        if let Err(reason) = apply_external_profiles(
            project,
            &prepared.document,
            &prepared.configuration.source_namespace,
        ) {
            prepared
                .preview
                .diagnostics
                .push(runtime_diagnostic("SEMANTIC_VALIDATION", reason));
            prepared.preview.recount();
            return prepared.preview;
        }
"""
new_profiles = """        if !used_embedded
            && let Err(reason) = apply_external_profiles(
                project,
                &prepared.document,
                &prepared.configuration.source_namespace,
            )
        {
            prepared
                .preview
                .diagnostics
                .push(runtime_diagnostic("SEMANTIC_VALIDATION", reason));
            prepared.preview.recount();
            return prepared.preview;
        }
"""
if old_profiles in runtime:
    runtime = runtime.replace(old_profiles, new_profiles, 1)
elif new_profiles not in runtime:
    raise SystemExit("external profile overlay marker not found")

marker = """    #[test]
    fn authoritative_remove_is_source_bound_and_late_parse_error_is_atomic() {
"""
regression = """    #[test]
    fn embedded_xmi_preview_and_apply_complete_without_recursive_project_lock() {
        use std::{sync::mpsc, thread, time::Duration};

        let (source, source_activity) = representative_states();
        let portable = semantic_only(portable_from_states(&source, &source_activity).unwrap());
        let xml = serialize_xmi(&portable).unwrap();
        assert!(xml.contains("sm:authoredState"));

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let target = WorkspaceState::default();
            let target_activity = ActivityWorkspaceState::default();
            let target_project = Project::new("Embedded XMI Target");
            let root = target_project.root_id;
            *target.project.lock().unwrap() = Some(target_project);
            let configuration = XmiImportConfiguration {
                source_namespace: "xmi:embedded-watchdog".into(),
                target_scope: root.to_string(),
                synchronization: XmiSynchronizationPolicy::AdditiveUpdate,
            };

            let preview = preview_xmi_xml(
                &xml,
                Some("embedded-watchdog.xmi"),
                configuration.clone(),
                &target,
                &target_activity,
            );
            let applied = apply_xmi_xml(
                &xml,
                Some("embedded-watchdog.xmi"),
                configuration,
                &target,
                &target_activity,
                None,
            );
            let _ = tx.send((preview, applied));
        });

        let (preview, applied) = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("embedded XMI preview/apply exceeded 10 seconds; probable lock deadlock");
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert!(applied.applied, "{:?}", applied.diagnostics);
    }

    #[test]
    fn authoritative_remove_is_source_bound_and_late_parse_error_is_atomic() {
"""
if "fn embedded_xmi_preview_and_apply_complete_without_recursive_project_lock()" not in runtime:
    if marker not in runtime:
        raise SystemExit("test insertion marker not found")
    runtime = runtime.replace(marker, regression, 1)

old_reduced = """        let reduced = UML_FIXTURE
            .replace(
"""
new_reduced = """        // Git may materialize the included fixture with CRLF on Windows. Normalize
        // only the test input used to remove rows so the authoritative-sync assertion
        // exercises the same two semantic removals on every runner.
        let normalized_fixture = UML_FIXTURE.replace("\\r\\n", "\\n");
        let reduced = normalized_fixture
            .replace(
"""
if old_reduced in runtime:
    runtime = runtime.replace(old_reduced, new_reduced, 1)
elif "let normalized_fixture = UML_FIXTURE.replace" not in runtime:
    raise SystemExit("authoritative fixture reduction marker not found")

runtime_path.write_text(runtime, encoding="utf-8")

interchange_path = Path("apps/desktop/src-tauri/src/workspace/xmi_interchange.rs")
interchange = interchange_path.read_text(encoding="utf-8")
helper_marker = """fn local_attribute<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
    local_name: &str,
) -> Option<&'a str> {
    node.attributes()
        .find(|attribute| attribute.name() == local_name)
        .map(|attribute| attribute.value())
}

"""
helper = helper_marker + """fn semantic_type_reference<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
) -> Option<&'a str> {
    node.attributes()
        .find(|attribute| {
            attribute.name() == "type"
                && !attribute.namespace().is_some_and(is_xmi_namespace)
        })
        .map(|attribute| attribute.value())
}

"""
if "fn semantic_type_reference" not in interchange:
    if helper_marker not in interchange:
        raise SystemExit("local_attribute helper marker not found")
    interchange = interchange.replace(helper_marker, helper, 1)

old_type_ref = """                type_reference: local_attribute(node, "type").map(ToOwned::to_owned),
"""
new_type_ref = """                type_reference: semantic_type_reference(node).map(ToOwned::to_owned),
"""
if old_type_ref in interchange:
    interchange = interchange.replace(old_type_ref, new_type_ref, 1)
elif new_type_ref not in interchange:
    raise SystemExit("semantic type-reference assignment not found")

interchange_path.write_text(interchange, encoding="utf-8")
