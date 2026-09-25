use super::*;
use systems_modeler_core::{ElementKind, Multiplicity, ParameterDirection};

#[test]
fn pr46_portable_json_round_trip_preserves_operation_parameter_reception_semantics() {
    let source_state = WorkspaceState::default();
    let source_activity = ActivityWorkspaceState::default();
    let mut project = Project::new("PR46 Portable");
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
            Multiplicity::new(0, Some(1)).unwrap(),
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
    *source_state.project.lock().unwrap() = Some(project);

    let json = export_from_states(&source_state, &source_activity).unwrap();
    let target_state = WorkspaceState::default();
    let target_activity = ActivityWorkspaceState::default();
    import_into_states(&json, &target_state, &target_activity).unwrap();

    let guard = target_state.project.lock().unwrap();
    let restored = guard.as_ref().unwrap();
    let op = restored.element(operation).unwrap();
    let param = restored.element(parameter).unwrap();
    let rec = restored.element(reception).unwrap();
    assert_eq!(op.owner_id, Some(controller));
    assert_eq!(op.external_id, "catia:pr46::OP-START");
    assert_eq!(param.owner_id, Some(operation));
    assert_eq!(param.type_id, Some(integer));
    assert_eq!(param.parameter_direction, Some(ParameterDirection::Return));
    assert_eq!(
        param.multiplicity,
        Some(Multiplicity::new(0, Some(1)).unwrap())
    );
    assert_eq!(param.default_value.as_deref(), Some("7"));
    assert_eq!(rec.owner_id, Some(controller));
    assert_eq!(rec.type_id, Some(signal));
    assert_eq!(rec.external_id, "catia:pr46::RECP-START");
    restored.validate().unwrap();
}
