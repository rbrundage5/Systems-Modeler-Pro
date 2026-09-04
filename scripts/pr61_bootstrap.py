from pathlib import Path


def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one match in {path}, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")

model = Path("crates/model-core/src/model.rs")
replace_once(
    model,
    '''        if self.relationships.values().any(|relationship| {
            relationship.kind == RelationshipKind::Copy && relationship.source_id == id
        }) {
            return Err(ModelError::CopiedRequirementIsReadOnly(id));
        }
        let text = text.into();
        {
''',
    '''        let text = text.into();
        // Copy makes the slave Requirement semantically read-only, but an importer
        // must be able to reassert the exact state it already authored. Treat an
        // identical ID/text assignment as an idempotent no-op before enforcing
        // the read-only boundary. This also permits an ordered transaction to
        // update the master first (which propagates text to the copy) and then
        // consume the copy row without manufacturing a false import blocker.
        if self.element(id).is_ok_and(|current| {
            current.kind == ElementKind::Requirement
                && current.requirement_id.as_deref() == Some(requirement_id.as_str())
                && current.requirement_text.as_deref() == Some(text.as_str())
        }) {
            return Ok(());
        }
        if self.relationships.values().any(|relationship| {
            relationship.kind == RelationshipKind::Copy && relationship.source_id == id
        }) {
            return Err(ModelError::CopiedRequirementIsReadOnly(id));
        }
        {
''',
)

test = Path("crates/model-core/tests/pr61_copy_reimport.rs")
test.write_text(r'''use systems_modeler_core::*;

fn fixture() -> (Project, ElementId, ElementId) {
    let mut project = Project::new("PR61 copied Requirement reimport");
    let package = project
        .create_element(ElementKind::Package, "Requirements", project.root_id)
        .unwrap();
    let master = project
        .create_requirement(
            "MasterRequirement",
            "REQ-MASTER",
            "The vehicle shall provide controlled propulsion.",
            package,
        )
        .unwrap();
    let copy = project
        .create_requirement(
            "VerificationCopy",
            "REQ-COPY",
            "The vehicle shall provide controlled propulsion.",
            package,
        )
        .unwrap();
    project
        .create_relationship(RelationshipKind::Copy, copy, master, Some(package))
        .unwrap();
    project.validate().unwrap();
    (project, master, copy)
}

#[test]
fn identical_reassertion_of_copied_requirement_is_idempotent() {
    let (mut project, _master, copy) = fixture();
    project
        .update_requirement(
            copy,
            "REQ-COPY",
            "The vehicle shall provide controlled propulsion.",
        )
        .unwrap();
    project.validate().unwrap();
    let copied = project.element(copy).unwrap();
    assert_eq!(copied.requirement_id.as_deref(), Some("REQ-COPY"));
    assert_eq!(
        copied.requirement_text.as_deref(),
        Some("The vehicle shall provide controlled propulsion.")
    );
}

#[test]
fn master_update_then_copy_row_reassertion_is_safe() {
    let (mut project, master, copy) = fixture();
    project
        .update_requirement(
            master,
            "REQ-MASTER",
            "The vehicle shall provide controlled propulsion in all commanded modes.",
        )
        .unwrap();
    assert_eq!(
        project.element(copy).unwrap().requirement_text.as_deref(),
        Some("The vehicle shall provide controlled propulsion in all commanded modes.")
    );
    project
        .update_requirement(
            copy,
            "REQ-COPY",
            "The vehicle shall provide controlled propulsion in all commanded modes.",
        )
        .unwrap();
    project.validate().unwrap();
}

#[test]
fn copied_requirement_still_rejects_real_semantic_change() {
    let (mut project, _master, copy) = fixture();
    let error = project
        .update_requirement(copy, "REQ-COPY", "Unauthorized divergent copy text")
        .unwrap_err();
    assert!(matches!(
        error,
        ModelError::CopiedRequirementIsReadOnly(id) if id == copy
    ));
}
''', encoding="utf-8")
