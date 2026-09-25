use super::{
    WorkspaceState, parse_diagram_id, parse_element_id, parse_relationship_id,
    relationship_display_kind,
};
use systems_modeler_core::{
    AggregationKind, ElementId, ElementKind, Multiplicity, RelationshipKind,
};

fn parse_multiplicity(value: &str) -> Result<Multiplicity, String> {
    let trimmed = value.trim();
    if trimmed == "*" {
        return Multiplicity::new(0, None).map_err(|error| error.to_string());
    }
    if let Some((lower, upper)) = trimmed.split_once("..") {
        let lower = lower
            .trim()
            .parse::<u32>()
            .map_err(|_| "invalid multiplicity lower bound")?;
        let upper = match upper.trim() {
            "*" => None,
            value => Some(
                value
                    .parse::<u32>()
                    .map_err(|_| "invalid multiplicity upper bound")?,
            ),
        };
        return Multiplicity::new(lower, upper).map_err(|error| error.to_string());
    }
    let exact = trimmed
        .parse::<u32>()
        .map_err(|_| "multiplicity must be N, N..M, N..*, or *")?;
    Multiplicity::new(exact, Some(exact)).map_err(|error| error.to_string())
}

fn parse_aggregation(value: &str) -> Result<AggregationKind, String> {
    match value {
        "none" => Ok(AggregationKind::None),
        "shared" => Ok(AggregationKind::Shared),
        "composite" => Ok(AggregationKind::Composite),
        _ => Err("aggregation must be none, shared, or composite".into()),
    }
}

fn duplicate_after_reconnect(
    project: &systems_modeler_core::Project,
    relationship_id: systems_modeler_core::RelationshipId,
    kind: &str,
    source_id: ElementId,
    target_id: ElementId,
) -> bool {
    project.relationships.values().any(|relationship| {
        relationship.id != relationship_id
            && relationship.source_id == source_id
            && relationship.target_id == target_id
            && relationship_display_kind(relationship) == kind
    })
}

#[tauri::command]
pub fn composition_property_choices(
    relationship_id: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<Vec<systems_modeler_core::ElementTypeChoice>, String> {
    let project = state.project.lock().map_err(|_| "project lock poisoned")?;
    project
        .as_ref()
        .ok_or("no project open")?
        .composition_property_choices(parse_relationship_id(&relationship_id)?)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn link_composition_property(
    relationship_id: String,
    property_id: Option<String>,
    state: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, super::activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, super::history::HistoryState>,
) -> Result<String, String> {
    link_composition_property_in_state(
        parse_relationship_id(&relationship_id)?,
        property_id.as_deref().map(parse_element_id).transpose()?,
        &state,
        &activity,
        &history,
    )
}

fn link_composition_property_in_state(
    relationship_id: systems_modeler_core::RelationshipId,
    property_id: Option<ElementId>,
    state: &WorkspaceState,
    activity: &super::activity_workspace::ActivityWorkspaceState,
    history: &super::history::HistoryState,
) -> Result<String, String> {
    let mut linked = None;
    super::history::apply_structural_specification(
        state,
        activity,
        history,
        |current, diagrams| {
            let mut candidate = current.clone();
            linked = Some(
                candidate
                    .link_composition_property(relationship_id, property_id)
                    .map_err(|error| error.to_string())?,
            );
            Ok((candidate, diagrams.to_vec()))
        },
    )?;
    linked
        .map(|id| id.to_string())
        .ok_or_else(|| "composition link produced no Property".into())
}

// Preserve the existing flat IPC arguments; the extra parameters are native state handles.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn update_association_end(
    relationship_id: String,
    end_id: String,
    role_name: String,
    multiplicity: String,
    navigable: bool,
    aggregation: String,
    state: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, super::activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, super::history::HistoryState>,
) -> Result<(), String> {
    update_association_end_in_state(
        AssociationEndEdit {
            relationship_id: parse_relationship_id(&relationship_id)?,
            end_id,
            role_name,
            multiplicity: parse_multiplicity(&multiplicity)?,
            navigable,
            aggregation: parse_aggregation(&aggregation)?,
        },
        &state,
        &activity,
        &history,
    )
}

struct AssociationEndEdit {
    relationship_id: systems_modeler_core::RelationshipId,
    end_id: String,
    role_name: String,
    multiplicity: Multiplicity,
    navigable: bool,
    aggregation: AggregationKind,
}

fn update_association_end_in_state(
    edit: AssociationEndEdit,
    state: &WorkspaceState,
    activity: &super::activity_workspace::ActivityWorkspaceState,
    history: &super::history::HistoryState,
) -> Result<(), String> {
    super::history::apply_structural_specification(state, activity, history, |current, diagrams| {
        let mut candidate = current.clone();
        edit_association_end(
            &mut candidate,
            edit.relationship_id,
            &edit.end_id,
            &edit.role_name,
            edit.multiplicity,
            edit.navigable,
            edit.aggregation,
        )?;
        Ok((candidate, diagrams.to_vec()))
    })
    .map(|_| ())
}

fn edit_association_end(
    project: &mut systems_modeler_core::Project,
    relationship_id: systems_modeler_core::RelationshipId,
    end_id: &str,
    role_name: &str,
    multiplicity: Multiplicity,
    navigable: bool,
    aggregation: AggregationKind,
) -> Result<(), String> {
    let original = project
        .relationship(relationship_id)
        .map_err(|error| error.to_string())?;
    if original.kind != RelationshipKind::Association || original.association_ends.len() != 2 {
        return Err(
            "association-end editing requires a binary Association-family relationship".into(),
        );
    }
    let end_index = original
        .association_ends
        .iter()
        .position(|end| end.id.to_string() == end_id)
        .ok_or("association end not found")?;
    let property_id = original.association_ends[end_index].property_id;
    let mut candidate = project.clone();
    if let Some(property_id) = property_id {
        if !navigable {
            return Err("a classifier-owned association Property is navigable".into());
        }
        if role_name.trim().is_empty() {
            return Err("the linked Property name cannot be empty".into());
        }
        candidate
            .element_mut(property_id)
            .map_err(|error| error.to_string())?
            .kind = if aggregation == AggregationKind::Composite {
            ElementKind::PartProperty
        } else {
            ElementKind::ReferenceProperty
        };
        candidate
            .rename_element(property_id, role_name.trim())
            .map_err(|error| error.to_string())?;
        candidate
            .set_multiplicity(property_id, multiplicity)
            .map_err(|error| error.to_string())?;
        candidate
            .set_aggregation(property_id, aggregation)
            .map_err(|error| error.to_string())?;
    } else {
        let end = &mut candidate
            .relationships
            .get_mut(&relationship_id)
            .ok_or("relationship not found")?
            .association_ends[end_index];
        end.role_name = role_name.trim().to_string();
        end.multiplicity = multiplicity;
        end.navigable = navigable;
        end.aggregation = aggregation;
    }
    candidate.validate().map_err(|error| error.to_string())?;
    *project = candidate;
    Ok(())
}

#[tauri::command]
pub fn reconnect_bdd_relationship(
    diagram_id: String,
    relationship_id: String,
    side: String,
    element_id: String,
    state: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, super::activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, super::history::HistoryState>,
) -> Result<(), String> {
    reconnect_bdd_relationship_in_state(
        diagram_id,
        relationship_id,
        side,
        element_id,
        &state,
        &activity,
        &history,
    )
}

fn reconnect_bdd_relationship_in_state(
    diagram_id: String,
    relationship_id: String,
    side: String,
    element_id: String,
    state: &WorkspaceState,
    activity: &super::activity_workspace::ActivityWorkspaceState,
    history: &super::history::HistoryState,
) -> Result<(), String> {
    let diagram_id = parse_diagram_id(&diagram_id)?;
    let relationship_id = parse_relationship_id(&relationship_id)?;
    let element_id = parse_element_id(&element_id)?;
    if side != "source" && side != "target" {
        return Err("relationship side must be source or target".into());
    }

    let diagram_key = diagram_id.to_string();
    super::history::apply_structural_specification_with_views(
        state,
        activity,
        history,
        Some(&diagram_key),
        |project, diagrams, ibds| {
            let replacement = project
                .element(element_id)
                .map_err(|error| error.to_string())?;
            if !replacement.is_classifier() {
                return Err("BDD relationship endpoints must be classifiers".into());
            }

            let original = project
                .relationship(relationship_id)
                .map_err(|error| error.to_string())?
                .clone();
            let mut new_source = original.source_id;
            let mut new_target = original.target_id;
            if side == "source" {
                new_source = element_id;
            } else {
                new_target = element_id;
            }
            if new_source == new_target {
                return Err("a BDD relationship cannot connect a Block to itself".into());
            }
            let display_kind = relationship_display_kind(&original);
            if !original
                .association_ends
                .iter()
                .any(|end| end.property_id.is_some())
                && duplicate_after_reconnect(
                    project,
                    relationship_id,
                    display_kind,
                    new_source,
                    new_target,
                )
            {
                return Err(format!("an equivalent {display_kind} already exists"));
            }
            // Prepare the complete semantic and affected-view change before publication.
            let mut candidate_project = project.clone();
            if let Some((index, property_id)) = original
                .association_ends
                .iter()
                .enumerate()
                .find_map(|(index, end)| end.property_id.map(|id| (index, id)))
            {
                let (owner_id, type_id) = if index == 1 {
                    (new_source, new_target)
                } else {
                    (new_target, new_source)
                };
                candidate_project
                    .move_element(property_id, owner_id)
                    .map_err(|error| error.to_string())?;
                candidate_project
                    .set_element_type(property_id, type_id)
                    .map_err(|error| error.to_string())?;
            } else {
                let candidate = candidate_project
                    .relationships
                    .get_mut(&relationship_id)
                    .ok_or("relationship not found")?;
                candidate.source_id = new_source;
                candidate.target_id = new_target;
                if candidate.kind == RelationshipKind::Association
                    && candidate.association_ends.len() == 2
                {
                    candidate.association_ends[0].classifier_id = new_source;
                    candidate.association_ends[1].classifier_id = new_target;
                }
            }
            candidate_project
                .validate()
                .map_err(|error| error.to_string())?;
            let selected = diagrams
                .iter()
                .find(|diagram| diagram.id == diagram_id.to_string())
                .ok_or("diagram not found")?;
            if !selected
                .edges
                .iter()
                .any(|edge| edge.relationship_id == relationship_id.to_string())
            {
                return Err("diagram edge not found".into());
            }
            Ok((candidate_project, ibds.to_vec()))
        },
    )
    .map(|_| ())
}

/// Preserve all existing presentations while staging endpoint changes. An absent
/// endpoint in another view gets its own presentation, never a semantic duplicate.
pub(super) fn stage_relationship_presentations(
    previous: &systems_modeler_core::Project,
    candidate: &systems_modeler_core::Project,
    diagrams: &[super::BddDiagram],
    required_diagram: Option<&str>,
) -> Result<Vec<super::BddDiagram>, String> {
    let changed: std::collections::HashMap<_, _> = candidate
        .relationships
        .values()
        .filter(|relationship| {
            previous
                .relationships
                .get(&relationship.id)
                .is_some_and(|old| {
                    old.source_id != relationship.source_id
                        || old.target_id != relationship.target_id
                })
        })
        .map(|relationship| (relationship.id.to_string(), relationship))
        .collect();
    let removed: std::collections::HashSet<_> = previous
        .relationships
        .keys()
        .filter(|id| !candidate.relationships.contains_key(id))
        .map(ToString::to_string)
        .collect();
    let mut staged = diagrams.to_vec();
    for diagram in &mut staged {
        diagram
            .edges
            .retain(|edge| !removed.contains(&edge.relationship_id));
        for edge_index in 0..diagram.edges.len() {
            let edge = &diagram.edges[edge_index];
            let Some(relationship) = changed.get(&edge.relationship_id) else {
                continue;
            };
            let mut endpoint_ids = Vec::new();
            for (element_id, old_node_id) in [
                (relationship.source_id, edge.source_node_id.clone()),
                (relationship.target_id, edge.target_node_id.clone()),
            ] {
                if let Some(node) = diagram
                    .nodes
                    .iter()
                    .find(|node| node.element_id == element_id.to_string())
                {
                    endpoint_ids.push(node.id.clone());
                    continue;
                }
                if required_diagram == Some(diagram.id.as_str()) {
                    return Err("new classifier must be presented on the selected BDD".into());
                }
                let mut node = diagram
                    .nodes
                    .iter()
                    .find(|node| node.id == old_node_id)
                    .cloned()
                    .ok_or("existing relationship endpoint presentation is missing")?;
                node.id = uuid::Uuid::new_v4().to_string();
                node.element_id = element_id.to_string();
                node.x = diagram
                    .nodes
                    .iter()
                    .map(|node| node.x + node.width)
                    .fold(0.0, f64::max)
                    + 60.0;
                node.actor_notation = None;
                node.parameter_presentations.clear();
                endpoint_ids.push(node.id.clone());
                diagram.nodes.push(node);
            }
            let source = diagram
                .nodes
                .iter()
                .find(|node| node.id == endpoint_ids[0])
                .ok_or("source presentation missing")?;
            let target = diagram
                .nodes
                .iter()
                .find(|node| node.id == endpoint_ids[1])
                .ok_or("target presentation missing")?;
            let lane = diagram.edges[..edge_index]
                .iter()
                .filter(|edge| edge.source_node_id == source.id && edge.target_node_id == target.id)
                .count();
            let points = super::route_relationship_at_lane(source, target, &diagram.nodes, lane)?;
            let edge = &mut diagram.edges[edge_index];
            edge.source_node_id = source.id.clone();
            edge.target_node_id = target.id.clone();
            edge.label_anchor = Some(super::routing::route_label_anchor(&points));
            edge.points = points;
        }
    }
    Ok(staged)
}

#[tauri::command]
pub fn delete_bdd_relationship(
    diagram_id: String,
    relationship_id: String,
    state: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, super::activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, super::history::HistoryState>,
) -> Result<(), String> {
    delete_bdd_relationship_in_state(diagram_id, relationship_id, &state, &activity, &history)
}

fn delete_bdd_relationship_in_state(
    diagram_id: String,
    relationship_id: String,
    state: &WorkspaceState,
    activity: &super::activity_workspace::ActivityWorkspaceState,
    history: &super::history::HistoryState,
) -> Result<(), String> {
    let diagram_id = parse_diagram_id(&diagram_id)?.to_string();
    let relationship_id = parse_relationship_id(&relationship_id)?;
    super::history::apply_structural_specification_with_views(
        state,
        activity,
        history,
        None,
        |project, diagrams, ibds| {
            let selected = diagrams
                .iter()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("diagram not found")?;
            if !selected
                .edges
                .iter()
                .any(|edge| edge.relationship_id == relationship_id.to_string())
            {
                return Err("relationship is not presented on the selected diagram".into());
            }
            let mut candidate = project.clone();
            if candidate.relationships.remove(&relationship_id).is_none() {
                return Err("relationship not found".into());
            }
            candidate.validate().map_err(|error| error.to_string())?;
            Ok((candidate, ibds.to_vec()))
        },
    )
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{BddDiagram, DiagramEdge, DiagramNode, route_relationship};
    use systems_modeler_core::{DiagramId, Project, RelationshipId};

    struct ReconnectFixture {
        state: WorkspaceState,
        activity: super::super::activity_workspace::ActivityWorkspaceState,
        history: super::super::history::HistoryState,
        diagram_id: String,
        relationship_id: RelationshipId,
        blocks: [ElementId; 3],
    }

    fn reconnect_fixture(kind: RelationshipKind) -> ReconnectFixture {
        let state = WorkspaceState::default();
        let mut project = Project::new("Reconnect rollback");
        let blocks = ["A", "B", "C"].map(|name| {
            project
                .create_element(ElementKind::Block, name, project.root_id)
                .unwrap()
        });
        let relationship_id = if kind == RelationshipKind::Association {
            project
                .create_association(
                    Some(project.root_id),
                    blocks[..2]
                        .iter()
                        .map(|id| {
                            Project::association_end(
                                *id,
                                "role",
                                Multiplicity::ONE,
                                true,
                                AggregationKind::None,
                            )
                        })
                        .collect(),
                )
                .unwrap()
        } else {
            project
                .create_relationship(kind, blocks[0], blocks[1], Some(project.root_id))
                .unwrap()
        };
        let nodes: Vec<_> = blocks
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
            relationship_id: relationship_id.to_string(),
            source_node_id: nodes[0].id.clone(),
            target_node_id: nodes[1].id.clone(),
            points: route_relationship(&nodes[0], &nodes[1], &nodes).unwrap(),
            label_anchor: None,
        };
        let diagram_id = DiagramId::new().to_string();
        state.diagrams.lock().unwrap().push(BddDiagram {
            id: diagram_id.clone(),
            name: "Structure".into(),
            owner_id: project.root_id.to_string(),
            family: "bdd".into(),
            semantic_context_id: None,
            subject_boundary: None,
            nodes,
            edges: vec![edge],
        });
        project.validate().unwrap();
        *state.project.lock().unwrap() = Some(project);
        ReconnectFixture {
            state,
            activity: Default::default(),
            history: Default::default(),
            diagram_id,
            relationship_id,
            blocks,
        }
    }

    fn snapshot(state: &WorkspaceState) -> (serde_json::Value, serde_json::Value) {
        (
            serde_json::to_value(&*state.project.lock().unwrap()).unwrap(),
            serde_json::to_value(&*state.diagrams.lock().unwrap()).unwrap(),
        )
    }

    fn reconnect(fixture: &ReconnectFixture, side: &str) -> Result<(), String> {
        reconnect_bdd_relationship_in_state(
            fixture.diagram_id.clone(),
            fixture.relationship_id.to_string(),
            side.into(),
            fixture.blocks[2].to_string(),
            &fixture.state,
            &fixture.activity,
            &fixture.history,
        )
    }

    #[test]
    fn rejected_reconnect_preserves_redo_and_all_authored_state() {
        let fixture = reconnect_fixture(RelationshipKind::Association);
        super::super::history::checkpoint_states(
            &fixture.state,
            &fixture.activity,
            &fixture.history,
        )
        .unwrap();
        fixture.state.project.lock().unwrap().as_mut().unwrap().name = "Redo revision".into();
        assert!(
            super::super::history::undo_states(
                &fixture.state,
                &fixture.activity,
                &fixture.history,
            )
            .unwrap()
        );
        let before = snapshot(&fixture.state);
        // The selected diagram is valid, but this would make a self-association.
        assert!(
            reconnect_bdd_relationship_in_state(
                fixture.diagram_id.clone(),
                fixture.relationship_id.to_string(),
                "target".into(),
                fixture.blocks[0].to_string(),
                &fixture.state,
                &fixture.activity,
                &fixture.history,
            )
            .is_err()
        );
        assert_eq!(snapshot(&fixture.state), before);
        assert_eq!(super::super::history::undo_len(&fixture.history), 0);
        assert!(
            super::super::history::redo_states(
                &fixture.state,
                &fixture.activity,
                &fixture.history,
            )
            .unwrap()
        );
        assert_eq!(
            fixture.state.project.lock().unwrap().as_ref().unwrap().name,
            "Redo revision"
        );
    }

    #[test]
    fn reconnect_history_covers_every_view_and_skips_noops() {
        let fixture = reconnect_fixture(RelationshipKind::Association);
        add_second_view(&fixture, false);
        let before = snapshot(&fixture.state);
        reconnect(&fixture, "target").unwrap();
        let after = snapshot(&fixture.state);
        assert_ne!(before, after);
        assert_eq!(super::super::history::undo_len(&fixture.history), 1);
        reconnect(&fixture, "target").unwrap();
        assert_eq!(super::super::history::undo_len(&fixture.history), 1);
        assert!(
            super::super::history::undo_states(
                &fixture.state,
                &fixture.activity,
                &fixture.history,
            )
            .unwrap()
        );
        assert_eq!(snapshot(&fixture.state), before);
        assert!(
            super::super::history::redo_states(
                &fixture.state,
                &fixture.activity,
                &fixture.history,
            )
            .unwrap()
        );
        assert_eq!(snapshot(&fixture.state), after);
    }

    #[test]
    fn reconnect_presentation_failures_preserve_semantics_and_diagrams() {
        for failure in 0..4 {
            let fixture = reconnect_fixture(RelationshipKind::Association);
            {
                let mut diagrams = fixture.state.diagrams.lock().unwrap();
                match failure {
                    0 => diagrams.clear(),
                    1 => {
                        diagrams[0].nodes.pop();
                    }
                    2 => diagrams[0].edges.clear(),
                    _ => {
                        diagrams[0].nodes[2].x = f64::NAN;
                        diagrams[0].nodes[2].y = f64::NAN;
                    }
                }
            }
            let before = snapshot(&fixture.state);
            assert!(reconnect(&fixture, "target").is_err(), "case {failure}");
            assert_eq!(snapshot(&fixture.state), before, "case {failure}");
        }
    }

    #[test]
    fn reconnect_poisoned_diagram_lock_preserves_project() {
        let fixture = reconnect_fixture(RelationshipKind::Association);
        let before = snapshot(&fixture.state).0;
        let state = &fixture.state;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = state.diagrams.lock().unwrap();
            panic!("inject a poisoned presentation lock");
        }));
        assert!(result.is_err());
        assert_eq!(
            reconnect(&fixture, "target").unwrap_err(),
            "diagram lock poisoned"
        );
        assert_eq!(
            serde_json::to_value(&*state.project.lock().unwrap()).unwrap(),
            before
        );
    }

    #[test]
    fn reconnect_generalization_cycle_rolls_back_semantics_and_routing() {
        let fixture = reconnect_fixture(RelationshipKind::Generalization);
        {
            let mut guard = fixture.state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            project
                .create_relationship(
                    RelationshipKind::Generalization,
                    fixture.blocks[2],
                    fixture.blocks[0],
                    Some(project.root_id),
                )
                .unwrap();
        }
        let before = snapshot(&fixture.state);
        assert!(
            reconnect(&fixture, "target")
                .unwrap_err()
                .contains("inheritance cycle")
        );
        assert_eq!(snapshot(&fixture.state), before);
    }

    #[test]
    fn reconnect_commits_both_sides_without_replacing_relationship_identity() {
        for side in ["source", "target"] {
            let fixture = reconnect_fixture(RelationshipKind::Association);
            let original = fixture
                .state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationship(fixture.relationship_id)
                .unwrap()
                .clone();
            let old_edge = fixture.state.diagrams.lock().unwrap()[0].edges[0].clone();
            reconnect(&fixture, side).unwrap();
            let project_guard = fixture.state.project.lock().unwrap();
            let project = project_guard.as_ref().unwrap();
            project.validate().unwrap();
            let relationship = project.relationship(fixture.relationship_id).unwrap();
            assert_eq!(relationship.id, original.id);
            assert_eq!(relationship.external_id, original.external_id);
            assert_eq!(relationship.owner_id, original.owner_id);
            let (source, target) = if side == "source" {
                (fixture.blocks[2], fixture.blocks[1])
            } else {
                (fixture.blocks[0], fixture.blocks[2])
            };
            assert_eq!(
                (relationship.source_id, relationship.target_id),
                (source, target)
            );
            for (index, id) in [source, target].iter().enumerate() {
                assert_eq!(relationship.association_ends[index].classifier_id, *id);
                assert_eq!(
                    relationship.association_ends[index].id,
                    original.association_ends[index].id
                );
            }
            let diagrams = fixture.state.diagrams.lock().unwrap();
            let diagram = &diagrams[0];
            let edge = &diagram.edges[0];
            assert_eq!(edge.id, old_edge.id);
            let source_node = diagram
                .nodes
                .iter()
                .find(|node| node.element_id == source.to_string())
                .unwrap();
            let target_node = diagram
                .nodes
                .iter()
                .find(|node| node.element_id == target.to_string())
                .unwrap();
            assert_eq!(edge.source_node_id, source_node.id);
            assert_eq!(edge.target_node_id, target_node.id);
            assert_eq!(
                edge.points,
                route_relationship(source_node, target_node, &diagram.nodes).unwrap()
            );
        }
    }

    fn add_second_view(fixture: &ReconnectFixture, malformed: bool) {
        let mut diagrams = fixture.state.diagrams.lock().unwrap();
        let mut view = diagrams[0].clone();
        view.id = DiagramId::new().to_string();
        view.nodes.pop();
        for (index, node) in view.nodes.iter_mut().enumerate() {
            node.id = uuid::Uuid::new_v4().to_string();
            if malformed && index == 0 {
                node.y = f64::NAN;
            }
        }
        view.edges[0].id = uuid::Uuid::new_v4().to_string();
        view.edges[0].source_node_id = view.nodes[0].id.clone();
        view.edges[0].target_node_id = view.nodes[1].id.clone();
        diagrams.push(view);
    }

    #[test]
    fn reconnect_updates_every_view_and_recomputes_attached_labels() {
        let fixture = reconnect_fixture(RelationshipKind::Association);
        add_second_view(&fixture, false);
        reconnect(&fixture, "target").unwrap();
        let project_guard = fixture.state.project.lock().unwrap();
        let project = project_guard.as_ref().unwrap();
        let diagrams = fixture.state.diagrams.lock().unwrap();
        super::super::validate_loaded_diagrams(project, &diagrams).unwrap();
        for diagram in diagrams.iter() {
            assert_eq!(diagram.nodes.len(), 3);
            let edge = &diagram.edges[0];
            let target = diagram
                .nodes
                .iter()
                .find(|node| node.id == edge.target_node_id)
                .unwrap();
            assert_eq!(target.element_id, fixture.blocks[2].to_string());
            assert_eq!(
                edge.label_anchor,
                Some(super::super::routing::route_label_anchor(&edge.points))
            );
        }
    }

    #[test]
    fn failed_secondary_view_route_rolls_back_every_view_and_semantics() {
        let fixture = reconnect_fixture(RelationshipKind::Association);
        add_second_view(&fixture, true);
        let before = snapshot(&fixture.state);
        assert!(reconnect(&fixture, "target").is_err());
        assert_eq!(snapshot(&fixture.state), before);
    }

    #[test]
    fn reconnect_linked_composition_retypes_the_original_property() {
        let mut fixture = reconnect_fixture(RelationshipKind::Association);
        let (relationship, property) = {
            let mut guard = fixture.state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            project.relationships.remove(&fixture.relationship_id);
            project
                .create_composition(
                    fixture.blocks[0],
                    fixture.blocks[1],
                    "part",
                    Multiplicity::ONE,
                    Some(project.root_id),
                )
                .unwrap()
        };
        fixture.relationship_id = relationship;
        fixture.state.diagrams.lock().unwrap()[0].edges[0].relationship_id =
            relationship.to_string();
        add_second_view(&fixture, false);
        reconnect(&fixture, "target").unwrap();
        let guard = fixture.state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        assert_eq!(
            project.element(property).unwrap().type_id,
            Some(fixture.blocks[2])
        );
        assert_eq!(
            project.relationship(relationship).unwrap().association_ends[1].property_id,
            Some(property)
        );
        project.validate().unwrap();
    }

    #[test]
    fn legacy_link_is_one_native_history_step_and_rejections_preserve_redo() {
        let fixture = reconnect_fixture(RelationshipKind::Association);
        {
            let mut guard = fixture.state.project.lock().unwrap();
            guard
                .as_mut()
                .unwrap()
                .relationships
                .get_mut(&fixture.relationship_id)
                .unwrap()
                .association_ends[0]
                .aggregation = AggregationKind::Composite;
        }
        let activity = super::super::activity_workspace::ActivityWorkspaceState::default();
        let history = super::super::history::HistoryState::default();
        let before = snapshot(&fixture.state);
        let property = link_composition_property_in_state(
            fixture.relationship_id,
            None,
            &fixture.state,
            &activity,
            &history,
        )
        .unwrap();
        let after = snapshot(&fixture.state);
        assert_ne!(after, before);
        assert_eq!(super::super::history::undo_len(&history), 1);
        assert_eq!(
            link_composition_property_in_state(
                fixture.relationship_id,
                None,
                &fixture.state,
                &activity,
                &history,
            )
            .unwrap(),
            property
        );
        assert_eq!(super::super::history::undo_len(&history), 1);
        super::super::history::undo_states(&fixture.state, &activity, &history).unwrap();
        assert_eq!(snapshot(&fixture.state), before);
        assert!(
            link_composition_property_in_state(
                fixture.relationship_id,
                Some(ElementId::new()),
                &fixture.state,
                &activity,
                &history,
            )
            .is_err()
        );
        assert_eq!(snapshot(&fixture.state), before);
        super::super::history::redo_states(&fixture.state, &activity, &history).unwrap();
        assert_eq!(snapshot(&fixture.state), after);
    }

    #[test]
    fn linked_association_end_editor_updates_property_and_rejects_partial_changes() {
        let mut project = Project::new("Linked end editor");
        let whole = project
            .create_element(ElementKind::Block, "Vehicle", project.root_id)
            .unwrap();
        let part = project
            .create_element(ElementKind::Block, "Wheel", project.root_id)
            .unwrap();
        let (relation, property) = project
            .create_composition(
                whole,
                part,
                "wheel",
                Multiplicity::ONE,
                Some(project.root_id),
            )
            .unwrap();
        let end_id = project.relationship(relation).unwrap().association_ends[1]
            .id
            .to_string();
        edit_association_end(
            &mut project,
            relation,
            &end_id,
            "front",
            Multiplicity::new(2, Some(2)).unwrap(),
            true,
            AggregationKind::Composite,
        )
        .unwrap();
        assert_eq!(project.element(property).unwrap().name, "front");
        assert_eq!(
            project
                .element(property)
                .unwrap()
                .multiplicity
                .unwrap()
                .notation(),
            "2"
        );
        let before = serde_json::to_value(&project).unwrap();
        assert!(
            edit_association_end(
                &mut project,
                relation,
                &end_id,
                "bad",
                Multiplicity::ONE,
                false,
                AggregationKind::Composite,
            )
            .is_err()
        );
        assert_eq!(serde_json::to_value(&project).unwrap(), before);
        let inverse_id = project.relationship(relation).unwrap().association_ends[0]
            .id
            .to_string();
        assert!(
            edit_association_end(
                &mut project,
                relation,
                &inverse_id,
                "",
                Multiplicity::new(0, None).unwrap(),
                false,
                AggregationKind::None,
            )
            .is_err()
        );
        assert_eq!(serde_json::to_value(&project).unwrap(), before);
        edit_association_end(
            &mut project,
            relation,
            &end_id,
            "sharedWheel",
            Multiplicity::ONE,
            true,
            AggregationKind::Shared,
        )
        .unwrap();
        assert_eq!(
            project.element(property).unwrap().kind,
            ElementKind::ReferenceProperty
        );
        assert_eq!(
            project.relationship(relation).unwrap().association_ends[1].property_id,
            Some(property)
        );
        project.validate().unwrap();
    }

    fn linked_end_fixture() -> (ReconnectFixture, ElementId) {
        let mut fixture = reconnect_fixture(RelationshipKind::Association);
        let property = {
            let mut guard = fixture.state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            project.relationships.remove(&fixture.relationship_id);
            let (relationship, property) = project
                .create_composition(
                    fixture.blocks[0],
                    fixture.blocks[1],
                    "unit",
                    Multiplicity::ONE,
                    Some(project.root_id),
                )
                .unwrap();
            fixture.relationship_id = relationship;
            property
        };
        fixture.state.diagrams.lock().unwrap()[0].edges[0].relationship_id =
            fixture.relationship_id.to_string();
        add_second_view(&fixture, false);
        (fixture, property)
    }

    fn apply_end(fixture: &ReconnectFixture, role: &str, navigable: bool) -> Result<(), String> {
        let end_id = fixture
            .state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .relationship(fixture.relationship_id)
            .unwrap()
            .association_ends[1]
            .id
            .to_string();
        update_association_end_in_state(
            AssociationEndEdit {
                relationship_id: fixture.relationship_id,
                end_id,
                role_name: role.into(),
                multiplicity: Multiplicity::new(2, Some(2)).unwrap(),
                navigable,
                aggregation: AggregationKind::Composite,
            },
            &fixture.state,
            &fixture.activity,
            &fixture.history,
        )
    }

    #[test]
    fn association_end_transaction_preserves_identity_reopen_and_one_step_history() {
        use super::super::history;
        let (fixture, property) = linked_end_fixture();
        let before = snapshot(&fixture.state);
        apply_end(&fixture, "primary", true).unwrap();
        let after = snapshot(&fixture.state);
        {
            let guard = fixture.state.project.lock().unwrap();
            let project = guard.as_ref().unwrap();
            let part = project.element(property).unwrap();
            assert_eq!(part.name, "primary");
            assert_eq!(part.type_id, Some(fixture.blocks[1]));
            assert_eq!(part.owner_id, Some(fixture.blocks[0]));
            assert_eq!(part.multiplicity.unwrap().notation(), "2");
            let end = &project
                .relationship(fixture.relationship_id)
                .unwrap()
                .association_ends[1];
            assert_eq!(end.property_id, Some(property));
            assert_eq!(end.role_name, "primary");
            super::super::validate_loaded_diagrams(
                project,
                &fixture.state.diagrams.lock().unwrap(),
            )
            .unwrap();
            let directory = tempfile::tempdir().unwrap();
            let mut database = systems_modeler_persistence::ProjectDatabase::open(
                directory.path().join("ends.smproj"),
            )
            .unwrap();
            database.save_project(project).unwrap();
            assert_eq!(
                serde_json::to_value(database.load_first_project().unwrap()).unwrap(),
                serde_json::to_value(project).unwrap()
            );
        }
        assert_eq!(history::undo_len(&fixture.history), 1);
        apply_end(&fixture, "primary", true).unwrap();
        assert_eq!(history::undo_len(&fixture.history), 1);
        assert!(history::undo_states(&fixture.state, &fixture.activity, &fixture.history).unwrap());
        assert_eq!(snapshot(&fixture.state), before);
        assert!(apply_end(&fixture, "", true).is_err());
        assert!(apply_end(&fixture, "invalid", false).is_err());
        assert_eq!(snapshot(&fixture.state), before);
        assert_eq!(history::undo_len(&fixture.history), 0);
        assert!(history::redo_states(&fixture.state, &fixture.activity, &fixture.history).unwrap());
        assert_eq!(snapshot(&fixture.state), after);
    }

    #[test]
    fn association_end_transaction_rejects_invalid_dependent_views_before_commit() {
        let (fixture, _) = linked_end_fixture();
        fixture.state.diagrams.lock().unwrap()[1].nodes[0].element_id =
            ElementId::new().to_string();
        let before = snapshot(&fixture.state);
        assert!(apply_end(&fixture, "primary", true).is_err());
        assert_eq!(snapshot(&fixture.state), before);
        assert_eq!(super::super::history::undo_len(&fixture.history), 0);
    }

    #[test]
    fn delete_rejects_item_flow_dependency_and_preserves_pending_redo() {
        use systems_modeler_core::{Connector, ConnectorEnd, ConnectorKind, ItemFlow};
        let fixture = reconnect_fixture(RelationshipKind::Association);
        let connector = {
            let mut guard = fixture.state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            let mut role = |name| {
                project
                    .create_typed_feature(
                        ElementKind::PartProperty,
                        name,
                        fixture.blocks[0],
                        fixture.blocks[1],
                        Multiplicity::ONE,
                    )
                    .unwrap()
            };
            let source = ConnectorEnd::role(role("left"));
            let target = ConnectorEnd::role(role("right"));
            let connector = project
                .create_connector(Connector {
                    context_id: fixture.blocks[0],
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
                    conveyed_item_ids: vec![fixture.blocks[2]],
                })
                .unwrap();
            project.validate().unwrap();
            connector
        };
        let delete = || {
            delete_bdd_relationship_in_state(
                fixture.diagram_id.clone(),
                connector.to_string(),
                &fixture.state,
                &fixture.activity,
                &fixture.history,
            )
        };
        let before = snapshot(&fixture.state);
        assert!(delete().unwrap_err().contains("not presented"));
        assert_eq!(snapshot(&fixture.state), before);
        // Even a forged generic-view presentation must not bypass ItemFlow integrity.
        fixture.state.diagrams.lock().unwrap()[0].edges[0].relationship_id = connector.to_string();
        super::super::history::checkpoint_states(
            &fixture.state,
            &fixture.activity,
            &fixture.history,
        )
        .unwrap();
        fixture.state.project.lock().unwrap().as_mut().unwrap().name = "Redo".into();
        assert!(
            super::super::history::undo_states(&fixture.state, &fixture.activity, &fixture.history)
                .unwrap()
        );
        let before = snapshot(&fixture.state);
        assert!(delete().is_err());
        assert_eq!(snapshot(&fixture.state), before);
        assert_eq!(super::super::history::undo_len(&fixture.history), 0);
        assert!(
            super::super::history::redo_states(&fixture.state, &fixture.activity, &fixture.history)
                .unwrap()
        );
        assert_eq!(
            fixture.state.project.lock().unwrap().as_ref().unwrap().name,
            "Redo"
        );
    }

    #[test]
    fn delete_composition_preserves_usage_and_types_and_undo_restores_every_view() {
        let mut fixture = reconnect_fixture(RelationshipKind::Association);
        let (relationship, property) = {
            let mut guard = fixture.state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            project.relationships.remove(&fixture.relationship_id);
            project
                .create_composition(
                    fixture.blocks[0],
                    fixture.blocks[1],
                    "part",
                    Multiplicity::ONE,
                    Some(project.root_id),
                )
                .unwrap()
        };
        fixture.relationship_id = relationship;
        fixture.state.diagrams.lock().unwrap()[0].edges[0].relationship_id =
            relationship.to_string();
        add_second_view(&fixture, false);
        let before = snapshot(&fixture.state);
        delete_bdd_relationship_in_state(
            fixture.diagram_id.clone(),
            relationship.to_string(),
            &fixture.state,
            &fixture.activity,
            &fixture.history,
        )
        .unwrap();
        assert_eq!(super::super::history::undo_len(&fixture.history), 1);
        {
            let guard = fixture.state.project.lock().unwrap();
            let project = guard.as_ref().unwrap();
            project.validate().unwrap();
            assert!(project.relationship(relationship).is_err());
            assert_eq!(
                project.element(property).unwrap().type_id,
                Some(fixture.blocks[1])
            );
            for id in fixture.blocks {
                assert!(project.element(id).is_ok());
            }
            assert!(
                fixture
                    .state
                    .diagrams
                    .lock()
                    .unwrap()
                    .iter()
                    .all(|diagram| diagram.edges.is_empty())
            );
        }
        let after = snapshot(&fixture.state);
        assert!(
            super::super::history::undo_states(&fixture.state, &fixture.activity, &fixture.history)
                .unwrap()
        );
        assert_eq!(snapshot(&fixture.state), before);
        assert!(
            super::super::history::redo_states(&fixture.state, &fixture.activity, &fixture.history)
                .unwrap()
        );
        assert_eq!(snapshot(&fixture.state), after);
    }

    #[test]
    fn delete_checks_project_before_waiting_for_a_diagram_lock() {
        let fixture = reconnect_fixture(RelationshipKind::Association);
        let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = fixture.state.project.lock().unwrap();
            panic!("inject project lock failure");
        }));
        assert!(poisoned.is_err());
        std::thread::scope(|scope| {
            let diagrams = fixture.state.diagrams.lock().unwrap();
            let (sender, receiver) = std::sync::mpsc::channel();
            let state = &fixture.state;
            let activity = &fixture.activity;
            let history = &fixture.history;
            let diagram_id = fixture.diagram_id.clone();
            let relationship_id = fixture.relationship_id.to_string();
            scope.spawn(move || {
                let result = delete_bdd_relationship_in_state(
                    diagram_id,
                    relationship_id,
                    state,
                    activity,
                    history,
                );
                sender.send(result).unwrap();
            });
            let result = receiver.recv_timeout(std::time::Duration::from_secs(2));
            drop(diagrams);
            assert_eq!(result.unwrap(), Err("project lock poisoned".into()));
        });
    }

    #[test]
    fn delete_diagram_lock_failure_does_not_remove_the_relationship() {
        let fixture = reconnect_fixture(RelationshipKind::Association);
        let before = snapshot(&fixture.state).0;
        let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = fixture.state.diagrams.lock().unwrap();
            panic!("inject diagram lock failure");
        }));
        assert!(poisoned.is_err());
        assert_eq!(
            delete_bdd_relationship_in_state(
                fixture.diagram_id.clone(),
                fixture.relationship_id.to_string(),
                &fixture.state,
                &fixture.activity,
                &fixture.history,
            ),
            Err("diagram lock poisoned".into())
        );
        assert_eq!(
            serde_json::to_value(&*fixture.state.project.lock().unwrap()).unwrap(),
            before
        );
    }

    #[test]
    fn parses_sysml_multiplicity_notation() {
        assert_eq!(parse_multiplicity("1").unwrap().notation(), "1");
        assert_eq!(parse_multiplicity("0..1").unwrap().notation(), "0..1");
        assert_eq!(parse_multiplicity("1..*").unwrap().notation(), "1..*");
        assert_eq!(parse_multiplicity("*").unwrap().notation(), "0..*");
        assert!(parse_multiplicity("3..2").is_err());
    }
}
