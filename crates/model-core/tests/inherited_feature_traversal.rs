use systems_modeler_core::{ElementId, ElementKind, ModelError, Project, RelationshipId, RelationshipKind};

fn block(project: &mut Project, name: &str) -> ElementId {
    project.create_element(ElementKind::Block, name, project.root_id).unwrap()
}

fn generalization(project: &mut Project, source: ElementId, target: ElementId) {
    project.create_relationship(RelationshipKind::Generalization, source, target, Some(project.root_id)).unwrap();
}

fn inherited_ids(project: &Project, id: ElementId) -> Vec<ElementId> {
    project.inherited_features(id).unwrap().into_iter().map(|element| element.id).collect()
}

#[test]
fn diamond_emits_each_ancestor_feature_once_and_preserves_distinct_same_named_features() {
    let mut project = Project::new("Multiple inheritance");
    let child = block(&mut project, "Child");
    let left = block(&mut project, "Left");
    let right = block(&mut project, "Right");
    let ancestor = block(&mut project, "Ancestor");
    for (source, target) in [(child, left), (child, right), (left, ancestor), (right, ancestor)] {
        generalization(&mut project, source, target);
    }
    let mut expected = Vec::new();
    for owner in [left, right, ancestor] {
        expected.push(project.create_element(ElementKind::Operation, "operate", owner).unwrap());
    }
    let own = project.create_element(ElementKind::Operation, "local", child).unwrap();
    let before = serde_json::to_value(&project).unwrap();
    let ids = inherited_ids(&project, child);
    assert_eq!(ids.len(), 3);
    for id in expected {
        assert_eq!(ids.iter().filter(|candidate| **candidate == id).count(), 1);
    }
    assert!(!ids.contains(&own));
    let reopened: Project = serde_json::from_value(before.clone()).unwrap();
    assert_eq!(inherited_ids(&reopened, child), ids);
    assert_eq!(serde_json::to_value(project).unwrap(), before);
}

#[test]
fn repeated_edges_and_malformed_cycles_terminate_without_duplicate_or_local_features() {
    let mut project = Project::new("Defensive query");
    let child = block(&mut project, "Child");
    let parent = block(&mut project, "Parent");
    generalization(&mut project, child, parent);
    let feature = project.create_element(ElementKind::Operation, "inherited", parent).unwrap();
    project.create_element(ElementKind::Operation, "local", child).unwrap();
    let template = project.relationships.values().next().unwrap().clone();
    for reverse in [false, true] {
        let mut edge = template.clone();
        edge.id = RelationshipId::new();
        edge.external_id = edge.id.to_string();
        if reverse {
            edge.source_id = parent;
            edge.target_id = child;
        }
        project.relationships.insert(edge.id, edge);
    }
    assert_eq!(inherited_ids(&project, child), vec![feature]);
    assert_eq!(project.validate(), Err(ModelError::GeneralizationCycle));
}

#[test]
fn deep_feature_query_does_not_recurse_or_scan_the_entire_repository_per_ancestor() {
    let mut project = Project::new("Deep inheritance");
    let nodes: Vec<_> = (0..20_001).map(|index| block(&mut project, &format!("Type {index}"))).collect();
    generalization(&mut project, nodes[0], nodes[1]);
    let template = project.relationships.values().next().unwrap().clone();
    for index in 1..20_000 {
        let mut edge = template.clone();
        edge.id = RelationshipId::new();
        edge.external_id = edge.id.to_string();
        edge.source_id = nodes[index];
        edge.target_id = nodes[index + 1];
        project.relationships.insert(edge.id, edge);
    }
    let feature = project.create_element(ElementKind::Operation, "deep feature", nodes[20_000]).unwrap();
    assert_eq!(inherited_ids(&project, nodes[0]), vec![feature]);
}

#[test]
fn missing_and_non_classifier_queries_preserve_existing_errors() {
    let project = Project::new("Invalid query");
    let missing = ElementId::new();
    assert!(matches!(project.inherited_features(missing), Err(ModelError::ElementNotFound(id)) if id == missing));
    assert!(matches!(project.inherited_features(project.root_id), Err(ModelError::GeneralizationRequiresClassifiers)));
}
