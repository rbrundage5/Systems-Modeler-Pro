use systems_modeler_core::{BindingEndpoint, ElementKind, ModelError, Multiplicity, Project};

fn endpoint(
    role_id: systems_modeler_core::ElementId,
    parameter_id: Option<systems_modeler_core::ElementId>,
) -> BindingEndpoint {
    BindingEndpoint {
        role_id,
        parameter_id,
    }
}

#[test]
fn real_constraint_parameter_cannot_bind_to_mass_until_types_are_aligned() {
    let mut project = Project::new("Binding diagnostics");
    let package = project
        .create_element(ElementKind::Package, "Analysis", project.root_id)
        .unwrap();
    let context = project
        .create_element(ElementKind::Block, "Vehicle", package)
        .unwrap();

    let real = project
        .create_element(ElementKind::PrimitiveType, "Real", package)
        .unwrap();
    let mass_type = project
        .create_element(ElementKind::ValueType, "Mass", package)
        .unwrap();
    let constraint_block = project
        .create_element(ElementKind::ConstraintBlock, "KineticEnergy", package)
        .unwrap();
    let mass_parameter = project
        .create_typed_feature(
            ElementKind::ConstraintParameter,
            "mass",
            constraint_block,
            real,
            Multiplicity::ONE,
        )
        .unwrap();
    let constraint_property = project
        .create_typed_feature(
            ElementKind::ConstraintProperty,
            "constraint",
            context,
            constraint_block,
            Multiplicity::ONE,
        )
        .unwrap();
    let mass_value = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "mass",
            context,
            mass_type,
            Multiplicity::ONE,
        )
        .unwrap();

    let rejected = project
        .create_binding_connector(
            context,
            endpoint(constraint_property, Some(mass_parameter)),
            endpoint(mass_value, None),
        )
        .unwrap_err();
    assert!(matches!(
        rejected,
        ModelError::IncompatibleBindingTypes { .. }
    ));
    assert!(project.relationships.is_empty());

    project.element_mut(mass_parameter).unwrap().type_id = Some(mass_type);
    project.validate().unwrap();

    let binding = project
        .create_binding_connector(
            context,
            endpoint(constraint_property, Some(mass_parameter)),
            endpoint(mass_value, None),
        )
        .expect("aligned Mass endpoints must bind");
    assert!(project.relationship(binding).unwrap().binding.is_some());
}
