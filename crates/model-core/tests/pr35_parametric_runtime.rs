use systems_modeler_core::*;

struct ForceFixture {
    project: Project,
    context: ElementId,
    scope: ParametricEvaluationScope,
    mass: ElementId,
    acceleration: ElementId,
    force: ElementId,
    equation: ElementId,
}

fn endpoint(role_id: ElementId, parameter_id: Option<ElementId>) -> BindingEndpoint {
    BindingEndpoint {
        role_id,
        parameter_id,
    }
}

fn force_fixture() -> ForceFixture {
    let mut project = Project::new("PR35 Parametric runtime");
    let package = project
        .create_element(ElementKind::Package, "Analysis", project.root_id)
        .unwrap();
    let real = project
        .create_element(ElementKind::PrimitiveType, "Real", package)
        .unwrap();
    let context = project
        .create_element(ElementKind::Block, "Vehicle", package)
        .unwrap();
    let mass = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "mass",
            context,
            real,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(mass).unwrap().default_value = Some("1000".into());
    let acceleration = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "acceleration",
            context,
            real,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(acceleration).unwrap().default_value = Some("2".into());
    let force = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "force",
            context,
            real,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(force).unwrap().is_derived = true;

    let definition = project
        .create_element(ElementKind::ConstraintBlock, "ForceEquation", package)
        .unwrap();
    let m = project
        .create_typed_feature(
            ElementKind::ConstraintParameter,
            "m",
            definition,
            real,
            Multiplicity::ONE,
        )
        .unwrap();
    let a = project
        .create_typed_feature(
            ElementKind::ConstraintParameter,
            "a",
            definition,
            real,
            Multiplicity::ONE,
        )
        .unwrap();
    let f = project
        .create_typed_feature(
            ElementKind::ConstraintParameter,
            "F",
            definition,
            real,
            Multiplicity::ONE,
        )
        .unwrap();
    project
        .element_mut(definition)
        .unwrap()
        .constraint_expression = "F = m * a".into();
    let equation = project
        .create_typed_feature(
            ElementKind::ConstraintProperty,
            "forceEquation",
            context,
            definition,
            Multiplicity::ONE,
        )
        .unwrap();
    let bindings = vec![
        project
            .create_binding_connector(context, endpoint(mass, None), endpoint(equation, Some(m)))
            .unwrap(),
        project
            .create_binding_connector(
                context,
                endpoint(acceleration, None),
                endpoint(equation, Some(a)),
            )
            .unwrap(),
        project
            .create_binding_connector(context, endpoint(force, None), endpoint(equation, Some(f)))
            .unwrap(),
    ];
    project.validate().unwrap();
    ForceFixture {
        project,
        context,
        scope: ParametricEvaluationScope {
            context_id: context,
            constraint_property_ids: vec![equation],
            value_property_ids: vec![mass, acceleration, force],
            binding_relationship_ids: bindings,
        },
        mass,
        acceleration,
        force,
        equation,
    }
}

fn session(project: &Project, root: ElementId) -> ExecutionSession {
    ExecutionSession::with_configuration(
        project,
        ExecutionConfiguration {
            root_semantic_id: root,
            random_seed: 0,
            max_steps: 100,
            max_queued_events: 100,
        },
    )
    .unwrap()
}

#[test]
fn inherited_roles_bind_and_evaluate_in_the_specialized_runtime_context() {
    let mut fixture = force_fixture();
    let derived = fixture
        .project
        .create_element(
            ElementKind::Block,
            "ElectricVehicle",
            fixture.project.root_id,
        )
        .unwrap();
    fixture
        .project
        .create_relationship(
            RelationshipKind::Generalization,
            derived,
            fixture.context,
            Some(fixture.project.root_id),
        )
        .unwrap();
    let mut derived_bindings = Vec::new();
    for id in &fixture.scope.binding_relationship_ids {
        let binding = fixture
            .project
            .relationship(*id)
            .unwrap()
            .binding
            .clone()
            .unwrap();
        derived_bindings.push(
            fixture
                .project
                .create_binding_connector(derived, binding.source, binding.target)
                .unwrap(),
        );
    }
    fixture.project.validate().unwrap();
    let duplicate = fixture
        .project
        .relationship(derived_bindings[0])
        .unwrap()
        .binding
        .clone()
        .unwrap();
    let before = serde_json::to_value(&fixture.project).unwrap();
    assert!(matches!(
        fixture
            .project
            .create_binding_connector(derived, duplicate.source, duplicate.target,),
        Err(ModelError::DuplicateBindingConnector)
    ));
    assert_eq!(serde_json::to_value(&fixture.project).unwrap(), before);
    fixture.scope.context_id = derived;
    // Ancestor wiring is usable too; select the local variant only for this run.
    for id in &fixture.scope.binding_relationship_ids {
        fixture
            .project
            .validate_binding_in_context(fixture.project.relationship(*id).unwrap(), derived)
            .unwrap();
    }
    fixture.scope.binding_relationship_ids = derived_bindings;
    let restored: Project = serde_json::from_value(before.clone()).unwrap();
    restored.validate().unwrap();
    let mut session = session(&restored, derived);
    let mut engine = ParametricExecutionEngine::new(fixture.scope);
    engine.initialize(&restored, &mut session).unwrap();
    assert_eq!(
        engine.step(&restored, &mut session).unwrap(),
        EngineStepOutcome::Completed
    );
    assert_eq!(
        session.value(engine.runtime_instance_id(), fixture.force),
        Some(&RuntimeValue::Real(2000.0))
    );
    assert_eq!(
        restored.element(fixture.force).unwrap().owner_id,
        Some(fixture.context)
    );
    assert_eq!(serde_json::to_value(&restored).unwrap(), before);
}

#[test]
fn inherited_binding_rejects_private_roles_without_mutation() {
    let mut fixture = force_fixture();
    let derived = fixture
        .project
        .create_element(ElementKind::Block, "Derived", fixture.project.root_id)
        .unwrap();
    fixture
        .project
        .create_relationship(
            RelationshipKind::Generalization,
            derived,
            fixture.context,
            Some(fixture.project.root_id),
        )
        .unwrap();
    fixture
        .project
        .element_mut(fixture.mass)
        .unwrap()
        .visibility = VisibilityKind::Private;
    let binding = fixture
        .project
        .relationship(fixture.scope.binding_relationship_ids[0])
        .unwrap()
        .binding
        .clone()
        .unwrap();
    let before = serde_json::to_value(&fixture.project).unwrap();
    assert!(
        fixture
            .project
            .create_binding_connector(derived, binding.source, binding.target)
            .is_err()
    );
    assert_eq!(serde_json::to_value(&fixture.project).unwrap(), before);
    fixture.project.validate().unwrap();
}

#[test]
fn evaluation_rejects_sibling_context_wiring_before_value_updates() {
    let mut fixture = force_fixture();
    let mut siblings = Vec::new();
    for name in ["VariantA", "VariantB"] {
        let id = fixture
            .project
            .create_element(ElementKind::Block, name, fixture.project.root_id)
            .unwrap();
        fixture
            .project
            .create_relationship(
                RelationshipKind::Generalization,
                id,
                fixture.context,
                Some(fixture.project.root_id),
            )
            .unwrap();
        siblings.push(id);
    }
    let binding = fixture
        .project
        .relationship(fixture.scope.binding_relationship_ids[0])
        .unwrap()
        .binding
        .clone()
        .unwrap();
    let foreign = fixture
        .project
        .create_binding_connector(siblings[1], binding.source, binding.target)
        .unwrap();
    fixture.scope.context_id = siblings[0];
    fixture.scope.binding_relationship_ids[0] = foreign;
    let before = serde_json::to_value(&fixture.project).unwrap();
    assert!(evaluate_parametrics(&mut fixture.project, &fixture.scope).is_err());
    assert_eq!(serde_json::to_value(&fixture.project).unwrap(), before);
}

#[test]
fn force_equation_evaluates_into_runtime_without_mutating_authored_model() {
    let fixture = force_fixture();
    let authored_before = serde_json::to_string(&fixture.project).unwrap();
    let mut session = session(&fixture.project, fixture.context);
    let mut engine = ParametricExecutionEngine::new(fixture.scope.clone());
    engine.initialize(&fixture.project, &mut session).unwrap();
    assert_eq!(
        engine.step(&fixture.project, &mut session).unwrap(),
        EngineStepOutcome::Completed
    );
    assert_eq!(session.state, ExecutionState::Completed);
    let instance = engine.runtime_instance_id().unwrap();
    assert_eq!(
        session.value(Some(instance), fixture.force),
        Some(&RuntimeValue::Real(2000.0))
    );
    assert_eq!(engine.snapshot(&session).updates[0].display_value, "2000");
    assert_eq!(
        serde_json::to_string(&fixture.project).unwrap(),
        authored_before
    );
    assert_eq!(
        fixture
            .project
            .element(fixture.force)
            .unwrap()
            .default_value,
        None
    );
}

#[test]
fn repeated_vehicle_occurrences_keep_parametric_values_isolated() {
    let mut fixture = force_fixture();
    let package = fixture
        .project
        .element(fixture.context)
        .unwrap()
        .owner_id
        .unwrap();
    let system = fixture
        .project
        .create_element(ElementKind::Block, "System", package)
        .unwrap();
    let left = fixture
        .project
        .create_typed_feature(
            ElementKind::PartProperty,
            "leftVehicle",
            system,
            fixture.context,
            Multiplicity::ONE,
        )
        .unwrap();
    let right = fixture
        .project
        .create_typed_feature(
            ElementKind::PartProperty,
            "rightVehicle",
            system,
            fixture.context,
            Multiplicity::ONE,
        )
        .unwrap();
    let preview = StructuralRuntime::build(
        &fixture.project,
        system,
        &StructuralRuntimeConfiguration::default(),
    )
    .unwrap();
    let left_path = preview.instances_for_usage(left)[0].qualified_path.clone();
    let mut session = session(&fixture.project, system);
    let mut engine = ParametricExecutionEngine::new(fixture.scope.clone())
        .with_runtime_instance_path(Some(left_path));
    engine.initialize(&fixture.project, &mut session).unwrap();
    let left_id = engine.runtime_instance_id().unwrap();
    let right_id = session
        .structural_runtime
        .as_ref()
        .unwrap()
        .instances_for_usage(right)[0]
        .id;
    session
        .set_value(
            &fixture.project,
            Some(left_id),
            fixture.mass,
            RuntimeValue::Real(4.0),
        )
        .unwrap();
    session
        .set_value(
            &fixture.project,
            Some(left_id),
            fixture.acceleration,
            RuntimeValue::Real(5.0),
        )
        .unwrap();
    session
        .set_value(
            &fixture.project,
            Some(right_id),
            fixture.mass,
            RuntimeValue::Real(7.0),
        )
        .unwrap();
    session
        .set_value(
            &fixture.project,
            Some(right_id),
            fixture.acceleration,
            RuntimeValue::Real(8.0),
        )
        .unwrap();
    engine.step(&fixture.project, &mut session).unwrap();
    assert_eq!(
        session.value(Some(left_id), fixture.force),
        Some(&RuntimeValue::Real(20.0))
    );
    assert_eq!(session.value(Some(right_id), fixture.force), None);
    assert_eq!(
        session.value(Some(right_id), fixture.mass),
        Some(&RuntimeValue::Real(7.0))
    );
}

#[test]
fn reset_replays_authored_inputs_deterministically() {
    let fixture = force_fixture();
    let mut session = session(&fixture.project, fixture.context);
    let mut engine = ParametricExecutionEngine::new(fixture.scope.clone());
    engine.initialize(&fixture.project, &mut session).unwrap();
    engine.step(&fixture.project, &mut session).unwrap();
    let first = engine.snapshot(&session).updates.clone();
    engine.reset(&fixture.project, &mut session).unwrap();
    engine.step(&fixture.project, &mut session).unwrap();
    let second = engine.snapshot(&session).updates.clone();
    assert_eq!(first, second);
    let instance = engine.runtime_instance_id().unwrap();
    assert_eq!(
        session.value(Some(instance), fixture.force),
        Some(&RuntimeValue::Real(2000.0))
    );
}

#[test]
fn missing_runtime_input_fails_with_readable_diagnostic_and_no_authored_mutation() {
    let mut fixture = force_fixture();
    fixture
        .project
        .element_mut(fixture.acceleration)
        .unwrap()
        .default_value = None;
    let authored_before = serde_json::to_string(&fixture.project).unwrap();
    let mut session = session(&fixture.project, fixture.context);
    let mut engine = ParametricExecutionEngine::new(fixture.scope.clone());
    engine.initialize(&fixture.project, &mut session).unwrap();
    engine.step(&fixture.project, &mut session).unwrap();
    assert_eq!(session.state, ExecutionState::Failed);
    let message = &session.diagnostics.last().unwrap().message;
    assert!(message.contains("unresolved parameter 'a'"));
    assert!(message.contains("forceEquation"));
    assert_eq!(
        serde_json::to_string(&fixture.project).unwrap(),
        authored_before
    );
}

#[test]
fn unsupported_expression_is_rejected_instead_of_executed() {
    let mut fixture = force_fixture();
    let definition = fixture
        .project
        .element(fixture.equation)
        .unwrap()
        .type_id
        .unwrap();
    fixture
        .project
        .element_mut(definition)
        .unwrap()
        .constraint_expression = "F = m % a".into();
    let mut session = session(&fixture.project, fixture.context);
    let mut engine = ParametricExecutionEngine::new(fixture.scope.clone());
    match engine.initialize(&fixture.project, &mut session) {
        Ok(()) => {
            engine.step(&fixture.project, &mut session).unwrap();
            assert_eq!(session.state, ExecutionState::Failed);
            assert!(
                session
                    .diagnostics
                    .last()
                    .unwrap()
                    .message
                    .contains("unsupported expression character")
            );
        }
        Err(error) => {
            assert!(
                error
                    .to_string()
                    .contains("unsupported expression character")
            );
        }
    }
}

#[test]
fn ambiguous_repeated_context_requires_explicit_occurrence_selection() {
    let mut fixture = force_fixture();
    let package = fixture
        .project
        .element(fixture.context)
        .unwrap()
        .owner_id
        .unwrap();
    let system = fixture
        .project
        .create_element(ElementKind::Block, "System", package)
        .unwrap();
    for name in ["first", "second"] {
        fixture
            .project
            .create_typed_feature(
                ElementKind::PartProperty,
                name,
                system,
                fixture.context,
                Multiplicity::ONE,
            )
            .unwrap();
    }
    let mut session = session(&fixture.project, system);
    let mut engine = ParametricExecutionEngine::new(fixture.scope.clone());
    let error = engine
        .initialize(&fixture.project, &mut session)
        .unwrap_err()
        .to_string();
    assert!(error.contains("resolves to 2 runtime occurrences"));
    assert!(error.contains("Vehicle"));
}
