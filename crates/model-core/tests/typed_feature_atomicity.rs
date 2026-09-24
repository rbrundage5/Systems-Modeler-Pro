use systems_modeler_core::{AggregationKind, ElementId, ElementKind, Multiplicity, Project};

fn fixture() -> (Project, ElementId, ElementId) {
    let mut project = Project::new("Atomic typed features");
    let owner = project
        .create_element(ElementKind::Block, "System", project.root_id)
        .unwrap();
    let real = project
        .create_element(ElementKind::PrimitiveType, "Real", project.root_id)
        .unwrap();
    project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "existing",
            owner,
            real,
            Multiplicity::ONE,
        )
        .unwrap();
    (project, owner, real)
}

#[test]
fn missing_or_incompatible_type_never_leaves_an_orphan_feature() {
    let (mut project, owner, real) = fixture();
    let before = serde_json::to_value(&project).unwrap();
    for type_id in [ElementId::new(), real] {
        assert!(
            project
                .create_typed_feature(
                    ElementKind::PartProperty,
                    "rejected",
                    owner,
                    type_id,
                    Multiplicity::ONE
                )
                .is_err()
        );
        assert_eq!(serde_json::to_value(&project).unwrap(), before);
        project.validate().unwrap();
    }
}

#[test]
fn late_multiplicity_failure_restores_the_previous_complete_project() {
    let (mut project, owner, real) = fixture();
    let before = serde_json::to_value(&project).unwrap();
    let invalid = Multiplicity {
        lower: 4,
        upper: Some(2),
    };
    assert!(
        project
            .create_typed_feature(ElementKind::ValueProperty, "rejected", owner, real, invalid)
            .is_err()
    );
    assert_eq!(serde_json::to_value(&project).unwrap(), before);
    project.validate().unwrap();
}

#[test]
fn successful_creation_preserves_part_and_port_initialization() {
    let (mut project, owner, _) = fixture();
    let interface = project
        .create_element(ElementKind::InterfaceBlock, "Control", project.root_id)
        .unwrap();
    let part = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "part",
            owner,
            owner,
            Multiplicity::ONE,
        )
        .unwrap();
    let port = project
        .create_typed_feature(
            ElementKind::ProxyPort,
            "port",
            owner,
            interface,
            Multiplicity::ONE,
        )
        .unwrap();
    assert_eq!(
        project.element(part).unwrap().aggregation,
        AggregationKind::Composite
    );
    assert_eq!(project.element(part).unwrap().type_id, Some(owner));
    assert_eq!(project.element(port).unwrap().type_id, Some(interface));
    assert_eq!(
        project.element(port).unwrap().multiplicity,
        Some(Multiplicity::ONE)
    );
    project.validate().unwrap();
}
