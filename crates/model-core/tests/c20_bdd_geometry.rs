use systems_modeler_core::structural_presentation::geometry::{
    BddGeometryCommand, BddRoutingScope, apply_bdd_geometry,
};
use systems_modeler_core::structural_presentation::{BddDiagram, DiagramEdge, DiagramNode};
use systems_modeler_core::{GeometryPoint, routing::RouteRect};

fn fixture() -> BddDiagram {
    BddDiagram {
        id: "diagram".into(),
        name: "Structure".into(),
        owner_id: "owner".into(),
        family: "bdd".into(),
        semantic_context_id: None,
        subject_boundary: None,
        nodes: [("source", 100.0), ("target", 500.0), ("disjoint", 900.0)]
            .into_iter()
            .map(|(id, x)| DiagramNode {
                id: id.into(),
                element_id: format!("semantic-{id}"),
                x,
                y: 100.0,
                width: 180.0,
                height: 105.0,
                actor_notation: None,
                parameter_presentations: Vec::new(),
            })
            .collect(),
        edges: vec![DiagramEdge {
            id: "edge".into(),
            relationship_id: "relationship".into(),
            source_node_id: "source".into(),
            target_node_id: "target".into(),
            points: vec![
                GeometryPoint { x: 280.0, y: 152.5 },
                GeometryPoint { x: 500.0, y: 152.5 },
            ],
            label_anchor: Some(GeometryPoint { x: 390.0, y: 140.0 }),
        }],
    }
}

fn movement(presentation: &str, x: f64) -> BddGeometryCommand {
    BddGeometryCommand::UpdateNode {
        presentation_id: presentation.into(),
        x,
        y: 350.0,
        width: 200.0,
        height: 120.0,
    }
}

#[test]
fn native_geometry_command_preserves_identity_and_reroutes_committed_endpoints() {
    let mut diagram = fixture();
    let previous = diagram.edges[0].points.clone();
    apply_bdd_geometry(
        &mut diagram,
        &movement("source", 100.0),
        BddRoutingScope::AllEdges,
        None,
    )
    .unwrap();
    assert_eq!(diagram.nodes[0].id, "source");
    assert_eq!(diagram.nodes[0].element_id, "semantic-source");
    assert_eq!(diagram.edges[0].relationship_id, "relationship");
    assert_eq!(diagram.nodes[0].y, 350.0);
    assert_ne!(diagram.edges[0].points, previous);
    assert!(
        diagram.edges[0]
            .points
            .iter()
            .all(|point| point.x.is_finite() && point.y.is_finite())
    );
    assert!(diagram.edges[0].label_anchor.is_some());
}

#[test]
fn invalid_geometry_missing_node_and_impossible_route_leave_every_field_unchanged() {
    let original = fixture();
    let before = serde_json::to_value(&original).unwrap();
    for (command, bounds) in [
        (movement("source", f64::NAN), None),
        (movement("missing", 100.0), None),
        (
            movement("source", 100.0),
            Some(RouteRect {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            }),
        ),
    ] {
        let mut diagram = original.clone();
        assert!(
            apply_bdd_geometry(&mut diagram, &command, BddRoutingScope::AllEdges, bounds).is_err()
        );
        assert_eq!(serde_json::to_value(&diagram).unwrap(), before);
    }
}

#[test]
fn offline_incident_policy_preserves_unrelated_routes_even_if_stale() {
    let mut diagram = fixture();
    diagram.edges[0].target_node_id = "missing".into();
    let edge_before = serde_json::to_value(&diagram.edges[0]).unwrap();
    apply_bdd_geometry(
        &mut diagram,
        &movement("disjoint", 1000.0),
        BddRoutingScope::IncidentEdges,
        None,
    )
    .unwrap();
    assert_eq!(diagram.nodes[2].x, 1000.0);
    assert_eq!(
        serde_json::to_value(&diagram.edges[0]).unwrap(),
        edge_before
    );
    let before = serde_json::to_value(&diagram).unwrap();
    assert!(
        apply_bdd_geometry(
            &mut diagram,
            &movement("disjoint", 1100.0),
            BddRoutingScope::AllEdges,
            None
        )
        .is_err()
    );
    assert_eq!(serde_json::to_value(&diagram).unwrap(), before);
}

#[test]
fn other_families_cannot_be_mutated_by_bdd_command() {
    let mut diagram = fixture();
    diagram.family = "parametric".into();
    let before = serde_json::to_value(&diagram).unwrap();
    assert!(
        apply_bdd_geometry(
            &mut diagram,
            &movement("source", 100.0),
            BddRoutingScope::AllEdges,
            None
        )
        .is_err()
    );
    assert_eq!(serde_json::to_value(&diagram).unwrap(), before);
}
