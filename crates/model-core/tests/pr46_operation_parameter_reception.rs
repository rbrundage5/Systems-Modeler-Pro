use systems_modeler_core::{ElementKind, ModelError, Multiplicity, ParameterDirection, Project};

#[test]
fn pr46_native_operation_parameter_reception_contracts_remain_authoritative() {
    let mut project = Project::new("PR46 Native");
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
            "mode",
            operation,
            integer,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(parameter).unwrap().parameter_direction = Some(ParameterDirection::In);
    let reception = project
        .create_element(ElementKind::Reception, "startRequest", controller)
        .unwrap();
    project.set_element_type(reception, signal).unwrap();
    project.validate().unwrap();

    assert!(
        project
            .create_element(ElementKind::Parameter, "bad", controller)
            .is_err()
    );
    let block = project
        .create_element(ElementKind::Block, "NotSignal", project.root_id)
        .unwrap();
    assert!(matches!(
        project.set_element_type(reception, block),
        Err(ModelError::InvalidTypeKind {
            kind: ElementKind::Reception,
            type_kind: ElementKind::Block
        })
    ));
}
