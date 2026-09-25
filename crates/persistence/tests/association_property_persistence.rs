use systems_modeler_core::{ElementKind, Multiplicity, Project};
use systems_modeler_persistence::ProjectDatabase;

#[test]
fn linked_composition_preserves_property_and_end_identity_through_sqlite() {
    let mut project = Project::new("Linked composition");
    let whole = project
        .create_element(ElementKind::Block, "Vehicle", project.root_id)
        .unwrap();
    let part_type = project
        .create_element(ElementKind::Block, "Wheel", project.root_id)
        .unwrap();
    let (relation, property) = project
        .create_composition(
            whole,
            part_type,
            "front",
            Multiplicity::new(2, Some(2)).unwrap(),
            Some(project.root_id),
        )
        .unwrap();
    let expected = serde_json::to_value(&project).unwrap();
    let mut database = ProjectDatabase::open_in_memory().unwrap();
    database.save_project(&project).unwrap();
    let reopened = database.load_project(project.id).unwrap();
    reopened.validate().unwrap();
    assert_eq!(serde_json::to_value(&reopened).unwrap(), expected);
    assert_eq!(
        reopened.relationship(relation).unwrap().association_ends[1].property_id,
        Some(property)
    );
    assert_eq!(reopened.element(property).unwrap().type_id, Some(part_type));
}
