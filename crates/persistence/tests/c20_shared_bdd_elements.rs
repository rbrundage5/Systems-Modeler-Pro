use systems_modeler_core::structural_presentation::creation::{BddElementKind, CreateBddElement};
use systems_modeler_core::{ElementKind, Project};
use systems_modeler_persistence::ProjectDatabase;
use systems_modeler_persistence::collaboration::{
    CollaborationError, EditRequest, ProjectRole, SharedEdit,
};
use uuid::Uuid;

fn creation(
    kind: BddElementKind,
    owner: systems_modeler_core::ElementId,
    revision: i64,
) -> EditRequest {
    EditRequest {
        operation_id: Uuid::new_v4(),
        expected_revision: revision,
        edit: SharedEdit::CreateBddElement(CreateBddElement {
            kind,
            owner,
            name: format!("Shared {kind:?}"),
        }),
    }
}

#[test]
fn supported_bdd_classifiers_use_core_ownership_preserve_identity_and_reverse_after_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("bdd-elements.sqlite");
    let mut database = ProjectDatabase::open(&path).unwrap();
    let project = Project::new("Shared classifiers");
    database.save_project(&project).unwrap();
    let actor = Uuid::new_v4();
    let viewer = Uuid::new_v4();
    database
        .provision_shared_member(project.id, actor, ProjectRole::Editor)
        .unwrap();
    database
        .provision_shared_member(project.id, viewer, ProjectRole::Viewer)
        .unwrap();
    let cases = [
        (BddElementKind::Block, ElementKind::Block),
        (
            BddElementKind::AssociationBlock,
            ElementKind::AssociationBlock,
        ),
        (BddElementKind::InterfaceBlock, ElementKind::InterfaceBlock),
        (
            BddElementKind::ConstraintBlock,
            ElementKind::ConstraintBlock,
        ),
        (BddElementKind::ValueType, ElementKind::ValueType),
        (BddElementKind::DataType, ElementKind::DataType),
        (BddElementKind::PrimitiveType, ElementKind::PrimitiveType),
        (BddElementKind::Enumeration, ElementKind::Enumeration),
        (BddElementKind::Signal, ElementKind::Signal),
        (BddElementKind::Unit, ElementKind::Unit),
        (BddElementKind::QuantityKind, ElementKind::QuantityKind),
        (
            BddElementKind::InstanceSpecification,
            ElementKind::InstanceSpecification,
        ),
        (BddElementKind::Comment, ElementKind::Comment),
        (BddElementKind::TestCase, ElementKind::TestCase),
        (BddElementKind::Actor, ElementKind::Actor),
        (BddElementKind::UseCase, ElementKind::UseCase),
    ];
    let mut final_request = None;
    let mut final_receipt = None;
    for (revision, (kind, expected_kind)) in cases.into_iter().enumerate() {
        let request = creation(kind, project.root_id, revision as i64);
        assert!(matches!(
            database.commit_shared_edit(project.id, viewer, &request),
            Err(CollaborationError::Forbidden)
        ));
        let receipt = database
            .commit_shared_edit(project.id, actor, &request)
            .unwrap();
        let (model, _, current) = database.shared_snapshot(project.id, actor).unwrap();
        let element = model.element(receipt.element).unwrap();
        assert_eq!(element.kind, expected_kind);
        assert_eq!(element.owner_id, Some(project.root_id));
        assert_eq!(current, revision as i64 + 1);
        model.validate().unwrap();
        final_request = Some(request);
        final_receipt = Some(receipt);
    }
    let request = final_request.unwrap();
    let receipt = final_receipt.unwrap();
    let invalid_owner = receipt.element; // A UseCase cannot own a Block.
    let before =
        serde_json::to_value(database.shared_snapshot(project.id, actor).unwrap()).unwrap();
    assert!(
        database
            .commit_shared_edit(
                project.id,
                actor,
                &creation(BddElementKind::Block, invalid_owner, 16),
            )
            .is_err()
    );
    assert!(matches!(
        database.commit_shared_edit(
            project.id,
            actor,
            &creation(BddElementKind::Block, project.root_id, 0),
        ),
        Err(CollaborationError::Conflict { .. })
    ));
    assert_eq!(
        serde_json::to_value(database.shared_snapshot(project.id, actor).unwrap()).unwrap(),
        before
    );
    assert_eq!(
        database.shared_history(project.id, actor).unwrap().len(),
        16
    );
    drop(database);
    let database = ProjectDatabase::open(&path).unwrap();
    assert_eq!(
        database
            .commit_shared_edit(project.id, actor, &request)
            .unwrap(),
        receipt
    );
    database
        .commit_shared_edit(
            project.id,
            actor,
            &EditRequest {
                operation_id: Uuid::new_v4(),
                expected_revision: 16,
                edit: SharedEdit::UndoOperation {
                    operation: request.operation_id,
                },
            },
        )
        .unwrap();
    let (restored, _, revision) = database.shared_snapshot(project.id, actor).unwrap();
    assert_eq!(revision, 17);
    assert!(!restored.elements.contains_key(&receipt.element));
    assert_eq!(restored.elements.len(), 16);
}

#[test]
fn feature_and_requirement_payloads_cannot_bypass_specialized_creation() {
    for kind in [
        "ProxyPort",
        "FullPort",
        "PartProperty",
        "ConstraintProperty",
        "Requirement",
        "Operation",
    ] {
        let payload = serde_json::json!({"CreateBddElement": {
            "kind": kind, "owner": Uuid::new_v4(), "name": "Invalid generic creation"
        }});
        assert!(serde_json::from_value::<SharedEdit>(payload).is_err());
    }
    let payload = serde_json::json!({"CreateBddElement": {
        "kind": "Block", "owner": Uuid::new_v4(), "name": "Actor spoofing", "actor": Uuid::new_v4()
    }});
    assert!(serde_json::from_value::<SharedEdit>(payload).is_err());
}
