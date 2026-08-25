use systems_modeler_core::{ElementKind, Project, VisibilityKind};
use systems_modeler_persistence::ProjectDatabase;

#[test]
fn imported_names_and_aliases_resolve_after_project_round_trip() {
    let mut project = Project::new("ImportResolution");
    let root = project.root_id;
    let library = project
        .create_element(ElementKind::ModelLibrary, "CommonLibrary", root)
        .unwrap();
    let consumer = project
        .create_element(ElementKind::Package, "Vehicle", root)
        .unwrap();
    let vehicle_type = project
        .create_element(ElementKind::Block, "VehicleType", library)
        .unwrap();
    let mass = project
        .create_element(ElementKind::ValueType, "Mass", library)
        .unwrap();

    project
        .create_package_import(consumer, library, VisibilityKind::Public)
        .unwrap();
    project
        .create_element_import(
            consumer,
            vehicle_type,
            VisibilityKind::Public,
            Some("CarType".into()),
        )
        .unwrap();

    let original_owner = project.element(vehicle_type).unwrap().owner_id;
    let mut database = ProjectDatabase::open_in_memory().unwrap();
    database.save_project(&project).unwrap();
    let restored = database.load_project(project.id).unwrap();

    assert_eq!(restored.resolve_name(consumer, "Mass").unwrap(), mass);
    assert_eq!(
        restored.resolve_name(consumer, "CarType").unwrap(),
        vehicle_type
    );
    assert_eq!(
        restored.element(vehicle_type).unwrap().owner_id,
        original_owner
    );
}
