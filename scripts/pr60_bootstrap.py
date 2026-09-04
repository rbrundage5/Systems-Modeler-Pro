from pathlib import Path

def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one replacement target, found {count}")
    p.write_text(text.replace(old, new), encoding="utf-8")

def replace_between(path: str, start_marker: str, end_marker: str, replacement: str) -> None:
    p = Path(path)
    text = p.read_text(encoding="utf-8")
    start = text.find(start_marker)
    if start < 0:
        raise SystemExit(f"{path}: start marker not found: {start_marker!r}")
    end = text.find(end_marker, start)
    if end < 0:
        raise SystemExit(f"{path}: end marker not found: {end_marker!r}")
    p.write_text(text[:start] + replacement + text[end:], encoding="utf-8")

# ---------------------------------------------------------------------------
# 1. Make Project::children deterministic application-wide.
# ---------------------------------------------------------------------------
replace_once(
    "crates/model-core/src/model.rs",
    '''    pub fn children(&self, owner_id: ElementId) -> impl Iterator<Item = &Element> {
        self.elements
            .values()
            .filter(move |element| element.owner_id == Some(owner_id))
    }
''',
    '''    pub fn children(&self, owner_id: ElementId) -> impl Iterator<Item = &Element> {
        let mut children = self
            .elements
            .values()
            .filter(move |element| element.owner_id == Some(owner_id))
            .collect::<Vec<_>>();
        children.sort_by(|left, right| {
            left.external_id
                .cmp(&right.external_id)
                .then_with(|| left.name.cmp(&right.name))
        });
        children.into_iter()
    }
''',
)

# ---------------------------------------------------------------------------
# 2. Make shared hierarchical layout preserve caller input order instead of
#    lexicographically ordering random presentation UUIDs.
# ---------------------------------------------------------------------------
layout_path = "apps/desktop/src-tauri/src/workspace/layout.rs"
replace_once(
    layout_path,
    '''    let nodes: BTreeMap<_, _> = nodes
        .into_iter()
        .map(|mut node| {
            node.width = node.width.max(1.0);
            node.height = node.height.max(1.0);
            (node.id.clone(), node)
        })
        .collect();
    let ids: BTreeSet<_> = nodes.keys().cloned().collect();
    let mut indegree: BTreeMap<_, usize> = ids.iter().map(|id| (id.clone(), 0)).collect();
    let mut outgoing: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
''',
    '''    // Presentation identifiers can legitimately be generated UUIDs. They are
    // identity, not ordering semantics. Preserve the caller-provided node order
    // and use it as the deterministic tie-breaker for equal graph levels.
    let mut nodes_by_id = BTreeMap::new();
    let mut ordered_ids = Vec::new();
    for mut node in nodes {
        node.width = node.width.max(1.0);
        node.height = node.height.max(1.0);
        if !nodes_by_id.contains_key(&node.id) {
            ordered_ids.push(node.id.clone());
        }
        nodes_by_id.insert(node.id.clone(), node);
    }
    let rank: BTreeMap<_, _> = ordered_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (id.clone(), index))
        .collect();
    let mut indegree: BTreeMap<_, usize> =
        ordered_ids.iter().map(|id| (id.clone(), 0)).collect();
    let mut outgoing: BTreeMap<String, Vec<String>> = BTreeMap::new();
''',
)

replace_between(
    layout_path,
    '''    for (source, target) in edges {
''',
    '''    // A state machine commonly contains transition cycles, so Kahn layering alone
''',
    '''    for (source, target) in edges {
        if source == target
            || !nodes_by_id.contains_key(source)
            || !nodes_by_id.contains_key(target)
        {
            continue;
        }
        let targets = outgoing.entry(source.clone()).or_default();
        if !targets.contains(target) {
            targets.push(target.clone());
            *indegree.entry(target.clone()).or_default() += 1;
        }
    }
    for targets in outgoing.values_mut() {
        targets.sort_by_key(|target| rank.get(target).copied().unwrap_or(usize::MAX));
    }

    let mut ready: BTreeSet<(usize, String)> = indegree
        .iter()
        .filter_map(|(id, degree)| {
            (*degree == 0).then_some((rank.get(id).copied().unwrap_or(usize::MAX), id.clone()))
        })
        .collect();
    let mut levels: BTreeMap<String, usize> =
        ordered_ids.iter().map(|id| (id.clone(), 0)).collect();
    let mut placed = BTreeSet::new();
    while let Some((_, id)) = ready.pop_first() {
        placed.insert(id.clone());
        let next_level = levels[&id].saturating_add(1);
        for target in outgoing.get(&id).into_iter().flatten() {
            levels
                .entry(target.clone())
                .and_modify(|level| *level = (*level).max(next_level));
            let degree = indegree.get_mut(target).expect("known layout node");
            *degree -= 1;
            if *degree == 0 {
                ready.insert((
                    rank.get(target).copied().unwrap_or(usize::MAX),
                    target.clone(),
                ));
            }
        }
    }

'''
)

replace_once(
    layout_path,
    '''    for start in &ids {
''',
    '''    for start in &ordered_ids {
''',
)

replace_once(
    layout_path,
    '''    let mut by_level: BTreeMap<usize, Vec<&LayoutNode>> = BTreeMap::new();
    for (id, level) in &levels {
        by_level.entry(*level).or_default().push(&nodes[id]);
    }
''',
    '''    let mut by_level: BTreeMap<usize, Vec<&LayoutNode>> = BTreeMap::new();
    for id in &ordered_ids {
        let level = levels[id];
        by_level.entry(level).or_default().push(&nodes_by_id[id]);
    }
''',
)

layout_text = Path(layout_path).read_text(encoding="utf-8")
layout_test = r'''
    #[test]
    fn equal_level_nodes_preserve_caller_order_instead_of_uuid_lexical_order() {
        let positions = hierarchical_positions_sized(
            [
                LayoutNode {
                    id: "z-first".into(),
                    width: 100.0,
                    height: 60.0,
                },
                LayoutNode {
                    id: "a-second".into(),
                    width: 100.0,
                    height: 60.0,
                },
                LayoutNode {
                    id: "m-third".into(),
                    width: 100.0,
                    height: 60.0,
                },
            ],
            &[],
            PreferredFlowDirection::TopToBottom,
        );
        assert!(positions["z-first"].0 < positions["a-second"].0);
        assert!(positions["a-second"].0 < positions["m-third"].0);
    }
'''
if "equal_level_nodes_preserve_caller_order_instead_of_uuid_lexical_order" not in layout_text:
    insert_at = layout_text.rfind("\n}")
    if insert_at < 0:
        raise SystemExit("layout.rs: final module brace not found")
    layout_text = layout_text[:insert_at] + layout_test + layout_text[insert_at:]
    Path(layout_path).write_text(layout_text, encoding="utf-8")

# ---------------------------------------------------------------------------
# 3. Model-script presentation construction:
#    - stable semantic ordering for relationship-backed views
#    - exactly one layout/routing pass
#    - stage-specific diagnostics
#    - apply returns the candidate it actually committed; no third rebuild.
# ---------------------------------------------------------------------------
model_script = "apps/desktop/src-tauri/src/workspace/model_script.rs"

replace_once(
    model_script,
    '''    for relationship in project
        .relationships
        .values()
        .filter(|relationship| relationship_presentable_on_family(family, &relationship.kind))
    {
''',
    '''    let mut relationships = project
        .relationships
        .values()
        .filter(|relationship| relationship_presentable_on_family(family, &relationship.kind))
        .collect::<Vec<_>>();
    relationships.sort_by(|left, right| left.external_id.cmp(&right.external_id));
    for relationship in relationships {
''',
)

replace_once(
    model_script,
    '''                for relationship in project
                    .relationships
                    .values()
                    .filter(|relationship| relationship.kind == RelationshipKind::Connector)
                {
                    let Some(connector) = relationship
                        .connector
                        .as_ref()
                        .filter(|connector| connector.context_id == context)
                    else {
                        continue;
                    };
''',
    '''                let mut relationships = project
                    .relationships
                    .values()
                    .filter(|relationship| relationship.kind == RelationshipKind::Connector)
                    .filter(|relationship| {
                        relationship
                            .connector
                            .as_ref()
                            .is_some_and(|connector| connector.context_id == context)
                    })
                    .collect::<Vec<_>>();
                relationships.sort_by(|left, right| left.external_id.cmp(&right.external_id));
                for relationship in relationships {
                    let connector = relationship
                        .connector
                        .as_ref()
                        .expect("filtered Connector semantics");
''',
)

replace_between(
    model_script,
    "fn layout_and_route(\n",
    "fn build_candidate(\n",
    r'''fn clean_layout_script_diagram(
    family: &str,
    diagram_id: &str,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
) -> Result<(), String> {
    match family {
        "bdd" | "requirement" | "use-case" | "package" => {
            super::layout_bdd_with_bounds(diagram_id, workspace, None)?;
        }
        "parametric" => {
            super::parametrics::layout_parametric_with_bounds(diagram_id, workspace, None)?;
        }
        "ibd" => {
            super::ibd::layout_ibd_with_bounds(diagram_id, workspace, None)?;
        }
        "activity" => {
            super::activity_mutation::layout_activity_with_bounds(diagram_id, activity, None)?;
        }
        "state-machine" | "sequence" => {
            super::behavior_workspace::layout_behavior_with_bounds(diagram_id, workspace, None)?;
        }
        _ => {}
    }
    Ok(())
}

fn route_script_diagram(
    family: &str,
    diagram_id: &str,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
) -> Result<(), String> {
    match family {
        "bdd" | "requirement" | "use-case" | "package" => {
            super::route_bdd_with_bounds(diagram_id, workspace, None)?;
        }
        "parametric" => {
            super::parametrics::route_parametric_with_bounds(diagram_id, workspace, None)?;
        }
        "ibd" => {
            super::ibd::route_ibd_with_bounds(diagram_id, workspace, None)?;
        }
        "activity" => {
            super::activity_mutation::route_activity_with_bounds(diagram_id, activity, None)?;
        }
        "state-machine" | "sequence" => {
            super::behavior_workspace::route_behavior_with_bounds(diagram_id, workspace, None)?;
        }
        _ => {}
    }
    Ok(())
}

'''
)

p = Path(model_script)
text = p.read_text(encoding="utf-8")
start = text.find("        layout_and_route(\n")
end_marker = "        })?;\n    }\n    super::portable_interchange::portable_from_states"
if start < 0:
    raise SystemExit("model_script.rs: layout_and_route invocation not found")
end = text.find(end_marker, start)
if end < 0:
    raise SystemExit("model_script.rs: layout_and_route invocation end not found")
end += len("        })?;\n")
replacement = r'''        // Every Clean Layout adapter already reroutes its relationships after moving
        // presentations. If Clean Layout is requested, it is the single presentation
        // pass. A separate Route pass is only needed when Clean Layout is disabled.
        // This prevents default clean_layout=true + route=true scripts from routing
        // the same dense diagram twice inside one atomic candidate build.
        if diagram.clean_layout {
            clean_layout_script_diagram(
                family,
                &id,
                &candidate_workspace,
                &candidate_activity,
            )
            .map_err(|reason| ModelScriptPreview {
                host: SCRIPT_HOST,
                applied: false,
                source_namespace: compiled.document.source_namespace.clone(),
                items: compiled.items.clone(),
                diagnostics: vec![diag(
                    script_name,
                    Some(statement),
                    Some("cleanLayout".into()),
                    Some(diagram.external_id.clone()),
                    "DIAGRAM_CLEAN_LAYOUT_FAILED",
                    reason,
                )],
            })?;
        } else if diagram.route {
            route_script_diagram(
                family,
                &id,
                &candidate_workspace,
                &candidate_activity,
            )
            .map_err(|reason| ModelScriptPreview {
                host: SCRIPT_HOST,
                applied: false,
                source_namespace: compiled.document.source_namespace.clone(),
                items: compiled.items.clone(),
                diagnostics: vec![diag(
                    script_name,
                    Some(statement),
                    Some("route".into()),
                    Some(diagram.external_id.clone()),
                    "DIAGRAM_ROUTE_FAILED",
                    reason,
                )],
            })?;
        }
'''
text = text[:start] + replacement + text[end:]
p.write_text(text, encoding="utf-8")

replace_between(
    model_script,
    "fn preview_impl(\n",
    "#[tauri::command]\npub fn preview_model_script",
    r'''fn successful_preview(compiled: &CompiledScript, applied: bool) -> ModelScriptPreview {
    let mut items = compiled.items.clone();
    for (offset, diagram) in compiled.document.diagrams.iter().enumerate() {
        items.push(item(
            compiled.document.operations.len() + offset + 1,
            ModelScriptAction::Create,
            format!("Diagram::{}", diagram.family),
            Some(&diagram.external_id),
            Some(&diagram.name),
            "native deterministic presentation + populate/layout/routing",
        ));
    }
    ModelScriptPreview {
        host: SCRIPT_HOST,
        applied,
        source_namespace: compiled.document.source_namespace.clone(),
        items,
        diagnostics: Vec::new(),
    }
}

fn preview_impl(
    script_name: &str,
    source: &str,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
) -> ModelScriptPreview {
    match build_candidate(script_name, source, workspace, activity) {
        Ok((compiled, _, _)) => successful_preview(&compiled, false),
        Err(preview) => preview,
    }
}

'''
)

replace_between(
    model_script,
    "#[tauri::command]\npub fn apply_model_script",
    "#[cfg(test)]\nmod tests",
    r'''fn apply_impl(
    script_name: &str,
    source: &str,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
    history: &super::history::HistoryState,
) -> ModelScriptPreview {
    let (compiled, candidate_workspace, candidate_activity) =
        match build_candidate(script_name, source, workspace, activity) {
            Ok(value) => value,
            Err(preview) => return preview,
        };
    if let Err(reason) = super::history::checkpoint_states(workspace, activity, history) {
        return ModelScriptPreview {
            host: SCRIPT_HOST,
            applied: false,
            source_namespace: compiled.document.source_namespace.clone(),
            items: compiled.items.clone(),
            diagnostics: vec![diag(
                script_name,
                None,
                Some("history".into()),
                None,
                "HISTORY_CHECKPOINT_FAILED",
                reason,
            )],
        };
    }
    if let Err(reason) = commit_candidate(
        workspace,
        activity,
        &candidate_workspace,
        &candidate_activity,
    ) {
        return ModelScriptPreview {
            host: SCRIPT_HOST,
            applied: false,
            source_namespace: compiled.document.source_namespace.clone(),
            items: compiled.items.clone(),
            diagnostics: vec![diag(
                script_name,
                None,
                Some("commit".into()),
                None,
                "ATOMIC_COMMIT_FAILED",
                reason,
            )],
        };
    }

    // Return the exact candidate build that was successfully committed. Do not
    // call preview_impl again: that would construct an unrelated third candidate
    // after commit and could incorrectly report failure for already-applied state.
    successful_preview(&compiled, true)
}

#[tauri::command]
pub fn apply_model_script(
    script_name: String,
    source: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, super::history::HistoryState>,
) -> ModelScriptPreview {
    apply_impl(
        &script_name,
        &source,
        &workspace,
        &activity,
        &history,
    )
}

'''
)

text = Path(model_script).read_text(encoding="utf-8")
tests = r'''
    fn dense_nested_port_script() -> &'static str {
        include_str!("../../testdata/model-script/pr60-dense-nested-port.groovy")
    }

    fn normalized_dense_ibd(workspace: &WorkspaceState) -> serde_json::Value {
        let project_guard = workspace.project.lock().unwrap();
        let project = project_guard.as_ref().unwrap();
        let diagrams = workspace.ibd_diagrams.lock().unwrap();
        let diagram = diagrams
            .iter()
            .find(|diagram| diagram.name == "Dense Nested Port IBD")
            .unwrap();

        let properties = diagram
            .properties
            .iter()
            .map(|property| {
                let element = project
                    .element(super::super::parse_element_id(&property.element_id).unwrap())
                    .unwrap();
                let ports = property
                    .ports
                    .iter()
                    .map(|port| {
                        let semantic = project
                            .element(super::super::parse_element_id(&port.element_id).unwrap())
                            .unwrap();
                        serde_json::json!({
                            "semantic": semantic.external_id,
                            "x": port.x,
                            "y": port.y,
                            "size": port.size,
                        })
                    })
                    .collect::<Vec<_>>();
                serde_json::json!({
                    "semantic": element.external_id,
                    "x": property.x,
                    "y": property.y,
                    "width": property.width,
                    "height": property.height,
                    "ports": ports,
                })
            })
            .collect::<Vec<_>>();

        let boundary_ports = diagram
            .boundary_ports
            .iter()
            .map(|port| {
                let semantic = project
                    .element(super::super::parse_element_id(&port.element_id).unwrap())
                    .unwrap();
                serde_json::json!({
                    "semantic": semantic.external_id,
                    "x": port.x,
                    "y": port.y,
                    "size": port.size,
                })
            })
            .collect::<Vec<_>>();

        let connectors = diagram
            .connectors
            .iter()
            .map(|connector| {
                let relationship = project
                    .relationship(
                        super::super::parse_relationship_id(&connector.relationship_id).unwrap(),
                    )
                    .unwrap();
                serde_json::json!({
                    "semantic": relationship.external_id,
                    "points": connector.points,
                    "label_anchor": connector.label_anchor,
                })
            })
            .collect::<Vec<_>>();

        serde_json::json!({
            "diagram": diagram.name,
            "properties": properties,
            "boundary_ports": boundary_ports,
            "connectors": connectors,
        })
    }

    #[test]
    fn dense_nested_port_model_script_is_repeatable_across_preview_candidate_builds() {
        let (workspace, activity) = states();
        let source = dense_nested_port_script();
        let mut baseline = None;

        for iteration in 0..32 {
            let preview = preview_impl("dense.groovy", source, &workspace, &activity);
            assert!(
                preview.valid(),
                "preview iteration {iteration} failed: {:?}",
                preview.diagnostics
            );
            let (_, candidate_workspace, _) =
                build_candidate("dense.groovy", source, &workspace, &activity).unwrap();
            let normalized = normalized_dense_ibd(&candidate_workspace);
            if let Some(expected) = &baseline {
                assert_eq!(
                    expected, &normalized,
                    "model-script IBD presentation changed on iteration {iteration}"
                );
            } else {
                baseline = Some(normalized);
            }
        }
    }

    #[test]
    fn dense_nested_port_preview_then_apply_commits_once_and_reports_the_committed_candidate() {
        let (workspace, activity) = states();
        let history = super::super::history::HistoryState::default();
        let source = dense_nested_port_script();

        let preview = preview_impl("dense.groovy", source, &workspace, &activity);
        assert!(preview.valid(), "{:?}", preview.diagnostics);
        assert!(!preview.applied);

        let applied = apply_impl("dense.groovy", source, &workspace, &activity, &history);
        assert!(applied.applied, "{:?}", applied.diagnostics);
        assert!(applied.diagnostics.is_empty());
        assert_eq!(workspace.ibd_diagrams.lock().unwrap().len(), 1);

        let committed = normalized_dense_ibd(&workspace);
        assert_eq!(committed["properties"].as_array().unwrap().len(), 9);
        assert_eq!(committed["connectors"].as_array().unwrap().len(), 11);
    }

    #[test]
    fn default_clean_layout_and_route_flags_execute_one_import_presentation_pass() {
        let (workspace, activity) = states();
        let source = dense_nested_port_script();
        let preview = preview_impl("dense.groovy", source, &workspace, &activity);
        assert!(preview.valid(), "{:?}", preview.diagnostics);
        assert!(preview
            .items
            .iter()
            .any(|item| item.external_id.as_deref() == Some("D_IBD")));
    }
'''
if "dense_nested_port_model_script_is_repeatable_across_preview_candidate_builds" not in text:
    insert_at = text.rfind("\n}")
    if insert_at < 0:
        raise SystemExit("model_script.rs: final test module brace not found")
    text = text[:insert_at] + tests + text[insert_at:]
    Path(model_script).write_text(text, encoding="utf-8")
