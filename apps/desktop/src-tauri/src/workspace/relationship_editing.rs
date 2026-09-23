use super::{
    WorkspaceState, parse_diagram_id, parse_element_id, parse_relationship_id,
    relationship_display_kind, route_relationship,
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
pub fn update_association_end(
    relationship_id: String,
    end_id: String,
    role_name: String,
    multiplicity: String,
    navigable: bool,
    aggregation: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let relationship_id = parse_relationship_id(&relationship_id)?;
    let multiplicity = parse_multiplicity(&multiplicity)?;
    let aggregation = parse_aggregation(&aggregation)?;

    let mut project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_mut().ok_or("no project open")?;
    let original = project
        .relationship(relationship_id)
        .map_err(|error| error.to_string())?
        .clone();
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
    {
        let relationship = project
            .relationships
            .get_mut(&relationship_id)
            .ok_or("relationship not found")?;
        relationship.association_ends[end_index].role_name = role_name.trim().to_string();
        relationship.association_ends[end_index].multiplicity = multiplicity;
        relationship.association_ends[end_index].navigable = navigable;
        relationship.association_ends[end_index].aggregation = aggregation;

        let decorated = relationship
            .association_ends
            .iter()
            .filter(|end| end.aggregation != AggregationKind::None)
            .count();
        if decorated > 1 {
            project.relationships.insert(relationship_id, original);
            return Err(
                "a binary association can have aggregation/composition on only one end".into(),
            );
        }
    }

    if let Err(error) = project.validate() {
        project.relationships.insert(relationship_id, original);
        return Err(error.to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn reconnect_bdd_relationship(
    diagram_id: String,
    relationship_id: String,
    side: String,
    element_id: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    reconnect_bdd_relationship_in_state(diagram_id, relationship_id, side, element_id, &state)
}

fn reconnect_bdd_relationship_in_state(
    diagram_id: String,
    relationship_id: String,
    side: String,
    element_id: String,
    state: &WorkspaceState,
) -> Result<(), String> {
    let diagram_id = parse_diagram_id(&diagram_id)?;
    let relationship_id = parse_relationship_id(&relationship_id)?;
    let element_id = parse_element_id(&element_id)?;
    if side != "source" && side != "target" {
        return Err("relationship side must be source or target".into());
    }

    let mut project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_mut().ok_or("no project open")?;
    let replacement = project
        .element(element_id)
        .map_err(|error| error.to_string())?;
    if replacement.kind != ElementKind::Block {
        return Err("BDD relationship endpoints must be Blocks".into());
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
    if duplicate_after_reconnect(
        project,
        relationship_id,
        display_kind,
        new_source,
        new_target,
    ) {
        return Err(format!("an equivalent {display_kind} already exists"));
    }
    // Resolve every fallible presentation dependency before publishing semantics.
    // Both guards remain held until the relationship and edge commit together.
    let mut diagrams = state.diagrams.lock().map_err(|_| "diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id.to_string())
        .ok_or("diagram not found")?;
    let source_node = diagram
        .nodes
        .iter()
        .find(|node| node.element_id == new_source.to_string())
        .cloned()
        .ok_or("new source Block must be presented on the BDD")?;
    let target_node = diagram
        .nodes
        .iter()
        .find(|node| node.element_id == new_target.to_string())
        .cloned()
        .ok_or("new target Block must be presented on the BDD")?;
    let points = route_relationship(&source_node, &target_node, &diagram.nodes)?;
    let edge = diagram
        .edges
        .iter_mut()
        .find(|edge| edge.relationship_id == relationship_id.to_string())
        .ok_or("diagram edge not found")?;

    let mut candidate = original.clone();
    candidate.source_id = new_source;
    candidate.target_id = new_target;
    if candidate.kind == RelationshipKind::Association && candidate.association_ends.len() == 2 {
        candidate.association_ends[0].classifier_id = new_source;
        candidate.association_ends[1].classifier_id = new_target;
    }
    project.relationships.insert(relationship_id, candidate);
    if let Err(error) = project.validate() {
        project.relationships.insert(relationship_id, original);
        return Err(error.to_string());
    }
    // No fallible work remains after successful semantic validation.
    edge.source_node_id = source_node.id;
    edge.target_node_id = target_node.id;
    edge.points = points;
    Ok(())
}

#[tauri::command]
pub fn delete_bdd_relationship(
    diagram_id: String,
    relationship_id: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    delete_bdd_relationship_in_state(diagram_id, relationship_id, &state)
}

fn delete_bdd_relationship_in_state(
    diagram_id: String,
    relationship_id: String,
    state: &WorkspaceState,
) -> Result<(), String> {
    let diagram_id = parse_diagram_id(&diagram_id)?;
    let relationship_id = parse_relationship_id(&relationship_id)?;

    let mut project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_mut().ok_or("no project open")?;

    let mut diagrams = state.diagrams.lock().map_err(|_| "diagram lock poisoned")?;
    if !diagrams
        .iter()
        .any(|diagram| diagram.id == diagram_id.to_string())
    {
        return Err("diagram not found".into());
    }

    if project.relationships.remove(&relationship_id).is_none() {
        return Err("relationship not found".into());
    }
    for diagram in diagrams.iter_mut() {
        diagram
            .edges
            .retain(|edge| edge.relationship_id != relationship_id.to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{BddDiagram, DiagramEdge, DiagramNode};
    use systems_modeler_core::{DiagramId, Project, RelationshipId};

    struct ReconnectFixture {
        state: WorkspaceState,
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
        )
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
            let diagram_id = fixture.diagram_id.clone();
            let relationship_id = fixture.relationship_id.to_string();
            scope.spawn(move || {
                let result = delete_bdd_relationship_in_state(diagram_id, relationship_id, state);
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
