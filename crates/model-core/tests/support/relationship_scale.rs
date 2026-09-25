use systems_modeler_core::{ElementKind, Project, RelationshipId, RelationshipKind};

// Build the same shape as a loaded repository, without timing repeated authoring
// commands and their separate duplicate checks. Every directed pair is unique.
pub fn mixed_relationship_project(source_count: usize, target_count: usize) -> Project {
    let mut project = Project::new("Relationship validation scale");
    let root = project.root_id;
    let sources: Vec<_> = (0..source_count)
        .map(|index| {
            project
                .create_element(ElementKind::Block, format!("Source {index}"), root)
                .unwrap()
        })
        .collect();
    let targets: Vec<_> = (0..target_count)
        .map(|index| {
            project
                .create_element(ElementKind::Block, format!("Target {index}"), root)
                .unwrap()
        })
        .collect();
    let templates: Vec<_> = [
        RelationshipKind::Allocate,
        RelationshipKind::Trace,
        RelationshipKind::Dependency,
    ]
    .into_iter()
    .map(|kind| {
        let id = project
            .create_relationship(kind, sources[0], targets[0], Some(root))
            .unwrap();
        project.relationships[&id].clone()
    })
    .collect();
    project.relationships.clear();
    project.relationships.reserve(source_count * target_count);
    for (source_index, source) in sources.iter().enumerate() {
        for (target_index, target) in targets.iter().enumerate() {
            let index = source_index * target_count + target_index;
            let mut relationship = templates[index % templates.len()].clone();
            relationship.id = RelationshipId::new();
            relationship.external_id = format!("scale-relationship-{index}");
            relationship.source_id = *source;
            relationship.target_id = *target;
            project.relationships.insert(relationship.id, relationship);
        }
    }
    project
}
