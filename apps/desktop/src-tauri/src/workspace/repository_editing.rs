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
        let mut removed_presentation_ids: HashSet<_> = removed_nodes
            .iter()
            .map(|node| node.id.clone())
            .collect();
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
    let element_id = parse_element_id(&element_id)?;
    let new_owner_id = parse_element_id(&new_owner_id)?;
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    if project
        .element(element_id)
        .map_err(|error| error.to_string())?
        .owner_id
        == Some(new_owner_id)
    {
        return Ok(());
    }
    project
        .move_element(element_id, new_owner_id)
        .map_err(|error| error.to_string())?;
    project.validate().map_err(|error| error.to_string())?;
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    let moved_kind = project
        .element(element_id)
        .map_err(|error| error.to_string())?
        .kind
        .clone();
    if matches!(
        moved_kind.clone(),
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
        for diagram in diagrams.iter_mut().filter(|diagram| diagram.family == "parametric") {
            for node in &mut diagram.nodes {
                super::parametrics::sync_parameter_presentations(node, &project)?;
            }
            diagram.edges = super::parametrics::routed_edges(diagram, None)?;
        }
    }
    validate_loaded_diagrams(&project, &diagrams)?;

    history::checkpoint_states(&workspace, &activity, &history)?;
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(project);
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    Ok(())
}

#[tauri::command]
pub fn delete_model_element(
    element_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    let element_id = parse_element_id(&element_id)?;
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
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
    let behavior = workspace
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?
        .clone();
    let behavior_diagrams = workspace
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?
        .clone();
    let activity_repository = activity
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?
        .clone();
    let activity_diagrams = activity
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?
        .clone();

    project
        .delete_element(element_id)
        .map_err(|error| error.to_string())?;
    remove_bdd_presentations(&mut diagrams, element_id);
    let deleted_id = element_id.to_string();
    for diagram in &mut diagrams {
        if diagram.semantic_context_id.as_deref() == Some(deleted_id.as_str()) {
            diagram.semantic_context_id = None;
            diagram.subject_boundary = None;
        }
    }
    diagrams.retain(|diagram| {
        !(diagram.family == "parametric" && diagram.semantic_context_id.is_none())
    });
    remove_ibd_presentations(&mut ibd_diagrams, element_id);

    project.validate().map_err(|error| error.to_string())?;
    validate_loaded_diagrams(&project, &diagrams)?;
    ibd::validate_ibd_diagrams(&project, &ibd_diagrams)?;
    behavior_workspace::validate_behavior_workspace(&project, &behavior, &behavior_diagrams)?;
    activity_repository
        .validate(&project)
        .map_err(|error| error.to_string())?;
    validate_all_diagram_owners(
        &project,
        &diagrams,
        &ibd_diagrams,
        &behavior_diagrams,
        &activity_diagrams,
    )?;

    history::checkpoint_states(&workspace, &activity, &history)?;
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(project);
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    *workspace
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")? = ibd_diagrams;
    Ok(())
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
