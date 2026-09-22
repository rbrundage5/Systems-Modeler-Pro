use systems_modeler_core::{ElementKind, ModelError, Project, RelationshipId, RelationshipKind};

fn graph(node_count: usize, edges: &[(usize, usize)]) -> Project {
    let mut project = Project::new("Imported inheritance graph");
    let root = project.root_id;
    let nodes: Vec<_> = (0..node_count)
        .map(|index| {
            project
                .create_element(ElementKind::Block, format!("Classifier {index}"), root)
                .unwrap()
        })
        .collect();
    let seed = project
        .create_relationship(
            RelationshipKind::Generalization,
            nodes[0],
            nodes[1],
            Some(root),
        )
        .unwrap();
    let template = project.relationships[&seed].clone();
    project.relationships.clear();
    // Loading/deserialization populates the repository without going through
    // create_relationship, so the complete validator must catch graph defects.
    for &(source, target) in edges {
        let mut relationship = template.clone();
        relationship.id = RelationshipId::new();
        relationship.external_id = relationship.id.to_string();
        relationship.source_id = nodes[source];
        relationship.target_id = nodes[target];
        project.relationships.insert(relationship.id, relationship);
    }
    project
}

fn assert_cycle_without_mutation(project: &Project) {
    let before = serde_json::to_value(project).unwrap();
    assert_eq!(project.validate(), Err(ModelError::GeneralizationCycle));
    assert_eq!(serde_json::to_value(project).unwrap(), before);
}

#[test]
fn rejects_self_generalization_loaded_into_the_repository() {
    assert_cycle_without_mutation(&graph(2, &[(0, 0)]));
}

#[test]
fn rejects_a_serialized_two_classifier_cycle_without_mutation() {
    let project = graph(2, &[(0, 1), (1, 0)]);
    let reopened: Project = serde_json::from_value(serde_json::to_value(project).unwrap()).unwrap();
    assert_cycle_without_mutation(&reopened);
}

#[test]
fn rejects_cycle_with_a_tail_even_when_another_component_is_acyclic() {
    let project = graph(7, &[(0, 1), (1, 2), (2, 0), (3, 0), (4, 5), (5, 6)]);
    assert_cycle_without_mutation(&project);
}

#[test]
fn accepts_diamond_multiple_inheritance_and_parallel_acyclic_edges() {
    let project = graph(5, &[(0, 1), (0, 2), (1, 3), (2, 3), (0, 1)]);
    assert_eq!(project.validate(), Ok(()));
}

#[test]
fn validates_twenty_thousand_inheritance_levels_without_recursion() {
    let edges: Vec<_> = (0..20_000).map(|index| (index, index + 1)).collect();
    let mut project = graph(20_001, &edges);
    assert_eq!(project.validate(), Ok(()));
    let source = project
        .elements
        .values()
        .find(|element| element.name == "Classifier 20000")
        .unwrap()
        .id;
    let target = project
        .elements
        .values()
        .find(|element| element.name == "Classifier 0")
        .unwrap()
        .id;
    let mut closing_edge = project.relationships.values().next().unwrap().clone();
    closing_edge.id = RelationshipId::new();
    closing_edge.external_id = closing_edge.id.to_string();
    closing_edge.source_id = source;
    closing_edge.target_id = target;
    project.relationships.insert(closing_edge.id, closing_edge);
    assert_eq!(project.validate(), Err(ModelError::GeneralizationCycle));
}

#[test]
fn invalid_classifier_endpoints_are_reported_before_graph_cycles() {
    let mut project = graph(2, &[(0, 1), (1, 0)]);
    let source = project.relationships.values().next().unwrap().source_id;
    project.elements.get_mut(&source).unwrap().kind = ElementKind::Package;
    assert_eq!(
        project.validate(),
        Err(ModelError::GeneralizationRequiresClassifiers)
    );
}
