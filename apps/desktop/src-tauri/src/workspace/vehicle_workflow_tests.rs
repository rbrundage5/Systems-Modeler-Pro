//! Cross-feature native qualification; this is not packaged UI acceptance.
use super::*;
use crate::workspace::{BddDiagram, ibd, ibd_geometry, ibd_structure, repository_editing};
use systems_modeler_core::{
    Multiplicity, Project, StructuralRuntime, StructuralRuntimeConfiguration,
};
use systems_modeler_persistence::ProjectDatabase;

#[test]
fn vehicle_bdd_authoring_nested_ibd_persistence_and_deletion_share_identity() {
    let state = WorkspaceState::default();
    let activity = activity_workspace::ActivityWorkspaceState::default();
    let history = history::HistoryState::default();
    let mut project = Project::new("Vehicle decomposition qualification");
    let package = project
        .create_element(ElementKind::Package, "VehicleDefinitions", project.root_id)
        .unwrap();
    let blocks = ["Vehicle", "Engine", "Piston", "Ring"].map(|name| {
        project
            .create_element(ElementKind::Block, name, package)
            .unwrap()
    });
    let view = uuid::Uuid::new_v4().to_string();
    state.diagrams.lock().unwrap().push(BddDiagram {
        id: view.clone(),
        name: "Vehicle decomposition".into(),
        owner_id: package.to_string(),
        family: "bdd".into(),
        semantic_context_id: None,
        subject_boundary: None,
        nodes: Vec::new(),
        edges: Vec::new(),
    });
    *state.project.lock().unwrap() = Some(project);
    for (i, (name, multiplicity)) in [("engine", "1"), ("pistons", "4"), ("rings", "3")]
        .into_iter()
        .enumerate()
    {
        author_in_state(
            &view,
            &PartCompositionRequest {
                owner_id: blocks[i].to_string(),
                type_id: blocks[i + 1].to_string(),
                property_id: None,
                name: Some(name.into()),
                multiplicity: Some(multiplicity.into()),
            },
            &state,
            &activity,
            &history,
        )
        .unwrap();
    }
    let mut project = state.project.lock().unwrap().as_ref().unwrap().clone();
    let parts: Vec<_> = blocks[..3]
        .iter()
        .map(|owner| project.owned_features(*owner).next().unwrap().id)
        .collect();
    for (i, part) in parts.iter().enumerate() {
        assert_eq!(project.element(*part).unwrap().owner_id, Some(blocks[i]));
        assert_eq!(project.element(*part).unwrap().type_id, Some(blocks[i + 1]));
    }
    for block in blocks {
        assert_eq!(project.element(block).unwrap().owner_id, Some(package));
    }
    assert_eq!(state.diagrams.lock().unwrap()[0].nodes.len(), 4);
    assert_eq!(state.diagrams.lock().unwrap()[0].edges.len(), 3);
    let runtime = StructuralRuntime::build(
        &project,
        blocks[0],
        &StructuralRuntimeConfiguration::default(),
    )
    .unwrap();
    for (block, count) in blocks.into_iter().zip([1, 1, 4, 12]) {
        assert_eq!(runtime.instances_for_classifier(block).len(), count);
    }
    let mut diagram = ibd::IbdDiagram {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Vehicle internal structure".into(),
        context_block_id: blocks[0].to_string(),
        owner_id: package.to_string(),
        context_frame: Some(ibd_geometry::default_context_frame()),
        properties: Vec::new(),
        boundary_ports: Vec::new(),
        connectors: Vec::new(),
    };
    ibd::populate_ibd_diagram_from_context(&project, &mut diagram).unwrap();
    let before = serde_json::to_value(&project).unwrap();
    for depth in 1..=2 {
        let id = diagram
            .properties
            .iter()
            .find(|p| p.property_path.len() == depth)
            .unwrap()
            .id
            .clone();
        ibd_structure::expand_property(&project, &mut diagram, &id, true).unwrap();
    }
    assert_eq!(diagram.properties.len(), 3);
    assert_eq!(serde_json::to_value(&project).unwrap(), before);
    project.rename_element(parts[1], "cylinders").unwrap();
    project
        .set_multiplicity(parts[1], Multiplicity::new(6, Some(6)).unwrap())
        .unwrap();
    let end = project
        .relationships
        .values()
        .flat_map(|r| &r.association_ends)
        .find(|end| end.property_id == Some(parts[1]))
        .unwrap();
    assert_eq!(end.role_name, "cylinders");
    assert_eq!(end.multiplicity.notation(), "6");
    let root = diagram.properties[0].id.clone();
    ibd_structure::expand_property(&project, &mut diagram, &root, false).unwrap();
    ibd::validate_ibd_diagrams(&project, &[diagram.clone()]).unwrap();
    let bdd = serde_json::to_string(&*state.diagrams.lock().unwrap()).unwrap();
    let ibd_payload = serde_json::to_string(&[diagram.clone()]).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("vehicle.smproj");
    let mut db = ProjectDatabase::open(&path).unwrap();
    db.save_project_with_metadata(
        &project,
        &[
            (crate::workspace::BDD_METADATA_KEY, &bdd),
            (ibd::IBD_METADATA_KEY, &ibd_payload),
        ],
    )
    .unwrap();
    drop(db);
    let db = ProjectDatabase::open(&path).unwrap();
    let reopened = db.load_first_project().unwrap();
    assert_eq!(
        serde_json::to_value(&reopened).unwrap(),
        serde_json::to_value(&project).unwrap()
    );
    let saved = ibd::load_ibd_metadata(&db, &reopened).unwrap();
    assert_eq!(serde_json::to_string(&saved).unwrap(), ibd_payload);
    assert_eq!(
        db.load_metadata(project.id, crate::workspace::BDD_METADATA_KEY)
            .unwrap()
            .unwrap(),
        bdd
    );
    assert!(ibd_structure::remove_property_presentation(
        &mut diagram,
        &root
    ));
    assert!(diagram.properties.is_empty());
    assert!(project.element(parts[0]).is_ok());
    *state.project.lock().unwrap() = Some(reopened);
    *state.ibd_diagrams.lock().unwrap() = saved;
    repository_editing::delete_model_element_in_state(parts[0], &state, &activity, &history)
        .unwrap();
    {
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        assert!(project.element(parts[0]).is_err());
        for block in blocks {
            assert!(project.element(block).is_ok());
        }
    }
    assert!(history::undo_states(&state, &activity, &history).unwrap());
    assert!(
        state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .element(parts[0])
            .is_ok()
    );
    assert!(history::redo_states(&state, &activity, &history).unwrap());
}
