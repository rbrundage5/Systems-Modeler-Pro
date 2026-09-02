from pathlib import Path

path = Path("apps/desktop/src-tauri/src/workspace/model_script.rs")
text = path.read_text(encoding="utf-8")

if "fn script_external_token(" not in text:
    anchor = "fn element_reference(\n"
    helper = '''fn script_external_token(token: &str) -> &str {
    token
        .strip_prefix("ext:")
        .or_else(|| token.strip_prefix("handle:"))
        .unwrap_or(token)
}

'''
    if anchor not in text:
        raise SystemExit("element_reference anchor not found")
    text = text.replace(anchor, helper + anchor, 1)

text = text.replace(
    'let external = token.strip_prefix("ext:").unwrap_or(token).trim();',
    'let external = script_external_token(token).trim();',
)
text = text.replace(
    'let external = token.strip_prefix("ext:").unwrap_or(token);',
    'let external = script_external_token(token);',
)
text = text.replace(
    'external.strip_prefix("ext:").unwrap_or(external),',
    'script_external_token(external),',
)
text = text.replace(
    'semantic.strip_prefix("ext:").unwrap_or(semantic)',
    'script_external_token(semantic)',
)

start = text.index("fn populate_bdd_like(")
end = text.index("\nfn create_script_diagram(", start)
population = r'''fn relationship_presentable_on_family(family: &str, kind: &RelationshipKind) -> bool {
    match family {
        "package" => matches!(
            kind,
            RelationshipKind::PackageImport
                | RelationshipKind::ElementImport
                | RelationshipKind::Dependency
        ),
        "use-case" => matches!(
            kind,
            RelationshipKind::Association
                | RelationshipKind::Include
                | RelationshipKind::Extend
                | RelationshipKind::Generalization
        ),
        "parametric" => *kind == RelationshipKind::BindingConnector,
        _ => !matches!(
            kind,
            RelationshipKind::Connector
                | RelationshipKind::ItemFlow
                | RelationshipKind::BindingConnector
        ),
    }
}

fn populate_bdd_like(
    diagram: &mut BddDiagram,
    project: &Project,
    family: &str,
    scope: ElementId,
) -> Result<(), String> {
    let mut x = 100.0;
    let mut y = 100.0;
    for element in project
        .children(scope)
        .into_iter()
        .filter(|element| bdd_family_accepts(family, &element.kind))
    {
        let constraint = family == "parametric" && element.kind == ElementKind::ConstraintProperty;
        let mut node = DiagramNode {
            id: uuid::Uuid::new_v4().to_string(),
            element_id: element.id.to_string(),
            x,
            y,
            width: if constraint {
                260.0
            } else if family == "parametric" {
                220.0
            } else {
                190.0
            },
            height: if constraint {
                170.0
            } else if family == "parametric" {
                72.0
            } else {
                110.0
            },
            actor_notation: None,
            parameter_presentations: Vec::new(),
        };
        if family == "parametric" {
            super::parametrics::sync_parameter_presentations(&mut node, project)?;
        }
        diagram.nodes.push(node);
        x += 260.0;
        if x > 900.0 {
            x = 100.0;
            y += 180.0;
        }
    }

    let node_for = |id: ElementId| {
        diagram
            .nodes
            .iter()
            .find(|node| node.element_id == id.to_string())
            .map(|node| node.id.clone())
    };
    let parametric_endpoint = |role_id: ElementId, parameter_id: Option<ElementId>| {
        let role = diagram
            .nodes
            .iter()
            .find(|node| node.element_id == role_id.to_string())?;
        match parameter_id {
            None => Some(role.id.clone()),
            Some(parameter_id) => role
                .parameter_presentations
                .iter()
                .find(|parameter| parameter.parameter_id == parameter_id.to_string())
                .map(|parameter| parameter.id.clone()),
        }
    };

    for relationship in project
        .relationships
        .values()
        .filter(|relationship| relationship_presentable_on_family(family, &relationship.kind))
    {
        let endpoints = if family == "parametric" {
            let binding = relationship
                .binding
                .as_ref()
                .ok_or("BindingConnector has no semantic endpoint details")?;
            (
                parametric_endpoint(binding.source.role_id, binding.source.parameter_id),
                parametric_endpoint(binding.target.role_id, binding.target.parameter_id),
            )
        } else {
            (node_for(relationship.source_id), node_for(relationship.target_id))
        };
        if let (Some(source), Some(target)) = endpoints {
            diagram.edges.push(DiagramEdge {
                id: uuid::Uuid::new_v4().to_string(),
                relationship_id: relationship.id.to_string(),
                source_node_id: source,
                target_node_id: target,
                points: vec![
                    DiagramPoint { x: 0.0, y: 0.0 },
                    DiagramPoint { x: 1.0, y: 1.0 },
                ],
                label_anchor: None,
            });
        }
    }
    Ok(())
}

fn stable_script_diagram_id(namespace: &str, external_id: &str) -> String {
    fn hash(seed: u64, bytes: &[u8]) -> u64 {
        bytes.iter().fold(seed, |value, byte| {
            (value ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        })
    }
    let key = format!("systems-modeler:model-script:{namespace}:{external_id}");
    let mut bytes = [0_u8; 16];
    bytes[..8].copy_from_slice(&hash(0xcbf29ce484222325, key.as_bytes()).to_be_bytes());
    bytes[8..].copy_from_slice(&hash(0x84222325cbf29ce4, key.as_bytes()).to_be_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes).to_string()
}
'''
text = text[:start] + population + text[end:]

function_start = text.index("fn create_script_diagram(")
function_end = text.index("\nfn layout_and_route(", function_start)
section = text[function_start:function_end]
section = section.replace(
    "let id = uuid::Uuid::new_v4().to_string();",
    "let id = stable_script_diagram_id(namespace, &diagram.external_id);",
)
section = section.replace(
    "populate_bdd_like(&mut native, &project, family, scope);",
    "populate_bdd_like(&mut native, &project, family, scope)?;",
)
section = section.replace(
    '''workspace
            .diagrams
            .lock()
            .map_err(|_| "diagram lock poisoned")?
            .push(native);''',
    '''let mut diagrams = workspace
            .diagrams
            .lock()
            .map_err(|_| "diagram lock poisoned")?;
        diagrams.retain(|existing| existing.id != id);
        diagrams.push(native);''',
    1,
)
section = section.replace(
    '''workspace
            .ibd_diagrams
            .lock()
            .map_err(|_| "IBD lock poisoned")?
            .push(native);''',
    '''let mut diagrams = workspace
            .ibd_diagrams
            .lock()
            .map_err(|_| "IBD lock poisoned")?;
        diagrams.retain(|existing| existing.id != id);
        diagrams.push(native);''',
    1,
)
section = section.replace(
    '''activity
            .diagrams
            .lock()
            .map_err(|_| "Activity diagram lock poisoned")?
            .push(native);''',
    '''let mut diagrams = activity
            .diagrams
            .lock()
            .map_err(|_| "Activity diagram lock poisoned")?;
        diagrams.retain(|existing| existing.id != id);
        diagrams.push(native);''',
    1,
)
section = section.replace(
    '''workspace
            .behavior_diagrams
            .lock()
            .map_err(|_| "behavior diagram lock poisoned")?
            .push(native);''',
    '''let mut diagrams = workspace
            .behavior_diagrams
            .lock()
            .map_err(|_| "behavior diagram lock poisoned")?;
        diagrams.retain(|existing| existing.id != id);
        diagrams.push(native);''',
    1,
)
text = text[:function_start] + section + text[function_end:]

if "PR51_REPRESENTATIVE_QUALIFICATION" not in text:
    tests = r'''

    // PR51_REPRESENTATIVE_QUALIFICATION
    fn representative_vehicle_script() -> &'static str {
        include_str!("../../../../../examples/model-script/vehicle-model.groovy")
    }

    fn applied_vehicle_states() -> (WorkspaceState, ActivityWorkspaceState) {
        let (workspace, activity) = states();
        let (compiled, candidate_workspace, candidate_activity) = build_candidate(
            "vehicle-model.groovy",
            representative_vehicle_script(),
            &workspace,
            &activity,
        )
        .unwrap_or_else(|preview| {
            panic!("representative script failed: {:?}", preview.diagnostics)
        });
        assert_eq!(compiled.document.diagrams.len(), 9);
        commit_candidate(
            &workspace,
            &activity,
            &candidate_workspace,
            &candidate_activity,
        )
        .unwrap();
        (workspace, activity)
    }

    #[test]
    fn representative_script_builds_native_semantics_and_all_nine_diagram_families() {
        let (workspace, activity) = states();
        let before = super::portable_interchange::export_from_states(&workspace, &activity).unwrap();
        let preview = preview_impl(
            "vehicle-model.groovy",
            representative_vehicle_script(),
            &workspace,
            &activity,
        );
        assert!(preview.valid(), "{:?}", preview.diagnostics);
        let after_preview = super::portable_interchange::export_from_states(&workspace, &activity).unwrap();
        assert_eq!(before, after_preview, "dry run mutated authored state");

        let (_, candidate_workspace, candidate_activity) = build_candidate(
            "vehicle-model.groovy",
            representative_vehicle_script(),
            &workspace,
            &activity,
        )
        .unwrap();
        super::portable_interchange::portable_from_states(
            &candidate_workspace,
            &candidate_activity,
        )
        .unwrap();
        assert_eq!(candidate_workspace.diagrams.lock().unwrap().len(), 5);
        assert_eq!(candidate_workspace.ibd_diagrams.lock().unwrap().len(), 1);
        assert_eq!(candidate_activity.diagrams.lock().unwrap().len(), 1);
        assert_eq!(candidate_workspace.behavior_diagrams.lock().unwrap().len(), 2);

        let project = candidate_workspace.project.lock().unwrap().clone().unwrap();
        assert!(project
            .elements
            .values()
            .any(|element| element.external_id == "groovy:vehicle-example::VEH"));
        assert!(project.relationships.values().any(|relationship| {
            relationship.external_id == "groovy:vehicle-example::BIND"
                && relationship.kind == RelationshipKind::BindingConnector
        }));
        assert!(project.relationships.values().any(|relationship| {
            relationship.external_id == "groovy:vehicle-example::CTRL_LINK"
                && relationship.kind == RelationshipKind::Connector
        }));
        assert!(project.relationships.values().any(|relationship| {
            relationship.external_id == "groovy:vehicle-example::STARTED_FLOW"
                && relationship.kind == RelationshipKind::ItemFlow
        }));
        project.validate().unwrap();

        let activities = candidate_activity.repository.lock().unwrap();
        let activity_record = activities
            .activities
            .values()
            .find(|record| record.external_id == "groovy:vehicle-example::ACT_START")
            .unwrap();
        assert!(activity_record.nodes.len() >= 4);
        assert!(activity_record.edges.len() >= 3);
        activities.validate(&project).unwrap();
        drop(activities);

        let behavior = candidate_workspace.behavior.lock().unwrap();
        assert!(behavior
            .state_machines
            .values()
            .any(|record| record.external_id == "groovy:vehicle-example::SM_MODES"));
        let interaction = behavior
            .interactions
            .values()
            .find(|record| record.external_id == "groovy:vehicle-example::INT_STARTUP")
            .unwrap();
        assert_eq!(interaction.messages.len(), 1);
        assert_eq!(interaction.executions.len(), 1);
        drop(behavior);

        let parametric = candidate_workspace
            .diagrams
            .lock()
            .unwrap()
            .iter()
            .find(|diagram| diagram.family == "parametric")
            .cloned()
            .unwrap();
        let constraint_node = parametric
            .nodes
            .iter()
            .find(|node| {
                super::parse_element_id(&node.element_id)
                    .ok()
                    .and_then(|id| project.element(id).ok())
                    .is_some_and(|element| element.kind == ElementKind::ConstraintProperty)
            })
            .unwrap();
        assert_eq!(constraint_node.parameter_presentations.len(), 1);
        assert_eq!(parametric.edges.len(), 1);
        assert!(parametric.edges[0].points.len() >= 2);

        commit_candidate(
            &workspace,
            &activity,
            &candidate_workspace,
            &candidate_activity,
        )
        .unwrap();
        let first_element_count = workspace
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .elements
            .len();
        let first_relationship_count = workspace
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .relationships
            .len();
        let first_diagram_count = workspace.diagrams.lock().unwrap().len()
            + workspace.ibd_diagrams.lock().unwrap().len()
            + activity.diagrams.lock().unwrap().len()
            + workspace.behavior_diagrams.lock().unwrap().len();
        assert_eq!(first_diagram_count, 9);

        let (_, second_workspace, second_activity) = build_candidate(
            "vehicle-model.groovy",
            representative_vehicle_script(),
            &workspace,
            &activity,
        )
        .unwrap();
        commit_candidate(&workspace, &activity, &second_workspace, &second_activity).unwrap();
        assert_eq!(
            workspace
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            first_element_count
        );
        assert_eq!(
            workspace
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationships
                .len(),
            first_relationship_count
        );
        let second_diagram_count = workspace.diagrams.lock().unwrap().len()
            + workspace.ibd_diagrams.lock().unwrap().len()
            + activity.diagrams.lock().unwrap().len()
            + workspace.behavior_diagrams.lock().unwrap().len();
        assert_eq!(second_diagram_count, 9, "script reapply duplicated a diagram");
    }

    #[test]
    fn exact_qualified_name_and_explicit_plan_local_handle_references_are_supported() {
        let (workspace, activity) = states();
        let package_qname = {
            let mut guard = workspace.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            let package = project
                .create_element(ElementKind::Package, "Existing", project.root_id)
                .unwrap();
            project.qualified_name(package).unwrap()
        };
        let source = format!(
            r#"{{"source_namespace":"qname-test","operations":[{{"op":"element","external_id":"BLOCK","kind":"Block","name":"QualifiedChild","owner":"qname:{package_qname}"}},{{"op":"element","external_id":"OP","kind":"Operation","name":"run","owner":"handle:BLOCK"}}]}}"#
        );
        let preview = preview_impl("qname.groovy", &source, &workspace, &activity);
        assert!(preview.valid(), "{:?}", preview.diagnostics);
        let (_, candidate_workspace, _) =
            build_candidate("qname.groovy", &source, &workspace, &activity).unwrap();
        let guard = candidate_workspace.project.lock().unwrap();
        assert!(guard.as_ref().unwrap().elements.values().any(|element| {
            element.external_id == "qname-test::OP"
        }));
    }

    #[test]
    fn script_created_model_round_trips_smproj_metadata_and_exports_portable_json_and_xlsx() {
        let (workspace, activity) = applied_vehicle_states();
        let portable_json =
            super::portable_interchange::export_from_states(&workspace, &activity).unwrap();
        assert!(portable_json.contains("groovy:vehicle-example::VEH"));
        assert!(portable_json.contains("groovy:vehicle-example::ACT_START"));
        assert!(portable_json.contains("groovy:vehicle-example::INT_STARTUP"));

        let xlsx = std::env::temp_dir().join(format!("pr51-{}.xlsx", uuid::Uuid::new_v4()));
        super::spreadsheet_interchange::export_workbook_to_path(
            xlsx.to_str().unwrap(),
            super::spreadsheet_interchange::SpreadsheetExportProfile::SystemsModeler,
            &workspace,
            &activity,
        )
        .unwrap();
        assert!(std::fs::metadata(&xlsx).unwrap().len() > 1024);

        let smproj =
            std::env::temp_dir().join(format!("pr51-{}.smproj", uuid::Uuid::new_v4()));
        let project = workspace.project.lock().unwrap().clone().unwrap();
        let diagrams = workspace.diagrams.lock().unwrap().clone();
        let ibd_diagrams = workspace.ibd_diagrams.lock().unwrap().clone();
        let behavior = workspace.behavior.lock().unwrap().clone();
        let behavior_diagrams = workspace.behavior_diagrams.lock().unwrap().clone();
        let activity_repository = activity.repository.lock().unwrap().clone();
        let activity_diagrams = activity.diagrams.lock().unwrap().clone();
        {
            let mut database =
                systems_modeler_persistence::ProjectDatabase::open(&smproj).unwrap();
            database.save_project(&project).unwrap();
            database
                .save_metadata(
                    project.id,
                    super::BDD_METADATA_KEY,
                    &serde_json::to_string(&diagrams).unwrap(),
                )
                .unwrap();
            super::ibd::save_ibd_metadata(&mut database, &project, &ibd_diagrams).unwrap();
            super::behavior_workspace::save_behavior_metadata(
                &mut database,
                &project,
                &behavior,
                &behavior_diagrams,
            )
            .unwrap();
            super::activity_workspace::save_activity_workspace_metadata(
                &mut database,
                &project,
                &activity_repository,
                &activity_diagrams,
            )
            .unwrap();
        }
        {
            let database = systems_modeler_persistence::ProjectDatabase::open(&smproj).unwrap();
            let reopened = database.load_first_project().unwrap();
            reopened.validate().unwrap();
            assert!(reopened
                .elements
                .values()
                .any(|element| element.external_id == "groovy:vehicle-example::VEH"));
            let bdd_payload = database
                .load_metadata(reopened.id, super::BDD_METADATA_KEY)
                .unwrap()
                .unwrap();
            let reopened_diagrams: Vec<BddDiagram> =
                serde_json::from_str(&bdd_payload).unwrap();
            assert_eq!(reopened_diagrams.len(), 5);
            let reopened_ibd = super::ibd::load_ibd_metadata(&database, &reopened).unwrap();
            assert_eq!(reopened_ibd.len(), 1);
            let (reopened_behavior, reopened_behavior_diagrams) =
                super::behavior_workspace::load_behavior_metadata(&database, &reopened).unwrap();
            assert_eq!(reopened_behavior.state_machines.len(), 1);
            assert_eq!(reopened_behavior.interactions.len(), 1);
            assert_eq!(reopened_behavior_diagrams.len(), 2);
            let (reopened_activity, reopened_activity_diagrams) =
                super::activity_workspace::load_activity_workspace_metadata(&database, &reopened)
                    .unwrap();
            assert_eq!(reopened_activity.activities.len(), 1);
            assert_eq!(reopened_activity_diagrams.len(), 1);
        }
        let _ = std::fs::remove_file(xlsx);
        let _ = std::fs::remove_file(smproj);
    }
'''
    close = text.rfind("\n}")
    if close < 0:
        raise SystemExit("test module closing brace not found")
    text = text[:close] + tests + text[close:]

path.write_text(text, encoding="utf-8")
