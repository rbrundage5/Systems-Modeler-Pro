use systems_modeler_core::{
    ElementId, ElementKind, ModelError, Project, RelationshipId, RelationshipKind,
};

fn fixture(kind: RelationshipKind) -> (Project, RelationshipId) {
    use RelationshipKind as R;
    let mut project = Project::new("Relationship validation");
    let root = project.root_id;
    let (source_kind, target_kind) = match kind {
        R::DeriveRequirement | R::Copy => (ElementKind::Requirement, ElementKind::Requirement),
        R::Satisfy => (ElementKind::Block, ElementKind::Requirement),
        R::Verify => (ElementKind::TestCase, ElementKind::Requirement),
        R::Refine => (ElementKind::Requirement, ElementKind::Block),
        R::PackageImport | R::ElementImport | R::PackageMerge => {
            (ElementKind::Package, ElementKind::Package)
        }
        _ => (ElementKind::Block, ElementKind::Block),
    };
    let mut endpoint = |kind, name| {
        if kind == ElementKind::Requirement {
            project.create_requirement(name, name, "Requirement text", root)
        } else {
            project.create_element(kind, name, root)
        }
        .unwrap()
    };
    let source = endpoint(source_kind, "Source");
    let target = endpoint(target_kind, "Target");
    let owner = match kind {
        R::PackageImport | R::ElementImport | R::PackageMerge => source,
        _ => root,
    };
    let id = project
        .create_relationship(kind, source, target, Some(owner))
        .unwrap();
    assert_eq!(project.validate(), Ok(()));
    (project, id)
}

fn insert_duplicate(project: &mut Project, id: RelationshipId) -> RelationshipId {
    let mut duplicate = project.relationships[&id].clone();
    duplicate.id = RelationshipId::new();
    duplicate.external_id = duplicate.id.to_string();
    let new_id = duplicate.id;
    project.relationships.insert(new_id, duplicate);
    new_id
}

#[test]
fn every_existing_unique_relationship_kind_rejects_duplicate_without_mutation() {
    use RelationshipKind as R;
    for kind in [
        R::Allocate,
        R::DeriveRequirement,
        R::Satisfy,
        R::Verify,
        R::Refine,
        R::Trace,
        R::Copy,
        R::Dependency,
        R::PackageImport,
        R::ElementImport,
        R::PackageMerge,
    ] {
        let (mut project, id) = fixture(kind.clone());
        insert_duplicate(&mut project, id);
        let relationship = &project.relationships[&id];
        let expected = match kind {
            R::Allocate => ModelError::DuplicateAllocationRelationship {
                source_id: relationship.source_id,
                target_id: relationship.target_id,
            },
            R::Dependency | R::PackageImport | R::ElementImport | R::PackageMerge => {
                ModelError::DuplicatePackageRelationship {
                    relationship: kind,
                    source_name: "Source".into(),
                    target_name: "Target".into(),
                }
            }
            _ => ModelError::DuplicateTraceabilityRelationship {
                relationship: kind,
                source_id: relationship.source_id,
                target_id: relationship.target_id,
            },
        };
        let before = serde_json::to_value(&project).unwrap();
        assert_eq!(project.validate(), Err(expected));
        assert_eq!(serde_json::to_value(&project).unwrap(), before);
    }
}

#[test]
fn direction_kind_and_parallel_associations_remain_distinct() {
    let (mut project, id) = fixture(RelationshipKind::Trace);
    let original = project.relationships[&id].clone();
    for kind in [
        RelationshipKind::Allocate,
        RelationshipKind::Trace,
        RelationshipKind::Dependency,
        RelationshipKind::Association,
    ] {
        for (source, target) in [
            (original.source_id, original.target_id),
            (original.target_id, original.source_id),
        ] {
            if kind == RelationshipKind::Trace && source == original.source_id {
                continue;
            }
            let id = project
                .create_relationship(kind.clone(), source, target, Some(project.root_id))
                .unwrap();
            if kind == RelationshipKind::Association {
                insert_duplicate(&mut project, id);
            }
        }
    }
    assert_eq!(project.validate(), Ok(()));
}

#[test]
fn repository_edits_cannot_leave_duplicate_detection_stale() {
    let (mut project, id) = fixture(RelationshipKind::Trace);
    let duplicate = insert_duplicate(&mut project, id);
    assert!(matches!(
        project.validate(),
        Err(ModelError::DuplicateTraceabilityRelationship { .. })
    ));
    project.relationships.remove(&duplicate);
    assert_eq!(project.validate(), Ok(()));
    insert_duplicate(&mut project, id);
    assert!(matches!(
        project.validate(),
        Err(ModelError::DuplicateTraceabilityRelationship { .. })
    ));
}

#[test]
fn endpoint_and_owner_errors_still_precede_duplicate_diagnostics() {
    let (mut project, id) = fixture(RelationshipKind::Trace);
    insert_duplicate(&mut project, id);
    let missing = ElementId::new();
    for relationship in project.relationships.values_mut() {
        relationship.target_id = missing;
        relationship.owner_id = None;
    }
    assert_eq!(
        project.validate(),
        Err(ModelError::ElementNotFound(missing))
    );
    let target = project
        .create_element(ElementKind::Block, "Replacement", project.root_id)
        .unwrap();
    for relationship in project.relationships.values_mut() {
        relationship.target_id = target;
    }
    assert_eq!(
        project.validate(),
        Err(ModelError::MissingTraceabilityOwner)
    );
}

#[test]
fn duplicate_check_preserves_semantic_id_comparison() {
    let (mut project, id) = fixture(RelationshipKind::Trace);
    let duplicate = insert_duplicate(&mut project, id);
    // Repository-key/semantic-ID parity is a separate validation concern. The
    // optimized duplicate rule must not silently change that existing contract.
    project.relationships.get_mut(&duplicate).unwrap().id = id;
    assert_eq!(project.validate(), Ok(()));
    project.relationships.get_mut(&duplicate).unwrap().id = duplicate;
    assert!(matches!(
        project.validate(),
        Err(ModelError::DuplicateTraceabilityRelationship { .. })
    ));
}
