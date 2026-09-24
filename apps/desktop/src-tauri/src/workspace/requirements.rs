//! Rust-authoritative Requirement and traceability workspace operations.
//!
//! Requirement diagrams deliberately reuse the qualified structural diagram
//! geometry, routing, persistence, and history infrastructure.

use super::*;
use serde::Deserialize;
use systems_modeler_core::{ElementKind, RelationshipKind};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequirementUpdateRequest {
    element_id: String,
    name: String,
    requirement_id: String,
    text: String,
    documentation: String,
}

fn traceability_kind(value: &str) -> Result<RelationshipKind, String> {
    match value {
        "DeriveRequirement" | "deriveReqt" => Ok(RelationshipKind::DeriveRequirement),
        "Satisfy" | "satisfy" => Ok(RelationshipKind::Satisfy),
        "Verify" | "verify" => Ok(RelationshipKind::Verify),
        "Refine" | "refine" => Ok(RelationshipKind::Refine),
        "Trace" | "trace" => Ok(RelationshipKind::Trace),
        "Copy" | "copy" => Ok(RelationshipKind::Copy),
        _ => Err(format!("unsupported Requirement relationship: {value}")),
    }
}

fn checkpoint(
    workspace: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
    history: &history::HistoryState,
) -> Result<(), String> {
    history::checkpoint_states(workspace, activity, history)
}

#[tauri::command]
pub fn create_requirement_diagram(
    owner_id: String,
    name: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    let owner_id = parse_element_id(&owner_id)?;
    let project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?;
    let owner = project
        .as_ref()
        .ok_or("no project open")?
        .element(owner_id)
        .map_err(|error| error.to_string())?;
    if !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {
        return Err("Requirement Diagram owner must be a Model or Package".into());
    }
    drop(project);
    checkpoint(&workspace, &activity, &history)?;
    let id = DiagramId::new().to_string();
    workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .push(BddDiagram {
            id: id.clone(),
            name,
            owner_id: owner_id.to_string(),
            family: "requirement".into(),
            semantic_context_id: None,
            subject_boundary: None,
            nodes: Vec::new(),
            edges: Vec::new(),
        });
    Ok(id)
}

#[tauri::command]
pub fn create_requirement(
    owner_id: String,
    name: String,
    requirement_id: String,
    text: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    let owner_id = parse_element_id(&owner_id)?;
    checkpoint(&workspace, &activity, &history)?;
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?;
    project
        .as_mut()
        .ok_or("no project open")?
        .create_requirement(name, requirement_id, text, owner_id)
        .map(|id| id.to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_test_case(
    owner_id: String,
    name: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    let owner_id = parse_element_id(&owner_id)?;
    checkpoint(&workspace, &activity, &history)?;
    workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .as_mut()
        .ok_or("no project open")?
        .create_element(ElementKind::TestCase, name, owner_id)
        .map(|id| id.to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_requirement(
    details: RequirementUpdateRequest,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    apply_requirement_update(&workspace, &activity, &history, &details).map(|_| ())
}

fn apply_requirement_update(
    workspace: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
    history: &history::HistoryState,
    details: &RequirementUpdateRequest,
) -> Result<bool, String> {
    let element_id = parse_element_id(&details.element_id)?;
    history::apply_structural_specification(workspace, activity, history, |project, diagrams| {
        let mut candidate = project.clone();
        candidate
            .update_requirement(
                element_id,
                details.requirement_id.clone(),
                details.text.clone(),
            )
            .map_err(|error| error.to_string())?;
        let requirement = candidate
            .element_mut(element_id)
            .map_err(|error| error.to_string())?;
        requirement.name = details.name.clone();
        requirement.documentation = details.documentation.clone();
        candidate.validate().map_err(|error| error.to_string())?;
        Ok((candidate, diagrams.to_vec()))
    })
}

#[tauri::command]
pub fn place_on_requirement_diagram(
    diagram_id: String,
    element_id: String,
    x: f64,
    y: f64,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    if !x.is_finite() || !y.is_finite() {
        return Err("Requirement placement coordinates must be finite".into());
    }
    let element_id = parse_element_id(&element_id)?;
    let project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?;
    let open_project = project.as_ref().ok_or("no project open")?;
    let element = open_project
        .element(element_id)
        .map_err(|error| error.to_string())?;
    let has_owned_content = open_project.children(element_id).next().is_some();
    let (width, height) = match element.kind {
        ElementKind::Requirement => (260.0, 180.0),
        ElementKind::TestCase => (220.0, 72.0),
        _ if has_owned_content => (220.0, 130.0),
        _ => (220.0, 58.0),
    };
    drop(project);
    checkpoint(&workspace, &activity, &history)?;
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|candidate| candidate.id == diagram_id && candidate.family == "requirement")
        .ok_or("Requirement Diagram not found")?;
    if diagram
        .nodes
        .iter()
        .any(|node| node.element_id == element_id.to_string())
    {
        return Err("element already has a presentation on this Requirement Diagram".into());
    }
    let id = uuid::Uuid::new_v4().to_string();
    diagram.nodes.push(DiagramNode {
        id: id.clone(),
        element_id: element_id.to_string(),
        x,
        y,
        width,
        height,
        actor_notation: None,
        parameter_presentations: Vec::new(),
    });
    Ok(id)
}

#[tauri::command]
pub fn reconnect_traceability_relationship(
    diagram_id: String,
    relationship_id: String,
    side: String,
    element_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    reconnect_traceability_in_state(
        diagram_id,
        relationship_id,
        side,
        element_id,
        &workspace,
        &activity,
        &history,
    )
}

fn reconnect_traceability_in_state(
    diagram_id: String,
    relationship_id: String,
    side: String,
    element_id: String,
    workspace: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
    history: &history::HistoryState,
) -> Result<(), String> {
    if side != "source" && side != "target" {
        return Err("relationship side must be source or target".into());
    }
    let relationship_id = parse_relationship_id(&relationship_id)?;
    let element_id = parse_element_id(&element_id)?;
    history::apply_structural_specification_with_views(
        workspace,
        activity,
        history,
        Some(&diagram_id),
        |project, diagrams, ibds| {
            let diagram = diagrams
                .iter()
                .find(|candidate| candidate.id == diagram_id && candidate.family == "requirement")
                .ok_or("Requirement Diagram not found")?;
            if !diagram
                .nodes
                .iter()
                .any(|node| node.element_id == element_id.to_string())
            {
                return Err(
                    "replacement endpoint must be presented on this Requirement Diagram".into(),
                );
            }
            if !diagram
                .edges
                .iter()
                .any(|edge| edge.relationship_id == relationship_id.to_string())
            {
                return Err("Requirement relationship presentation not found".into());
            }
            let original = project
                .relationship(relationship_id)
                .map_err(|error| error.to_string())?;
            let (source, target) = if side == "source" {
                (element_id, original.target_id)
            } else {
                (original.source_id, element_id)
            };
            Ok((
                project.stage_traceability_reconnect(relationship_id, source, target)?,
                ibds.to_vec(),
            ))
        },
    )
    .map(|_| ())
}

#[tauri::command]
pub fn create_traceability_relationship(
    diagram_id: String,
    relationship_kind: String,
    source_node_id: String,
    target_node_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    let kind = traceability_kind(&relationship_kind)?;
    checkpoint(&workspace, &activity, &history)?;
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|candidate| candidate.id == diagram_id && candidate.family == "requirement")
        .ok_or("Requirement Diagram not found")?;
    let source = diagram
        .nodes
        .iter()
        .find(|node| node.id == source_node_id)
        .ok_or("source node not found")?;
    let target = diagram
        .nodes
        .iter()
        .find(|node| node.id == target_node_id)
        .ok_or("target node not found")?;
    let source_element = parse_element_id(&source.element_id)?;
    let target_element = parse_element_id(&target.element_id)?;
    let owner_id = parse_element_id(&diagram.owner_id)?;
    let points = vec![
        DiagramPoint {
            x: source.x + source.width / 2.0,
            y: source.y + source.height,
        },
        DiagramPoint {
            x: target.x + target.width / 2.0,
            y: target.y,
        },
    ];
    let relationship_id = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .as_mut()
        .ok_or("no project open")?
        .create_relationship(kind, source_element, target_element, Some(owner_id))
        .map_err(|error| error.to_string())?;
    diagram.edges.push(DiagramEdge {
        id: uuid::Uuid::new_v4().to_string(),
        relationship_id: relationship_id.to_string(),
        source_node_id,
        target_node_id,
        points,
        label_anchor: None,
    });
    Ok(relationship_id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requirement_reconnect_is_atomic_across_copy_text_views_and_history() {
        let mut project = Project::new("Reconnect");
        let owner = project.root_id;
        let ids = ["Old", "Client", "Leaf", "New"]
            .map(|name| project.create_requirement(name, name, name, owner).unwrap());
        let relationship = project
            .create_relationship(RelationshipKind::Copy, ids[1], ids[0], Some(owner))
            .unwrap();
        project
            .create_relationship(RelationshipKind::Copy, ids[2], ids[1], Some(owner))
            .unwrap();
        let nodes: Vec<_> = ids
            .iter()
            .enumerate()
            .map(|(index, id)| DiagramNode {
                id: uuid::Uuid::new_v4().to_string(),
                element_id: id.to_string(),
                x: index as f64 * 250.0,
                y: index as f64 * 150.0,
                width: 100.0,
                height: 60.0,
                actor_notation: None,
                parameter_presentations: Vec::new(),
            })
            .collect();
        let edge = DiagramEdge {
            id: uuid::Uuid::new_v4().to_string(),
            relationship_id: relationship.to_string(),
            source_node_id: nodes[1].id.clone(),
            target_node_id: nodes[0].id.clone(),
            points: route_relationship(&nodes[1], &nodes[0], &nodes).unwrap(),
            label_anchor: None,
        };
        let selected = DiagramId::new().to_string();
        let diagram = BddDiagram {
            id: selected.clone(),
            name: "Requirements".into(),
            owner_id: owner.to_string(),
            family: "requirement".into(),
            semantic_context_id: None,
            subject_boundary: None,
            nodes,
            edges: vec![edge],
        };
        let mut second = diagram.clone();
        second.id = DiagramId::new().to_string();
        second.nodes.pop();
        for node in &mut second.nodes {
            node.id = uuid::Uuid::new_v4().to_string();
        }
        second.edges[0].id = uuid::Uuid::new_v4().to_string();
        second.edges[0].source_node_id = second.nodes[1].id.clone();
        second.edges[0].target_node_id = second.nodes[0].id.clone();
        let workspace = WorkspaceState::default();
        let activity = activity_workspace::ActivityWorkspaceState::default();
        let history = history::HistoryState::default();
        *workspace.project.lock().unwrap() = Some(project);
        *workspace.diagrams.lock().unwrap() = vec![diagram, second];
        let value = || {
            serde_json::to_value((
                &*workspace.project.lock().unwrap(),
                &*workspace.diagrams.lock().unwrap(),
            ))
            .unwrap()
        };
        let reconnect = |target: ElementId| {
            reconnect_traceability_in_state(
                selected.clone(),
                relationship.to_string(),
                "target".into(),
                target.to_string(),
                &workspace,
                &activity,
                &history,
            )
        };
        history::checkpoint_states(&workspace, &activity, &history).unwrap();
        workspace.project.lock().unwrap().as_mut().unwrap().name = "Redo".into();
        assert!(history::undo_states(&workspace, &activity, &history).unwrap());
        let before = value();
        let saved_edge = workspace.diagrams.lock().unwrap()[0].edges.pop().unwrap();
        assert!(reconnect(ids[3]).is_err());
        workspace.diagrams.lock().unwrap()[0].edges.push(saved_edge);
        assert_eq!(value(), before);
        assert_eq!(history::undo_len(&history), 0);
        assert!(history::redo_states(&workspace, &activity, &history).unwrap());
        assert!(history::undo_states(&workspace, &activity, &history).unwrap());
        assert_eq!(value(), before);
        reconnect(ids[3]).unwrap();
        let after = value();
        reconnect(ids[3]).unwrap();
        assert_eq!(history::undo_len(&history), 1);
        {
            let guard = workspace.project.lock().unwrap();
            let project = guard.as_ref().unwrap();
            for id in [ids[1], ids[2]] {
                assert_eq!(
                    project.element(id).unwrap().requirement_text.as_deref(),
                    Some("New")
                );
            }
            let diagrams = workspace.diagrams.lock().unwrap();
            validate_loaded_diagrams(project, &diagrams).unwrap();
            for diagram in diagrams.iter() {
                let edge = &diagram.edges[0];
                let target = diagram
                    .nodes
                    .iter()
                    .find(|node| node.id == edge.target_node_id)
                    .unwrap();
                assert_eq!(target.element_id, ids[3].to_string());
                assert!(edge.label_anchor.is_some());
            }
            let directory = tempfile::tempdir().unwrap();
            let mut database = ProjectDatabase::open(directory.path().join("copy.smproj")).unwrap();
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
        assert!(history::redo_states(&workspace, &activity, &history).unwrap());
        assert_eq!(value(), after);
    }

    #[test]
    fn requirement_edit_is_one_transaction_with_transitive_copy_undo_and_redo() {
        let mut project = Project::new("Requirement transaction");
        let owner = project.root_id;
        let master = project
            .create_requirement("Master", "R1", "Old", owner)
            .unwrap();
        let copy = project
            .create_requirement("Copy", "R2", "Old", owner)
            .unwrap();
        let leaf = project
            .create_requirement("Leaf", "R3", "Old", owner)
            .unwrap();
        project
            .create_relationship(RelationshipKind::Copy, copy, master, Some(owner))
            .unwrap();
        project
            .create_relationship(RelationshipKind::Copy, leaf, copy, Some(owner))
            .unwrap();
        let workspace = WorkspaceState::default();
        let activity = activity_workspace::ActivityWorkspaceState::default();
        let history = history::HistoryState::default();
        *workspace.project.lock().unwrap() = Some(project);
        let before = serde_json::to_value(&*workspace.project.lock().unwrap()).unwrap();
        let diagrams_before = serde_json::to_value(&*workspace.diagrams.lock().unwrap()).unwrap();
        let mut details = RequirementUpdateRequest {
            element_id: master.to_string(),
            name: "Revised master".into(),
            requirement_id: "R1-REV".into(),
            text: "Revised text".into(),
            documentation: "Rationale".into(),
        };
        let apply = |details: &RequirementUpdateRequest| {
            apply_requirement_update(&workspace, &activity, &history, details)
        };
        assert!(apply(&details).unwrap());
        assert!(!apply(&details).unwrap());
        assert_eq!(history::undo_len(&history), 1);
        {
            let guard = workspace.project.lock().unwrap();
            let project = guard.as_ref().unwrap();
            for id in [master, copy, leaf] {
                assert_eq!(
                    project.element(id).unwrap().requirement_text.as_deref(),
                    Some("Revised text")
                );
            }
            let master = project.element(master).unwrap();
            assert_eq!(master.name, "Revised master");
            assert_eq!(master.requirement_id.as_deref(), Some("R1-REV"));
            assert_eq!(master.documentation, "Rationale");
        }
        let after = serde_json::to_value(&*workspace.project.lock().unwrap()).unwrap();
        assert!(history::undo_states(&workspace, &activity, &history).unwrap());
        assert_eq!(
            serde_json::to_value(&*workspace.project.lock().unwrap()).unwrap(),
            before
        );
        // Both duplicate identity and read-only Copy rejection preserve pending redo.
        details.requirement_id = "R2".into();
        assert!(apply(&details).is_err());
        details.element_id = copy.to_string();
        assert!(apply(&details).is_err());
        assert_eq!(history::undo_len(&history), 0);
        assert_eq!(
            serde_json::to_value(&*workspace.project.lock().unwrap()).unwrap(),
            before
        );
        assert!(history::redo_states(&workspace, &activity, &history).unwrap());
        assert_eq!(
            serde_json::to_value(&*workspace.project.lock().unwrap()).unwrap(),
            after
        );
        assert_eq!(
            serde_json::to_value(&*workspace.diagrams.lock().unwrap()).unwrap(),
            diagrams_before
        );
    }

    #[test]
    fn invalid_requirement_request_does_not_checkpoint_an_empty_workspace() {
        let workspace = WorkspaceState::default();
        let activity = activity_workspace::ActivityWorkspaceState::default();
        let history = history::HistoryState::default();
        let mut details = RequirementUpdateRequest {
            element_id: "invalid".into(),
            name: "Name".into(),
            requirement_id: "R1".into(),
            text: "Text".into(),
            documentation: String::new(),
        };
        assert!(apply_requirement_update(&workspace, &activity, &history, &details).is_err());
        details.element_id = systems_modeler_core::ElementId::new().to_string();
        assert!(apply_requirement_update(&workspace, &activity, &history, &details).is_err());
        assert_eq!(history::undo_len(&history), 0);
        assert!(workspace.project.lock().unwrap().is_none());
    }
}
