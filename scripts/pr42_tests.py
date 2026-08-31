from pathlib import Path
from html import escape
from zipfile import ZipFile, ZIP_DEFLATED

ROOT = Path(__file__).resolve().parents[1]


def write(path: str, content: str) -> None:
    target = ROOT / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(content, encoding="utf-8")


write(
    "crates/model-core/tests/pr42_allocation.rs",
    r'''use systems_modeler_core::{ElementKind, ModelError, Project, Relationship, RelationshipKind};

#[test]
fn pr42_allocate_is_native_directional_serializable_and_has_no_requirement_side_effects() {
    let mut project = Project::new("Allocation");
    let package = project
        .create_element(ElementKind::Package, "Architecture", project.root_id)
        .unwrap();
    let controller = project
        .create_element(ElementKind::Block, "Controller", package)
        .unwrap();
    let requirement = project
        .create_element(ElementKind::Requirement, "Control Requirement", package)
        .unwrap();
    project
        .update_requirement(requirement, "REQ-42", "Controller shall provide control")
        .unwrap();

    let relationship_id = project
        .create_relationship(
            RelationshipKind::Allocate,
            controller,
            requirement,
            Some(package),
        )
        .unwrap();
    {
        let relationship = project.relationships.get_mut(&relationship_id).unwrap();
        relationship.external_id = "catia:pr42::ALLOC-1".into();
        relationship.name = "ControllerAllocation".into();
        relationship.documentation = "Explicit allocation".into();
    }

    let relationship = project.relationship(relationship_id).unwrap();
    assert_eq!(relationship.kind, RelationshipKind::Allocate);
    assert_eq!(relationship.source_id, controller);
    assert_eq!(relationship.target_id, requirement);
    assert_eq!(relationship.owner_id, Some(package));
    let encoded = serde_json::to_string(relationship).unwrap();
    let decoded: Relationship = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.kind, RelationshipKind::Allocate);
    assert_eq!(decoded.id, relationship_id);
    assert_eq!(decoded.external_id, "catia:pr42::ALLOC-1");
    assert_eq!(decoded.source_id, controller);
    assert_eq!(decoded.target_id, requirement);

    let requirement = project.element(requirement).unwrap();
    assert_eq!(requirement.requirement_id.as_deref(), Some("REQ-42"));
    assert_eq!(
        requirement.requirement_text.as_deref(),
        Some("Controller shall provide control")
    );
    project.validate().unwrap();
}

#[test]
fn pr42_allocate_rejects_self_duplicate_admin_endpoint_and_illegal_owner() {
    let mut project = Project::new("Allocation Validation");
    let package = project
        .create_element(ElementKind::Package, "Architecture", project.root_id)
        .unwrap();
    let a = project
        .create_element(ElementKind::Block, "A", package)
        .unwrap();
    let b = project
        .create_element(ElementKind::Block, "B", package)
        .unwrap();
    let note = project
        .create_element(ElementKind::Comment, "Administrative Note", package)
        .unwrap();

    assert!(matches!(
        project.create_relationship(RelationshipKind::Allocate, a, a, Some(package)),
        Err(ModelError::AllocationSelfReference)
    ));
    assert!(matches!(
        project.create_relationship(RelationshipKind::Allocate, note, b, Some(package)),
        Err(ModelError::InvalidAllocationEndpoints { .. })
    ));
    assert!(matches!(
        project.create_relationship(RelationshipKind::Allocate, a, b, None),
        Err(ModelError::MissingAllocationOwner)
    ));
    assert!(matches!(
        project.create_relationship(RelationshipKind::Allocate, a, b, Some(a)),
        Err(ModelError::InvalidAllocationOwner(id)) if id == a
    ));

    project
        .create_relationship(RelationshipKind::Allocate, a, b, Some(package))
        .unwrap();
    assert!(matches!(
        project.create_relationship(RelationshipKind::Allocate, a, b, Some(package)),
        Err(ModelError::DuplicateAllocationRelationship { source_id, target_id })
            if source_id == a && target_id == b
    ));
}
''',
)

write(
    "crates/persistence/tests/pr42_allocation_persistence.rs",
    r'''use systems_modeler_core::{ElementKind, Project, RelationshipKind};
use systems_modeler_persistence::ProjectDatabase;

#[test]
fn pr42_allocate_round_trips_through_native_project_database() {
    let mut project = Project::new("Allocation Persistence");
    let package = project
        .create_element(ElementKind::Package, "Architecture", project.root_id)
        .unwrap();
    let source = project
        .create_element(ElementKind::Block, "LogicalController", package)
        .unwrap();
    let target = project
        .create_element(ElementKind::Block, "PhysicalController", package)
        .unwrap();
    let id = project
        .create_relationship(RelationshipKind::Allocate, source, target, Some(package))
        .unwrap();
    {
        let relationship = project.relationships.get_mut(&id).unwrap();
        relationship.external_id = "catia:pr42::ALLOC-PERSIST".into();
        relationship.name = "Controller allocation".into();
        relationship.documentation = "Persisted allocation".into();
    }

    let mut db = ProjectDatabase::open_in_memory().unwrap();
    db.save_project(&project).unwrap();
    let restored = db.load_project(project.id).unwrap();
    let allocation = restored.relationship(id).unwrap();

    assert_eq!(allocation.kind, RelationshipKind::Allocate);
    assert_eq!(allocation.id, id);
    assert_eq!(allocation.external_id, "catia:pr42::ALLOC-PERSIST");
    assert_eq!(allocation.owner_id, Some(package));
    assert_eq!(allocation.source_id, source);
    assert_eq!(allocation.target_id, target);
    assert_eq!(allocation.name, "Controller allocation");
    assert_eq!(allocation.documentation, "Persisted allocation");
    restored.validate().unwrap();
}
''',
)

portable = ROOT / "apps/desktop/src-tauri/src/workspace/portable_interchange.rs"
portable_text = portable.read_text(encoding="utf-8")
portable_marker = "mod pr42_allocation_tests"
if portable_marker not in portable_text:
    portable_text += r'''

#[cfg(test)]
mod pr42_allocation_tests {
    use super::*;
    use systems_modeler_core::{ElementKind, RelationshipKind};

    #[test]
    fn pr42_portable_json_round_trip_preserves_native_allocate() {
        let source = WorkspaceState::default();
        let activity = ActivityWorkspaceState::default();
        let mut project = Project::new("PR42 Portable Source");
        let package = project
            .create_element(ElementKind::Package, "Architecture", project.root_id)
            .unwrap();
        let logical = project
            .create_element(ElementKind::Block, "LogicalController", package)
            .unwrap();
        let physical = project
            .create_element(ElementKind::Block, "PhysicalController", package)
            .unwrap();
        let allocation = project
            .create_relationship(RelationshipKind::Allocate, logical, physical, Some(package))
            .unwrap();
        {
            let relationship = project.relationships.get_mut(&allocation).unwrap();
            relationship.external_id = "catia:pr42::ALLOC-PORTABLE".into();
            relationship.name = "Portable allocation".into();
            relationship.documentation = "Portable round trip".into();
        }
        *source.project.lock().unwrap() = Some(project);

        let json = export_from_states(&source, &activity).unwrap();
        assert!(json.contains("Allocate"));

        let target = WorkspaceState::default();
        let target_activity = ActivityWorkspaceState::default();
        import_into_states(&json, &target, &target_activity).unwrap();
        let guard = target.project.lock().unwrap();
        let restored = guard.as_ref().unwrap();
        let relationship = restored.relationship(allocation).unwrap();
        assert_eq!(relationship.kind, RelationshipKind::Allocate);
        assert_eq!(relationship.external_id, "catia:pr42::ALLOC-PORTABLE");
        assert_eq!(relationship.source_id, logical);
        assert_eq!(relationship.target_id, physical);
        assert_eq!(relationship.owner_id, Some(package));
        assert_eq!(relationship.name, "Portable allocation");
        assert_eq!(relationship.documentation, "Portable round trip");
    }
}
'''
    portable.write_text(portable_text, encoding="utf-8")

spreadsheet = ROOT / "apps/desktop/src-tauri/src/workspace/spreadsheet_import.rs"
spreadsheet_text = spreadsheet.read_text(encoding="utf-8")
spreadsheet_marker = "mod pr42_tests"
if spreadsheet_marker not in spreadsheet_text:
    spreadsheet_text += r'''

#[cfg(test)]
mod pr42_tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use systems_modeler_core::RelationshipId;

    const NS: &str = "catia:pr42-fixture";

    fn workspace(name: &str) -> (WorkspaceState, ElementId, ElementId) {
        let state = WorkspaceState::default();
        let mut project = Project::new(name);
        let root = project.root_id;
        let package = project
            .create_element(ElementKind::Package, "Allocation", root)
            .unwrap();
        *state.project.lock().unwrap() = Some(project);
        (state, root, package)
    }

    fn temp_csv(contents: &str) -> String {
        let path = std::env::temp_dir().join(format!("pr42-{}.csv", uuid::Uuid::new_v4()));
        fs::write(&path, contents).unwrap();
        path.to_string_lossy().into_owned()
    }

    fn fixture_path() -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/pr42_allocations.xlsx")
            .to_string_lossy()
            .into_owned()
    }

    fn seed(
        project: &mut Project,
        owner: ElementId,
        kind: ElementKind,
        name: &str,
        external_id: &str,
    ) -> ElementId {
        let id = project.create_element(kind, name, owner).unwrap();
        project
            .set_external_id(id, external_key(NS, external_id))
            .unwrap();
        id
    }

    fn relationship_map(
        name: &str,
        source: String,
        worksheet: Option<&str>,
        target: ElementId,
        identity: SpreadsheetRelationshipIdentityPolicy,
        columns: &[(&str, SpreadsheetSemanticProperty)],
    ) -> SpreadsheetImportMap {
        SpreadsheetImportMap {
            name: name.into(),
            source,
            worksheet: worksheet.map(ToOwned::to_owned),
            header_row: 1,
            element_kind: ElementKind::Block,
            relationship_kind: Some(RelationshipKind::Allocate),
            relationship_identity: identity,
            target_scope: target,
            identification_property: SpreadsheetIdentificationProperty::ExternalId,
            search_scope: SpreadsheetSearchScope::TargetRecursive,
            source_namespace: NS.into(),
            mapping_version: "1".into(),
            column_mappings: columns
                .iter()
                .map(|(source_column, property)| SpreadsheetColumnMapping {
                    source_column: (*source_column).into(),
                    property: *property,
                })
                .collect(),
        }
    }

    fn element_map(
        name: &str,
        source: String,
        target: ElementId,
    ) -> SpreadsheetImportMap {
        SpreadsheetImportMap {
            name: name.into(),
            source,
            worksheet: None,
            header_row: 1,
            element_kind: ElementKind::Block,
            relationship_kind: None,
            relationship_identity: SpreadsheetRelationshipIdentityPolicy::ExternalId,
            target_scope: target,
            identification_property: SpreadsheetIdentificationProperty::ExternalId,
            search_scope: SpreadsheetSearchScope::TargetRecursive,
            source_namespace: NS.into(),
            mapping_version: "1".into(),
            column_mappings: vec![
                SpreadsheetColumnMapping {
                    source_column: "Element Key".into(),
                    property: SpreadsheetSemanticProperty::ExternalId,
                },
                SpreadsheetColumnMapping {
                    source_column: "Element Label".into(),
                    property: SpreadsheetSemanticProperty::Name,
                },
            ],
        }
    }

    fn business_columns() -> Vec<(&'static str, SpreadsheetSemanticProperty)> {
        vec![
            ("Allocation ID", SpreadsheetSemanticProperty::ExternalId),
            ("Function", SpreadsheetSemanticProperty::Source),
            ("Allocated Component", SpreadsheetSemanticProperty::Target),
            ("Description", SpreadsheetSemanticProperty::Documentation),
        ]
    }

    #[test]
    fn pr42_xlsx_maps_to_native_allocate_plan_and_preview_is_non_mutating() {
        let (state, _root, package) = workspace("PR42 XLSX");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            for (name, external) in [
                ("LogicalController", "LOGICAL"),
                ("PhysicalController", "PHYSICAL"),
                ("BrakeFunction", "BRAKE-FN"),
                ("BrakeController", "BRAKE-CTRL"),
                ("PowerFunction", "POWER-FN"),
                ("PowerUnit", "POWER-UNIT"),
            ] {
                seed(project, package, ElementKind::Block, name, external);
            }
        }
        let mapping = relationship_map(
            "Functional Allocation",
            fixture_path(),
            Some("Functional Allocation"),
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &business_columns(),
        );
        let group = SpreadsheetImportMapGroup {
            mappings: vec![mapping],
        };
        let before = state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .relationships
            .len();
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(preview.totals.create, 3);
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationships
                .len(),
            before
        );

        let snapshot = state.project.lock().unwrap().as_ref().unwrap().clone();
        let prepared = prepare_spreadsheet_import(&group, &snapshot);
        assert!(prepared.plan.operations.iter().any(|operation| matches!(
            operation,
            ModelBuildOperation::CreateRelationship {
                kind: RelationshipKind::Allocate,
                ..
            }
        )));
        assert!(!prepared.plan.operations.iter().any(|operation| matches!(
            operation,
            ModelBuildOperation::CreateDiagram { .. }
                | ModelBuildOperation::PresentElement { .. }
                | ModelBuildOperation::PresentRelationship { .. }
        )));

        apply_spreadsheet_import_group(&group, &state).unwrap();
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        assert_eq!(
            project
                .relationships
                .values()
                .filter(|relationship| relationship.kind == RelationshipKind::Allocate)
                .count(),
            3
        );
        let logical = project
            .relationships
            .values()
            .find(|relationship| relationship.external_id == external_key(NS, "ALLOC-001"))
            .unwrap();
        assert_eq!(logical.owner_id, Some(package));
        assert_eq!(logical.documentation, "Logical to physical allocation");
    }

    #[test]
    fn pr42_csv_external_ids_qnames_and_reimport_update_without_duplication() {
        let (state, _root, package) = workspace("PR42 CSV");
        let (a, b, c) = {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            (
                seed(project, package, ElementKind::Block, "A", "A"),
                seed(project, package, ElementKind::Block, "B", "B"),
                seed(project, package, ElementKind::Block, "C", "C"),
            )
        };
        let columns = [
            ("Allocation Key", SpreadsheetSemanticProperty::ExternalId),
            ("From Element", SpreadsheetSemanticProperty::Source),
            ("To Element", SpreadsheetSemanticProperty::Target),
            ("Notes", SpreadsheetSemanticProperty::Documentation),
        ];
        let first = relationship_map(
            "CSV Allocation",
            temp_csv("Allocation Key,From Element,To Element,Notes\nALLOC-CSV,A,B,first\n"),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &columns,
        );
        let first_group = SpreadsheetImportMapGroup {
            mappings: vec![first],
        };
        apply_spreadsheet_import_group(&first_group, &state).unwrap();
        let relationship_id = state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .relationships
            .values()
            .next()
            .unwrap()
            .id;
        assert_eq!(
            preview_spreadsheet_import_group(&first_group, &state)
                .totals
                .no_change,
            1
        );

        let metadata_update = relationship_map(
            "CSV Allocation",
            temp_csv("Allocation Key,From Element,To Element,Notes\nALLOC-CSV,A,B,updated\n"),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &columns,
        );
        let metadata_group = SpreadsheetImportMapGroup {
            mappings: vec![metadata_update],
        };
        assert_eq!(
            preview_spreadsheet_import_group(&metadata_group, &state)
                .totals
                .update,
            1
        );
        apply_spreadsheet_import_group(&metadata_group, &state).unwrap();

        let endpoint_update = relationship_map(
            "CSV Allocation",
            temp_csv("Allocation Key,From Element,To Element,Notes\nALLOC-CSV,A,C,updated\n"),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &columns,
        );
        let endpoint_group = SpreadsheetImportMapGroup {
            mappings: vec![endpoint_update],
        };
        assert_eq!(
            preview_spreadsheet_import_group(&endpoint_group, &state)
                .totals
                .update,
            1
        );
        apply_spreadsheet_import_group(&endpoint_group, &state).unwrap();
        {
            let guard = state.project.lock().unwrap();
            let project = guard.as_ref().unwrap();
            let relationship = project.relationship(relationship_id).unwrap();
            assert_eq!(relationship.id, relationship_id);
            assert_eq!(relationship.source_id, a);
            assert_eq!(relationship.target_id, c);
            assert_eq!(relationship.documentation, "updated");
            assert_eq!(project.relationships.len(), 1);
        }

        let (source_qname, target_qname) = {
            let guard = state.project.lock().unwrap();
            let project = guard.as_ref().unwrap();
            (
                project.qualified_name(b).unwrap(),
                project.qualified_name(c).unwrap(),
            )
        };
        let qname_map = relationship_map(
            "Qualified Allocation",
            temp_csv(&format!(
                "Allocation Key,From Element,To Element,Notes\nALLOC-Q,{source_qname},{target_qname},qualified\n"
            )),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &columns,
        );
        apply_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![qname_map],
            },
            &state,
        )
        .unwrap();
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationships
                .len(),
            2
        );
    }

    #[test]
    fn pr42_plan_local_endpoints_and_late_invalid_row_are_atomic() {
        let (blocked_state, _root, package) = workspace("PR42 Atomic");
        let blocks = element_map(
            "Blocks",
            temp_csv("Element Key,Element Label\nA,A\nB,B\n"),
            package,
        );
        let links = relationship_map(
            "Allocations",
            temp_csv(
                "Allocation ID,Function,Allocated Component,Description\nALLOC-1,A,B,valid\nALLOC-BAD,A,Missing,invalid\n",
            ),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &business_columns(),
        );
        let group = SpreadsheetImportMapGroup {
            mappings: vec![blocks, links],
        };
        let before = blocked_state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .elements
            .len();
        let preview = preview_spreadsheet_import_group(&group, &blocked_state);
        assert!(!preview.is_valid());
        assert!(
            preview
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "TARGET_UNRESOLVED")
        );
        assert_eq!(
            blocked_state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            before
        );
        assert!(
            blocked_state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationships
                .is_empty()
        );
        assert!(apply_spreadsheet_import_group(&group, &blocked_state).is_err());

        let (state, _root, package) = workspace("PR42 Plan Local");
        let blocks = element_map(
            "Blocks",
            temp_csv("Element Key,Element Label\nA,A\nB,B\n"),
            package,
        );
        let links = relationship_map(
            "Allocations",
            temp_csv("Allocation ID,Function,Allocated Component,Description\nALLOC-1,A,B,valid\n"),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &business_columns(),
        );
        let valid_group = SpreadsheetImportMapGroup {
            mappings: vec![blocks, links],
        };
        let project = state.project.lock().unwrap().as_ref().unwrap().clone();
        let prepared = prepare_spreadsheet_import(&valid_group, &project);
        assert!(prepared.preview.is_valid(), "{:?}", prepared.preview.diagnostics);
        let create_element_pos = prepared
            .plan
            .operations
            .iter()
            .position(|operation| matches!(operation, ModelBuildOperation::CreateElement { .. }))
            .unwrap();
        let allocate_pos = prepared
            .plan
            .operations
            .iter()
            .position(|operation| matches!(
                operation,
                ModelBuildOperation::CreateRelationship {
                    kind: RelationshipKind::Allocate,
                    ..
                }
            ))
            .unwrap();
        assert!(create_element_pos < allocate_pos);
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            2
        );
        apply_spreadsheet_import_group(&valid_group, &state).unwrap();
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationships
                .len(),
            1
        );
    }

    #[test]
    fn pr42_self_invalid_endpoint_owner_and_reference_failures_are_blocked() {
        let (state, _root, package) = workspace("PR42 Diagnostics");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            seed(project, package, ElementKind::Block, "A", "A");
            seed(project, package, ElementKind::Block, "B", "B");
            seed(project, package, ElementKind::Comment, "Note", "NOTE");
            seed(project, package, ElementKind::Block, "Dup", "DUP-1");
            seed(project, package, ElementKind::Block, "Dup", "DUP-2");
            seed(project, package, ElementKind::Block, "TargetDup", "TGT-1");
            seed(project, package, ElementKind::Block, "TargetDup", "TGT-2");
        }
        let columns = business_columns();
        let preview_for = |csv: &str| {
            let map = relationship_map(
                "Diagnostics",
                temp_csv(csv),
                None,
                package,
                SpreadsheetRelationshipIdentityPolicy::ExternalId,
                &columns,
            );
            preview_spreadsheet_import_group(
                &SpreadsheetImportMapGroup {
                    mappings: vec![map],
                },
                &state,
            )
        };

        let self_ref = preview_for(
            "Allocation ID,Function,Allocated Component,Description\nSELF,A,A,self\n",
        );
        assert!(!self_ref.is_valid());
        assert!(
            self_ref
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "ALLOCATION_SELF_REFERENCE")
        );

        let invalid_endpoint = preview_for(
            "Allocation ID,Function,Allocated Component,Description\nBAD-END,NOTE,B,invalid\n",
        );
        assert!(!invalid_endpoint.is_valid());
        assert!(invalid_endpoint.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "SEMANTIC_VALIDATION"
                && diagnostic.reason.contains("invalid Allocation endpoints")
        }));

        let unresolved_source = preview_for(
            "Allocation ID,Function,Allocated Component,Description\nMISS-S,Missing,B,missing\n",
        );
        assert!(
            unresolved_source
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "SOURCE_UNRESOLVED")
        );
        let unresolved_target = preview_for(
            "Allocation ID,Function,Allocated Component,Description\nMISS-T,A,Missing,missing\n",
        );
        assert!(
            unresolved_target
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "TARGET_UNRESOLVED")
        );
        let ambiguous_source = preview_for(
            "Allocation ID,Function,Allocated Component,Description\nAMB-S,Dup,B,ambiguous\n",
        );
        assert!(
            ambiguous_source
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "SOURCE_AMBIGUOUS")
        );
        let ambiguous_target = preview_for(
            "Allocation ID,Function,Allocated Component,Description\nAMB-T,A,TargetDup,ambiguous\n",
        );
        assert!(
            ambiguous_target
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "TARGET_AMBIGUOUS")
        );

        let owner_columns = [
            ("Allocation ID", SpreadsheetSemanticProperty::ExternalId),
            ("Function", SpreadsheetSemanticProperty::Source),
            ("Allocated Component", SpreadsheetSemanticProperty::Target),
            ("Owner", SpreadsheetSemanticProperty::Owner),
        ];
        let unresolved_owner = relationship_map(
            "Owner unresolved",
            temp_csv("Allocation ID,Function,Allocated Component,Owner\nOWN-1,A,B,MissingOwner\n"),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &owner_columns,
        );
        let unresolved_owner = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![unresolved_owner],
            },
            &state,
        );
        assert!(
            unresolved_owner
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "OWNER_UNRESOLVED")
        );
        let illegal_owner = relationship_map(
            "Owner illegal",
            temp_csv("Allocation ID,Function,Allocated Component,Owner\nOWN-2,A,B,A\n"),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &owner_columns,
        );
        let illegal_owner = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![illegal_owner],
            },
            &state,
        );
        assert!(!illegal_owner.is_valid());
        assert!(illegal_owner.diagnostics.iter().any(|diagnostic| {
            diagnostic.reason.contains("Allocation relationships must be owned by a Model or Package")
        }));
        assert!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationships
                .is_empty()
        );
    }

    #[test]
    fn pr42_duplicate_source_id_fallback_ambiguity_and_invalid_update_are_blocked() {
        let (state, _root, package) = workspace("PR42 Identity");
        let (a, b, c) = {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            (
                seed(project, package, ElementKind::Block, "A", "A"),
                seed(project, package, ElementKind::Block, "B", "B"),
                seed(project, package, ElementKind::Block, "C", "C"),
            )
        };
        let duplicate = relationship_map(
            "Duplicate IDs",
            temp_csv(
                "Allocation ID,Function,Allocated Component,Description\nDUP,A,B,one\nDUP,A,C,two\n",
            ),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &business_columns(),
        );
        let duplicate = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![duplicate],
            },
            &state,
        );
        assert!(
            duplicate
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "DUPLICATE_SOURCE_EXTERNAL_ID")
        );

        let initial = relationship_map(
            "Initial",
            temp_csv("Allocation ID,Function,Allocated Component,Description\nALLOC-1,A,B,initial\n"),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &business_columns(),
        );
        apply_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![initial],
            },
            &state,
        )
        .unwrap();
        let relationship_id = state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .relationships
            .values()
            .next()
            .unwrap()
            .id;

        let invalid_update = relationship_map(
            "Invalid update",
            temp_csv("Allocation ID,Function,Allocated Component,Description\nALLOC-1,NOTE,B,invalid\n"),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &business_columns(),
        );
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            seed(project, package, ElementKind::Comment, "NOTE", "NOTE");
        }
        let invalid_preview = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![invalid_update],
            },
            &state,
        );
        assert!(!invalid_preview.is_valid());
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationship(relationship_id)
                .unwrap()
                .source_id,
            a
        );

        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            let first = project
                .create_relationship(RelationshipKind::Allocate, b, c, Some(package))
                .unwrap();
            project.relationships.get_mut(&first).unwrap().external_id = "manual::one".into();
            let mut second = project.relationship(first).unwrap().clone();
            second.id = RelationshipId::new();
            second.external_id = "manual::two".into();
            project.relationships.insert(second.id, second);
        }
        let fallback_columns = [
            ("Function", SpreadsheetSemanticProperty::Source),
            ("Allocated Component", SpreadsheetSemanticProperty::Target),
        ];
        let fallback = relationship_map(
            "Fallback ambiguity",
            temp_csv("Function,Allocated Component\nB,C\n"),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::KindSourceTarget,
            &fallback_columns,
        );
        let fallback = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![fallback],
            },
            &state,
        );
        assert!(
            fallback
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "AMBIGUOUS_RELATIONSHIP")
        );
    }
}
'''
    spreadsheet.write_text(spreadsheet_text, encoding="utf-8")


def col_name(index: int) -> str:
    value = index + 1
    result = ""
    while value:
        value, rem = divmod(value - 1, 26)
        result = chr(65 + rem) + result
    return result


def make_xlsx(path: Path, sheet_name: str, rows: list[list[str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    sheet_rows = []
    for r_index, row in enumerate(rows, start=1):
        cells = []
        for c_index, value in enumerate(row):
            ref = f"{col_name(c_index)}{r_index}"
            cells.append(
                f'<c r="{ref}" t="inlineStr"><is><t>{escape(str(value))}</t></is></c>'
            )
        sheet_rows.append(f'<row r="{r_index}">{"".join(cells)}</row>')
    sheet_xml = (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
        f'<sheetData>{"".join(sheet_rows)}</sheetData></worksheet>'
    )
    with ZipFile(path, "w", ZIP_DEFLATED) as z:
        z.writestr(
            "[Content_Types].xml",
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
            '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
            '<Default Extension="xml" ContentType="application/xml"/>'
            '<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>'
            '<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>'
            '</Types>',
        )
        z.writestr(
            "_rels/.rels",
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
            '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>'
            '</Relationships>',
        )
        z.writestr(
            "xl/workbook.xml",
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" '
            'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">'
            f'<sheets><sheet name="{escape(sheet_name)}" sheetId="1" r:id="rId1"/></sheets></workbook>',
        )
        z.writestr(
            "xl/_rels/workbook.xml.rels",
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
            '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>'
            '</Relationships>',
        )
        z.writestr("xl/worksheets/sheet1.xml", sheet_xml)


make_xlsx(
    ROOT / "apps/desktop/src-tauri/tests/fixtures/pr42_allocations.xlsx",
    "Functional Allocation",
    [
        ["Allocation ID", "Function", "Allocated Component", "Description"],
        ["ALLOC-001", "LogicalController", "PhysicalController", "Logical to physical allocation"],
        ["ALLOC-002", "BrakeFunction", "BrakeController", "Brake allocation"],
        ["ALLOC-003", "PowerFunction", "PowerUnit", "Power allocation"],
    ],
)
