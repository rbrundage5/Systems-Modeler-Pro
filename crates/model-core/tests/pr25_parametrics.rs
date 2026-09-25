use systems_modeler_core::{
    BindingEndpoint, ElementId, ElementKind, ModelError, Multiplicity, ParametricEvaluationScope,
    Project, RelationshipKind, evaluate_parametrics,
};

fn definition(
    project: &mut Project,
    package: ElementId,
    name: &str,
    dimension: &str,
    unit_name: &str,
    unit_symbol: &str,
    unit_scale: f64,
) -> (ElementId, ElementId, ElementId) {
    let quantity = project
        .create_element(ElementKind::QuantityKind, name, package)
        .expect("QuantityKind");
    project
        .element_mut(quantity)
        .expect("QuantityKind")
        .quantity_dimension = Some(dimension.into());
    let quantity_external = project.element(quantity).unwrap().external_id.clone();
    let unit = project
        .create_element(ElementKind::Unit, unit_name, package)
        .expect("Unit");
    {
        let unit = project.element_mut(unit).unwrap();
        unit.quantity_kind_external_id = Some(quantity_external.clone());
        unit.unit_symbol = Some(unit_symbol.into());
        unit.unit_scale_to_base = unit_scale;
    }
    let unit_external = project.element(unit).unwrap().external_id.clone();
    let value_type = project
        .create_element(ElementKind::ValueType, name, package)
        .expect("ValueType");
    {
        let value_type = project.element_mut(value_type).unwrap();
        value_type.quantity_kind_external_id = Some(quantity_external);
        value_type.unit_external_id = Some(unit_external);
    }
    (quantity, unit, value_type)
}

fn endpoint(role_id: ElementId, parameter_id: Option<ElementId>) -> BindingEndpoint {
    BindingEndpoint {
        role_id,
        parameter_id,
    }
}

#[test]
fn kinetic_energy_evaluates_with_units_and_reusable_definition_parameters() {
    let mut project = Project::new("Parametrics");
    let package = project
        .create_element(ElementKind::Package, "Analysis", project.root_id)
        .unwrap();
    let context = project
        .create_element(ElementKind::Block, "Vehicle", package)
        .unwrap();
    let (_, _, mass_type) = definition(&mut project, package, "Mass", "M", "kilogram", "kg", 1.0);
    let (_, _, velocity_type) = definition(
        &mut project,
        package,
        "Velocity",
        "L*T^-1",
        "metres per second",
        "m/s",
        1.0,
    );
    let (_, _, energy_type) = definition(
        &mut project,
        package,
        "Energy",
        "M*L^2*T^-2",
        "joule",
        "J",
        1.0,
    );
    let block = project
        .create_element(ElementKind::ConstraintBlock, "KineticEnergy", package)
        .unwrap();
    let mass_parameter = project
        .create_typed_feature(
            ElementKind::ConstraintParameter,
            "mass",
            block,
            mass_type,
            Multiplicity::ONE,
        )
        .unwrap();
    let velocity_parameter = project
        .create_typed_feature(
            ElementKind::ConstraintParameter,
            "velocity",
            block,
            velocity_type,
            Multiplicity::ONE,
        )
        .unwrap();
    let energy_parameter = project
        .create_typed_feature(
            ElementKind::ConstraintParameter,
            "energy",
            block,
            energy_type,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(block).unwrap().constraint_expression =
        "energy = 0.5 * mass * velocity^2".into();

    let constraint = project
        .create_typed_feature(
            ElementKind::ConstraintProperty,
            "kineticEnergy",
            context,
            block,
            Multiplicity::ONE,
        )
        .unwrap();
    let mass = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "mass",
            context,
            mass_type,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(mass).unwrap().default_value = Some("1500 kg".into());
    let velocity = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "velocity",
            context,
            velocity_type,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(velocity).unwrap().default_value = Some("20 m/s".into());
    let energy = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "energy",
            context,
            energy_type,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(energy).unwrap().is_derived = true;

    let bindings = [
        (mass_parameter, mass),
        (velocity_parameter, velocity),
        (energy_parameter, energy),
    ]
    .map(|(parameter, value)| {
        project
            .create_binding_connector(
                context,
                endpoint(constraint, Some(parameter)),
                endpoint(value, None),
            )
            .unwrap()
    });
    project.validate().expect("valid Parametric model");

    let report = evaluate_parametrics(
        &mut project,
        &ParametricEvaluationScope {
            context_id: context,
            constraint_property_ids: vec![constraint],
            value_property_ids: vec![mass, velocity, energy],
            binding_relationship_ids: bindings.to_vec(),
        },
    )
    .expect("deterministic evaluation");
    assert_eq!(report.evaluated_constraints, 1);
    assert_eq!(report.updates.len(), 1);
    assert_eq!(
        project.element(energy).unwrap().default_value.as_deref(),
        Some("300000 J")
    );
    assert_eq!(
        project.element(mass_parameter).unwrap().owner_id,
        Some(block)
    );
}

#[test]
fn binding_rejects_incompatible_quantity_kinds_and_self_connections() {
    let mut project = Project::new("Bindings");
    let package = project
        .create_element(ElementKind::Package, "Analysis", project.root_id)
        .unwrap();
    let context = project
        .create_element(ElementKind::Block, "System", package)
        .unwrap();
    let (_, _, mass_type) = definition(&mut project, package, "Mass", "M", "kg", "kg", 1.0);
    let (_, _, time_type) = definition(&mut project, package, "Time", "T", "second", "s", 1.0);
    let mass = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "mass",
            context,
            mass_type,
            Multiplicity::ONE,
        )
        .unwrap();
    let time = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "time",
            context,
            time_type,
            Multiplicity::ONE,
        )
        .unwrap();
    let error = project
        .create_binding_connector(context, endpoint(mass, None), endpoint(time, None))
        .unwrap_err();
    assert!(matches!(error, ModelError::IncompatibleBindingTypes { .. }));
    assert_eq!(
        project
            .create_binding_connector(context, endpoint(mass, None), endpoint(mass, None))
            .unwrap_err(),
        ModelError::BindingSelfConnection
    );
}

#[test]
fn project_validation_rejects_duplicate_binding_connectors_from_loaded_data() {
    let mut project = Project::new("Duplicate bindings");
    let package = project
        .create_element(ElementKind::Package, "Analysis", project.root_id)
        .unwrap();
    let context = project
        .create_element(ElementKind::Block, "System", package)
        .unwrap();
    let (_, _, scalar) = definition(&mut project, package, "Scalar", "1", "one", "1", 1.0);
    let left = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "left",
            context,
            scalar,
            Multiplicity::ONE,
        )
        .unwrap();
    let right = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "right",
            context,
            scalar,
            Multiplicity::ONE,
        )
        .unwrap();
    let first = project
        .create_binding_connector(context, endpoint(left, None), endpoint(right, None))
        .unwrap();
    let binding = project.relationship(first).unwrap().binding.clone();
    let duplicate = project
        .create_relationship(
            RelationshipKind::BindingConnector,
            left,
            right,
            Some(context),
        )
        .unwrap();
    project.relationships.get_mut(&duplicate).unwrap().binding = binding;

    assert_eq!(
        project.validate().unwrap_err(),
        ModelError::DuplicateBindingConnector
    );
}

#[test]
fn evaluation_reports_unbound_mandatory_parameters_without_mutating_values() {
    let mut project = Project::new("Unbound");
    let package = project
        .create_element(ElementKind::Package, "Analysis", project.root_id)
        .unwrap();
    let context = project
        .create_element(ElementKind::Block, "System", package)
        .unwrap();
    let (_, _, scalar) = definition(&mut project, package, "Scalar", "1", "one", "1", 1.0);
    let block = project
        .create_element(ElementKind::ConstraintBlock, "Double", package)
        .unwrap();
    project
        .create_typed_feature(
            ElementKind::ConstraintParameter,
            "input",
            block,
            scalar,
            Multiplicity::ONE,
        )
        .unwrap();
    project
        .create_typed_feature(
            ElementKind::ConstraintParameter,
            "output",
            block,
            scalar,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(block).unwrap().constraint_expression = "output = input * 2".into();
    let constraint = project
        .create_typed_feature(
            ElementKind::ConstraintProperty,
            "double",
            context,
            block,
            Multiplicity::ONE,
        )
        .unwrap();
    let error = evaluate_parametrics(
        &mut project,
        &ParametricEvaluationScope {
            context_id: context,
            constraint_property_ids: vec![constraint],
            value_property_ids: vec![],
            binding_relationship_ids: vec![],
        },
    )
    .unwrap_err();
    assert!(
        matches!(error, ModelError::ParametricEvaluation(message) if message.contains("unbound"))
    );
}

#[test]
fn evaluation_rejects_constraint_dependency_cycles() {
    let mut project = Project::new("Cycle");
    let package = project
        .create_element(ElementKind::Package, "Analysis", project.root_id)
        .unwrap();
    let context = project
        .create_element(ElementKind::Block, "System", package)
        .unwrap();
    let (_, _, scalar) = definition(&mut project, package, "Scalar", "1", "one", "1", 1.0);
    let x = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "x",
            context,
            scalar,
            Multiplicity::ONE,
        )
        .unwrap();
    let y = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "y",
            context,
            scalar,
            Multiplicity::ONE,
        )
        .unwrap();

    let mut constraints = Vec::new();
    let mut bindings = Vec::new();
    for (name, input_value, output_value) in [("makeX", y, x), ("makeY", x, y)] {
        let block = project
            .create_element(ElementKind::ConstraintBlock, name, package)
            .unwrap();
        let input = project
            .create_typed_feature(
                ElementKind::ConstraintParameter,
                "input",
                block,
                scalar,
                Multiplicity::ONE,
            )
            .unwrap();
        let output = project
            .create_typed_feature(
                ElementKind::ConstraintParameter,
                "output",
                block,
                scalar,
                Multiplicity::ONE,
            )
            .unwrap();
        project.element_mut(block).unwrap().constraint_expression = "output = input".into();
        let property = project
            .create_typed_feature(
                ElementKind::ConstraintProperty,
                name,
                context,
                block,
                Multiplicity::ONE,
            )
            .unwrap();
        constraints.push(property);
        bindings.push(
            project
                .create_binding_connector(
                    context,
                    endpoint(property, Some(input)),
                    endpoint(input_value, None),
                )
                .unwrap(),
        );
        bindings.push(
            project
                .create_binding_connector(
                    context,
                    endpoint(property, Some(output)),
                    endpoint(output_value, None),
                )
                .unwrap(),
        );
    }
    let error = evaluate_parametrics(
        &mut project,
        &ParametricEvaluationScope {
            context_id: context,
            constraint_property_ids: constraints,
            value_property_ids: vec![x, y],
            binding_relationship_ids: bindings,
        },
    )
    .unwrap_err();
    assert!(
        matches!(error, ModelError::ParametricEvaluation(message) if message.contains("cycle"))
    );
}
