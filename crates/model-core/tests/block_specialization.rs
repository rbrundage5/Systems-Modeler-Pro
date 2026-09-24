use systems_modeler_core::{ElementKind, ModelError, Project, RelationshipKind};

const BLOCK_KINDS: [ElementKind; 4] = [
    ElementKind::Block,
    ElementKind::AssociationBlock,
    ElementKind::InterfaceBlock,
    ElementKind::ConstraintBlock,
];

#[test]
fn block_specialization_accepts_builtin_block_stereotype_specializations() {
    for specific in &BLOCK_KINDS {
        for general in &BLOCK_KINDS {
            let mut project = Project::new("Block inheritance");
            let child = project
                .create_element(specific.clone(), "Specific", project.root_id)
                .unwrap();
            let parent = project
                .create_element(general.clone(), "General", project.root_id)
                .unwrap();
            project
                .create_relationship(
                    RelationshipKind::Generalization,
                    child,
                    parent,
                    Some(project.root_id),
                )
                .unwrap();
            project.validate().unwrap();
        }
    }
}

#[test]
fn non_block_specializations_are_rejected_before_mutation() {
    for general in &BLOCK_KINDS {
        for specific in [
            ElementKind::DataType,
            ElementKind::PrimitiveType,
            ElementKind::ValueType,
            ElementKind::Enumeration,
            ElementKind::Signal,
            ElementKind::Requirement,
            ElementKind::TestCase,
        ] {
            let mut project = Project::new("Invalid inheritance");
            let child = project
                .create_element(specific, "Specific", project.root_id)
                .unwrap();
            let parent = project
                .create_element(general.clone(), "General", project.root_id)
                .unwrap();
            let before = serde_json::to_value(&project).unwrap();
            assert_eq!(
                project.create_relationship(
                    RelationshipKind::Generalization,
                    child,
                    parent,
                    Some(project.root_id),
                ),
                Err(ModelError::BlockSpecializationRequiresBlock)
            );
            assert_eq!(serde_json::to_value(&project).unwrap(), before);
        }
    }
}

#[test]
fn deserialized_or_reconnected_non_block_specialization_fails_project_validation() {
    let mut project = Project::new("Reopen inheritance");
    let child = project
        .create_element(ElementKind::Block, "Specific", project.root_id)
        .unwrap();
    let parent = project
        .create_element(ElementKind::Block, "General", project.root_id)
        .unwrap();
    let relation = project
        .create_relationship(
            RelationshipKind::Generalization,
            child,
            parent,
            Some(project.root_id),
        )
        .unwrap();
    let signal = project
        .create_element(ElementKind::Signal, "Signal", project.root_id)
        .unwrap();
    project.relationships.get_mut(&relation).unwrap().source_id = signal;
    let reopened: Project =
        serde_json::from_str(&serde_json::to_string(&project).unwrap()).unwrap();
    assert_eq!(
        reopened.validate(),
        Err(ModelError::BlockSpecializationRequiresBlock)
    );
}

#[test]
fn ordinary_non_block_generalization_is_preserved() {
    let mut project = Project::new("Signal inheritance");
    let child = project
        .create_element(ElementKind::Signal, "Specific", project.root_id)
        .unwrap();
    let parent = project
        .create_element(ElementKind::Signal, "General", project.root_id)
        .unwrap();
    project
        .create_relationship(
            RelationshipKind::Generalization,
            child,
            parent,
            Some(project.root_id),
        )
        .unwrap();
    project.validate().unwrap();
}
