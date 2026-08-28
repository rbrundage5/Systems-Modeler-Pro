use crate::{
    ElementId, ElementKind, EngineStepOutcome, ExecutionEngine, ExecutionError, ExecutionSession,
    ExecutionSnapshot, ExecutionState, ParametricEvaluationScope, Project, RuntimeEvent,
    RuntimeInstanceId, RuntimeValue, evaluate_parametrics,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParametricRuntimeValueUpdate {
    pub element_id: ElementId,
    pub element_name: String,
    pub previous_value: Option<RuntimeValue>,
    pub value: RuntimeValue,
    pub display_value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParametricExecutionSnapshot {
    pub execution: ExecutionSnapshot,
    pub context_id: ElementId,
    pub runtime_instance_id: Option<RuntimeInstanceId>,
    pub runtime_instance_path: Option<String>,
    pub evaluated_constraints: usize,
    pub evaluated_constraint_property_ids: Vec<ElementId>,
    pub updates: Vec<ParametricRuntimeValueUpdate>,
}

/// Runtime adapter for the existing PR25 deterministic Parametric evaluator.
///
/// The evaluator runs against a scratch clone whose ValueProperty defaults are
/// overlaid from one `ExecutionSession` occurrence. Only the resulting transient
/// values are written back to the shared session. The authored `Project` is never
/// mutated by this engine.
pub struct ParametricExecutionEngine {
    scope: ParametricEvaluationScope,
    requested_runtime_instance_path: Option<String>,
    runtime_instance_id: Option<RuntimeInstanceId>,
    evaluated_constraints: usize,
    evaluated_constraint_property_ids: Vec<ElementId>,
    updates: Vec<ParametricRuntimeValueUpdate>,
}

impl ParametricExecutionEngine {
    pub fn new(scope: ParametricEvaluationScope) -> Self {
        Self {
            scope,
            requested_runtime_instance_path: None,
            runtime_instance_id: None,
            evaluated_constraints: 0,
            evaluated_constraint_property_ids: Vec::new(),
            updates: Vec::new(),
        }
    }

    pub fn with_runtime_instance_path(mut self, path: Option<String>) -> Self {
        self.requested_runtime_instance_path = path.filter(|value| !value.trim().is_empty());
        self
    }

    pub fn runtime_instance_id(&self) -> Option<RuntimeInstanceId> {
        self.runtime_instance_id
    }

    pub fn scope(&self) -> &ParametricEvaluationScope {
        &self.scope
    }

    pub fn snapshot(&self, session: &ExecutionSession) -> ParametricExecutionSnapshot {
        let runtime_instance_path = self
            .runtime_instance_id
            .and_then(|id| session.instances.get(&id))
            .map(|instance| instance.qualified_path.clone());
        ParametricExecutionSnapshot {
            execution: session.snapshot(),
            context_id: self.scope.context_id,
            runtime_instance_id: self.runtime_instance_id,
            runtime_instance_path,
            evaluated_constraints: self.evaluated_constraints,
            evaluated_constraint_property_ids: self.evaluated_constraint_property_ids.clone(),
            updates: self.updates.clone(),
        }
    }

    pub fn reset(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<(), ExecutionError> {
        session.reset(project)?;
        self.clear_results();
        self.resolve_runtime_instance(project, session)?;
        session.record_engine_trace(
            Some(self.scope.context_id),
            format!(
                "Reset Parametric runtime for {}",
                readable_element(project, self.scope.context_id)
            ),
        );
        Ok(())
    }

    fn clear_results(&mut self) {
        self.evaluated_constraints = 0;
        self.evaluated_constraint_property_ids.clear();
        self.updates.clear();
    }

    fn resolve_runtime_instance(
        &mut self,
        project: &Project,
        session: &ExecutionSession,
    ) -> Result<(), ExecutionError> {
        let context = project
            .element(self.scope.context_id)
            .map_err(|_| ExecutionError::SemanticElementNotFound)?;
        if context.kind == ElementKind::ConstraintBlock {
            if self.requested_runtime_instance_path.is_some() {
                return Err(engine_error(format!(
                    "ConstraintBlock '{}' is a reusable definition and has no structural runtime occurrence.",
                    context.name
                )));
            }
            self.runtime_instance_id = None;
            return Ok(());
        }
        if !matches!(context.kind, ElementKind::Block | ElementKind::AssociationBlock) {
            return Err(engine_error(format!(
                "Parametric context '{}' ({:?}) is not an executable structural classifier.",
                context.name, context.kind
            )));
        }
        let runtime = session
            .structural_runtime
            .as_ref()
            .ok_or(ExecutionError::StructuralRuntimeUnavailable)?;
        if let Some(path) = self.requested_runtime_instance_path.as_deref() {
            let instance = runtime.instance_by_path(path).ok_or_else(|| {
                engine_error(format!(
                    "Parametric runtime occurrence '{}' was not found under the configured structural root.",
                    path
                ))
            })?;
            if !runtime.instance_conforms_to(project, instance.id, self.scope.context_id) {
                return Err(engine_error(format!(
                    "Runtime occurrence '{}' is typed by '{}', not Parametric context '{}'.",
                    path, instance.classifier_name, context.name
                )));
            }
            self.runtime_instance_id = Some(instance.id);
            return Ok(());
        }
        let paths = runtime.compatible_instance_paths(project, self.scope.context_id);
        match paths.as_slice() {
            [path] => {
                self.runtime_instance_id = runtime.instance_by_path(path).map(|instance| instance.id);
                Ok(())
            }
            [] => Err(engine_error(format!(
                "Parametric context '{}' has no compatible runtime occurrence under the configured structural root.",
                context.name
            ))),
            _ => Err(engine_error(format!(
                "Parametric context '{}' resolves to {} runtime occurrences: {}. Select one in Runtime configuration.",
                context.name,
                paths.len(),
                paths.join(", ")
            ))),
        }
    }

    fn evaluate_once(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<(), ExecutionError> {
        let mut scratch = project.clone();
        for element_id in &self.scope.value_property_ids {
            if let Some(runtime_value) = session
                .value_in_instance_context(self.runtime_instance_id, *element_id)
                .cloned()
            {
                let authored = runtime_value_to_authored(&runtime_value).map_err(|message| {
                    engine_error(format!(
                        "Parametric input '{}' is invalid: {message}",
                        readable_element(project, *element_id)
                    ))
                })?;
                scratch
                    .element_mut(*element_id)
                    .map_err(|_| ExecutionError::SemanticElementNotFound)?
                    .default_value = authored;
            }
        }

        let report = match evaluate_parametrics(&mut scratch, &self.scope) {
            Ok(report) => report,
            Err(error) => {
                let message = readable_model_error(project, error.to_string());
                session.fail(Some(self.scope.context_id), message);
                self.clear_results();
                return Ok(());
            }
        };

        let mut runtime_updates = Vec::new();
        for update in report.updates {
            let value = runtime_value_from_formatted(project, update.element_id, &update.value)?;
            let previous_value = session
                .value_in_instance_context(self.runtime_instance_id, update.element_id)
                .cloned();
            session.set_value(
                project,
                self.runtime_instance_id,
                update.element_id,
                value.clone(),
            )?;
            runtime_updates.push(ParametricRuntimeValueUpdate {
                element_id: update.element_id,
                element_name: readable_element(project, update.element_id),
                previous_value,
                value,
                display_value: update.value,
            });
        }

        self.evaluated_constraints = report.evaluated_constraints;
        self.evaluated_constraint_property_ids = self.scope.constraint_property_ids.clone();
        self.evaluated_constraint_property_ids
            .sort_by_key(ToString::to_string);
        self.updates = runtime_updates;

        let mut active = self.evaluated_constraint_property_ids.clone();
        active.extend(self.updates.iter().map(|update| update.element_id));
        session.set_active_semantic_elements(project, active)?;
        let target = self
            .runtime_instance_id
            .and_then(|id| session.instances.get(&id))
            .map(|instance| instance.qualified_path.clone())
            .unwrap_or_else(|| readable_element(project, self.scope.context_id));
        session.record_engine_trace(
            Some(self.scope.context_id),
            format!(
                "Evaluated {} Parametric constraint(s) on {}",
                self.evaluated_constraints, target
            ),
        );
        session.complete()?;
        Ok(())
    }
}

impl ExecutionEngine for ParametricExecutionEngine {
    fn initialize(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<(), ExecutionError> {
        session.initialize(project)?;
        self.clear_results();
        self.resolve_runtime_instance(project, session)?;
        let target = self
            .runtime_instance_id
            .and_then(|id| session.instances.get(&id))
            .map(|instance| instance.qualified_path.clone())
            .unwrap_or_else(|| readable_element(project, self.scope.context_id));
        session.record_engine_trace(
            Some(self.scope.context_id),
            format!("Initialized Parametric runtime for {target}"),
        );
        Ok(())
    }

    fn step(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<EngineStepOutcome, ExecutionError> {
        if !matches!(
            session.state,
            ExecutionState::Initialized | ExecutionState::Running | ExecutionState::Paused
        ) {
            return Err(engine_error(format!(
                "Cannot evaluate Parametrics while execution session is {:?}.",
                session.state
            )));
        }
        session.consume_step_budget()?;
        self.evaluate_once(project, session)?;
        Ok(EngineStepOutcome::Completed)
    }

    fn handle_event(
        &mut self,
        _project: &Project,
        _session: &mut ExecutionSession,
        _event: &RuntimeEvent,
    ) -> Result<EngineStepOutcome, ExecutionError> {
        Ok(EngineStepOutcome::Idle)
    }
}

fn runtime_value_to_authored(value: &RuntimeValue) -> Result<Option<String>, String> {
    match value {
        RuntimeValue::Unset => Ok(None),
        RuntimeValue::Integer(value) => Ok(Some(value.to_string())),
        RuntimeValue::Real(value) if value.is_finite() => Ok(Some(format_number(*value))),
        RuntimeValue::Real(_) => Err("numeric value is not finite".into()),
        RuntimeValue::Text(value) => Ok(Some(value.clone())),
        RuntimeValue::Boolean(_) => Err("boolean values are not numeric Parametric inputs".into()),
        RuntimeValue::ElementReference(_) => {
            Err("element references are not numeric Parametric inputs".into())
        }
    }
}

fn runtime_value_from_formatted(
    project: &Project,
    element_id: ElementId,
    formatted: &str,
) -> Result<RuntimeValue, ExecutionError> {
    let property = project
        .element(element_id)
        .map_err(|_| ExecutionError::SemanticElementNotFound)?;
    let type_id = property
        .type_id
        .ok_or_else(|| engine_error(format!("ValueProperty '{}' has no type.", property.name)))?;
    let value_type = project
        .element(type_id)
        .map_err(|_| ExecutionError::ClassifierNotFound)?;
    let number_text = formatted.split_whitespace().next().ok_or_else(|| {
        engine_error(format!(
            "Parametric result for '{}' is empty.",
            property.name
        ))
    })?;
    let number = number_text.parse::<f64>().map_err(|_| {
        engine_error(format!(
            "Parametric result '{}' for '{}' is not numeric.",
            formatted, property.name
        ))
    })?;
    if !number.is_finite() {
        return Err(ExecutionError::NonFiniteValue);
    }
    if value_type.kind == ElementKind::PrimitiveType {
        match value_type.name.trim().to_ascii_lowercase().as_str() {
            "integer" | "int" | "natural" => {
                if number.fract() != 0.0
                    || number < i64::MIN as f64
                    || number > i64::MAX as f64
                    || (value_type.name.eq_ignore_ascii_case("natural") && number < 0.0)
                {
                    return Err(engine_error(format!(
                        "Parametric result '{}' is incompatible with integer type '{}' for '{}'.",
                        formatted, value_type.name, property.name
                    )));
                }
                return Ok(RuntimeValue::Integer(number as i64));
            }
            "boolean" | "bool" | "string" | "text" => {
                return Err(engine_error(format!(
                    "Parametric result for '{}' cannot be assigned to nonnumeric type '{}'.",
                    property.name, value_type.name
                )));
            }
            _ => {}
        }
    }
    Ok(RuntimeValue::Real(number))
}

fn format_number(value: f64) -> String {
    let mut text = format!("{value:.12}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

fn readable_element(project: &Project, id: ElementId) -> String {
    project.qualified_name(id).unwrap_or_else(|_| {
        project
            .element(id)
            .map(|element| element.name.clone())
            .unwrap_or_else(|_| id.to_string())
    })
}

fn readable_model_error(project: &Project, mut message: String) -> String {
    for element in project.elements.values() {
        let id = element.id.to_string();
        if message.contains(&id) {
            message = message.replace(
                &id,
                &format!("'{}'", readable_element(project, element.id)),
            );
        }
    }
    message
}

fn engine_error(message: String) -> ExecutionError {
    ExecutionError::Engine { message }
}
