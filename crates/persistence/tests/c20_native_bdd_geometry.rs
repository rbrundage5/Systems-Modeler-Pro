use systems_modeler_core::structural_presentation::geometry::{
    BddGeometryCommand, BddRoutingScope, apply_bdd_geometry,
};
use systems_modeler_core::{DiagramId, ElementKind, Project, RelationshipKind};
use systems_modeler_persistence::ProjectDatabase;
use systems_modeler_persistence::collaboration::{
    CollaborationError, EditRequest, ProjectRole, SharedEdit,
};
use uuid::Uuid;

fn request(revision: i64, edit: SharedEdit) -> EditRequest {
    EditRequest {
        operation_id: Uuid::new_v4(),
        expected_revision: revision,
        edit,
    }
}

#[test]
fn shared_geometry_matches_native_preserves_wire_shape_retries_and_reverses_after_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("native-shared.sqlite");
    let mut database = ProjectDatabase::open(&path).unwrap();
    let mut project = Project::new("Native and shared");
    let source = project
        .create_element(ElementKind::Block, "Source", project.root_id)
        .unwrap();
    let target = project
        .create_element(ElementKind::Block, "Target", project.root_id)
        .unwrap();
    let relationship = project
        .create_relationship(
            RelationshipKind::Generalization,
            source,
            target,
            Some(project.root_id),
        )
        .unwrap();
    database.save_project(&project).unwrap();
    let actor = Uuid::new_v4();
    let viewer = Uuid::new_v4();
    database
        .provision_shared_member(project.id, actor, ProjectRole::Editor)
        .unwrap();
    database
        .provision_shared_member(project.id, viewer, ProjectRole::Viewer)
        .unwrap();
    let diagram = DiagramId::new();
    let source_node = Uuid::new_v4();
    let target_node = Uuid::new_v4();
    let edge = Uuid::new_v4();
    for (revision, edit) in [
        SharedEdit::CreateBddDiagram {
            diagram,
            owner: project.root_id,
            name: "Structure".into(),
        },
        SharedEdit::PlaceBddElement {
            diagram,
            node: source_node,
            element: source,
        },
        SharedEdit::PlaceBddElement {
            diagram,
            node: target_node,
            element: target,
        },
        SharedEdit::PresentBddRelationship {
            diagram,
            edge,
            relationship,
        },
    ]
    .into_iter()
    .enumerate()
    {
        database
            .commit_shared_edit(project.id, actor, &request(revision as i64, edit))
            .unwrap();
    }
    let (_, before_diagrams, revision) = database.shared_snapshot(project.id, actor).unwrap();
    let original = &before_diagrams[0];
    let mut native = original.native_presentation();
    assert_eq!(native.id, diagram.to_string());
    assert_eq!(native.nodes[0].element_id, source.to_string());
    assert_eq!(native.edges[0].relationship_id, relationship.to_string());
    let command = BddGeometryCommand::UpdateNode {
        presentation_id: source_node.to_string(),
        x: 100.0,
        y: 360.0,
        width: 210.0,
        height: 120.0,
    };
    apply_bdd_geometry(&mut native, &command, BddRoutingScope::AllEdges, None).unwrap();
    let edit = request(
        revision,
        SharedEdit::UpdateBddNodeGeometry {
            diagram,
            node: source_node,
            x: 100.0,
            y: 360.0,
            width: 210.0,
            height: 120.0,
        },
    );
    assert!(matches!(
        database.commit_shared_edit(project.id, viewer, &edit),
        Err(CollaborationError::Forbidden)
    ));
    let receipt = database
        .commit_shared_edit(project.id, actor, &edit)
        .unwrap();
    let (_, after, after_revision) = database.shared_snapshot(project.id, actor).unwrap();
    assert_eq!(after_revision, revision + 1);
    assert_eq!(
        serde_json::to_value(after[0].native_presentation()).unwrap(),
        serde_json::to_value(&native).unwrap()
    );
    let original_keys: Vec<_> = serde_json::to_value(original)
        .unwrap()
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    let after_keys: Vec<_> = serde_json::to_value(&after[0])
        .unwrap()
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert_eq!(original_keys, after_keys);
    assert!(matches!(
        database.commit_shared_edit(
            project.id,
            actor,
            &request(revision, SharedEdit::RouteBddDiagram { diagram })
        ),
        Err(CollaborationError::Conflict { .. })
    ));
    let invalid = request(
        after_revision,
        SharedEdit::UpdateBddNodeGeometry {
            diagram,
            node: source_node,
            x: 100.0,
            y: 100.0,
            width: 1.0,
            height: 1.0,
        },
    );
    assert!(
        database
            .commit_shared_edit(project.id, actor, &invalid)
            .is_err()
    );
    let (_, rejected, rejected_revision) = database.shared_snapshot(project.id, actor).unwrap();
    assert_eq!(rejected, after);
    assert_eq!(rejected_revision, after_revision);
    assert_eq!(
        database.shared_history(project.id, actor).unwrap().len(),
        after_revision as usize
    );
    drop(database);
    let database = ProjectDatabase::open(&path).unwrap();
    assert_eq!(
        database
            .commit_shared_edit(project.id, actor, &edit)
            .unwrap(),
        receipt
    );
    database
        .commit_shared_edit(
            project.id,
            actor,
            &request(
                after_revision,
                SharedEdit::UndoOperation {
                    operation: edit.operation_id,
                },
            ),
        )
        .unwrap();
    let (_, restored, restored_revision) = database.shared_snapshot(project.id, actor).unwrap();
    assert_eq!(restored_revision, after_revision + 1);
    assert_eq!(restored, before_diagrams);
}
