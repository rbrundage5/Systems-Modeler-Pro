use serde_json::json;
use systems_modeler_core::structural_presentation::{BddDiagram, validate_structural_diagrams};
use systems_modeler_core::{DiagramId, ElementKind, Project, RelationshipKind};

fn fixture() -> (Project, BddDiagram) {
    let mut project = Project::new("Shared engineering");
    let source = project.create_element(ElementKind::Block, "Specific", project.root_id).unwrap();
    let target = project.create_element(ElementKind::Block, "General", project.root_id).unwrap();
    let relationship = project.create_relationship(
        RelationshipKind::Generalization, source, target, Some(project.root_id),
    ).unwrap();
    let diagram = serde_json::from_value(json!({
        "id": DiagramId::new().to_string(), "name": "Structure",
        "owner_id": project.root_id.to_string(),
        "nodes": [
            {"id": "10000000-0000-0000-0000-000000000001", "element_id": source.to_string(),
             "x": 100.0, "y": 100.0, "width": 160.0, "height": 100.0},
            {"id": "10000000-0000-0000-0000-000000000002", "element_id": target.to_string(),
             "x": 400.0, "y": 100.0, "width": 160.0, "height": 100.0}
        ],
        "edges": [{"id": "20000000-0000-0000-0000-000000000001",
            "relationship_id": relationship.to_string(),
            "source_node_id": "10000000-0000-0000-0000-000000000001",
            "target_node_id": "10000000-0000-0000-0000-000000000002",
            "points": [{"x": 260.0, "y": 150.0}, {"x": 400.0, "y": 150.0}]}]
    })).unwrap();
    (project, diagram)
}

#[test]
fn legacy_native_shape_defaults_and_roundtrips_without_identity_changes() {
    let (project, diagram) = fixture();
    assert_eq!(diagram.family, "bdd");
    assert!(diagram.semantic_context_id.is_none());
    assert!(diagram.subject_boundary.is_none());
    assert!(diagram.nodes[0].actor_notation.is_none());
    assert!(diagram.nodes[0].parameter_presentations.is_empty());
    assert!(diagram.edges[0].label_anchor.is_none());
    validate_structural_diagrams(&project, std::slice::from_ref(&diagram)).unwrap();
    let encoded = serde_json::to_value(&diagram).unwrap();
    let decoded: BddDiagram = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(encoded, serde_json::to_value(&decoded).unwrap());
    validate_structural_diagrams(&project, &[decoded]).unwrap();
}

#[test]
fn malformed_identity_owner_and_context_fail_without_changing_semantics() {
    let (project, diagram) = fixture();
    let original = serde_json::to_value(&project).unwrap();
    let mut invalid = diagram.clone();
    invalid.id = "invalid".into();
    assert!(validate_structural_diagrams(&project, &[invalid]).unwrap_err().contains("invalid diagram id"));
    let mut invalid = diagram.clone();
    invalid.owner_id = invalid.nodes[0].element_id.clone();
    assert!(validate_structural_diagrams(&project, &[invalid]).unwrap_err().contains("owner"));
    let mut invalid = diagram.clone();
    invalid.family = "parametric".into();
    assert!(validate_structural_diagrams(&project, &[invalid]).unwrap_err().contains("requires a semantic context"));
    let mut invalid = diagram;
    invalid.semantic_context_id = Some("30000000-0000-0000-0000-000000000001".into());
    assert!(validate_structural_diagrams(&project, &[invalid]).is_err());
    assert_eq!(original, serde_json::to_value(&project).unwrap());
}

#[test]
fn duplicate_presentations_missing_endpoints_and_reversed_semantic_ends_fail() {
    let (project, diagram) = fixture();
    assert!(validate_structural_diagrams(&project, &[diagram.clone(), diagram.clone()]).unwrap_err().contains("duplicate diagram"));
    let mut invalid = diagram.clone();
    invalid.nodes[1].id = invalid.nodes[0].id.clone();
    assert!(validate_structural_diagrams(&project, &[invalid]).unwrap_err().contains("duplicate diagram node"));
    let mut invalid = diagram.clone();
    invalid.nodes.pop();
    assert!(validate_structural_diagrams(&project, &[invalid]).unwrap_err().contains("target node not found"));
    let mut invalid = diagram;
    let edge = &mut invalid.edges[0];
    std::mem::swap(&mut edge.source_node_id, &mut edge.target_node_id);
    assert!(validate_structural_diagrams(&project, &[invalid]).unwrap_err().contains("endpoints do not match"));
}

#[test]
fn package_validation_remains_part_of_shared_authority() {
    let (project, mut diagram) = fixture();
    diagram.family = "package".into();
    // Blocks are packageable, but Generalization is not a Package Diagram edge.
    assert!(validate_structural_diagrams(&project, &[diagram]).unwrap_err().contains("not valid on a Package Diagram"));
}
