use systems_modeler_core::{
    AggregationKind, ElementKind, ModelError, Multiplicity, Project, RelationshipKind, notation,
};

#[test]
fn bdd_supports_structural_definition_and_cross_diagram_identity() {
    let mut project = Project::new("Drone");
    let structure = project
        .create_element(ElementKind::Package, "Structure", project.root_id)
        .unwrap();
    let drone = project
        .create_element(ElementKind::Block, "Drone", structure)
        .unwrap();
    let battery = project
        .create_element(ElementKind::Block, "Battery", structure)
        .unwrap();
    let quantity_kind = project
        .create_element(ElementKind::QuantityKind, "Mass", structure)
        .unwrap();
    let unit = project
        .create_element(ElementKind::Unit, "kg", structure)
        .unwrap();
    let mass = project
        .create_element(ElementKind::ValueType, "MassValue", structure)
        .unwrap();

    let quantity_kind_external_id = project.element(quantity_kind).unwrap().external_id.clone();
    let unit_external_id = project.element(unit).unwrap().external_id.clone();
    project.element_mut(mass).unwrap().quantity_kind_external_id = Some(quantity_kind_external_id);
    project.element_mut(mass).unwrap().unit_external_id = Some(unit_external_id);

    let battery_part = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "battery",
            drone,
            battery,
            Multiplicity::ONE,
        )
        .unwrap();
    let mass_value = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "mass",
            drone,
            mass,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(mass_value).unwrap().default_value = Some("2.5".into());

    assert_eq!(
        project.element(battery_part).unwrap().aggregation,
        AggregationKind::Composite
    );
    assert_eq!(project.element(mass_value).unwrap().type_id, Some(mass));
    assert_eq!(project.element(drone).unwrap().id, drone);
    assert!(project.validate().is_ok());
}

#[test]
fn bdd_association_ends_are_semantic_not_visual_only() {
    let mut project = Project::new("Vehicle");
    let structure = project
        .create_element(ElementKind::Package, "Structure", project.root_id)
        .unwrap();
    let vehicle = project
        .create_element(ElementKind::Block, "Vehicle", structure)
        .unwrap();
    let wheel = project
        .create_element(ElementKind::Block, "Wheel", structure)
        .unwrap();

    let association = project
        .create_association(
            Some(structure),
            vec![
                Project::association_end(
                    vehicle,
                    "vehicle",
                    Multiplicity::ONE,
                    false,
                    AggregationKind::Composite,
                ),
                Project::association_end(
                    wheel,
                    "wheels",
                    Multiplicity::new(4, Some(4)).unwrap(),
                    true,
                    AggregationKind::None,
                ),
            ],
        )
        .unwrap();

    let relation = project.relationship(association).unwrap();
    assert_eq!(relation.association_ends[0].role_name, "vehicle");
    assert_eq!(relation.association_ends[1].role_name, "wheels");
    assert_eq!(relation.association_ends[1].multiplicity.notation(), "4");
    assert!(relation.association_ends[1].navigable);
    assert_eq!(
        notation::relationship_notation(relation).source_decoration,
        notation::EndDecoration::FilledDiamond
    );
}

#[test]
fn bdd_rejects_semantically_invalid_feature_typing() {
    let mut project = Project::new("Invalid");
    let structure = project
        .create_element(ElementKind::Package, "Structure", project.root_id)
        .unwrap();
    let block = project
        .create_element(ElementKind::Block, "Vehicle", structure)
        .unwrap();
    let constraint = project
        .create_element(ElementKind::ConstraintBlock, "MassEquation", structure)
        .unwrap();

    let result = project.create_typed_feature(
        ElementKind::PartProperty,
        "notAValidPart",
        block,
        constraint,
        Multiplicity::ONE,
    );
    assert!(matches!(result, Err(ModelError::InvalidTypeKind { .. })));
}

#[test]
fn bdd_relationship_notation_matches_sysml_uml_conventions() {
    let mut project = Project::new("Notation");
    let structure = project
        .create_element(ElementKind::Package, "Structure", project.root_id)
        .unwrap();
    let specific = project
        .create_element(ElementKind::Block, "Specific", structure)
        .unwrap();
    let general = project
        .create_element(ElementKind::Block, "General", structure)
        .unwrap();
    let generalization = project
        .create_relationship(
            RelationshipKind::Generalization,
            specific,
            general,
            Some(structure),
        )
        .unwrap();

    let notation = notation::relationship_notation(project.relationship(generalization).unwrap());
    assert_eq!(notation.line, notation::LineStyle::Solid);
    assert_eq!(
        notation.target_decoration,
        notation::EndDecoration::HollowTriangle
    );
}
