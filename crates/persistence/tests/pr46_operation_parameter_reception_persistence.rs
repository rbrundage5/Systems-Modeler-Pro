use systems_modeler_core::{ElementKind, Multiplicity, ParameterDirection, Project};
use systems_modeler_persistence::ProjectDatabase;

#[test]
fn pr46_database_round_trip_preserves_operation_parameter_reception_semantics() {
    let mut project = Project::new("PR46 Persistence");
    let controller = project
        .create_element(ElementKind::Block, "Controller", project.root_id)
        .unwrap();
    let integer = project
        .create_element(ElementKind::PrimitiveType, "Integer", project.root_id)
        .unwrap();
    let signal = project
        .create_element(ElementKind::Signal, "StartSignal", project.root_id)
        .unwrap();
    let operation = project
        .create_element(ElementKind::Operation, "start", controller)
        .unwrap();
    let parameter = project
        .create_typed_feature(
            ElementKind::Parameter,
            "result",
            operation,
            integer,
            Multiplicity::new(1, Some(4)).unwrap(),
        )
        .unwrap();
    {
        let parameter = project.element_mut(parameter).unwrap();
        parameter.parameter_direction = Some(ParameterDirection::Return);
        parameter.default_value = Some("7".into());
        parameter.external_id = "catia:pr46::PARAM-RESULT".into();
    }
    let reception = project
        .create_element(ElementKind::Reception, "startRequest", controller)
        .unwrap();
    project.set_element_type(reception, signal).unwrap();
    project.element_mut(operation).unwrap().external_id = "catia:pr46::OP-START".into();
    project.element_mut(reception).unwrap().external_id = "catia:pr46::RECP-START".into();
    project.validate().unwrap();

    let mut database = ProjectDatabase::open_in_memory().unwrap();
    database.save_project(&project).unwrap();
    let restored = database.load_project(project.id).unwrap();
    let op = restored.element(operation).unwrap();
    let param = restored.element(parameter).unwrap();
    let rec = restored.element(reception).unwrap();
    assert_eq!(op.owner_id, Some(controller));
    assert_eq!(param.owner_id, Some(operation));
    assert_eq!(param.type_id, Some(integer));
    assert_eq!(param.parameter_direction, Some(ParameterDirection::Return));
    assert_eq!(
        param.multiplicity,
        Some(Multiplicity::new(1, Some(4)).unwrap())
    );
    assert_eq!(param.default_value.as_deref(), Some("7"));
    assert_eq!(rec.owner_id, Some(controller));
    assert_eq!(rec.type_id, Some(signal));
    restored.validate().unwrap();
}
