from pathlib import Path

execution = Path('crates/model-core/src/execution.rs')
text = execution.read_text()
old = '''    pub fn value_in_instance_context(
        &self,
        instance_id: Option<RuntimeInstanceId>,
        semantic_element_id: ElementId,
    ) -> Option<&RuntimeValue> {
        // A classifier-scoped value is retained as the compatibility fallback
        // for PR31/PR32 sessions. PR33 callers do not create that fallback and
        // therefore resolve the owning occurrence's independent value.
        self.value(None, semantic_element_id)
            .or_else(|| instance_id.and_then(|id| self.value(Some(id), semantic_element_id)))
    }
'''
new = '''    pub fn value_in_instance_context(
        &self,
        instance_id: Option<RuntimeInstanceId>,
        semantic_element_id: ElementId,
    ) -> Option<&RuntimeValue> {
        // Runtime-occurrence state is authoritative when an instance context is
        // supplied. The classifier/model-scoped value remains only a compatibility
        // fallback for legacy PR31/PR32 sessions that have no occurrence value.
        instance_id
            .and_then(|id| self.value(Some(id), semantic_element_id))
            .or_else(|| self.value(None, semantic_element_id))
    }
'''
if text.count(old) != 1:
    raise SystemExit(f'value_in_instance_context pattern count {text.count(old)}')
text = text.replace(old, new, 1)
old = 'fn validate_runtime_assignment(\n'
if text.count(old) != 1:
    raise SystemExit(f'validate_runtime_assignment pattern count {text.count(old)}')
text = text.replace(old, 'pub(crate) fn validate_runtime_assignment(\n', 1)
execution.write_text(text)

runtime = Path('crates/model-core/src/structural_runtime.rs')
text = runtime.read_text()
anchor = 'use crate::{\n'
if text.count(anchor) != 1:
    raise SystemExit('structural use anchor mismatch')
text = text.replace(anchor, 'use crate::execution::validate_runtime_assignment;\nuse crate::{\n', 1)

error_anchor = '''    #[error(
        "{path} -> recursive composite instantiation. Remove or replace a PartProperty in this ownership cycle."
    )]
    RecursiveComposition { path: String },
'''
error_insert = '''    #[error(
        "{path} -> recursive composite instantiation. Remove or replace a PartProperty in this ownership cycle."
    )]
    RecursiveComposition { path: String },
    #[error(
        "Runtime occurrence path '{path}' is defined more than once. Give configured/root occurrences unique names or remove the conflicting configuration."
    )]
    DuplicateRuntimePath { path: String },
    #[error(
        "Runtime identity collision at '{path}' ({runtime_id}). Runtime construction stopped rather than aliasing two engineering occurrences."
    )]
    RuntimeIdentityCollision {
        path: String,
        runtime_id: RuntimeInstanceId,
    },
    #[error(
        "PartProperty {property} at {owner_path} has multiple equally scoped population decisions. Keep only one population decision for this occurrence."
    )]
    DuplicatePopulationDecision {
        property: String,
        owner_path: String,
    },
    #[error(
        "ReferenceProperty {reference} at {owner_path} has multiple runtime binding decisions. Keep exactly one binding decision for this occurrence/reference pair."
    )]
    DuplicateReferenceBindingDecision {
        reference: String,
        owner_path: String,
    },
'''
if text.count(error_anchor) != 1:
    raise SystemExit('error anchor mismatch')
text = text.replace(error_anchor, error_insert, 1)

id_anchor = '''        let id = deterministic_instance_id(
            self.project,
            semantic_element_id,
            classifier_id,
            &qualified_path,
            ordinal,
        );
        let name = usage_id
'''
id_insert = '''        if self
            .runtime
            .instances
            .values()
            .any(|instance| instance.qualified_path == qualified_path)
        {
            return Err(StructuralRuntimeError::DuplicateRuntimePath {
                path: qualified_path.clone(),
            });
        }
        let id = deterministic_instance_id(
            self.project,
            semantic_element_id,
            classifier_id,
            &qualified_path,
            ordinal,
        );
        if self.runtime.instances.contains_key(&id) {
            return Err(StructuralRuntimeError::RuntimeIdentityCollision {
                path: qualified_path.clone(),
                runtime_id: id,
            });
        }
        let name = usage_id
'''
if text.count(id_anchor) != 1:
    raise SystemExit('runtime identity anchor mismatch')
text = text.replace(id_anchor, id_insert, 1)

population_old = '''        let mut decisions: Vec<_> = self
            .configuration
            .populations
            .iter()
            .filter(|decision| {
                decision.part_property_id == part.id
                    && decision
                        .owner_runtime_path
                        .as_deref()
                        .is_none_or(|path| path == owner_path)
            })
            .collect();
        decisions.sort_by_key(|decision| decision.owner_runtime_path.is_none());
        let count = decisions
            .first()
            .map(|decision| decision.count)
            .unwrap_or(multiplicity.lower);
'''
population_new = '''        let exact: Vec<_> = self
            .configuration
            .populations
            .iter()
            .filter(|decision| {
                decision.part_property_id == part.id
                    && decision.owner_runtime_path.as_deref() == Some(owner_path)
            })
            .collect();
        let generic: Vec<_> = self
            .configuration
            .populations
            .iter()
            .filter(|decision| {
                decision.part_property_id == part.id && decision.owner_runtime_path.is_none()
            })
            .collect();
        if exact.len() > 1 || (exact.is_empty() && generic.len() > 1) {
            return Err(StructuralRuntimeError::DuplicatePopulationDecision {
                property: readable_element(self.project, part.id),
                owner_path: owner_path.to_string(),
            });
        }
        let count = exact
            .first()
            .or_else(|| generic.first())
            .map(|decision| decision.count)
            .unwrap_or(multiplicity.lower);
'''
if text.count(population_old) != 1:
    raise SystemExit('population pattern mismatch')
text = text.replace(population_old, population_new, 1)

default_old = '''            if let Some(authored) = property.default_value.as_deref() {
                let value = parse_authored_runtime_default(authored);
                self.runtime.initial_values.insert(
'''
default_new = '''            if let Some(authored) = property.default_value.as_deref() {
                let value = parse_authored_runtime_default(authored);
                validate_runtime_assignment(self.project, property, &value).map_err(|error| {
                    StructuralRuntimeError::InvalidDefault {
                        property: readable_element(self.project, property.id),
                        details: error.to_string(),
                    }
                })?;
                self.runtime.initial_values.insert(
'''
if text.count(default_old) != 1:
    raise SystemExit('default validation pattern mismatch')
text = text.replace(default_old, default_new, 1)

reference_old = '''                let decision = self
                    .configuration
                    .reference_bindings
                    .iter()
                    .find(|decision| {
                        decision.owner_runtime_path == owner.qualified_path
                            && decision.reference_property_id == reference.id
                    });
                let mut target_ids = Vec::new();
                if let Some(decision) = decision {
'''
reference_new = '''                let decisions: Vec<_> = self
                    .configuration
                    .reference_bindings
                    .iter()
                    .filter(|decision| {
                        decision.owner_runtime_path == owner.qualified_path
                            && decision.reference_property_id == reference.id
                    })
                    .collect();
                if decisions.len() > 1 {
                    return Err(StructuralRuntimeError::DuplicateReferenceBindingDecision {
                        reference: readable_element(self.project, reference.id),
                        owner_path: owner.qualified_path.clone(),
                    });
                }
                let decision = decisions.first().copied();
                let mut target_ids = Vec::new();
                if let Some(decision) = decision {
'''
if text.count(reference_old) != 1:
    raise SystemExit('reference decision pattern mismatch')
text = text.replace(reference_old, reference_new, 1)

old = '''                && flow.conveyed_item_ids.iter().any(|authored| {
                    classifier_conforms(project, conveyed_id, *authored)
                        || classifier_conforms(project, *authored, conveyed_id)
                })
'''
new = '''                && flow
                    .conveyed_item_ids
                    .iter()
                    .any(|authored| classifier_conforms(project, conveyed_id, *authored))
'''
if text.count(old) != 1:
    raise SystemExit('link conveyed conformance mismatch')
text = text.replace(old, new, 1)

old = '''        .filter(|contract| {
            classifier_conforms(project, conveyed_id, contract.type_id)
                || classifier_conforms(project, contract.type_id, conveyed_id)
        })
'''
new = '''        .filter(|contract| classifier_conforms(project, conveyed_id, contract.type_id))
'''
if text.count(old) != 1:
    raise SystemExit('flow contract conformance mismatch')
text = text.replace(old, new, 1)

old = '''            reception.type_id.is_some_and(|accepted| {
                classifier_conforms(project, signal_id, accepted)
                    || classifier_conforms(project, accepted, signal_id)
            })
'''
new = '''            reception
                .type_id
                .is_some_and(|accepted| classifier_conforms(project, signal_id, accepted))
'''
if text.count(old) != 1:
    raise SystemExit('reception conformance mismatch')
text = text.replace(old, new, 1)
runtime.write_text(text)

tests = Path('crates/model-core/tests/pr33_structural_runtime.rs')
text = tests.read_text()
if 'fn pr33_semantic_hardening_tests_marker()' not in text:
    text += r'''

#[test]
fn pr33_instance_context_prefers_occurrence_value_over_legacy_global_fallback() {
    let fixture = vehicle_fixture();
    let mut session = ExecutionSession::with_configuration(
        &fixture.project,
        ExecutionConfiguration {
            root_semantic_id: fixture.vehicle,
            random_seed: 0,
            max_steps: 100,
            max_queued_events: 100,
        },
    )
    .unwrap();
    let mut structural = vehicle_configuration();
    structural.reference_bindings[0].reference_property_id = fixture.sensor_reference;
    session.set_structural_configuration(structural).unwrap();
    session.initialize(&fixture.project).unwrap();
    let left = session
        .structural_runtime
        .as_ref()
        .unwrap()
        .instance_by_path("vehicle.leftSensor")
        .unwrap()
        .id;
    session
        .set_value(
            &fixture.project,
            None,
            fixture.sensor_reading,
            RuntimeValue::Real(99.0),
        )
        .unwrap();
    session
        .set_value(
            &fixture.project,
            Some(left),
            fixture.sensor_reading,
            RuntimeValue::Real(12.5),
        )
        .unwrap();
    assert_eq!(
        session.value_in_instance_context(Some(left), fixture.sensor_reading),
        Some(&RuntimeValue::Real(12.5))
    );
}

#[test]
fn duplicate_runtime_configuration_decisions_are_rejected_instead_of_first_match_wins() {
    let mut project = Project::new("PR33 duplicate decisions");
    let package = project
        .create_element(ElementKind::Package, "Model", project.root_id)
        .unwrap();
    let component = project
        .create_element(ElementKind::Block, "Component", package)
        .unwrap();
    let root = project
        .create_element(ElementKind::Block, "Root", package)
        .unwrap();
    let parts = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "parts",
            root,
            component,
            Multiplicity::new(0, Some(4)).unwrap(),
        )
        .unwrap();
    let error = StructuralRuntime::build(
        &project,
        root,
        &StructuralRuntimeConfiguration {
            populations: vec![
                RuntimePopulationDecision {
                    owner_runtime_path: None,
                    part_property_id: parts,
                    count: 1,
                },
                RuntimePopulationDecision {
                    owner_runtime_path: None,
                    part_property_id: parts,
                    count: 2,
                },
            ],
            ..StructuralRuntimeConfiguration::default()
        },
    )
    .unwrap_err();
    assert!(matches!(
        error,
        StructuralRuntimeError::DuplicatePopulationDecision { .. }
    ));
}

#[test]
fn duplicate_reference_bindings_and_runtime_paths_are_rejected() {
    let fixture = vehicle_fixture();
    let duplicate_binding = RuntimeReferenceBindingDecision {
        owner_runtime_path: "vehicle.guidance".into(),
        reference_property_id: fixture.sensor_reference,
        target_runtime_paths: vec!["vehicle.leftSensor".into()],
    };
    let error = StructuralRuntime::build(
        &fixture.project,
        fixture.vehicle,
        &StructuralRuntimeConfiguration {
            root_instance_name: Some("vehicle".into()),
            reference_bindings: vec![duplicate_binding.clone(), duplicate_binding],
            ..StructuralRuntimeConfiguration::default()
        },
    )
    .unwrap_err();
    assert!(matches!(
        error,
        StructuralRuntimeError::DuplicateReferenceBindingDecision { .. }
    ));

    let mut project = Project::new("Duplicate paths");
    let package = project
        .create_element(ElementKind::Package, "Model", project.root_id)
        .unwrap();
    let block = project
        .create_element(ElementKind::Block, "Thing", package)
        .unwrap();
    let first = project
        .create_element(ElementKind::InstanceSpecification, "same", package)
        .unwrap();
    project.set_element_type(first, block).unwrap();
    let second = project
        .create_element(ElementKind::InstanceSpecification, "same", package)
        .unwrap();
    project.set_element_type(second, block).unwrap();
    let error = StructuralRuntime::build(
        &project,
        first,
        &StructuralRuntimeConfiguration {
            configured_instance_specification_ids: vec![second],
            ..StructuralRuntimeConfiguration::default()
        },
    )
    .unwrap_err();
    assert!(matches!(
        error,
        StructuralRuntimeError::DuplicateRuntimePath { .. }
    ));
}

#[test]
fn invalid_authored_default_blocks_structural_runtime_construction() {
    let mut project = Project::new("Invalid default");
    let package = project
        .create_element(ElementKind::Package, "Model", project.root_id)
        .unwrap();
    let real = project
        .create_element(ElementKind::PrimitiveType, "Real", package)
        .unwrap();
    let block = project
        .create_element(ElementKind::Block, "Controller", package)
        .unwrap();
    let value = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "gain",
            block,
            real,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(value).unwrap().default_value = Some("not-a-number".into());
    let error = StructuralRuntime::build(
        &project,
        block,
        &StructuralRuntimeConfiguration::default(),
    )
    .unwrap_err();
    assert!(matches!(error, StructuralRuntimeError::InvalidDefault { .. }));
    assert!(error.to_string().contains("gain"));
}

#[test]
fn flow_contract_type_compatibility_is_not_bidirectional() {
    let mut project = Project::new("Transport conformance");
    let package = project
        .create_element(ElementKind::Package, "Model", project.root_id)
        .unwrap();
    let base_signal = project
        .create_element(ElementKind::Signal, "BaseSignal", package)
        .unwrap();
    let specific_signal = project
        .create_element(ElementKind::Signal, "SpecificSignal", package)
        .unwrap();
    project
        .create_relationship(
            RelationshipKind::Generalization,
            specific_signal,
            base_signal,
            Some(package),
        )
        .unwrap();
    let interface = project
        .create_element(ElementKind::InterfaceBlock, "SpecificInterface", package)
        .unwrap();
    let flow = project
        .create_typed_feature(
            ElementKind::FlowProperty,
            "specific",
            interface,
            specific_signal,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(flow).unwrap().flow_direction = Some(FlowDirection::Out);
    let source_type = project
        .create_element(ElementKind::Block, "Source", package)
        .unwrap();
    let source_port = project
        .create_typed_feature(
            ElementKind::ProxyPort,
            "out",
            source_type,
            interface,
            Multiplicity::ONE,
        )
        .unwrap();
    let target_type = project
        .create_element(ElementKind::Block, "Target", package)
        .unwrap();
    let target_port = project
        .create_typed_feature(
            ElementKind::ProxyPort,
            "in",
            target_type,
            interface,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(target_port).unwrap().is_conjugated = true;
    let system = project
        .create_element(ElementKind::Block, "System", package)
        .unwrap();
    let source_part = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "source",
            system,
            source_type,
            Multiplicity::ONE,
        )
        .unwrap();
    let target_part = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "target",
            system,
            target_type,
            Multiplicity::ONE,
        )
        .unwrap();
    let source_end = ConnectorEnd::nested_port(vec![source_part], source_port);
    let target_end = ConnectorEnd::nested_port(vec![target_part], target_port);
    let connector = project
        .create_connector(Connector {
            context_id: system,
            kind: ConnectorKind::Assembly,
            source: source_end.clone(),
            target: target_end.clone(),
        })
        .unwrap();
    project
        .create_item_flow(ItemFlow {
            connector_id: connector,
            source: source_end,
            target: target_end,
            conveyed_item_ids: vec![base_signal],
        })
        .unwrap();
    let error = StructuralRuntime::build(
        &project,
        system,
        &StructuralRuntimeConfiguration::default(),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        StructuralRuntimeError::ItemFlowTypeMismatch { .. }
    ));
}

#[test]
fn reception_does_not_accept_a_more_general_signal_than_it_declares() {
    let mut project = Project::new("Reception conformance");
    let package = project
        .create_element(ElementKind::Package, "Model", project.root_id)
        .unwrap();
    let base_signal = project
        .create_element(ElementKind::Signal, "BaseSignal", package)
        .unwrap();
    let specific_signal = project
        .create_element(ElementKind::Signal, "SpecificSignal", package)
        .unwrap();
    project
        .create_relationship(
            RelationshipKind::Generalization,
            specific_signal,
            base_signal,
            Some(package),
        )
        .unwrap();
    let interface = project
        .create_element(ElementKind::InterfaceBlock, "UntypedContract", package)
        .unwrap();
    let source_type = project
        .create_element(ElementKind::Block, "Source", package)
        .unwrap();
    let source_port = project
        .create_typed_feature(
            ElementKind::ProxyPort,
            "out",
            source_type,
            interface,
            Multiplicity::ONE,
        )
        .unwrap();
    let target_type = project
        .create_element(ElementKind::Block, "Target", package)
        .unwrap();
    let target_port = project
        .create_typed_feature(
            ElementKind::ProxyPort,
            "in",
            target_type,
            interface,
            Multiplicity::ONE,
        )
        .unwrap();
    let reception = project
        .create_element(ElementKind::Reception, "specificOnly", target_type)
        .unwrap();
    project.set_element_type(reception, specific_signal).unwrap();
    let system = project
        .create_element(ElementKind::Block, "System", package)
        .unwrap();
    let source_part = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "source",
            system,
            source_type,
            Multiplicity::ONE,
        )
        .unwrap();
    let target_part = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "target",
            system,
            target_type,
            Multiplicity::ONE,
        )
        .unwrap();
    project
        .create_connector(Connector {
            context_id: system,
            kind: ConnectorKind::Assembly,
            source: ConnectorEnd::nested_port(vec![source_part], source_port),
            target: ConnectorEnd::nested_port(vec![target_part], target_port),
        })
        .unwrap();
    let runtime = StructuralRuntime::build(
        &project,
        system,
        &StructuralRuntimeConfiguration::default(),
    )
    .unwrap();
    let source = runtime.instance_by_path("System.source").unwrap();
    let error = runtime
        .signal_destinations(&project, source.id, source_port, base_signal)
        .unwrap_err();
    assert!(matches!(error, StructuralRuntimeError::ReceptionMismatch { .. }));
}

#[test]
fn structural_runtime_snapshot_is_json_safe_and_deterministic() {
    let fixture = vehicle_fixture();
    let first = build_vehicle(&fixture).snapshot();
    let second = build_vehicle(&fixture).snapshot();
    assert_eq!(first, second);
    let encoded = serde_json::to_string(&first).unwrap();
    let decoded: StructuralRuntimeSnapshot = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, first);
}

#[allow(dead_code)]
fn pr33_semantic_hardening_tests_marker() {}
'''
tests.write_text(text)
