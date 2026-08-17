use systems_modeler_core::{
    AggregationKind, ElementKind, Multiplicity, Project, RelationshipKind,
};

fn bdd_project() -> (Project, systems_modeler_core::ElementId) {
    let mut project = Project::new("PR8 BDD");
    let package = project
        .create_element(ElementKind::Package, "Structure", project.root_id)
        .unwrap();
    (project, package)
}

#[test]
fn creates_supported_bdd_classifier_families_under_packages() {
    let (mut project, package) = bdd_project();
    for kind in [
        ElementKind::Block,
        ElementKind::InterfaceBlock,
        ElementKind::ValueType,
        ElementKind::DataType,
        ElementKind::Enumeration,
        ElementKind::ConstraintBlock,
    ] {
        let id = project
            .create_element(kind.clone(), format!("{kind:?}"), package)
            .unwrap();
        assert_eq!(project.element(id).unwrap().kind, kind);
    }
    project.validate().unwrap();
}

#[test]
fn enforces_owned_feature_type_and_aggregation_semantics() {
    let (mut project, package) = bdd_project();
    let vehicle = project
        .create_element(ElementKind::Block, "Vehicle", package)
        .unwrap();
    let wheel = project
        .create_element(ElementKind::Block, "Wheel", package)
        .unwrap();
    let mass_type = project
        .create_element(ElementKind::ValueType, "Mass", package)
        .unwrap();
    let constraint_type = project
        .create_element(ElementKind::ConstraintBlock, "MassConstraint", package)
        .unwrap();

    let part = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "wheel",
            vehicle,
            wheel,
            Multiplicity::new(4, Some(4)).unwrap(),
        )
        .unwrap();
    assert_eq!(
        project.element(part).unwrap().aggregation,
        AggregationKind::Composite
    );

    project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "mass",
            vehicle,
            mass_type,
            Multiplicity::ONE,
        )
        .unwrap();
    project
        .create_typed_feature(
            ElementKind::ConstraintProperty,
            "massRule",
            vehicle,
            constraint_type,
            Multiplicity::ONE,
        )
        .unwrap();

    project.validate().unwrap();
}

#[test]
fn preserves_unbounded_multiplicity_semantics() {
    let multiplicity = Multiplicity::new(1, None).unwrap();
    assert_eq!(multiplicity.notation(), "1..*");

    let (mut project, package) = bdd_project();
    let vehicle = project
        .create_element(ElementKind::Block, "Vehicle", package)
        .unwrap();
    let component = project
        .create_element(ElementKind::Block, "Component", package)
        .unwrap();
    let reference = project
        .create_typed_feature(
            ElementKind::ReferenceProperty,
            "components",
            vehicle,
            component,
            multiplicity,
        )
        .unwrap();
    assert_eq!(
        project.element(reference).unwrap().multiplicity.unwrap().notation(),
        "1..*"
    );
}

#[test]
fn supports_generalization_between_non_block_bdd_classifiers() {
    let (mut project, package) = bdd_project();
    let specific = project
        .create_element(ElementKind::ValueType, "Distance", package)
        .unwrap();
    let general = project
        .create_element(ElementKind::DataType, "Scalar", package)
        .unwrap();

    let relationship = project
        .create_relationship(
            RelationshipKind::Generalization,
            specific,
            general,
            Some(package),
        )
        .unwrap();
    assert_eq!(project.relationship(relationship).unwrap().source_id, specific);
    assert_eq!(project.relationship(relationship).unwrap().target_id, general);
    project.validate().unwrap();
}

#[test]
fn enumeration_literals_are_owned_only_by_enumerations() {
    let (mut project, package) = bdd_project();
    let enumeration = project
        .create_element(ElementKind::Enumeration, "Mode", package)
        .unwrap();
    project
        .create_element(ElementKind::EnumerationLiteral, "AUTO", enumeration)
        .unwrap();

    let block = project
        .create_element(ElementKind::Block, "Controller", package)
        .unwrap();
    assert!(
        project
            .create_element(ElementKind::EnumerationLiteral, "INVALID", block)
            .is_err()
    );
    project.validate().unwrap();
}
