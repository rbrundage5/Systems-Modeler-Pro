use systems_modeler_core::{ElementId, ElementKind, ModelError, Project};

#[path = "support/unit_scale.rs"]
mod unit_scale;

fn element_id(project: &Project, kind: ElementKind) -> ElementId {
    project
        .elements
        .values()
        .find(|element| element.kind == kind)
        .unwrap()
        .id
}

fn assert_rejected_without_mutation(project: &Project, id: ElementId, error: ModelError) {
    let before = serde_json::to_value(project).unwrap();
    assert_eq!(project.validate_element(id).unwrap_err(), error);
    assert_eq!(project.validate().unwrap_err(), error);
    assert_eq!(serde_json::to_value(project).unwrap(), before);
}

#[test]
fn valid_reference_graph_survives_serialization_and_standalone_validation() {
    let project = unit_scale::unit_project(20);
    let reopened: Project = serde_json::from_value(serde_json::to_value(project).unwrap()).unwrap();
    assert_eq!(reopened.validate(), Ok(()));
    for element in reopened.elements.values() {
        assert_eq!(reopened.validate_element(element.id), Ok(()));
    }
}

#[test]
fn missing_quantity_then_missing_unit_preserve_diagnostics_and_state() {
    let mut project = unit_scale::unit_project(1);
    let id = element_id(&project, ElementKind::ValueType);
    let original_quantity = project.elements[&id].quantity_kind_external_id.clone();
    let element = project.elements.get_mut(&id).unwrap();
    element.quantity_kind_external_id = Some("missing-quantity".into());
    element.unit_external_id = Some("missing-unit".into());
    assert_rejected_without_mutation(
        &project,
        id,
        ModelError::InvalidQuantityKindReference("missing-quantity".into()),
    );
    project
        .elements
        .get_mut(&id)
        .unwrap()
        .quantity_kind_external_id = original_quantity;
    assert_rejected_without_mutation(
        &project,
        id,
        ModelError::InvalidUnitReference("missing-unit".into()),
    );
}

#[test]
fn existing_external_identity_must_resolve_to_the_correct_kind() {
    for quantity_reference in [true, false] {
        let mut project = unit_scale::unit_project(1);
        let id = element_id(&project, ElementKind::ValueType);
        let wrong_kind = if quantity_reference {
            ElementKind::Unit
        } else {
            ElementKind::QuantityKind
        };
        let wrong_id = element_id(&project, wrong_kind);
        let external_id = project.elements[&wrong_id].external_id.clone();
        let element = project.elements.get_mut(&id).unwrap();
        let error = if quantity_reference {
            element.quantity_kind_external_id = Some(external_id.clone());
            ModelError::InvalidQuantityKindReference(external_id)
        } else {
            element.unit_external_id = Some(external_id.clone());
            ModelError::InvalidUnitReference(external_id)
        };
        assert_rejected_without_mutation(&project, id, error);
    }
}

#[test]
fn external_identity_edits_are_visible_on_the_next_validation() {
    let mut project = unit_scale::unit_project(1);
    let unit = element_id(&project, ElementKind::Unit);
    let value = element_id(&project, ElementKind::ValueType);
    let old_external_id = project.elements[&unit].external_id.clone();
    assert_eq!(project.validate(), Ok(()));
    project.elements.get_mut(&unit).unwrap().external_id = "renamed-unit".into();
    assert_rejected_without_mutation(
        &project,
        value,
        ModelError::InvalidUnitReference(old_external_id),
    );
    project.elements.get_mut(&value).unwrap().unit_external_id = Some("renamed-unit".into());
    assert_eq!(project.validate(), Ok(()));
    assert_eq!(project.validate_element(value), Ok(()));
}

#[test]
fn duplicate_external_ids_are_not_hidden_by_the_reference_index() {
    let mut project = unit_scale::unit_project(1);
    let unit = element_id(&project, ElementKind::Unit);
    let external_id = project.elements[&unit].external_id.clone();
    let duplicate = project
        .create_element(ElementKind::Block, "Duplicate identity", project.root_id)
        .unwrap();
    project.elements.get_mut(&duplicate).unwrap().external_id = external_id.clone();
    assert_eq!(
        project.validate(),
        Err(ModelError::DuplicateExternalId(external_id))
    );
}
