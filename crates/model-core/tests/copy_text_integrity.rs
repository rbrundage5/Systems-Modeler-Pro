use systems_modeler_core::{ElementId, ModelError, Project, RelationshipId, RelationshipKind};

fn fixture() -> (Project, [ElementId; 3]) {
    let mut project = Project::new("Copy text integrity");
    let requirements = ["A", "B", "C"].map(|id| {
        project
            .create_requirement(id, id, "Shared requirement text", project.root_id)
            .unwrap()
    });
    (project, requirements)
}

fn copy(project: &mut Project, client: ElementId, supplier: ElementId) -> RelationshipId {
    project
        .create_relationship(
            RelationshipKind::Copy,
            client,
            supplier,
            Some(project.root_id),
        )
        .unwrap()
}

fn assert_mismatch_without_mutation(project: &Project, relationship_id: RelationshipId) {
    let before = serde_json::to_value(project).unwrap();
    assert!(matches!(
        project.validate(),
        Err(ModelError::RequirementCopyTextMismatch(id)) if id == relationship_id
    ));
    assert_eq!(serde_json::to_value(project).unwrap(), before);
}

#[test]
fn malformed_reopened_copy_rejects_client_or_supplier_text_mismatch() {
    for corrupt_supplier in [false, true] {
        let (mut project, [a, b, _]) = fixture();
        let relationship_id = copy(&mut project, b, a);
        let modified = if corrupt_supplier { a } else { b };
        project
            .elements
            .get_mut(&modified)
            .unwrap()
            .requirement_text = Some("Divergent".into());
        let loaded: Project =
            serde_json::from_value(serde_json::to_value(project).unwrap()).unwrap();
        assert_mismatch_without_mutation(&loaded, relationship_id);
    }
}

#[test]
fn copy_validation_checks_late_chain_edges_and_every_supplier() {
    let (mut chain, [a, b, c]) = fixture();
    copy(&mut chain, b, a);
    let late = copy(&mut chain, c, b);
    chain.elements.get_mut(&c).unwrap().requirement_text = Some("Changed leaf".into());
    assert_mismatch_without_mutation(&chain, late);

    let (mut multiple, [a, b, c]) = fixture();
    copy(&mut multiple, c, a);
    let second = copy(&mut multiple, c, b);
    multiple.validate().unwrap();
    multiple.elements.get_mut(&b).unwrap().requirement_text = Some("Changed supplier".into());
    assert_mismatch_without_mutation(&multiple, second);
}

#[test]
fn valid_copy_propagation_and_local_identifiers_survive_roundtrip() {
    let (mut project, [a, b, c]) = fixture();
    copy(&mut project, b, a);
    copy(&mut project, c, b);
    project.update_requirement(a, "A", "Revised text").unwrap();
    project
        .update_requirement(c, "LOCAL-C", "Revised text")
        .unwrap();
    project.validate().unwrap();
    let loaded: Project = serde_json::from_value(serde_json::to_value(&project).unwrap()).unwrap();
    loaded.validate().unwrap();
    assert_eq!(
        serde_json::to_value(loaded).unwrap(),
        serde_json::to_value(project).unwrap()
    );
}
