//! Present a classifier-owned part without creating another usage or Block.
use super::{
    DiagramEdge, DiagramNode, WorkspaceState, activity_workspace, history, parse_diagram_id,
    parse_element_id, route_relationship_at_lane,
};
use systems_modeler_core::{ElementId, ElementKind};

#[tauri::command]
pub fn present_part_composition(
    diagram_id: String,
    property_id: String,
    state: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    present_part_composition_in_state(
        &parse_diagram_id(&diagram_id)?.to_string(),
        parse_element_id(&property_id)?,
        &state,
        &activity,
        &history,
    )
}

fn present_part_composition_in_state(
    diagram_id: &str,
    property_id: ElementId,
    state: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
    history: &history::HistoryState,
) -> Result<String, String> {
    let mut result = None;
    history::edit_authored_if_changed(state, activity, history, |candidate| {
        let project = candidate.project.as_mut().ok_or("no project open")?;
        let property = project.element(property_id).map_err(|error| error.to_string())?;
        if property.kind != ElementKind::PartProperty {
            return Err("Select a PartProperty to show its composition.".into());
        }
        let owner = property.owner_id.ok_or("part has no owning Block")?;
        let type_id = property.type_id.ok_or("part has no type")?;
        if owner == type_id {
            return Err("Self-typed parts are not yet supported by BDD composition routing.".into());
        }
        let diagram = candidate.diagrams.iter_mut()
            .find(|diagram| diagram.id == diagram_id && diagram.family == "bdd")
            .ok_or("Select a Block Definition Diagram to show the composition.")?;
        let matches: Vec<_> = project.relationships.values()
            .filter(|relationship| relationship.association_ends.iter()
                .any(|end| end.property_id == Some(property_id)))
            .map(|relationship| relationship.id).collect();
        let relationship_id = match matches.as_slice() {
            [] => project.create_property_association(property_id, Some(parse_element_id(&diagram.owner_id)?))
                .map_err(|error| error.to_string())?,
            [id] => *id,
            _ => return Err("Part is linked to multiple associations; resolve the duplicate links first.".into()),
        };
        result = Some(relationship_id.to_string());
        if diagram.edges.iter().any(|edge| edge.relationship_id == relationship_id.to_string()) {
            return Ok(false);
        }
        let mut endpoints = Vec::new();
        for element_id in [owner, type_id] {
            if let Some(node) = diagram.nodes.iter().find(|node| node.element_id == element_id.to_string()) {
                endpoints.push(node.id.clone());
            } else {
                let node = DiagramNode {
                    id: uuid::Uuid::new_v4().to_string(),
                    element_id: element_id.to_string(),
                    x: diagram.nodes.iter().map(|node| node.x + node.width).fold(20.0, f64::max) + 80.0,
                    y: 100.0,
                    width: 200.0,
                    height: 120.0,
                    actor_notation: None,
                    parameter_presentations: Vec::new(),
                };
                endpoints.push(node.id.clone());
                diagram.nodes.push(node);
            }
        }
        let source = diagram.nodes.iter().find(|node| node.id == endpoints[0]).unwrap();
        let target = diagram.nodes.iter().find(|node| node.id == endpoints[1]).unwrap();
        let lane = diagram.edges.iter().filter(|edge| edge.source_node_id == source.id && edge.target_node_id == target.id).count();
        let points = route_relationship_at_lane(source, target, &diagram.nodes, lane)?;
        diagram.edges.push(DiagramEdge {
            id: uuid::Uuid::new_v4().to_string(),
            relationship_id: relationship_id.to_string(),
            source_node_id: source.id.clone(),
            target_node_id: target.id.clone(),
            label_anchor: Some(super::routing::route_label_anchor(&points)),
            points,
        });
        project.validate().map_err(|error| error.to_string())?;
        super::validate_loaded_diagrams(project, &candidate.diagrams)?;
        super::ibd::validate_ibd_diagrams(project, &candidate.ibd_diagrams)?;
        super::behavior_workspace::validate_behavior_workspace(project, &candidate.behavior, &candidate.behavior_diagrams)?;
        candidate.activity_repository.validate(project).map_err(|error| error.to_string())?;
        Ok(true)
    })?;
    result.ok_or_else(|| "composition presentation produced no relationship".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::BddDiagram;
    use systems_modeler_core::{Multiplicity, Project};

    struct Fixture {
        state: WorkspaceState,
        activity: activity_workspace::ActivityWorkspaceState,
        history: history::HistoryState,
        view: String,
        parts: [ElementId; 2],
        block: ElementId,
    }

    fn fixture() -> Fixture {
        let state = WorkspaceState::default();
        let mut project = Project::new("Reuse");
        let owner = project.create_element(ElementKind::Block, "Station", project.root_id).unwrap();
        let block = project.create_element(ElementKind::Block, "Transmitter", project.root_id).unwrap();
        let parts = ["primary", "backup"].map(|name| project.create_typed_feature(
            ElementKind::PartProperty, name, owner, block, Multiplicity::ONE,
        ).unwrap());
        let view = uuid::Uuid::new_v4().to_string();
        state.diagrams.lock().unwrap().push(BddDiagram {
            id: view.clone(), name: "Decomposition".into(), owner_id: project.root_id.to_string(),
            family: "bdd".into(), semantic_context_id: None, subject_boundary: None,
            nodes: Vec::new(), edges: Vec::new(),
        });
        *state.project.lock().unwrap() = Some(project);
        Fixture { state, activity: Default::default(), history: Default::default(), view, parts, block }
    }

    fn show(f: &Fixture, part: ElementId) -> Result<String, String> {
        present_part_composition_in_state(&f.view, part, &f.state, &f.activity, &f.history)
    }

    fn snapshot(f: &Fixture) -> serde_json::Value {
        serde_json::to_value((&*f.state.project.lock().unwrap(), &*f.state.diagrams.lock().unwrap())).unwrap()
    }

    #[test]
    fn existing_parts_reuse_types_and_relationships_across_views_without_duplicates() {
        let f = fixture();
        let elements = f.state.project.lock().unwrap().as_ref().unwrap().elements.len();
        let first = show(&f, f.parts[0]).unwrap();
        let after_first = snapshot(&f);
        assert_eq!(show(&f, f.parts[0]).unwrap(), first);
        assert_eq!(snapshot(&f), after_first);
        assert_eq!(history::undo_len(&f.history), 1);
        let second = show(&f, f.parts[1]).unwrap();
        assert_ne!(first, second);
        {
            let guard = f.state.project.lock().unwrap();
            let project = guard.as_ref().unwrap();
            assert_eq!(project.elements.len(), elements);
            assert_eq!(project.relationships.len(), 2);
            for (part, id) in f.parts.into_iter().zip([&first, &second]) {
                assert_eq!(project.element(part).unwrap().type_id, Some(f.block));
                let relationship = project.relationships.values().find(|r| r.id.to_string() == *id).unwrap();
                assert!(relationship.association_ends.iter().any(|end| end.property_id == Some(part)));
            }
            let directory = tempfile::tempdir().unwrap();
            let mut db = systems_modeler_persistence::ProjectDatabase::open(directory.path().join("reuse.smproj")).unwrap();
            db.save_project(project).unwrap();
            assert_eq!(serde_json::to_value(db.load_first_project().unwrap()).unwrap(), serde_json::to_value(project).unwrap());
        }
        let new_view = {
            let mut diagrams = f.state.diagrams.lock().unwrap();
            assert_eq!(diagrams[0].nodes.len(), 2);
            assert_eq!(diagrams[0].edges.len(), 2);
            let mut other = diagrams[0].clone();
            other.id = uuid::Uuid::new_v4().to_string();
            other.nodes.clear();
            other.edges.clear();
            let id = other.id.clone();
            diagrams.push(other);
            id
        };
        assert_eq!(present_part_composition_in_state(&new_view, f.parts[0], &f.state, &f.activity, &f.history).unwrap(), first);
        assert_eq!(f.state.project.lock().unwrap().as_ref().unwrap().relationships.len(), 2);
        assert_eq!(f.state.diagrams.lock().unwrap()[1].edges.len(), 1);
    }

    #[test]
    fn part_presentation_history_is_atomic_and_rejection_preserves_redo() {
        let f = fixture();
        let before = snapshot(&f);
        show(&f, f.parts[0]).unwrap();
        let after = snapshot(&f);
        assert!(history::undo_states(&f.state, &f.activity, &f.history).unwrap());
        assert_eq!(snapshot(&f), before);
        assert!(show(&f, f.block).is_err());
        assert!(present_part_composition_in_state("missing-view", f.parts[0], &f.state, &f.activity, &f.history).is_err());
        assert_eq!(snapshot(&f), before);
        assert_eq!(history::undo_len(&f.history), 0);
        assert!(history::redo_states(&f.state, &f.activity, &f.history).unwrap());
        assert_eq!(snapshot(&f), after);
        show(&f, f.parts[1]).unwrap();
        assert!(history::undo_states(&f.state, &f.activity, &f.history).unwrap());
        show(&f, f.parts[0]).unwrap(); // already visible: do not consume pending redo
        assert!(history::redo_states(&f.state, &f.activity, &f.history).unwrap());
        assert_eq!(f.state.project.lock().unwrap().as_ref().unwrap().relationships.len(), 2);
    }

    #[test]
    fn part_presentation_routing_and_lock_failures_publish_nothing() {
        let f = fixture();
        show(&f, f.parts[0]).unwrap();
        f.state.diagrams.lock().unwrap()[0].nodes[0].width = -1.0;
        let before = snapshot(&f);
        assert!(show(&f, f.parts[1]).is_err());
        assert_eq!(snapshot(&f), before);
        assert_eq!(history::undo_len(&f.history), 1);
        let _busy = f.activity.repository.lock().unwrap();
        assert!(show(&f, f.parts[1]).is_err());
        assert_eq!(snapshot(&f), before);
    }
}
