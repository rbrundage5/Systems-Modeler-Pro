from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
path = ROOT / "apps/desktop/src-tauri/src/workspace/spreadsheet_import.rs"
text = path.read_text(encoding="utf-8")

marker = "mod pr40_tests {"
if marker in text:
    raise SystemExit("PR40 tests already present")

text += r'''

#[cfg(test)]
mod pr40_tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use systems_modeler_core::AggregationKind;

    const NS: &str = "catia:pr40-fixture";

    fn workspace(name: &str) -> (WorkspaceState, ElementId) {
        let state = WorkspaceState::default();
        let project = Project::new(name);
        let root = project.root_id;
        *state.project.lock().unwrap() = Some(project);
        (state, root)
    }

    fn fixture_path() -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/pr40_core_relationships.xlsx")
            .to_string_lossy()
            .into_owned()
    }

    fn temp_csv(contents: &str) -> String {
        let path = std::env::temp_dir().join(format!("pr40-{}.csv", uuid::Uuid::new_v4()));
        fs::write(&path, contents).unwrap();
        path.to_string_lossy().into_owned()
    }

    fn relationship_map(
        name: &str,
        source: String,
        worksheet: Option<&str>,
        header_row: usize,
        target: ElementId,
        configured_kind: Option<RelationshipKind>,
        identity: SpreadsheetRelationshipIdentityPolicy,
        columns: &[(&str, SpreadsheetSemanticProperty)],
    ) -> SpreadsheetImportMap {
        SpreadsheetImportMap {
            name: name.into(),
            source,
            worksheet: worksheet.map(ToOwned::to_owned),
            header_row,
            element_kind: ElementKind::Block,
            relationship_kind: configured_kind,
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
        kind: ElementKind,
        target: ElementId,
        columns: &[(&str, SpreadsheetSemanticProperty)],
    ) -> SpreadsheetImportMap {
        SpreadsheetImportMap {
            name: name.into(),
            source,
            worksheet: None,
            header_row: 1,
            element_kind: kind,
            relationship_kind: None,
            relationship_identity: SpreadsheetRelationshipIdentityPolicy::ExternalId,
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

    fn seed_structure(state: &WorkspaceState, root: ElementId) -> (ElementId, ElementId, ElementId, ElementId, ElementId, ElementId) {
        let mut guard = state.project.lock().unwrap();
        let project = guard.as_mut().unwrap();
        let structure = seed(project, root, ElementKind::Package, "Structure", "PKG-STRUCT");
        let vehicle = seed(project, structure, ElementKind::Block, "Vehicle", "VEH");
        let engine = seed(project, structure, ElementKind::Block, "Engine", "ENG");
        let controller = seed(project, structure, ElementKind::Block, "Controller", "CTRL");
        let electric = seed(project, structure, ElementKind::Block, "ElectricVehicle", "EV");
        let interface = seed(project, structure, ElementKind::InterfaceBlock, "PowertrainInterface", "IFACE");
        (structure, vehicle, engine, controller, electric, interface)
    }

    fn fixture_columns() -> Vec<(&'static str, SpreadsheetSemanticProperty)> {
        vec![
            ("Connection ID", SpreadsheetSemanticProperty::ExternalId),
            ("Relationship", SpreadsheetSemanticProperty::RelationshipKind),
            ("Source Component", SpreadsheetSemanticProperty::Source),
            ("Target Component", SpreadsheetSemanticProperty::Target),
            ("Relationship Name", SpreadsheetSemanticProperty::Name),
            ("Source Role", SpreadsheetSemanticProperty::SourceEndRole),
            ("Target Role", SpreadsheetSemanticProperty::TargetEndRole),
            ("Source Cardinality", SpreadsheetSemanticProperty::SourceMultiplicity),
            ("Target Cardinality", SpreadsheetSemanticProperty::TargetMultiplicity),
            ("Source Navigable", SpreadsheetSemanticProperty::SourceNavigable),
            ("Target Navigable", SpreadsheetSemanticProperty::TargetNavigable),
            ("Source Aggregation", SpreadsheetSemanticProperty::SourceAggregation),
            ("Target Aggregation", SpreadsheetSemanticProperty::TargetAggregation),
            ("Description", SpreadsheetSemanticProperty::Documentation),
            ("Semantic Owner", SpreadsheetSemanticProperty::Owner),
        ]
    }

    fn basic_relationship_columns() -> Vec<(&'static str, SpreadsheetSemanticProperty)> {
        vec![
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Source", SpreadsheetSemanticProperty::Source),
            ("Target", SpreadsheetSemanticProperty::Target),
            ("Owner", SpreadsheetSemanticProperty::Owner),
        ]
    }

    #[test]
    fn pr40_xlsx_creates_all_four_kinds_and_preserves_association_end_semantics() {
        let (state, root) = workspace("PR40 XLSX");
        let (structure, vehicle, engine, controller, electric, interface) = seed_structure(&state, root);
        let map = relationship_map(
            "Architecture Connections",
            fixture_path(),
            Some("Architecture Connections"),
            1,
            root,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &fixture_columns(),
        );
        let group = SpreadsheetImportMapGroup { mappings: vec![map] };

        let before = state.project.lock().unwrap().as_ref().unwrap().relationships.len();
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(preview.totals.create, 5);
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap().relationships.len(), before);

        let project_before = state.project.lock().unwrap().as_ref().unwrap().clone();
        let prepared = prepare_spreadsheet_import(&group, &project_before);
        assert!(prepared.plan.operations.iter().any(|operation| matches!(
            operation,
            ModelBuildOperation::CreateRelationship { .. }
        )));
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap().relationships.len(), before);

        apply_spreadsheet_import_group(&group, &state).unwrap();
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        assert_eq!(project.relationships.len(), 5);

        let association = project
            .relationships
            .values()
            .find(|relationship| relationship.external_id == external_key(NS, "ASSOC-VEH-ENGINE"))
            .unwrap();
        assert_eq!(association.kind, RelationshipKind::Association);
        assert_eq!(association.name, "VehicleEngine");
        assert_eq!(association.documentation, "Vehicle contains engines");
        assert_eq!(association.owner_id, Some(structure));
        assert_eq!(association.source_id, vehicle);
        assert_eq!(association.target_id, engine);
        assert_eq!(association.association_ends.len(), 2);
        assert_eq!(association.association_ends[0].role_name, "vehicle");
        assert_eq!(association.association_ends[1].role_name, "engine");
        assert_eq!(association.association_ends[0].multiplicity, Multiplicity::ONE);
        assert_eq!(association.association_ends[1].multiplicity, Multiplicity::new(1, None).unwrap());
        assert!(!association.association_ends[0].navigable);
        assert!(association.association_ends[1].navigable);
        assert_eq!(association.association_ends[0].aggregation, AggregationKind::Composite);
        assert_eq!(association.association_ends[1].aggregation, AggregationKind::None);

        let generalization = project
            .relationships
            .values()
            .find(|relationship| relationship.external_id == external_key(NS, "GEN-EV-VEH"))
            .unwrap();
        assert_eq!(generalization.kind, RelationshipKind::Generalization);
        assert_eq!((generalization.source_id, generalization.target_id), (electric, vehicle));

        let dependency = project
            .relationships
            .values()
            .find(|relationship| relationship.external_id == external_key(NS, "DEP-CTRL-IF"))
            .unwrap();
        assert_eq!(dependency.kind, RelationshipKind::Dependency);
        assert_eq!((dependency.source_id, dependency.target_id), (controller, interface));

        let realization = project
            .relationships
            .values()
            .find(|relationship| relationship.external_id == external_key(NS, "REAL-CTRL-IF"))
            .unwrap();
        assert_eq!(realization.kind, RelationshipKind::Realization);
        assert_eq!((realization.source_id, realization.target_id), (controller, interface));
        drop(guard);

        let second = preview_spreadsheet_import_group(&group, &state);
        assert!(second.is_valid(), "{:?}", second.diagnostics);
        assert_eq!(second.totals.no_change, 5);
        assert_eq!(second.totals.create, 0);
        assert_eq!(second.totals.update, 0);
    }

    #[test]
    fn pr40_association_reimport_updates_endpoint_and_fields_without_duplication() {
        let (state, root) = workspace("PR40 Update");
        let (_structure, vehicle, engine, controller, _electric, _interface) = seed_structure(&state, root);
        let columns = vec![
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Source", SpreadsheetSemanticProperty::Source),
            ("Target", SpreadsheetSemanticProperty::Target),
            ("Owner", SpreadsheetSemanticProperty::Owner),
            ("Name", SpreadsheetSemanticProperty::Name),
            ("Source Role", SpreadsheetSemanticProperty::SourceEndRole),
            ("Target Role", SpreadsheetSemanticProperty::TargetEndRole),
            ("Source Mult", SpreadsheetSemanticProperty::SourceMultiplicity),
            ("Target Mult", SpreadsheetSemanticProperty::TargetMultiplicity),
            ("Source Nav", SpreadsheetSemanticProperty::SourceNavigable),
            ("Target Nav", SpreadsheetSemanticProperty::TargetNavigable),
            ("Source Agg", SpreadsheetSemanticProperty::SourceAggregation),
            ("Target Agg", SpreadsheetSemanticProperty::TargetAggregation),
            ("Description", SpreadsheetSemanticProperty::Documentation),
        ];
        let first_source = temp_csv(
            "ID,Source,Target,Owner,Name,Source Role,Target Role,Source Mult,Target Mult,Source Nav,Target Nav,Source Agg,Target Agg,Description\nASSOC-100,VEH,ENG,Structure,Powertrain,vehicle,engine,1,1,false,true,composite,none,First\n",
        );
        let first_map = relationship_map(
            "Association",
            first_source,
            None,
            1,
            root,
            Some(RelationshipKind::Association),
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &columns,
        );
        apply_spreadsheet_import_group(&SpreadsheetImportMapGroup { mappings: vec![first_map] }, &state).unwrap();
        let original_id = {
            let guard = state.project.lock().unwrap();
            guard
                .as_ref()
                .unwrap()
                .relationships
                .values()
                .find(|relationship| relationship.external_id == external_key(NS, "ASSOC-100"))
                .unwrap()
                .id
        };

        let second_source = temp_csv(
            "ID,Source,Target,Owner,Name,Source Role,Target Role,Source Mult,Target Mult,Source Nav,Target Nav,Source Agg,Target Agg,Description\nASSOC-100,VEH,CTRL,Structure,Powertrain,system,controller,1,0..1,true,true,shared,none,Updated\n",
        );
        let second_map = relationship_map(
            "Association",
            second_source,
            None,
            1,
            root,
            Some(RelationshipKind::Association),
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &columns,
        );
        let second_group = SpreadsheetImportMapGroup { mappings: vec![second_map] };
        let preview = preview_spreadsheet_import_group(&second_group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(preview.totals.update, 1);
        apply_spreadsheet_import_group(&second_group, &state).unwrap();

        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        assert_eq!(project.relationships.len(), 1);
        let association = project.relationship(original_id).unwrap();
        assert_eq!(association.source_id, vehicle);
        assert_eq!(association.target_id, controller);
        assert_ne!(association.target_id, engine);
        assert_eq!(association.documentation, "Updated");
        assert_eq!(association.association_ends[0].role_name, "system");
        assert_eq!(association.association_ends[1].role_name, "controller");
        assert_eq!(association.association_ends[1].multiplicity, Multiplicity::new(0, Some(1)).unwrap());
        assert_eq!(association.association_ends[0].aggregation, AggregationKind::Shared);
        drop(guard);

        let third = preview_spreadsheet_import_group(&second_group, &state);
        assert_eq!(third.totals.no_change, 1);
    }

    #[test]
    fn pr40_resolves_external_and_exact_qualified_endpoints() {
        let (state, root) = workspace("PR40 Endpoint Identity");
        let (structure, a, b, _controller, _electric, _interface) = seed_structure(&state, root);
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            project.rename_element(a, "A").unwrap();
            project.rename_element(b, "B").unwrap();
        }
        let source = temp_csv(
            "ID,Source,Target,Owner\nREL-EXT,VEH,ENG,Structure\nREL-QN,Structure::A,Structure::B,Structure\n",
        );
        let map = relationship_map(
            "Endpoint Identity",
            source,
            None,
            1,
            root,
            Some(RelationshipKind::Association),
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_relationship_columns(),
        );
        let group = SpreadsheetImportMapGroup { mappings: vec![map] };
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(preview.totals.create, 2);
        apply_spreadsheet_import_group(&group, &state).unwrap();
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        assert_eq!(project.relationships.len(), 2);
        assert!(project.relationships.values().all(|relationship| relationship.owner_id == Some(structure)));
    }

    #[test]
    fn pr40_ordered_mapgroup_resolves_plan_local_owner_and_endpoints_without_early_commit() {
        let (state, root) = workspace("PR40 Pending");
        let packages = element_map(
            "Packages",
            temp_csv("ID,Name\nPKG-STRUCT,Structure\n"),
            ElementKind::Package,
            root,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Name", SpreadsheetSemanticProperty::Name),
            ],
        );
        let blocks = element_map(
            "Blocks",
            temp_csv("ID,Name,Owner\nA-ID,A,PKG-STRUCT\nB-ID,B,PKG-STRUCT\n"),
            ElementKind::Block,
            root,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Name", SpreadsheetSemanticProperty::Name),
                ("Owner", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let relationship = relationship_map(
            "Relationships",
            temp_csv("ID,Source,Target,Owner\nASSOC-PENDING,A-ID,B-ID,PKG-STRUCT\n"),
            None,
            1,
            root,
            Some(RelationshipKind::Association),
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_relationship_columns(),
        );
        let group = SpreadsheetImportMapGroup { mappings: vec![packages, blocks, relationship] };
        let before = state.project.lock().unwrap().as_ref().unwrap().elements.len();
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(preview.totals.create, 4);
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap().elements.len(), before);
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap().relationships.len(), 0);

        let snapshot = state.project.lock().unwrap().as_ref().unwrap().clone();
        let prepared = prepare_spreadsheet_import(&group, &snapshot);
        assert_eq!(prepared.plan.source_namespace, NS);
        assert!(prepared.plan.operations.iter().any(|operation| matches!(
            operation,
            ModelBuildOperation::CreateRelationship {
                source: BuildReference::External(source),
                target: BuildReference::External(target),
                owner: Some(BuildReference::External(owner)),
                ..
            } if source == "A-ID" && target == "B-ID" && owner == "PKG-STRUCT"
        )));

        apply_spreadsheet_import_group(&group, &state).unwrap();
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        assert_eq!(project.elements.len(), before + 3);
        assert_eq!(project.relationships.len(), 1);
    }

    #[test]
    fn pr40_blocks_duplicate_unsupported_ambiguous_and_unresolved_rows() {
        let (state, root) = workspace("PR40 Diagnostics");
        let (structure, _vehicle, _engine, _controller, _electric, _interface) = seed_structure(&state, root);

        let duplicate = relationship_map(
            "Duplicate",
            temp_csv("ID,Source,Target,Owner\nREL-1,VEH,ENG,Structure\nREL-1,VEH,CTRL,Structure\n"),
            None,
            1,
            root,
            Some(RelationshipKind::Association),
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_relationship_columns(),
        );
        let duplicate_preview = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup { mappings: vec![duplicate] },
            &state,
        );
        assert!(!duplicate_preview.is_valid());
        assert!(duplicate_preview.diagnostics.iter().any(|item| item.code == "DUPLICATE_SOURCE_EXTERNAL_ID"));

        let unsupported = relationship_map(
            "Unsupported",
            temp_csv("ID,Kind,Source,Target,Owner\nREL-2,Satisfy,VEH,ENG,Structure\n"),
            None,
            1,
            root,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Kind", SpreadsheetSemanticProperty::RelationshipKind),
                ("Source", SpreadsheetSemanticProperty::Source),
                ("Target", SpreadsheetSemanticProperty::Target),
                ("Owner", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let unsupported_preview = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup { mappings: vec![unsupported] },
            &state,
        );
        assert!(!unsupported_preview.is_valid());
        assert!(unsupported_preview.diagnostics.iter().any(|item| item.code == "RELATIONSHIP_KIND_UNSUPPORTED"));

        let unresolved = relationship_map(
            "Unresolved",
            temp_csv("ID,Source,Target,Owner\nREL-3,MISSING,ALSO-MISSING,Structure\n"),
            None,
            1,
            root,
            Some(RelationshipKind::Association),
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_relationship_columns(),
        );
        let unresolved_preview = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup { mappings: vec![unresolved] },
            &state,
        );
        assert!(!unresolved_preview.is_valid());
        assert!(unresolved_preview.diagnostics.iter().any(|item| item.code == "SOURCE_UNRESOLVED"));

        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            seed(project, structure, ElementKind::Block, "Duplicate", "DUP-A");
            seed(project, structure, ElementKind::Block, "Duplicate", "DUP-B");
            seed(project, structure, ElementKind::Block, "Unique", "UNIQUE");
        }
        let ambiguous = relationship_map(
            "Ambiguous",
            temp_csv("ID,Source,Target,Owner\nREL-4,Duplicate,Unique,Structure\n"),
            None,
            1,
            structure,
            Some(RelationshipKind::Association),
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_relationship_columns(),
        );
        let ambiguous_preview = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup { mappings: vec![ambiguous] },
            &state,
        );
        assert!(!ambiguous_preview.is_valid());
        assert!(ambiguous_preview.diagnostics.iter().any(|item| item.code == "SOURCE_AMBIGUOUS"));
    }

    #[test]
    fn pr40_explicit_fallback_identity_reuses_unique_match_and_blocks_ambiguity() {
        let (state, root) = workspace("PR40 Fallback");
        let (structure, vehicle, engine, _controller, _electric, _interface) = seed_structure(&state, root);
        let unique_id = {
            let mut guard = state.project.lock().unwrap();
            guard
                .as_mut()
                .unwrap()
                .create_association(
                    Some(structure),
                    vec![
                        Project::association_end(vehicle, "", Multiplicity::ONE, true, AggregationKind::None),
                        Project::association_end(engine, "", Multiplicity::ONE, true, AggregationKind::None),
                    ],
                )
                .unwrap()
        };
        let fallback_columns = [
            ("Source", SpreadsheetSemanticProperty::Source),
            ("Target", SpreadsheetSemanticProperty::Target),
            ("Owner", SpreadsheetSemanticProperty::Owner),
        ];
        let unique = relationship_map(
            "Fallback",
            temp_csv("Source,Target,Owner\nVEH,ENG,Structure\n"),
            None,
            1,
            root,
            Some(RelationshipKind::Association),
            SpreadsheetRelationshipIdentityPolicy::KindSourceTarget,
            &fallback_columns,
        );
        let unique_group = SpreadsheetImportMapGroup { mappings: vec![unique] };
        let preview = preview_spreadsheet_import_group(&unique_group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(preview.totals.update, 1);
        apply_spreadsheet_import_group(&unique_group, &state).unwrap();
        {
            let guard = state.project.lock().unwrap();
            let project = guard.as_ref().unwrap();
            assert_eq!(project.relationships.len(), 1);
            assert!(project.relationship(unique_id).unwrap().external_id.starts_with(&format!("{NS}::fallback::Association::")));
        }
        let stable = preview_spreadsheet_import_group(&unique_group, &state);
        assert_eq!(stable.totals.no_change, 1);

        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            project.relationships.get_mut(&unique_id).unwrap().external_id = "legacy::one".into();
            let second = project
                .create_association(
                    Some(structure),
                    vec![
                        Project::association_end(vehicle, "x", Multiplicity::ONE, true, AggregationKind::None),
                        Project::association_end(engine, "y", Multiplicity::ONE, true, AggregationKind::None),
                    ],
                )
                .unwrap();
            project.relationships.get_mut(&second).unwrap().external_id = "legacy::two".into();
        }
        let ambiguous = preview_spreadsheet_import_group(&unique_group, &state);
        assert!(!ambiguous.is_valid());
        assert!(ambiguous.diagnostics.iter().any(|item| item.code == "AMBIGUOUS_RELATIONSHIP"));
    }

    #[test]
    fn pr40_reuses_model_core_validation_for_generalization_dependency_and_owner() {
        let (state, root) = workspace("PR40 Validation");
        let (structure, _vehicle, _engine, _controller, _electric, _interface) = seed_structure(&state, root);
        let note = {
            let mut guard = state.project.lock().unwrap();
            seed(guard.as_mut().unwrap(), structure, ElementKind::Comment, "Note", "NOTE")
        };

        for (kind, csv) in [
            (RelationshipKind::Generalization, "ID,Source,Target,Owner\nBAD-GEN,NOTE,VEH,Structure\n"),
            (RelationshipKind::Dependency, "ID,Source,Target,Owner\nBAD-DEP,NOTE,VEH,Structure\n"),
        ] {
            let map = relationship_map(
                "Invalid endpoints",
                temp_csv(csv),
                None,
                1,
                root,
                Some(kind),
                SpreadsheetRelationshipIdentityPolicy::ExternalId,
                &basic_relationship_columns(),
            );
            let preview = preview_spreadsheet_import_group(&SpreadsheetImportMapGroup { mappings: vec![map] }, &state);
            assert!(!preview.is_valid());
            assert!(preview.diagnostics.iter().any(|item| item.code == "SEMANTIC_VALIDATION"));
        }

        let illegal_owner = relationship_map(
            "Illegal owner",
            temp_csv("ID,Source,Target,Owner\nBAD-OWNER,VEH,ENG,NOTE\n"),
            None,
            1,
            root,
            Some(RelationshipKind::Association),
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_relationship_columns(),
        );
        let owner_preview = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup { mappings: vec![illegal_owner] },
            &state,
        );
        assert!(!owner_preview.is_valid());
        assert!(owner_preview.diagnostics.iter().any(|item| item.code == "SEMANTIC_VALIDATION"));
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap().relationship(note).err(), Some(systems_modeler_core::ModelError::RelationshipNotFound(systems_modeler_core::RelationshipId(note.0))));
    }

    #[test]
    fn pr40_invalid_endpoint_update_and_blocked_mapgroup_leave_zero_mutation() {
        let (state, root) = workspace("PR40 Atomic");
        let (structure, vehicle, engine, _controller, _electric, _interface) = seed_structure(&state, root);
        let note = {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            let note = seed(project, structure, ElementKind::Comment, "Note", "NOTE");
            let relationship = project
                .create_relationship(RelationshipKind::Generalization, vehicle, engine, Some(structure))
                .unwrap();
            project.relationships.get_mut(&relationship).unwrap().external_id = external_key(NS, "GEN-UPDATE");
            note
        };
        let invalid_update = relationship_map(
            "Invalid update",
            temp_csv("ID,Source,Target,Owner\nGEN-UPDATE,VEH,NOTE,Structure\n"),
            None,
            1,
            root,
            Some(RelationshipKind::Generalization),
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_relationship_columns(),
        );
        let update_group = SpreadsheetImportMapGroup { mappings: vec![invalid_update] };
        let before = state.project.lock().unwrap().as_ref().unwrap().clone();
        let preview = preview_spreadsheet_import_group(&update_group, &state);
        assert!(!preview.is_valid());
        assert!(apply_spreadsheet_import_group(&update_group, &state).is_err());
        let after = state.project.lock().unwrap().as_ref().unwrap().clone();
        let original = before.relationships.values().find(|relationship| relationship.external_id == external_key(NS, "GEN-UPDATE")).unwrap();
        let unchanged = after.relationship(original.id).unwrap();
        assert_eq!((unchanged.source_id, unchanged.target_id), (vehicle, engine));
        assert_eq!(after.elements.len(), before.elements.len());
        assert_eq!(after.relationships.len(), before.relationships.len());
        assert!(after.element(note).is_ok());

        let (state2, root2) = workspace("PR40 Whole Group");
        let packages = element_map(
            "Packages",
            temp_csv("ID,Name\nPKG,Structure\n"),
            ElementKind::Package,
            root2,
            &[("ID", SpreadsheetSemanticProperty::ExternalId), ("Name", SpreadsheetSemanticProperty::Name)],
        );
        let blocks = element_map(
            "Blocks",
            temp_csv("ID,Name,Owner\nA,A,PKG\n"),
            ElementKind::Block,
            root2,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Name", SpreadsheetSemanticProperty::Name),
                ("Owner", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let broken_relationship = relationship_map(
            "Broken relationship",
            temp_csv("ID,Source,Target,Owner\nBROKEN,A,MISSING,PKG\n"),
            None,
            1,
            root2,
            Some(RelationshipKind::Association),
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_relationship_columns(),
        );
        let whole_group = SpreadsheetImportMapGroup { mappings: vec![packages, blocks, broken_relationship] };
        let before2 = state2.project.lock().unwrap().as_ref().unwrap().clone();
        let preview2 = preview_spreadsheet_import_group(&whole_group, &state2);
        assert!(!preview2.is_valid());
        assert!(apply_spreadsheet_import_group(&whole_group, &state2).is_err());
        let after2 = state2.project.lock().unwrap();
        let after2 = after2.as_ref().unwrap();
        assert_eq!(after2.elements.len(), before2.elements.len());
        assert_eq!(after2.relationships.len(), before2.relationships.len());
    }
}
'''

path.write_text(text, encoding="utf-8")
print("PR40 focused tests added")
