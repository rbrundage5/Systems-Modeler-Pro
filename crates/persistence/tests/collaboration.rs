use systems_modeler_core::{
    AggregationKind, DiagramId, ElementId, ElementKind, Multiplicity, Project, RelationshipKind,
    routing::{RouteRect, route_is_clear},
};
use systems_modeler_persistence::collaboration::{
    CollaborationError, EditRequest, ProjectRole, SharedBddDiagram, SharedEdit,
    SharedRelationshipKind,
};
use systems_modeler_persistence::{PersistenceError, ProjectDatabase};
use uuid::Uuid;

fn create(root: ElementId, revision: i64) -> EditRequest {
    EditRequest {
        operation_id: Uuid::new_v4(),
        expected_revision: revision,
        edit: SharedEdit::CreateBlock {
            owner: root,
            name: "Engine".into(),
        },
    }
}

fn edit(revision: i64, edit: SharedEdit) -> EditRequest {
    EditRequest {
        operation_id: Uuid::new_v4(),
        expected_revision: revision,
        edit,
    }
}

#[test]
fn legacy_shared_bdd_payload_without_edges_defaults_to_empty() {
    let diagram = SharedBddDiagram {
        id: DiagramId::new(),
        name: "Legacy structure".into(),
        owner: ElementId(Uuid::new_v4()),
        nodes: Vec::new(),
        edges: Vec::new(),
    };
    let mut payload = serde_json::to_value(&diagram).unwrap();
    payload.as_object_mut().unwrap().remove("edges");

    assert_eq!(
        serde_json::from_value::<SharedBddDiagram>(payload).unwrap(),
        diagram
    );
}

#[test]
fn commits_survive_reopen_and_retries_do_not_duplicate() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shared.sqlite");
    let project = Project::new("Shared");
    let actor = Uuid::new_v4();
    let request = create(project.root_id, 0);
    let receipt = {
        let mut db = ProjectDatabase::open(&path).unwrap();
        db.save_project(&project).unwrap();
        db.provision_shared_member(project.id, actor, ProjectRole::Editor)
            .unwrap();
        db.commit_shared_edit(project.id, actor, &request).unwrap()
    };
    let db = ProjectDatabase::open(&path).unwrap();
    assert_eq!(
        db.commit_shared_edit(project.id, actor, &request).unwrap(),
        receipt
    );
    let (loaded, diagrams, revision) = db.shared_snapshot(project.id, actor).unwrap();
    assert_eq!(revision, 1);
    assert!(diagrams.is_empty());
    assert_eq!(loaded.elements.len(), 2);
    assert_eq!(loaded.element(receipt.element).unwrap().name, "Engine");
    let mut altered = request;
    altered.expected_revision = 1;
    assert!(matches!(
        db.commit_shared_edit(project.id, actor, &altered),
        Err(CollaborationError::OperationIdReused)
    ));
}

#[test]
fn stale_clients_cannot_overwrite_and_can_resynchronize() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shared.sqlite");
    let project = Project::new("Shared");
    let actor = Uuid::new_v4();
    let mut first = ProjectDatabase::open(&path).unwrap();
    first.save_project(&project).unwrap();
    first
        .provision_shared_member(project.id, actor, ProjectRole::Editor)
        .unwrap();
    let second = ProjectDatabase::open(&path).unwrap();
    first
        .commit_shared_edit(project.id, actor, &create(project.root_id, 0))
        .unwrap();
    assert!(matches!(
        second.commit_shared_edit(project.id, actor, &create(project.root_id, 0)),
        Err(CollaborationError::Conflict { current: 1, .. })
    ));
    let (_, _, revision) = second.shared_snapshot(project.id, actor).unwrap();
    second
        .commit_shared_edit(project.id, actor, &create(project.root_id, revision))
        .unwrap();
    assert_eq!(first.shared_snapshot(project.id, actor).unwrap().2, 2);
}

#[test]
fn permissions_invalid_edits_and_legacy_save_preserve_state() {
    let project = Project::new("Shared");
    let editor = Uuid::new_v4();
    let viewer = Uuid::new_v4();
    let outsider = Uuid::new_v4();
    let mut db = ProjectDatabase::open_in_memory().unwrap();
    db.save_project(&project).unwrap();
    db.provision_shared_member(project.id, editor, ProjectRole::Editor)
        .unwrap();
    db.provision_shared_member(project.id, viewer, ProjectRole::Viewer)
        .unwrap();
    assert!(db.shared_snapshot(project.id, viewer).is_ok());
    assert!(matches!(
        db.shared_snapshot(project.id, outsider),
        Err(CollaborationError::Forbidden)
    ));
    for actor in [viewer, outsider] {
        assert!(matches!(
            db.commit_shared_edit(project.id, actor, &create(project.root_id, 0)),
            Err(CollaborationError::Forbidden)
        ));
    }
    assert!(
        db.commit_shared_edit(project.id, editor, &create(ElementId::new(), 0))
            .is_err()
    );
    assert!(matches!(
        db.save_project(&project),
        Err(PersistenceError::SharedProject)
    ));
    let (loaded, diagrams, revision) = db.shared_snapshot(project.id, editor).unwrap();
    assert_eq!(revision, 0);
    assert!(diagrams.is_empty());
    assert_eq!(loaded.elements.len(), 1);
    let receipt = db
        .commit_shared_edit(project.id, editor, &create(project.root_id, 0))
        .unwrap();
    let rename = EditRequest {
        operation_id: Uuid::new_v4(),
        expected_revision: 1,
        edit: SharedEdit::RenameElement {
            element: receipt.element,
            name: "Motor".into(),
        },
    };
    db.commit_shared_edit(project.id, editor, &rename).unwrap();
    assert_eq!(
        db.shared_snapshot(project.id, viewer)
            .unwrap()
            .0
            .element(receipt.element)
            .unwrap()
            .name,
        "Motor"
    );
    db.provision_shared_member(project.id, editor, ProjectRole::Viewer)
        .unwrap();
    assert!(matches!(
        db.commit_shared_edit(project.id, editor, &rename),
        Err(CollaborationError::Forbidden)
    ));
}

#[test]
fn shared_bdd_node_edits_share_the_project_revision_and_survive_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shared-bdd.sqlite");
    let project = Project::new("Shared");
    let actor = Uuid::new_v4();
    let mut db = ProjectDatabase::open(&path).unwrap();
    db.save_project(&project).unwrap();
    db.provision_shared_member(project.id, actor, ProjectRole::Editor)
        .unwrap();

    let block = db
        .commit_shared_edit(project.id, actor, &create(project.root_id, 0))
        .unwrap()
        .element;
    let diagram = DiagramId::new();
    db.commit_shared_edit(
        project.id,
        actor,
        &edit(
            1,
            SharedEdit::CreateBddDiagram {
                diagram,
                owner: project.root_id,
                name: "Structure".into(),
            },
        ),
    )
    .unwrap();
    let node = Uuid::new_v4();
    db.commit_shared_edit(
        project.id,
        actor,
        &edit(
            2,
            SharedEdit::PlaceBddElement {
                diagram,
                node,
                element: block,
            },
        ),
    )
    .unwrap();
    db.commit_shared_edit(
        project.id,
        actor,
        &edit(
            3,
            SharedEdit::UpdateBddNodeGeometry {
                diagram,
                node,
                x: 320.0,
                y: 240.0,
                width: 210.0,
                height: 130.0,
            },
        ),
    )
    .unwrap();
    db.commit_shared_edit(
        project.id,
        actor,
        &edit(
            4,
            SharedEdit::RenameBddDiagram {
                diagram,
                name: "Vehicle Structure".into(),
            },
        ),
    )
    .unwrap();
    drop(db);

    let db = ProjectDatabase::open(&path).unwrap();
    let (_, diagrams, revision) = db.shared_snapshot(project.id, actor).unwrap();
    assert_eq!(revision, 5);
    assert_eq!(diagrams.len(), 1);
    assert_eq!(diagrams[0].id, diagram);
    assert_eq!(diagrams[0].name, "Vehicle Structure");
    assert_eq!(diagrams[0].nodes.len(), 1);
    assert_eq!(diagrams[0].nodes[0].id, node);
    assert_eq!(diagrams[0].nodes[0].element, block);
    assert_eq!(diagrams[0].nodes[0].x, 320.0);

    assert!(matches!(
        db.commit_shared_edit(
            project.id,
            actor,
            &edit(
                5,
                SharedEdit::PlaceBddElement {
                    diagram,
                    node: Uuid::new_v4(),
                    element: block,
                },
            ),
        ),
        Err(CollaborationError::InvalidDiagram(_))
    ));
    assert_eq!(db.shared_snapshot(project.id, actor).unwrap().2, 5);
}

#[test]
fn failed_receipt_write_rolls_back_model_and_revision() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shared.sqlite");
    let project = Project::new("Shared");
    let actor = Uuid::new_v4();
    let mut db = ProjectDatabase::open(&path).unwrap();
    db.save_project(&project).unwrap();
    db.provision_shared_member(project.id, actor, ProjectRole::Editor)
        .unwrap();
    let fault = rusqlite::Connection::open(&path).unwrap();
    fault.execute_batch("CREATE TRIGGER fail_receipt BEFORE INSERT ON shared_operations BEGIN SELECT RAISE(ABORT, 'injected receipt failure'); END;").unwrap();
    let request = create(project.root_id, 0);
    assert!(db.commit_shared_edit(project.id, actor, &request).is_err());
    let (loaded, diagrams, revision) = db.shared_snapshot(project.id, actor).unwrap();
    assert_eq!(revision, 0);
    assert!(diagrams.is_empty());
    assert_eq!(loaded.elements.len(), 1);
    fault.execute_batch("DROP TRIGGER fail_receipt").unwrap();
    let receipt = db.commit_shared_edit(project.id, actor, &request).unwrap();
    assert_eq!(receipt.revision, 1);
}

#[test]
fn simple_relationship_edits_commit_retry_delete_and_survive_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shared-relationships.sqlite");
    let project = Project::new("Shared");
    let actor = Uuid::new_v4();
    let mut db = ProjectDatabase::open(&path).unwrap();
    db.save_project(&project).unwrap();
    db.provision_shared_member(project.id, actor, ProjectRole::Editor)
        .unwrap();

    let base = db
        .commit_shared_edit(project.id, actor, &create(project.root_id, 0))
        .unwrap()
        .element;
    let derived = db
        .commit_shared_edit(
            project.id,
            actor,
            &edit(
                1,
                SharedEdit::CreateBlock {
                    owner: project.root_id,
                    name: "Derived".into(),
                },
            ),
        )
        .unwrap()
        .element;
    let request = edit(
        2,
        SharedEdit::CreateRelationship {
            kind: SharedRelationshipKind::Generalization,
            source: derived,
            target: base,
            owner: project.root_id,
        },
    );
    let receipt = db.commit_shared_edit(project.id, actor, &request).unwrap();
    assert_eq!(receipt.element, derived);
    assert_eq!(
        db.commit_shared_edit(project.id, actor, &request).unwrap(),
        receipt
    );
    let relationship = db
        .shared_snapshot(project.id, actor)
        .unwrap()
        .0
        .relationships
        .values()
        .find(|relationship| relationship.kind == RelationshipKind::Generalization)
        .unwrap()
        .id;
    db.commit_shared_edit(
        project.id,
        actor,
        &edit(3, SharedEdit::DeleteRelationship { relationship }),
    )
    .unwrap();
    drop(db);

    let db = ProjectDatabase::open(&path).unwrap();
    let (loaded, _, revision) = db.shared_snapshot(project.id, actor).unwrap();
    assert_eq!(revision, 4);
    assert!(loaded.relationships.is_empty());
    assert_eq!(loaded.elements.len(), 3);
    loaded.validate().unwrap();
}

#[test]
fn every_advertised_simple_relationship_kind_uses_model_core_validation() {
    let mut project = Project::new("Shared relationship kinds");
    let block_a = project
        .create_element(ElementKind::Block, "Block A", project.root_id)
        .unwrap();
    let block_b = project
        .create_element(ElementKind::Block, "Block B", project.root_id)
        .unwrap();
    let requirement_a = project
        .create_requirement("Requirement A", "REQ-A", "A", project.root_id)
        .unwrap();
    let requirement_b = project
        .create_requirement("Requirement B", "REQ-B", "B", project.root_id)
        .unwrap();
    let copy_a = project
        .create_requirement("Copy A", "REQ-COPY-A", "A", project.root_id)
        .unwrap();
    let copy_b = project
        .create_requirement("Copy B", "REQ-COPY-B", "B", project.root_id)
        .unwrap();
    let test_case = project
        .create_element(ElementKind::TestCase, "Test", project.root_id)
        .unwrap();
    let use_case_a = project
        .create_element(ElementKind::UseCase, "Use A", project.root_id)
        .unwrap();
    let use_case_b = project
        .create_element(ElementKind::UseCase, "Use B", project.root_id)
        .unwrap();
    let actor = Uuid::new_v4();
    let mut db = ProjectDatabase::open_in_memory().unwrap();
    db.save_project(&project).unwrap();
    db.provision_shared_member(project.id, actor, ProjectRole::Editor)
        .unwrap();
    let cases = [
        (SharedRelationshipKind::Dependency, block_a, block_b),
        (SharedRelationshipKind::Generalization, block_a, block_b),
        (SharedRelationshipKind::Realization, block_a, block_b),
        (SharedRelationshipKind::Allocate, block_a, block_b),
        (
            SharedRelationshipKind::DeriveRequirement,
            requirement_a,
            requirement_b,
        ),
        (SharedRelationshipKind::Satisfy, block_a, requirement_a),
        (SharedRelationshipKind::Verify, test_case, requirement_a),
        (SharedRelationshipKind::Refine, block_b, requirement_b),
        (SharedRelationshipKind::Trace, block_a, requirement_b),
        (SharedRelationshipKind::Copy, copy_a, copy_b),
        (SharedRelationshipKind::Include, use_case_a, use_case_b),
        (SharedRelationshipKind::Extend, use_case_b, use_case_a),
    ];
    for (revision, (kind, source, target)) in cases.into_iter().enumerate() {
        db.commit_shared_edit(
            project.id,
            actor,
            &edit(
                revision as i64,
                SharedEdit::CreateRelationship {
                    kind,
                    source,
                    target,
                    owner: project.root_id,
                },
            ),
        )
        .unwrap();
    }
    let (loaded, _, revision) = db.shared_snapshot(project.id, actor).unwrap();
    assert_eq!(revision, 12);
    assert_eq!(loaded.relationships.len(), 12);
    loaded.validate().unwrap();
}

#[test]
fn invalid_and_specialized_relationship_edits_roll_back_atomically() {
    let mut project = Project::new("Shared");
    let base = project
        .create_element(ElementKind::Block, "Base", project.root_id)
        .unwrap();
    let derived = project
        .create_element(ElementKind::Block, "Derived", project.root_id)
        .unwrap();
    let association = project
        .create_association(
            Some(project.root_id),
            vec![
                Project::association_end(
                    base,
                    "base",
                    Multiplicity::ONE,
                    true,
                    AggregationKind::None,
                ),
                Project::association_end(
                    derived,
                    "derived",
                    Multiplicity::ONE,
                    true,
                    AggregationKind::None,
                ),
            ],
        )
        .unwrap();
    let actor = Uuid::new_v4();
    let mut db = ProjectDatabase::open_in_memory().unwrap();
    db.save_project(&project).unwrap();
    db.provision_shared_member(project.id, actor, ProjectRole::Editor)
        .unwrap();

    assert!(matches!(
        db.commit_shared_edit(
            project.id,
            actor,
            &edit(
                0,
                SharedEdit::CreateRelationship {
                    kind: SharedRelationshipKind::Generalization,
                    source: derived,
                    target: derived,
                    owner: project.root_id,
                },
            ),
        ),
        Err(CollaborationError::Model(_))
    ));
    assert!(matches!(
        db.commit_shared_edit(
            project.id,
            actor,
            &edit(
                0,
                SharedEdit::DeleteRelationship {
                    relationship: association
                }
            ),
        ),
        Err(CollaborationError::InvalidRelationship(_))
    ));
    let (loaded, _, revision) = db.shared_snapshot(project.id, actor).unwrap();
    assert_eq!(revision, 0);
    assert_eq!(loaded.relationships.len(), 1);
    assert_eq!(
        loaded.relationship(association).unwrap().kind,
        RelationshipKind::Association
    );
}

#[test]
fn shared_bdd_relationship_presentations_route_retry_move_remove_and_survive_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shared-bdd-edges.sqlite");
    let mut project = Project::new("Shared BDD edges");
    let source = project
        .create_element(ElementKind::Block, "Source", project.root_id)
        .unwrap();
    let target = project
        .create_element(ElementKind::Block, "Target", project.root_id)
        .unwrap();
    let obstacle = project
        .create_element(ElementKind::Block, "Obstacle", project.root_id)
        .unwrap();
    let relationship = project
        .create_relationship(
            RelationshipKind::Dependency,
            source,
            target,
            Some(project.root_id),
        )
        .unwrap();
    let actor = Uuid::new_v4();
    let mut db = ProjectDatabase::open(&path).unwrap();
    db.save_project(&project).unwrap();
    db.provision_shared_member(project.id, actor, ProjectRole::Editor)
        .unwrap();

    let diagram = DiagramId::new();
    db.commit_shared_edit(
        project.id,
        actor,
        &edit(
            0,
            SharedEdit::CreateBddDiagram {
                diagram,
                owner: project.root_id,
                name: "Structure".into(),
            },
        ),
    )
    .unwrap();
    let source_node = Uuid::new_v4();
    let target_node = Uuid::new_v4();
    let obstacle_node = Uuid::new_v4();
    for (revision, node, element) in [
        (1, source_node, source),
        (2, target_node, target),
        (3, obstacle_node, obstacle),
    ] {
        db.commit_shared_edit(
            project.id,
            actor,
            &edit(
                revision,
                SharedEdit::PlaceBddElement {
                    diagram,
                    node,
                    element,
                },
            ),
        )
        .unwrap();
    }
    db.commit_shared_edit(
        project.id,
        actor,
        &edit(
            4,
            SharedEdit::UpdateBddNodeGeometry {
                diagram,
                node: target_node,
                x: 720.0,
                y: 90.0,
                width: 190.0,
                height: 115.0,
            },
        ),
    )
    .unwrap();
    db.commit_shared_edit(
        project.id,
        actor,
        &edit(
            5,
            SharedEdit::UpdateBddNodeGeometry {
                diagram,
                node: obstacle_node,
                x: 400.0,
                y: 80.0,
                width: 190.0,
                height: 135.0,
            },
        ),
    )
    .unwrap();

    let edge = Uuid::new_v4();
    let request = edit(
        6,
        SharedEdit::PresentBddRelationship {
            diagram,
            edge,
            relationship,
        },
    );
    let receipt = db.commit_shared_edit(project.id, actor, &request).unwrap();
    assert_eq!(receipt.revision, 7);
    assert_eq!(db.commit_shared_edit(project.id, actor, &request).unwrap(), receipt);
    drop(db);

    let db = ProjectDatabase::open(&path).unwrap();
    let (_, diagrams, revision) = db.shared_snapshot(project.id, actor).unwrap();
    assert_eq!(revision, 7);
    let edge_before_move = &diagrams[0].edges[0];
    assert_eq!(edge_before_move.id, edge);
    assert_eq!(edge_before_move.relationship, relationship);
    assert!(edge_before_move.points.windows(2).all(|segment| {
        (segment[0].x - segment[1].x).abs() <= 0.001
            || (segment[0].y - segment[1].y).abs() <= 0.001
    }));
    assert!(route_is_clear(
        &edge_before_move.points,
        &[RouteRect {
            x: 400.0,
            y: 80.0,
            width: 190.0,
            height: 135.0,
        }],
    ));
    let previous_points = edge_before_move.points.clone();

    db.commit_shared_edit(
        project.id,
        actor,
        &edit(
            7,
            SharedEdit::UpdateBddNodeGeometry {
                diagram,
                node: obstacle_node,
                x: 400.0,
                y: 350.0,
                width: 190.0,
                height: 135.0,
            },
        ),
    )
    .unwrap();
    let (_, diagrams, revision) = db.shared_snapshot(project.id, actor).unwrap();
    assert_eq!(revision, 8);
    assert_ne!(diagrams[0].edges[0].points, previous_points);
    db.commit_shared_edit(
        project.id,
        actor,
        &edit(8, SharedEdit::RouteBddDiagram { diagram }),
    )
    .unwrap();
    db.commit_shared_edit(
        project.id,
        actor,
        &edit(9, SharedEdit::RemoveBddEdge { diagram, edge }),
    )
    .unwrap();
    let (_, diagrams, revision) = db.shared_snapshot(project.id, actor).unwrap();
    assert_eq!(revision, 10);
    assert!(diagrams[0].edges.is_empty());
}

#[test]
fn shared_bdd_edge_invariants_and_cascades_preserve_semantic_state() {
    let mut project = Project::new("Shared BDD edge invariants");
    let source = project
        .create_element(ElementKind::Block, "Source", project.root_id)
        .unwrap();
    let target = project
        .create_element(ElementKind::Block, "Target", project.root_id)
        .unwrap();
    let relationship = project
        .create_relationship(
            RelationshipKind::Dependency,
            source,
            target,
            Some(project.root_id),
        )
        .unwrap();
    let actor = Uuid::new_v4();
    let mut db = ProjectDatabase::open_in_memory().unwrap();
    db.save_project(&project).unwrap();
    db.provision_shared_member(project.id, actor, ProjectRole::Editor)
        .unwrap();
    let diagram = DiagramId::new();
    let source_node = Uuid::new_v4();
    let target_node = Uuid::new_v4();
    db.commit_shared_edit(
        project.id,
        actor,
        &edit(
            0,
            SharedEdit::CreateBddDiagram {
                diagram,
                owner: project.root_id,
                name: "Structure".into(),
            },
        ),
    )
    .unwrap();
    for (revision, node, element) in [(1, source_node, source), (2, target_node, target)] {
        db.commit_shared_edit(
            project.id,
            actor,
            &edit(
                revision,
                SharedEdit::PlaceBddElement {
                    diagram,
                    node,
                    element,
                },
            ),
        )
        .unwrap();
    }
    let edge = Uuid::new_v4();
    db.commit_shared_edit(
        project.id,
        actor,
        &edit(
            3,
            SharedEdit::PresentBddRelationship {
                diagram,
                edge,
                relationship,
            },
        ),
    )
    .unwrap();
    assert!(matches!(
        db.commit_shared_edit(
            project.id,
            actor,
            &edit(
                4,
                SharedEdit::PresentBddRelationship {
                    diagram,
                    edge: Uuid::new_v4(),
                    relationship,
                },
            ),
        ),
        Err(CollaborationError::InvalidDiagram(_))
    ));
    assert_eq!(db.shared_snapshot(project.id, actor).unwrap().2, 4);

    db.commit_shared_edit(
        project.id,
        actor,
        &edit(4, SharedEdit::DeleteRelationship { relationship }),
    )
    .unwrap();
    let (model, diagrams, revision) = db.shared_snapshot(project.id, actor).unwrap();
    assert_eq!(revision, 5);
    assert!(model.relationships.is_empty());
    assert!(diagrams[0].edges.is_empty());

    db.commit_shared_edit(
        project.id,
        actor,
        &edit(
            5,
            SharedEdit::CreateRelationship {
                kind: SharedRelationshipKind::Dependency,
                source,
                target,
                owner: project.root_id,
            },
        ),
    )
    .unwrap();
    let new_relationship = db
        .shared_snapshot(project.id, actor)
        .unwrap()
        .0
        .relationships
        .values()
        .next()
        .unwrap()
        .id;
    db.commit_shared_edit(
        project.id,
        actor,
        &edit(
            6,
            SharedEdit::PresentBddRelationship {
                diagram,
                edge: Uuid::new_v4(),
                relationship: new_relationship,
            },
        ),
    )
    .unwrap();
    db.commit_shared_edit(
        project.id,
        actor,
        &edit(
            7,
            SharedEdit::RemoveBddNode {
                diagram,
                node: source_node,
            },
        ),
    )
    .unwrap();
    let (model, diagrams, revision) = db.shared_snapshot(project.id, actor).unwrap();
    assert_eq!(revision, 8);
    assert_eq!(model.relationships.len(), 1);
    assert!(diagrams[0].edges.is_empty());
    assert!(matches!(
        db.commit_shared_edit(
            project.id,
            actor,
            &edit(
                8,
                SharedEdit::PresentBddRelationship {
                    diagram,
                    edge: Uuid::new_v4(),
                    relationship: new_relationship,
                },
            ),
        ),
        Err(CollaborationError::InvalidDiagram(_))
    ));
    assert_eq!(db.shared_snapshot(project.id, actor).unwrap().2, 8);
}
