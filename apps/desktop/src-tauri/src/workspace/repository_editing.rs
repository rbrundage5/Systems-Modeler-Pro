use super::*;
use systems_modeler_core::{ElementId, ElementKind, Project};

fn namespace_owner(project: &Project, owner_id: ElementId) -> Result<(), String> {
    let owner = project
        .element(owner_id)
        .map_err(|error| error.to_string())?;
    if !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {
        return Err("repository owner must be a Model or Package".into());
    }
    Ok(())
}

fn remove_bdd_presentations(diagrams: &mut [BddDiagram], element_id: ElementId) {
    let element_id = element_id.to_string();
    for diagram in diagrams {
        let removed_nodes: Vec<_> = diagram
            .nodes
            .iter()
            .filter(|node| node.element_id == element_id)
            .cloned()
            .collect();
        let mut removed_presentation_ids: HashSet<_> =
            removed_nodes.iter().map(|node| node.id.clone()).collect();
        removed_presentation_ids.extend(
            removed_nodes
                .iter()
                .flat_map(|node| node.parameter_presentations.iter())
                .map(|parameter| parameter.id.clone()),
        );
        for node in &mut diagram.nodes {
            node.parameter_presentations.retain(|parameter| {
                if parameter.parameter_id == element_id {
                    removed_presentation_ids.insert(parameter.id.clone());
                    false
                } else {
                    true
                }
            });
        }
        diagram
            .nodes
            .retain(|node| !removed_presentation_ids.contains(&node.id));
        diagram.edges.retain(|edge| {
            !removed_presentation_ids.contains(&edge.source_node_id)
                && !removed_presentation_ids.contains(&edge.target_node_id)
        });
    }
}

fn remove_ibd_presentations(diagrams: &mut [ibd::IbdDiagram], element_id: ElementId) {
    let element_id = element_id.to_string();
    for diagram in diagrams {
        let mut removed_presentation_ids = HashSet::new();
        diagram.properties.retain_mut(|property| {
            if property.element_id == element_id || property.property_path.contains(&element_id) {
                removed_presentation_ids.insert(property.id.clone());
                removed_presentation_ids.extend(property.ports.iter().map(|port| port.id.clone()));
                return false;
            }
            property.ports.retain(|port| {
                let remove =
                    port.element_id == element_id || port.property_path.contains(&element_id);
                if remove {
                    removed_presentation_ids.insert(port.id.clone());
                }
                !remove
            });
            true
        });
        diagram.boundary_ports.retain(|port| {
            let remove = port.element_id == element_id;
            if remove {
                removed_presentation_ids.insert(port.id.clone());
            }
            !remove
        });
        diagram.connectors.retain(|connector| {
            !removed_presentation_ids.contains(&connector.source_presentation_id)
                && !removed_presentation_ids.contains(&connector.target_presentation_id)
        });
    }
}

fn validate_all_diagram_owners(
    project: &Project,
    bdd: &[BddDiagram],
    ibd: &[ibd::IbdDiagram],
    behavior: &[behavior_workspace::BehaviorDiagram],
    activity: &[activity_workspace::ActivityDiagram],
) -> Result<(), String> {
    for owner_id in bdd
        .iter()
        .map(|diagram| diagram.owner_id.as_str())
        .chain(ibd.iter().map(|diagram| diagram.owner_id.as_str()))
        .chain(behavior.iter().map(|diagram| diagram.owner_id.as_str()))
        .chain(activity.iter().map(|diagram| diagram.owner_id.as_str()))
    {
        namespace_owner(project, parse_element_id(owner_id)?)?;
    }
    Ok(())
}

#[tauri::command]
pub fn move_repository_element(
    element_id: String,
    new_owner_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    move_repository_element_in_state(
        parse_element_id(&element_id)?,
        parse_element_id(&new_owner_id)?,
        &workspace,
        &activity,
        &history,
    )
    .map(|_| ())
}

fn move_repository_element_in_state(
    element_id: ElementId,
    new_owner_id: ElementId,
    workspace: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
    history: &history::HistoryState,
) -> Result<bool, String> {
    history::edit_authored_if_changed(workspace, activity, history, |candidate| {
        let current = candidate.project.as_ref().ok_or("no project open")?;
        if current
            .element(element_id)
            .map_err(|error| error.to_string())?
            .owner_id
            == Some(new_owner_id)
        {
            return Ok(false);
        }
        let mut project = current.clone();
        project
            .move_element(element_id, new_owner_id)
            .map_err(|error| error.to_string())?;
        project.validate().map_err(|error| error.to_string())?;
        let mut diagrams = super::relationship_editing::stage_relationship_presentations(
            current,
            &project,
            &candidate.diagrams,
            None,
        )?;
        let moved_kind = project
            .element(element_id)
            .map_err(|error| error.to_string())?
            .kind
            .clone();
        if matches!(
            moved_kind,
            ElementKind::ConstraintProperty | ElementKind::ValueProperty
        ) {
            let new_owner = new_owner_id.to_string();
            for diagram in diagrams.iter_mut().filter(|diagram| {
                diagram.family == "parametric"
                    && diagram.semantic_context_id.as_deref() != Some(new_owner.as_str())
            }) {
                remove_bdd_presentations(std::slice::from_mut(diagram), element_id);
            }
        }
        if moved_kind == ElementKind::ConstraintParameter {
            for diagram in diagrams
                .iter_mut()
                .filter(|diagram| diagram.family == "parametric")
            {
                for node in &mut diagram.nodes {
                    super::parametrics::sync_parameter_presentations(node, &project)?;
                }
                diagram.edges = super::parametrics::routed_edges(diagram, None)?;
            }
        }
        validate_loaded_diagrams(&project, &diagrams)?;
        ibd::validate_ibd_diagrams(&project, &candidate.ibd_diagrams)?;
        behavior_workspace::validate_behavior_workspace(
            &project,
            &candidate.behavior,
            &candidate.behavior_diagrams,
        )?;
        candidate
            .activity_repository
            .validate(&project)
            .map_err(|error| error.to_string())?;
        candidate.project = Some(project);
        candidate.diagrams = diagrams;
        Ok(true)
    })
}

#[tauri::command]
pub fn delete_model_element(
    element_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    delete_model_element_in_state(
        parse_element_id(&element_id)?,
        &workspace,
        &activity,
        &history,
    )
}

pub(super) fn delete_model_element_in_state(
    element_id: ElementId,
    workspace: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
    history: &history::HistoryState,
) -> Result<(), String> {
    history::edit_authored(workspace, activity, history, |candidate| {
        let deletion = candidate
            .project
            .as_ref()
            .ok_or("no project open")?
            .stage_connected_element_deletion(element_id)?;
        let deleted: HashSet<_> = deletion.elements.iter().map(ToString::to_string).collect();
        let relationships: HashSet<_> = deletion
            .relationships
            .iter()
            .map(ToString::to_string)
            .collect();
        for id in &deletion.elements {
            remove_bdd_presentations(&mut candidate.diagrams, *id);
            remove_ibd_presentations(&mut candidate.ibd_diagrams, *id);
        }
        candidate.diagrams.retain(|diagram| {
            !deleted.contains(&diagram.owner_id)
                && !diagram
                    .semantic_context_id
                    .as_ref()
                    .is_some_and(|id| deleted.contains(id))
        });
        for diagram in &mut candidate.diagrams {
            diagram
                .edges
                .retain(|edge| !relationships.contains(&edge.relationship_id));
        }
        candidate.ibd_diagrams.retain(|diagram| {
            !deleted.contains(&diagram.owner_id) && !deleted.contains(&diagram.context_block_id)
        });
        for diagram in &mut candidate.ibd_diagrams {
            diagram
                .connectors
                .retain(|edge| !relationships.contains(&edge.relationship_id));
        }
        candidate
            .behavior
            .remove_deleted_contexts(&deletion.elements);
        candidate
            .activity_repository
            .remove_deleted_contexts(&deletion.elements);
        candidate.behavior_diagrams.retain(|diagram| {
            !deleted.contains(&diagram.owner_id) && !deleted.contains(&diagram.context_id)
        });
        candidate.activity_diagrams.retain(|diagram| {
            !deleted.contains(&diagram.owner_id)
                && candidate
                    .activity_repository
                    .activities
                    .keys()
                    .any(|id| id.to_string() == diagram.activity_id)
        });
        let project = deletion.project;
        validate_loaded_diagrams(&project, &candidate.diagrams)?;
        ibd::validate_ibd_diagrams(&project, &candidate.ibd_diagrams)?;
        behavior_workspace::validate_behavior_workspace(
            &project,
            &candidate.behavior,
            &candidate.behavior_diagrams,
        )?;
        candidate
            .activity_repository
            .validate(&project)
            .map_err(|error| error.to_string())?;
        validate_all_diagram_owners(
            &project,
            &candidate.diagrams,
            &candidate.ibd_diagrams,
            &candidate.behavior_diagrams,
            &candidate.activity_diagrams,
        )?;
        candidate.project = Some(project);
        Ok(())
    })
}

fn update_diagram_owner(
    diagram_id: &str,
    new_owner_id: ElementId,
    bdd: &mut [BddDiagram],
    ibd: &mut [ibd::IbdDiagram],
    behavior: &mut [behavior_workspace::BehaviorDiagram],
    activity: &mut [activity_workspace::ActivityDiagram],
) -> Result<(), String> {
    let new_owner_id = new_owner_id.to_string();
    let mut matches = 0;
    for owner_id in bdd
        .iter_mut()
        .filter(|diagram| diagram.id == diagram_id)
        .map(|diagram| &mut diagram.owner_id)
        .chain(
            ibd.iter_mut()
                .filter(|diagram| diagram.id == diagram_id)
                .map(|diagram| &mut diagram.owner_id),
        )
        .chain(
            behavior
                .iter_mut()
                .filter(|diagram| diagram.id == diagram_id)
                .map(|diagram| &mut diagram.owner_id),
        )
        .chain(
            activity
                .iter_mut()
                .filter(|diagram| diagram.id == diagram_id)
                .map(|diagram| &mut diagram.owner_id),
        )
    {
        *owner_id = new_owner_id.clone();
        matches += 1;
    }
    match matches {
        0 => Err("diagram not found".into()),
        1 => Ok(()),
        _ => Err("diagram identity is duplicated across workspace families".into()),
    }
}

#[tauri::command]
pub fn move_repository_diagram(
    diagram_id: String,
    new_owner_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    parse_diagram_id(&diagram_id)?;
    let new_owner_id = parse_element_id(&new_owner_id)?;
    let project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    namespace_owner(&project, new_owner_id)?;
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    let mut ibd_diagrams = workspace
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")?
        .clone();
    let mut behavior_diagrams = workspace
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?
        .clone();
    let mut activity_diagrams = activity
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?
        .clone();

    update_diagram_owner(
        &diagram_id,
        new_owner_id,
        &mut diagrams,
        &mut ibd_diagrams,
        &mut behavior_diagrams,
        &mut activity_diagrams,
    )?;
    validate_all_diagram_owners(
        &project,
        &diagrams,
        &ibd_diagrams,
        &behavior_diagrams,
        &activity_diagrams,
    )?;

    history::checkpoint_states(&workspace, &activity, &history)?;
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    *workspace
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")? = ibd_diagrams;
    *workspace
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")? = behavior_diagrams;
    *activity
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")? = activity_diagrams;
    Ok(())
}

fn remove_diagram<T>(diagrams: &mut Vec<T>, matches: impl Fn(&T) -> bool) -> usize {
    let original_len = diagrams.len();
    diagrams.retain(|diagram| !matches(diagram));
    original_len - diagrams.len()
}

#[tauri::command]
pub fn delete_repository_diagram(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
    shared: tauri::State<'_, shared_workspace::SharedWorkspaceState>,
) -> Result<(), String> {
    parse_diagram_id(&diagram_id)?;
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    let mut ibd_diagrams = workspace
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")?
        .clone();
    let mut behavior_diagrams = workspace
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?
        .clone();
    let mut activity_diagrams = activity
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?
        .clone();

    let removed = remove_diagram(&mut diagrams, |diagram| diagram.id == diagram_id)
        + remove_diagram(&mut ibd_diagrams, |diagram| diagram.id == diagram_id)
        + remove_diagram(&mut behavior_diagrams, |diagram| diagram.id == diagram_id)
        + remove_diagram(&mut activity_diagrams, |diagram| diagram.id == diagram_id);
    match removed {
        0 => return Err("diagram not found".into()),
        1 => {}
        _ => return Err("diagram identity is duplicated across workspace families".into()),
    }

    history::checkpoint_states(&workspace, &activity, &history)?;
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    *workspace
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")? = ibd_diagrams;
    *workspace
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")? = behavior_diagrams;
    *activity
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")? = activity_diagrams;
    shared.forget_diagram(&diagram_id)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connected_part_deletion_cleans_every_view_and_is_one_undoable_edit() {
        use systems_modeler_core::{
            Connector, ConnectorEnd, ConnectorKind, ItemFlow, Multiplicity,
        };
        let mut project = Project::new("Connected deletion");
        let owner = project.root_id;
        let system = project
            .create_element(ElementKind::Block, "System", owner)
            .unwrap();
        let component = project
            .create_element(ElementKind::Block, "Component", owner)
            .unwrap();
        let signal = project
            .create_element(ElementKind::Signal, "Signal", owner)
            .unwrap();
        let (composition, part) = project
            .create_composition(system, component, "part", Multiplicity::ONE, Some(owner))
            .unwrap();
        let sibling = project
            .create_typed_feature(
                ElementKind::PartProperty,
                "other",
                system,
                component,
                Multiplicity::ONE,
            )
            .unwrap();
        let source = ConnectorEnd::role(part);
        let target = ConnectorEnd::role(sibling);
        let connector = project
            .create_connector(Connector {
                context_id: system,
                kind: ConnectorKind::Assembly,
                source: source.clone(),
                target: target.clone(),
            })
            .unwrap();
        project
            .create_item_flow(ItemFlow {
                connector_id: connector,
                source,
                target,
                conveyed_item_ids: vec![signal],
            })
            .unwrap();
        let nodes: Vec<_> = [system, component]
            .into_iter()
            .enumerate()
            .map(|(index, id)| DiagramNode {
                id: uuid::Uuid::new_v4().to_string(),
                element_id: id.to_string(),
                x: index as f64 * 300.0,
                y: 0.0,
                width: 100.0,
                height: 60.0,
                actor_notation: None,
                parameter_presentations: Vec::new(),
            })
            .collect();
        let edge = DiagramEdge {
            id: uuid::Uuid::new_v4().to_string(),
            relationship_id: composition.to_string(),
            source_node_id: nodes[0].id.clone(),
            target_node_id: nodes[1].id.clone(),
            points: route_relationship(&nodes[0], &nodes[1], &nodes).unwrap(),
            label_anchor: None,
        };
        let bdd = BddDiagram {
            id: DiagramId::new().to_string(),
            name: "Structure".into(),
            owner_id: owner.to_string(),
            family: "bdd".into(),
            semantic_context_id: None,
            subject_boundary: None,
            nodes,
            edges: vec![edge],
        };
        let properties: Vec<_> = [part, sibling]
            .into_iter()
            .enumerate()
            .map(|(index, id)| ibd::IbdPropertyPresentation {
                collapsed: false,
                id: uuid::Uuid::new_v4().to_string(),
                element_id: id.to_string(),
                property_path: vec![id.to_string()],
                x: index as f64 * 300.0,
                y: 0.0,
                width: 100.0,
                height: 60.0,
                ports: Vec::new(),
            })
            .collect();
        let ibd_edge = ibd::IbdConnectorPresentation {
            id: uuid::Uuid::new_v4().to_string(),
            relationship_id: connector.to_string(),
            source_presentation_id: properties[0].id.clone(),
            target_presentation_id: properties[1].id.clone(),
            points: Vec::new(),
            label_anchor: None,
        };
        let internal = ibd::IbdDiagram {
            id: DiagramId::new().to_string(),
            name: "Internal".into(),
            context_block_id: system.to_string(),
            owner_id: owner.to_string(),
            context_frame: None,
            properties,
            boundary_ports: Vec::new(),
            connectors: vec![ibd_edge],
        };
        let workspace = WorkspaceState::default();
        let activity = activity_workspace::ActivityWorkspaceState::default();
        let history = history::HistoryState::default();
        let mut second = bdd.clone();
        second.id = DiagramId::new().to_string();
        for node in &mut second.nodes {
            node.id = uuid::Uuid::new_v4().to_string();
        }
        second.edges[0].id = uuid::Uuid::new_v4().to_string();
        second.edges[0].source_node_id = second.nodes[0].id.clone();
        second.edges[0].target_node_id = second.nodes[1].id.clone();
        *workspace.project.lock().unwrap() = Some(project);
        *workspace.diagrams.lock().unwrap() = vec![bdd, second];
        *workspace.ibd_diagrams.lock().unwrap() = vec![internal];
        let value = || {
            serde_json::to_value((
                &*workspace.project.lock().unwrap(),
                &*workspace.diagrams.lock().unwrap(),
                &*workspace.ibd_diagrams.lock().unwrap(),
            ))
            .unwrap()
        };
        let before = value();
        delete_model_element_in_state(part, &workspace, &activity, &history).unwrap();
        let after = value();
        assert_eq!(history::undo_len(&history), 1);
        assert!(
            workspace
                .diagrams
                .lock()
                .unwrap()
                .iter()
                .all(|diagram| diagram.edges.is_empty())
        );
        assert_eq!(
            workspace.ibd_diagrams.lock().unwrap()[0].properties.len(),
            1
        );
        assert!(
            workspace.ibd_diagrams.lock().unwrap()[0]
                .connectors
                .is_empty()
        );
        {
            let guard = workspace.project.lock().unwrap();
            let project = guard.as_ref().unwrap();
            assert!(project.element(component).is_ok());
            assert!(project.element(sibling).is_ok());
            assert!(project.relationships.is_empty());
            let directory = tempfile::tempdir().unwrap();
            let mut database =
                ProjectDatabase::open(directory.path().join("deleted.smproj")).unwrap();
            database.save_project(project).unwrap();
            let loaded = database.load_first_project().unwrap();
            loaded.validate().unwrap();
            assert_eq!(
                serde_json::to_value(loaded).unwrap(),
                serde_json::to_value(project).unwrap()
            );
        }
        assert!(history::undo_states(&workspace, &activity, &history).unwrap());
        assert_eq!(value(), before);
        assert!(delete_model_element_in_state(owner, &workspace, &activity, &history).is_err());
        assert_eq!(value(), before);
        assert_eq!(history::undo_len(&history), 0);
        assert!(history::redo_states(&workspace, &activity, &history).unwrap());
        assert_eq!(value(), after);
    }

    #[test]
    fn removing_a_semantic_element_removes_each_bdd_presentation_and_incident_edge() {
        let element_id = ElementId::new();
        let other_id = ElementId::new();
        let mut diagrams = vec![BddDiagram {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Structure".into(),
            owner_id: ElementId::new().to_string(),
            family: "bdd".into(),
            semantic_context_id: None,
            subject_boundary: None,
            nodes: vec![
                DiagramNode {
                    id: "removed".into(),
                    element_id: element_id.to_string(),
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 50.0,
                    actor_notation: None,
                    parameter_presentations: Vec::new(),
                },
                DiagramNode {
                    id: "retained".into(),
                    element_id: other_id.to_string(),
                    x: 200.0,
                    y: 0.0,
                    width: 100.0,
                    height: 50.0,
                    actor_notation: None,
                    parameter_presentations: Vec::new(),
                },
            ],
            edges: vec![DiagramEdge {
                id: "edge".into(),
                relationship_id: uuid::Uuid::new_v4().to_string(),
                source_node_id: "removed".into(),
                target_node_id: "retained".into(),
                points: vec![],
                label_anchor: None,
            }],
        }];

        remove_bdd_presentations(&mut diagrams, element_id);

        assert_eq!(diagrams[0].nodes.len(), 1);
        assert_eq!(diagrams[0].nodes[0].element_id, other_id.to_string());
        assert!(diagrams[0].edges.is_empty());
    }
}

#[cfg(test)]
mod move_transaction_tests {
    use super::*;

    #[test]
    fn moving_ownership_is_one_transaction_and_rejection_preserves_redo() {
        let workspace = WorkspaceState::default();
        let activity = activity_workspace::ActivityWorkspaceState::default();
        let history = history::HistoryState::default();
        let mut project = Project::new("Move");
        let root = project.root_id;
        let package = project
            .create_element(ElementKind::Package, "Destination", root)
            .unwrap();
        let block = project
            .create_element(ElementKind::Block, "Block", root)
            .unwrap();
        *workspace.project.lock().unwrap() = Some(project);
        let apply =
            |owner| move_repository_element_in_state(block, owner, &workspace, &activity, &history);
        assert!(apply(package).unwrap());
        assert_eq!(history::undo_len(&history), 1);
        assert!(!apply(package).unwrap());
        assert_eq!(history::undo_len(&history), 1);
        assert!(history::undo_states(&workspace, &activity, &history).unwrap());
        assert!(apply(block).is_err());
        assert!(apply(ElementId::new()).is_err());
        assert!(!apply(root).unwrap());
        assert_eq!(history::undo_len(&history), 0);
        assert!(history::redo_states(&workspace, &activity, &history).unwrap());
        assert_eq!(
            workspace
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .element(block)
                .unwrap()
                .owner_id,
            Some(package)
        );
        let before =
            serde_json::to_value(workspace.project.lock().unwrap().as_ref().unwrap()).unwrap();
        let _busy = activity.repository.lock().unwrap();
        assert!(apply(root).is_err());
        assert_eq!(
            serde_json::to_value(workspace.project.lock().unwrap().as_ref().unwrap()).unwrap(),
            before
        );
        assert_eq!(history::undo_len(&history), 1);
    }
}
