//! Native SysML Parametric semantics and deterministic expression evaluation.
//!
//! ConstraintBlocks remain reusable definitions. ConstraintProperties identify
//! usages of those definitions, and BindingEndpoints therefore include both the
//! usage role and (when applicable) the definition-owned ConstraintParameter.

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BindingEndpoint {
    pub role_id: ElementId,
    #[serde(default)]
    pub parameter_id: Option<ElementId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingConnector {
    pub source: BindingEndpoint,
    pub target: BindingEndpoint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParametricEvaluationScope {
    pub context_id: ElementId,
    pub constraint_property_ids: Vec<ElementId>,
    pub value_property_ids: Vec<ElementId>,
    pub binding_relationship_ids: Vec<RelationshipId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParametricValueUpdate {
    pub element_id: ElementId,
    pub previous_value: Option<String>,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParametricEvaluationReport {
    pub evaluated_constraints: usize,
    pub updates: Vec<ParametricValueUpdate>,
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Left,
    Right,
    Equal,
}

#[derive(Debug, Clone, PartialEq)]
enum Expr {
    Number(f64),
    Variable(String),
    Negate(Box<Expr>),
    Binary(Box<Expr>, char, Box<Expr>),
}

#[derive(Debug, Clone)]
struct ParsedConstraint {
    output: String,
    expression: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Dimension(BTreeMap<String, i32>);

impl Dimension {
    fn parse(value: &str) -> Result<Self, String> {
        let compact = value.split_whitespace().collect::<Vec<_>>().join("*");
        if compact.is_empty() || compact == "1" {
            return Ok(Self::default());
        }
        let mut result = BTreeMap::new();
        let mut start = 0;
        let mut sign = 1;
        let bytes = compact.as_bytes();
        for index in 0..=bytes.len() {
            let delimiter = index == bytes.len() || matches!(bytes[index], b'*' | b'/');
            if !delimiter {
                continue;
            }
            let factor = &compact[start..index];
            if factor.is_empty() {
                return Err("dimension contains an empty factor".into());
            }
            let (symbol, exponent) = factor
                .split_once('^')
                .map_or((factor, Ok(1)), |(symbol, exponent)| {
                    (symbol, exponent.parse::<i32>())
                });
            let exponent = exponent.map_err(|_| "dimension exponent must be an integer")?;
            if symbol.is_empty()
                || !symbol
                    .chars()
                    .all(|character| character.is_ascii_alphabetic() || character == '_')
            {
                return Err("dimension symbols must be ASCII identifiers".into());
            }
            *result.entry(symbol.to_owned()).or_insert(0) += sign * exponent;
            if index < bytes.len() {
                sign = if bytes[index] == b'/' { -1 } else { 1 };
            }
            start = index + 1;
        }
        result.retain(|_, exponent| *exponent != 0);
        Ok(Self(result))
    }

    fn multiply(&self, other: &Self, sign: i32) -> Self {
        let mut result = self.0.clone();
        for (symbol, exponent) in &other.0 {
            *result.entry(symbol.clone()).or_insert(0) += sign * exponent;
        }
        result.retain(|_, exponent| *exponent != 0);
        Self(result)
    }

    fn pow(&self, exponent: i32) -> Self {
        Self(
            self.0
                .iter()
                .map(|(symbol, power)| (symbol.clone(), power * exponent))
                .filter(|(_, power)| *power != 0)
                .collect(),
        )
    }
}

fn lex(expression: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut characters = expression.char_indices().peekable();
    while let Some((index, character)) = characters.next() {
        if character.is_whitespace() {
            continue;
        }
        let token = match character {
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            '^' => Token::Caret,
            '(' => Token::Left,
            ')' => Token::Right,
            '=' => Token::Equal,
            value if value.is_ascii_digit() || value == '.' => {
                let mut end = index + value.len_utf8();
                while let Some((next_index, next)) = characters.peek().copied() {
                    if !next.is_ascii_digit() && next != '.' {
                        break;
                    }
                    characters.next();
                    end = next_index + next.len_utf8();
                }
                Token::Number(
                    expression[index..end]
                        .parse()
                        .map_err(|_| "invalid numeric literal")?,
                )
            }
            value if value.is_ascii_alphabetic() || value == '_' => {
                let mut end = index + value.len_utf8();
                while let Some((next_index, next)) = characters.peek().copied() {
                    if !next.is_ascii_alphanumeric() && next != '_' {
                        break;
                    }
                    characters.next();
                    end = next_index + next.len_utf8();
                }
                Token::Ident(expression[index..end].to_owned())
            }
            _ => return Err(format!("unsupported expression character: {character}")),
        };
        tokens.push(token);
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.cursor)
    }

    fn take(&mut self) -> Option<Token> {
        let token = self.current()?.clone();
        self.cursor += 1;
        Some(token)
    }

    fn parse_constraint(mut self) -> Result<ParsedConstraint, String> {
        let output = match self.take() {
            Some(Token::Ident(value)) => value,
            _ => return Err("constraint expression must begin with an output parameter".into()),
        };
        if self.take() != Some(Token::Equal) {
            return Err("constraint expression must use `output = expression` form".into());
        }
        let expression = self.parse_sum()?;
        if self.current().is_some() {
            return Err("unexpected tokens after constraint expression".into());
        }
        Ok(ParsedConstraint { output, expression })
    }

    fn parse_sum(&mut self) -> Result<Expr, String> {
        let mut value = self.parse_product()?;
        loop {
            let operation = match self.current() {
                Some(Token::Plus) => '+',
                Some(Token::Minus) => '-',
                _ => break,
            };
            self.take();
            value = Expr::Binary(Box::new(value), operation, Box::new(self.parse_product()?));
        }
        Ok(value)
    }

    fn parse_product(&mut self) -> Result<Expr, String> {
        let mut value = self.parse_power()?;
        loop {
            let operation = match self.current() {
                Some(Token::Star) => '*',
                Some(Token::Slash) => '/',
                _ => break,
            };
            self.take();
            value = Expr::Binary(Box::new(value), operation, Box::new(self.parse_power()?));
        }
        Ok(value)
    }

    fn parse_power(&mut self) -> Result<Expr, String> {
        let value = self.parse_unary()?;
        if self.current() == Some(&Token::Caret) {
            self.take();
            return Ok(Expr::Binary(
                Box::new(value),
                '^',
                Box::new(self.parse_power()?),
            ));
        }
        Ok(value)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if self.current() == Some(&Token::Minus) {
            self.take();
            return Ok(Expr::Negate(Box::new(self.parse_unary()?)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.take() {
            Some(Token::Number(value)) => Ok(Expr::Number(value)),
            Some(Token::Ident(value)) => Ok(Expr::Variable(value)),
            Some(Token::Left) => {
                let value = self.parse_sum()?;
                if self.take() != Some(Token::Right) {
                    return Err("missing closing parenthesis".into());
                }
                Ok(value)
            }
            _ => Err("expected a number, parameter, or parenthesized expression".into()),
        }
    }
}

fn parse_constraint(expression: &str) -> Result<ParsedConstraint, String> {
    Parser {
        tokens: lex(expression)?,
        cursor: 0,
    }
    .parse_constraint()
}

fn expression_variables(expression: &Expr, variables: &mut HashSet<String>) {
    match expression {
        Expr::Variable(name) => {
            variables.insert(name.clone());
        }
        Expr::Negate(value) => expression_variables(value, variables),
        Expr::Binary(left, _, right) => {
            expression_variables(left, variables);
            expression_variables(right, variables);
        }
        Expr::Number(_) => {}
    }
}

fn evaluate_expression(expression: &Expr, values: &HashMap<String, f64>) -> Result<f64, String> {
    match expression {
        Expr::Number(value) => Ok(*value),
        Expr::Variable(name) => values
            .get(name)
            .copied()
            .ok_or_else(|| format!("unresolved parameter: {name}")),
        Expr::Negate(value) => Ok(-evaluate_expression(value, values)?),
        Expr::Binary(left, operation, right) => {
            let left = evaluate_expression(left, values)?;
            let right = evaluate_expression(right, values)?;
            match operation {
                '+' => Ok(left + right),
                '-' => Ok(left - right),
                '*' => Ok(left * right),
                '/' if right != 0.0 => Ok(left / right),
                '/' => Err("division by zero".into()),
                '^' => Ok(left.powf(right)),
                _ => Err("unsupported expression operation".into()),
            }
        }
    }
}

fn expression_dimension(
    expression: &Expr,
    dimensions: &HashMap<String, Dimension>,
) -> Result<Dimension, String> {
    match expression {
        Expr::Number(_) => Ok(Dimension::default()),
        Expr::Variable(name) => dimensions
            .get(name)
            .cloned()
            .ok_or_else(|| format!("parameter has no dimension metadata: {name}")),
        Expr::Negate(value) => expression_dimension(value, dimensions),
        Expr::Binary(left, operation, right) => {
            let left_dimension = expression_dimension(left, dimensions)?;
            let right_dimension = expression_dimension(right, dimensions)?;
            match operation {
                '+' | '-' if left_dimension == right_dimension => Ok(left_dimension),
                '+' | '-' => Err("addition/subtraction requires identical dimensions".into()),
                '*' => Ok(left_dimension.multiply(&right_dimension, 1)),
                '/' => Ok(left_dimension.multiply(&right_dimension, -1)),
                '^' => {
                    if right_dimension != Dimension::default() {
                        return Err("an exponent must be dimensionless".into());
                    }
                    let exponent = evaluate_expression(right, &HashMap::new())?;
                    if exponent.fract() != 0.0 || !exponent.is_finite() {
                        return Err("a dimensional exponent must be a finite integer".into());
                    }
                    Ok(left_dimension.pow(exponent as i32))
                }
                _ => Err("unsupported dimension operation".into()),
            }
        }
    }
}

fn type_is_numeric(element: &Element) -> bool {
    if element.kind == ElementKind::ValueType {
        return true;
    }
    if element.kind != ElementKind::PrimitiveType {
        return false;
    }
    matches!(
        element.name.trim().to_ascii_lowercase().as_str(),
        "real" | "float" | "double" | "decimal" | "integer" | "int" | "number" | "natural"
    )
}

fn element_by_external_id<'a>(project: &'a Project, external_id: &str) -> Option<&'a Element> {
    project
        .elements
        .values()
        .find(|element| element.external_id == external_id)
}

fn endpoint_feature<'a>(project: &'a Project, endpoint: &BindingEndpoint) -> Result<&'a Element, ModelError> {
    let role = project.element(endpoint.role_id)?;
    match (role.kind.clone(), endpoint.parameter_id) {
        (ElementKind::ValueProperty, None) => Ok(role),
        (ElementKind::ConstraintProperty, Some(parameter_id)) => {
            let constraint_type = role.type_id.ok_or(ModelError::TypeRequired(role.id))?;
            let parameter = project.element(parameter_id)?;
            if parameter.kind != ElementKind::ConstraintParameter
                || parameter.owner_id != Some(constraint_type)
            {
                return Err(ModelError::InvalidBindingEndpoint(format!(
                    "parameter {parameter_id} is not owned by ConstraintBlock {constraint_type}"
                )));
            }
            Ok(parameter)
        }
        _ => Err(ModelError::InvalidBindingEndpoint(format!(
            "{} must identify a ValueProperty or a ConstraintProperty parameter",
            endpoint.role_id
        ))),
    }
}

fn endpoint_type<'a>(project: &'a Project, endpoint: &BindingEndpoint) -> Result<&'a Element, ModelError> {
    let feature = endpoint_feature(project, endpoint)?;
    project.element(feature.type_id.ok_or(ModelError::TypeRequired(feature.id))?)
}

fn quantity_kind_id(feature: &Element, value_type: &Element) -> Option<String> {
    feature
        .quantity_kind_external_id
        .clone()
        .or_else(|| value_type.quantity_kind_external_id.clone())
}

fn declared_unit<'a>(project: &'a Project, feature: &Element, value_type: &Element) -> Option<&'a Element> {
    feature
        .unit_external_id
        .as_deref()
        .or(value_type.unit_external_id.as_deref())
        .and_then(|external_id| element_by_external_id(project, external_id))
}

fn endpoint_dimension(project: &Project, endpoint: &BindingEndpoint) -> Result<Dimension, ModelError> {
    let feature = endpoint_feature(project, endpoint)?;
    let value_type = endpoint_type(project, endpoint)?;
    if !type_is_numeric(value_type) {
        return Err(ModelError::ParametricEvaluation(format!(
            "{} is typed by nonnumeric {}",
            feature.name, value_type.name
        )));
    }
    let Some(quantity_kind_id) = quantity_kind_id(feature, value_type) else {
        return Ok(Dimension::default());
    };
    let quantity_kind = element_by_external_id(project, &quantity_kind_id)
        .ok_or_else(|| ModelError::InvalidQuantityKindReference(quantity_kind_id.clone()))?;
    if quantity_kind.kind != ElementKind::QuantityKind {
        return Err(ModelError::InvalidQuantityKindReference(quantity_kind_id));
    }
    Dimension::parse(quantity_kind.quantity_dimension.as_deref().unwrap_or("1"))
        .map_err(|_| ModelError::InvalidQuantityDimension(quantity_kind.id))
}

fn binding_types_compatible(
    project: &Project,
    source: &BindingEndpoint,
    target: &BindingEndpoint,
) -> Result<bool, ModelError> {
    let source_feature = endpoint_feature(project, source)?;
    let target_feature = endpoint_feature(project, target)?;
    let source_type = endpoint_type(project, source)?;
    let target_type = endpoint_type(project, target)?;
    let source_numeric = type_is_numeric(source_type);
    let target_numeric = type_is_numeric(target_type);
    if source_numeric != target_numeric {
        return Ok(false);
    }
    let source_quantity = quantity_kind_id(source_feature, source_type);
    let target_quantity = quantity_kind_id(target_feature, target_type);
    if source_quantity.is_some() || target_quantity.is_some() {
        if source_quantity.is_none() || source_quantity != target_quantity {
            return Ok(false);
        }
        if endpoint_dimension(project, source)? != endpoint_dimension(project, target)? {
            return Ok(false);
        }
    }
    if source_type.id == target_type.id {
        return Ok(true);
    }
    Ok(source_type.kind == ElementKind::ValueType
        && target_type.kind == ElementKind::ValueType
        && source_quantity.is_some())
}

fn endpoint_scale(project: &Project, endpoint: &BindingEndpoint) -> Result<f64, ModelError> {
    let feature = endpoint_feature(project, endpoint)?;
    let value_type = endpoint_type(project, endpoint)?;
    Ok(declared_unit(project, feature, value_type)
        .map(|unit| unit.unit_scale_to_base)
        .unwrap_or(1.0))
}

fn parse_value(project: &Project, endpoint: &BindingEndpoint, value: &str) -> Result<f64, ModelError> {
    let mut parts = value.split_whitespace();
    let number = parts
        .next()
        .ok_or_else(|| ModelError::ParametricEvaluation("numeric value is empty".into()))?
        .parse::<f64>()
        .map_err(|_| ModelError::ParametricEvaluation(format!("invalid numeric value: {value}")))?;
    if !number.is_finite() {
        return Err(ModelError::ParametricEvaluation(
            "numeric values must be finite".into(),
        ));
    }
    let inline_unit = parts.next();
    if parts.next().is_some() {
        return Err(ModelError::ParametricEvaluation(format!(
            "invalid value/unit notation: {value}"
        )));
    }
    let scale = if let Some(symbol) = inline_unit {
        let unit = project
            .elements
            .values()
            .find(|element| {
                element.kind == ElementKind::Unit
                    && element.unit_symbol.as_deref() == Some(symbol)
            })
            .ok_or_else(|| ModelError::ParametricEvaluation(format!("unknown unit: {symbol}")))?;
        let feature = endpoint_feature(project, endpoint)?;
        let value_type = endpoint_type(project, endpoint)?;
        let expected_quantity = quantity_kind_id(feature, value_type);
        if expected_quantity.is_none() || unit.quantity_kind_external_id != expected_quantity {
            return Err(ModelError::ParametricEvaluation(format!(
                "unit {symbol} is dimensionally incompatible with {}",
                feature.name
            )));
        }
        unit.unit_scale_to_base
    } else {
        endpoint_scale(project, endpoint)?
    };
    Ok(number * scale)
}

fn format_value(project: &Project, endpoint: &BindingEndpoint, base_value: f64) -> Result<String, ModelError> {
    let scale = endpoint_scale(project, endpoint)?;
    let value = base_value / scale;
    let mut text = format!("{value:.12}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    let feature = endpoint_feature(project, endpoint)?;
    let value_type = endpoint_type(project, endpoint)?;
    if let Some(symbol) = declared_unit(project, feature, value_type)
        .and_then(|unit| unit.unit_symbol.as_deref())
    {
        text.push(' ');
        text.push_str(symbol);
    }
    Ok(text)
}

impl Project {
    pub fn create_binding_connector(
        &mut self,
        owner_id: ElementId,
        source: BindingEndpoint,
        target: BindingEndpoint,
    ) -> Result<RelationshipId, ModelError> {
        let owner = self.element(owner_id)?;
        if !matches!(
            owner.kind,
            ElementKind::Block | ElementKind::AssociationBlock | ElementKind::ConstraintBlock
        ) {
            return Err(ModelError::InvalidOwner(owner_id));
        }
        if source == target {
            return Err(ModelError::BindingSelfConnection);
        }
        for endpoint in [&source, &target] {
            let role = self.element(endpoint.role_id)?;
            if role.owner_id != Some(owner_id) {
                return Err(ModelError::InvalidBindingEndpoint(format!(
                    "{} is not owned by parametric context {owner_id}",
                    role.id
                )));
            }
            endpoint_feature(self, endpoint)?;
        }
        if !binding_types_compatible(self, &source, &target)? {
            return Err(ModelError::IncompatibleBindingTypes {
                source_id: source.role_id,
                target_id: target.role_id,
            });
        }
        if self.relationships.values().any(|relationship| {
            relationship.binding.as_ref().is_some_and(|binding| {
                (binding.source == source && binding.target == target)
                    || (binding.source == target && binding.target == source)
            })
        }) {
            return Err(ModelError::DuplicateBindingConnector);
        }
        let id = self.create_relationship(
            RelationshipKind::BindingConnector,
            source.role_id,
            target.role_id,
            Some(owner_id),
        )?;
        self.relationships
            .get_mut(&id)
            .expect("new BindingConnector relationship")
            .binding = Some(BindingConnector { source, target });
        Ok(id)
    }

    pub fn validate_binding_connector(
        &self,
        relationship: &Relationship,
    ) -> Result<(), ModelError> {
        if relationship.kind != RelationshipKind::BindingConnector {
            return Err(ModelError::RelationshipIsNotBindingConnector(
                relationship.id,
            ));
        }
        let binding = relationship
            .binding
            .as_ref()
            .ok_or(ModelError::RelationshipIsNotBindingConnector(
                relationship.id,
            ))?;
        if binding.source == binding.target {
            return Err(ModelError::BindingSelfConnection);
        }
        if relationship.source_id != binding.source.role_id
            || relationship.target_id != binding.target.role_id
        {
            return Err(ModelError::InvalidBindingEndpoint(
                "relationship endpoints do not match binding roles".into(),
            ));
        }
        let owner_id = relationship.owner_id.ok_or_else(|| {
            ModelError::InvalidBindingEndpoint("BindingConnector requires an owner".into())
        })?;
        for endpoint in [&binding.source, &binding.target] {
            let role = self.element(endpoint.role_id)?;
            if role.owner_id != Some(owner_id) {
                return Err(ModelError::InvalidBindingEndpoint(format!(
                    "{} is outside BindingConnector context {owner_id}",
                    role.id
                )));
            }
            endpoint_feature(self, endpoint)?;
        }
        if !binding_types_compatible(self, &binding.source, &binding.target)? {
            return Err(ModelError::IncompatibleBindingTypes {
                source_id: binding.source.role_id,
                target_id: binding.target.role_id,
            });
        }
        if self.relationships.values().any(|candidate| {
            candidate.id != relationship.id
                && candidate.binding.as_ref().is_some_and(|candidate_binding| {
                    (candidate_binding.source == binding.source
                        && candidate_binding.target == binding.target)
                        || (candidate_binding.source == binding.target
                            && candidate_binding.target == binding.source)
                })
        }) {
            return Err(ModelError::DuplicateBindingConnector);
        }
        Ok(())
    }
}

#[derive(Debug)]
struct EvaluationConstraint {
    property_id: ElementId,
    parsed: ParsedConstraint,
    parameters: HashMap<String, BindingEndpoint>,
}

fn endpoint_components(
    bindings: &[BindingConnector],
) -> (HashMap<BindingEndpoint, usize>, Vec<Vec<BindingEndpoint>>) {
    let mut adjacency: HashMap<BindingEndpoint, Vec<BindingEndpoint>> = HashMap::new();
    for binding in bindings {
        adjacency
            .entry(binding.source.clone())
            .or_default()
            .push(binding.target.clone());
        adjacency
            .entry(binding.target.clone())
            .or_default()
            .push(binding.source.clone());
    }
    let mut component_by_endpoint = HashMap::new();
    let mut components = Vec::new();
    for endpoint in adjacency.keys() {
        if component_by_endpoint.contains_key(endpoint) {
            continue;
        }
        let component_id = components.len();
        let mut component = Vec::new();
        let mut stack = vec![endpoint.clone()];
        while let Some(current) = stack.pop() {
            if component_by_endpoint
                .insert(current.clone(), component_id)
                .is_some()
            {
                continue;
            }
            component.push(current.clone());
            stack.extend(adjacency.get(&current).into_iter().flatten().cloned());
        }
        components.push(component);
    }
    (component_by_endpoint, components)
}

pub fn evaluate_parametrics(
    project: &mut Project,
    scope: &ParametricEvaluationScope,
) -> Result<ParametricEvaluationReport, ModelError> {
    let context = project.element(scope.context_id)?;
    if !matches!(
        context.kind,
        ElementKind::Block | ElementKind::AssociationBlock | ElementKind::ConstraintBlock
    ) {
        return Err(ModelError::InvalidOwner(scope.context_id));
    }

    let bindings = scope
        .binding_relationship_ids
        .iter()
        .map(|id| {
            let relationship = project.relationship(*id)?;
            project.validate_binding_connector(relationship)?;
            relationship
                .binding
                .clone()
                .ok_or(ModelError::RelationshipIsNotBindingConnector(*id))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (component_by_endpoint, components) = endpoint_components(&bindings);

    let constraints = scope
        .constraint_property_ids
        .iter()
        .map(|property_id| {
            let property = project.element(*property_id)?;
            if property.kind != ElementKind::ConstraintProperty
                || property.owner_id != Some(scope.context_id)
            {
                return Err(ModelError::InvalidBindingEndpoint(format!(
                    "{property_id} is not a ConstraintProperty in the Parametric context"
                )));
            }
            let block_id = property.type_id.ok_or(ModelError::TypeRequired(*property_id))?;
            let block = project.element(block_id)?;
            let parsed = parse_constraint(&block.constraint_expression).map_err(|reason| {
                ModelError::InvalidConstraintExpression {
                    element_id: block_id,
                    reason,
                }
            })?;
            let parameters: HashMap<_, _> = project
                .children(block_id)
                .filter(|parameter| parameter.kind == ElementKind::ConstraintParameter)
                .map(|parameter| {
                    (
                        parameter.name.clone(),
                        BindingEndpoint {
                            role_id: *property_id,
                            parameter_id: Some(parameter.id),
                        },
                    )
                })
                .collect();
            if !parameters.contains_key(&parsed.output) {
                return Err(ModelError::InvalidConstraintExpression {
                    element_id: block_id,
                    reason: format!("output parameter '{}' does not exist", parsed.output),
                });
            }
            let mut variables = HashSet::new();
            expression_variables(&parsed.expression, &mut variables);
            for variable in variables {
                if !parameters.contains_key(&variable) {
                    return Err(ModelError::InvalidConstraintExpression {
                        element_id: block_id,
                        reason: format!("parameter '{variable}' does not exist"),
                    });
                }
            }
            for parameter in parameters.values() {
                let feature = endpoint_feature(project, parameter)?;
                if feature.multiplicity.unwrap_or(Multiplicity::ONE).lower > 0
                    && !component_by_endpoint.contains_key(parameter)
                {
                    return Err(ModelError::ParametricEvaluation(format!(
                        "mandatory parameter '{}' is unbound",
                        feature.name
                    )));
                }
            }
            Ok(EvaluationConstraint {
                property_id: *property_id,
                parsed,
                parameters,
            })
        })
        .collect::<Result<Vec<_>, ModelError>>()?;

    let mut producer_by_component = HashMap::new();
    for (index, constraint) in constraints.iter().enumerate() {
        let output = &constraint.parameters[&constraint.parsed.output];
        let component = *component_by_endpoint.get(output).ok_or_else(|| {
            ModelError::ParametricEvaluation(format!(
                "output parameter '{}' is unbound",
                constraint.parsed.output
            ))
        })?;
        if producer_by_component.insert(component, index).is_some() {
            return Err(ModelError::ParametricEvaluation(
                "multiple constraints produce the same bound value".into(),
            ));
        }
    }

    let mut dependencies = vec![HashSet::new(); constraints.len()];
    let mut indegree = vec![0usize; constraints.len()];
    for (consumer, constraint) in constraints.iter().enumerate() {
        let mut variables = HashSet::new();
        expression_variables(&constraint.parsed.expression, &mut variables);
        for variable in variables {
            let endpoint = &constraint.parameters[&variable];
            let component = component_by_endpoint[endpoint];
            if let Some(producer) = producer_by_component.get(&component).copied()
                && producer != consumer
                && dependencies[producer].insert(consumer)
            {
                indegree[consumer] += 1;
            }
        }
    }
    let mut queue: VecDeque<_> = indegree
        .iter()
        .enumerate()
        .filter_map(|(index, degree)| (*degree == 0).then_some(index))
        .collect();
    let mut order = Vec::new();
    while let Some(index) = queue.pop_front() {
        order.push(index);
        for dependent in dependencies[index].iter().copied() {
            indegree[dependent] -= 1;
            if indegree[dependent] == 0 {
                queue.push_back(dependent);
            }
        }
    }
    if order.len() != constraints.len() {
        return Err(ModelError::ParametricEvaluation(
            "constraint dependency cycle detected".into(),
        ));
    }

    let output_components: HashSet<_> = producer_by_component.keys().copied().collect();
    let mut values_by_component = HashMap::new();
    for value_property_id in &scope.value_property_ids {
        let endpoint = BindingEndpoint {
            role_id: *value_property_id,
            parameter_id: None,
        };
        let property = endpoint_feature(project, &endpoint)?;
        if property.owner_id != Some(scope.context_id) {
            return Err(ModelError::InvalidBindingEndpoint(format!(
                "{value_property_id} is outside the Parametric context"
            )));
        }
        let Some(component) = component_by_endpoint.get(&endpoint).copied() else {
            continue;
        };
        if output_components.contains(&component) {
            continue;
        }
        let Some(value) = property
            .default_value
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        else {
            continue;
        };
        let parsed = parse_value(project, &endpoint, value)?;
        if let Some(existing) = values_by_component.insert(component, parsed)
            && (existing - parsed).abs() > 1e-9
        {
            return Err(ModelError::ParametricEvaluation(
                "bound input values disagree".into(),
            ));
        }
    }

    for index in order {
        let constraint = &constraints[index];
        let mut values = HashMap::new();
        let mut dimensions = HashMap::new();
        let mut variables = HashSet::new();
        expression_variables(&constraint.parsed.expression, &mut variables);
        for variable in variables {
            let endpoint = &constraint.parameters[&variable];
            let component = component_by_endpoint[endpoint];
            let value = values_by_component.get(&component).copied().ok_or_else(|| {
                ModelError::ParametricEvaluation(format!(
                    "unresolved parameter '{}' in constraint property {}",
                    variable, constraint.property_id
                ))
            })?;
            values.insert(variable.clone(), value);
            dimensions.insert(variable, endpoint_dimension(project, endpoint)?);
        }
        let output = &constraint.parameters[&constraint.parsed.output];
        let expected_dimension = endpoint_dimension(project, output)?;
        let actual_dimension = expression_dimension(&constraint.parsed.expression, &dimensions)
            .map_err(ModelError::ParametricEvaluation)?;
        if actual_dimension != expected_dimension {
            return Err(ModelError::ParametricEvaluation(format!(
                "constraint '{}' result dimension does not match output '{}'",
                constraint.property_id, constraint.parsed.output
            )));
        }
        let result = evaluate_expression(&constraint.parsed.expression, &values)
            .map_err(ModelError::ParametricEvaluation)?;
        if !result.is_finite() {
            return Err(ModelError::ParametricEvaluation(
                "constraint result is not finite".into(),
            ));
        }
        values_by_component.insert(component_by_endpoint[output], result);
    }

    let mut updates = Vec::new();
    for (component, endpoints) in components.iter().enumerate() {
        let Some(value) = values_by_component.get(&component).copied() else {
            continue;
        };
        for endpoint in endpoints {
            if endpoint.parameter_id.is_some() {
                continue;
            }
            let property = project.element(endpoint.role_id)?;
            if property.kind != ElementKind::ValueProperty {
                continue;
            }
            let next = format_value(project, endpoint, value)?;
            if property.default_value.as_deref() == Some(next.as_str()) {
                continue;
            }
            updates.push(ParametricValueUpdate {
                element_id: property.id,
                previous_value: property.default_value.clone(),
                value: next,
            });
        }
    }
    for update in &updates {
        project.element_mut(update.element_id)?.default_value = Some(update.value.clone());
    }
    Ok(ParametricEvaluationReport {
        evaluated_constraints: constraints.len(),
        updates,
    })
}

pub(crate) fn validate_constraint_block(project: &Project, block: &Element) -> Result<(), ModelError> {
    if block.constraint_expression.trim().is_empty() {
        return Ok(());
    }
    let parsed = parse_constraint(&block.constraint_expression).map_err(|reason| {
        ModelError::InvalidConstraintExpression {
            element_id: block.id,
            reason,
        }
    })?;
    let parameters: HashSet<_> = project
        .children(block.id)
        .filter(|parameter| parameter.kind == ElementKind::ConstraintParameter)
        .map(|parameter| parameter.name.as_str())
        .collect();
    let mut used = HashSet::new();
    expression_variables(&parsed.expression, &mut used);
    used.insert(parsed.output);
    if let Some(missing) = used.iter().find(|name| !parameters.contains(name.as_str())) {
        return Err(ModelError::InvalidConstraintExpression {
            element_id: block.id,
            reason: format!("parameter '{missing}' does not exist"),
        });
    }
    Ok(())
}

pub(crate) fn validate_dimension(value: &str) -> bool {
    Dimension::parse(value).is_ok()
}
