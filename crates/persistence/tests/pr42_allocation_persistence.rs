use systems_modeler_core::{ElementKind, Project, RelationshipKind};
use systems_modeler_persistence::ProjectDatabase;

#[test]
fn pr42_allocate_round_trips_through_native_project_database() {
    let mut project = Project::new("Allocation Persistence");
    let package = project
        .create_element(ElementKind::Package, "Architecture", project.root_id)
        .unwrap();
    let source = project
        .create_element(ElementKind::Block, "LogicalController", package)
        .unwrap();
    let target = project
        .create_element(ElementKind::Block, "PhysicalController", package)
        .unwrap();
    let id = project
        .create_relationship(RelationshipKind::Allocate, source, target, Some(package))
        .unwrap();
    {
        let relationship = project.relationships.get_mut(&id).unwrap();
        relationship.external_id = "catia:pr42::ALLOC-PERSIST".into();
        relationship.name = "Controller allocation".into();
        relationship.documentation = "Persisted allocation".into();
    }

    let mut db = ProjectDatabase::open_in_memory().unwrap();
    db.save_project(&project).unwrap();
    let restored = db.load_project(project.id).unwrap();
    let allocation = restored.relationship(id).unwrap();

    assert_eq!(allocation.kind, RelationshipKind::Allocate);
    assert_eq!(allocation.id, id);
    assert_eq!(allocation.external_id, "catia:pr42::ALLOC-PERSIST");
    assert_eq!(allocation.owner_id, Some(package));
    assert_eq!(allocation.source_id, source);
    assert_eq!(allocation.target_id, target);
    assert_eq!(allocation.name, "Controller allocation");
    assert_eq!(allocation.documentation, "Persisted allocation");
    restored.validate().unwrap();
}
