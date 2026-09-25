use systems_modeler_core::{BindingEndpoint, ElementKind, Multiplicity, Project};
use systems_modeler_persistence::ProjectDatabase;

#[test]
fn parametric_semantics_and_binding_endpoints_round_trip_through_sqlite() {
    let mut project = Project::new("Parametric persistence");
    let package = project
        .create_element(ElementKind::Package, "Analysis", project.root_id)
        .unwrap();
    let context = project
        .create_element(ElementKind::Block, "Vehicle", package)
        .unwrap();
    let quantity = project
        .create_element(ElementKind::QuantityKind, "Mass", package)
        .unwrap();
    project.element_mut(quantity).unwrap().quantity_dimension = Some("M".into());
    let quantity_external = project.element(quantity).unwrap().external_id.clone();
    let unit = project
        .create_element(ElementKind::Unit, "kilogram", package)
        .unwrap();
    {
        let unit = project.element_mut(unit).unwrap();
        unit.quantity_kind_external_id = Some(quantity_external.clone());
        unit.unit_symbol = Some("kg".into());
        unit.unit_scale_to_base = 1.0;
    }
    let unit_external = project.element(unit).unwrap().external_id.clone();
    let value_type = project
        .create_element(ElementKind::ValueType, "Mass", package)
        .unwrap();
    {
        let value_type = project.element_mut(value_type).unwrap();
        value_type.quantity_kind_external_id = Some(quantity_external);
        value_type.unit_external_id = Some(unit_external);
    }
    let definition = project
        .create_element(ElementKind::ConstraintBlock, "Identity", package)
        .unwrap();
    let parameter = project
        .create_typed_feature(
            ElementKind::ConstraintParameter,
            "mass",
            definition,
            value_type,
            Multiplicity::ONE,
        )
        .unwrap();
    project
        .element_mut(definition)
        .unwrap()
        .constraint_expression = "mass = mass".into();
    let property = project
        .create_typed_feature(
            ElementKind::ConstraintProperty,
            "identity",
            context,
            definition,
            Multiplicity::ONE,
        )
        .unwrap();
    let value = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "mass",
            context,
            value_type,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(value).unwrap().default_value = Some("1500 kg".into());
    let binding = project
        .create_binding_connector(
            context,
            BindingEndpoint {
                role_id: property,
                parameter_id: Some(parameter),
            },
            BindingEndpoint {
                role_id: value,
                parameter_id: None,
            },
        )
        .unwrap();
    project.validate().unwrap();

    let mut database = ProjectDatabase::open_in_memory().unwrap();
    database.save_project(&project).unwrap();
    let restored = database.load_project(project.id).unwrap();
    restored.validate().unwrap();

    assert_eq!(
        restored.element(definition).unwrap().constraint_expression,
        "mass = mass"
    );
    assert_eq!(
        restored
            .element(quantity)
            .unwrap()
            .quantity_dimension
            .as_deref(),
        Some("M")
    );
    assert_eq!(
        restored.element(unit).unwrap().unit_symbol.as_deref(),
        Some("kg")
    );
    let connector = restored
        .relationship(binding)
        .unwrap()
        .binding
        .as_ref()
        .unwrap();
    assert_eq!(connector.source.role_id, property);
    assert_eq!(connector.source.parameter_id, Some(parameter));
    assert_eq!(connector.target.role_id, value);
}
