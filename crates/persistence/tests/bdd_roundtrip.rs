use systems_modeler_core::{AggregationKind, ElementKind, Multiplicity, Project};
use systems_modeler_persistence::ProjectDatabase;

#[test]
fn bdd_semantics_round_trip_through_sqlite() {
    let mut project = Project::new("Drone");
    let structure = project
        .create_element(ElementKind::Package, "Structure", project.root_id)
        .unwrap();
    let drone = project
        .create_element(ElementKind::Block, "Drone", structure)
        .unwrap();
    let battery = project
        .create_element(ElementKind::Block, "Battery", structure)
        .unwrap();
    let mass_type = project
        .create_element(ElementKind::ValueType, "Mass", structure)
        .unwrap();
    project
        .element_mut(mass_type)
        .unwrap()
        .quantity_kind_external_id = Some("Mass".into());
    project.element_mut(mass_type).unwrap().unit_external_id = Some("kg".into());

    let part = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "battery",
            drone,
            battery,
            Multiplicity::ONE,
        )
        .unwrap();
    let value = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "mass",
            drone,
            mass_type,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(value).unwrap().default_value = Some("2.5".into());

    project
        .create_association(
            Some(structure),
            vec![
                Project::association_end(
                    drone,
                    "drone",
                    Multiplicity::ONE,
                    false,
                    AggregationKind::Composite,
                ),
                Project::association_end(
                    battery,
                    "battery",
                    Multiplicity::ONE,
                    true,
                    AggregationKind::None,
                ),
            ],
        )
        .unwrap();

    let mut db = ProjectDatabase::open_in_memory().unwrap();
    db.save_project(&project).unwrap();
    let restored = db.load_project(project.id).unwrap();

    assert_eq!(restored.element(part).unwrap().type_id, Some(battery));
    assert_eq!(
        restored.element(part).unwrap().aggregation,
        AggregationKind::Composite
    );
    assert_eq!(
        restored.element(value).unwrap().default_value.as_deref(),
        Some("2.5")
    );
    assert_eq!(restored.relationships.len(), 1);
    let relation = restored.relationships.values().next().unwrap();
    assert_eq!(relation.association_ends.len(), 2);
    assert_eq!(relation.association_ends[1].role_name, "battery");
    assert!(restored.validate().is_ok());
}
