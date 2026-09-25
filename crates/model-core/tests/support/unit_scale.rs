use systems_modeler_core::{ElementKind, Project};

pub fn unit_project(count: usize) -> Project {
    let mut project = Project::new("Unit reference validation scale");
    let root = project.root_id;
    for index in 0..count {
        let quantity = project
            .create_element(ElementKind::QuantityKind, format!("Quantity {index}"), root)
            .unwrap();
        let unit = project
            .create_element(ElementKind::Unit, format!("Unit {index}"), root)
            .unwrap();
        let value = project
            .create_element(ElementKind::ValueType, format!("Value type {index}"), root)
            .unwrap();
        let quantity_external_id = project.elements[&quantity].external_id.clone();
        let unit_external_id = project.elements[&unit].external_id.clone();
        project
            .elements
            .get_mut(&unit)
            .unwrap()
            .quantity_kind_external_id = Some(quantity_external_id.clone());
        let value_type = project.elements.get_mut(&value).unwrap();
        value_type.quantity_kind_external_id = Some(quantity_external_id);
        value_type.unit_external_id = Some(unit_external_id);
    }
    project
}
