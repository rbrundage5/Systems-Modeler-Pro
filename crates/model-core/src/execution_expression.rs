use crate::RuntimeValue;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ExecutionExpressionError {
    #[error("expression is empty")]
    Empty,
    #[error("invalid token at byte {position}: {message}")]
    InvalidToken { position: usize, message: String },
    #[error("unexpected token at byte {position}: expected {expected}")]
    UnexpectedToken {
        position: usize,
        expected: &'static str,
    },
    #[error("runtime value reference '{name}' is unresolved")]
    UnresolvedReference { name: String },
    #[error("operator '{operator}' does not accept {actual}")]
    InvalidUnaryOperand {
        operator: &'static str,
        actual: &'static str,
    },
    #[error("operator '{operator}' does not accept {left} and {right}")]
    InvalidBinaryOperands {
        operator: &'static str,
        left: &'static str,
        right: &'static str,
    },
    #[error("numeric operation '{operator}' overflowed")]
    NumericOverflow { operator: &'static str },
    #[error("division by zero")]
    DivisionByZero,
    #[error("numeric result is not finite")]
    NonFiniteResult,
}

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Boolean(bool),
    Integer(i64),
    Real(f64),
    Identifier(String),
    LeftParen,
    RightParen,
    Plus,
    Minus,
    Star,
    Slash,
    Power,
    Bang,
    And,
    Or,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    End,
}

#[derive(Debug, Clone, PartialEq)]
struct Token {
    kind: TokenKind,
    position: usize,
}

pub fn evaluate_execution_expression<F>(
    expression: &str,
    resolver: F,
) -> Result<RuntimeValue, ExecutionExpressionError>
where
    F: FnMut(&str) -> Option<RuntimeValue>,
{
    if expression.trim().is_empty() {
        return Err(ExecutionExpressionError::Empty);
    }
    let tokens = tokenize(expression)?;
    let mut parser = Parser {
        tokens,
        cursor: 0,
        resolver,
    };
    let value = parser.parse_or()?;
    if !matches!(parser.current().kind, TokenKind::End) {
        return Err(ExecutionExpressionError::UnexpectedToken {
            position: parser.current().position,
            expected: "end of expression",
        });
    }
    Ok(value)
}

fn tokenize(expression: &str) -> Result<Vec<Token>, ExecutionExpressionError> {
    let bytes = expression.as_bytes();
    let mut cursor = 0;
    let mut tokens = Vec::new();
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if byte.is_ascii_whitespace() {
            cursor += 1;
            continue;
        }
        let position = cursor;
        let two = bytes.get(cursor..cursor + 2);
        let paired = match two {
            Some(b"&&") => Some(TokenKind::And),
            Some(b"||") => Some(TokenKind::Or),
            Some(b"==") => Some(TokenKind::Equal),
            Some(b"!=") => Some(TokenKind::NotEqual),
            Some(b"<=") => Some(TokenKind::LessEqual),
            Some(b">=") => Some(TokenKind::GreaterEqual),
            _ => None,
        };
        if let Some(kind) = paired {
            tokens.push(Token { kind, position });
            cursor += 2;
            continue;
        }
        let single = match byte {
            b'(' => Some(TokenKind::LeftParen),
            b')' => Some(TokenKind::RightParen),
            b'+' => Some(TokenKind::Plus),
            b'-' => Some(TokenKind::Minus),
            b'*' => Some(TokenKind::Star),
            b'/' => Some(TokenKind::Slash),
            b'^' => Some(TokenKind::Power),
            b'!' => Some(TokenKind::Bang),
            b'<' => Some(TokenKind::Less),
            b'>' => Some(TokenKind::Greater),
            _ => None,
        };
        if let Some(kind) = single {
            tokens.push(Token { kind, position });
            cursor += 1;
            continue;
        }
        if byte.is_ascii_digit() || byte == b'.' {
            let start = cursor;
            let mut has_decimal = byte == b'.';
            let mut has_exponent = false;
            cursor += 1;
            while cursor < bytes.len() {
                match bytes[cursor] {
                    b'0'..=b'9' => cursor += 1,
                    b'.' if !has_decimal && !has_exponent => {
                        has_decimal = true;
                        cursor += 1;
                    }
                    b'e' | b'E' if !has_exponent => {
                        has_exponent = true;
                        cursor += 1;
                        if matches!(bytes.get(cursor), Some(b'+' | b'-')) {
                            cursor += 1;
                        }
                    }
                    _ => break,
                }
            }
            let literal = &expression[start..cursor];
            let kind = if has_decimal || has_exponent {
                let value = literal.parse::<f64>().map_err(|_| {
                    ExecutionExpressionError::InvalidToken {
                        position,
                        message: format!("invalid numeric literal '{literal}'"),
                    }
                })?;
                if !value.is_finite() {
                    return Err(ExecutionExpressionError::NonFiniteResult);
                }
                TokenKind::Real(value)
            } else {
                TokenKind::Integer(literal.parse::<i64>().map_err(|_| {
                    ExecutionExpressionError::InvalidToken {
                        position,
                        message: format!("invalid integer literal '{literal}'"),
                    }
                })?)
            };
            tokens.push(Token { kind, position });
            continue;
        }
        if byte.is_ascii_alphabetic() || byte == b'_' {
            let start = cursor;
            cursor += 1;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric()
                    || matches!(bytes[cursor], b'_' | b'.' | b':'))
            {
                cursor += 1;
            }
            let identifier = &expression[start..cursor];
            let kind = match identifier {
                "true" => TokenKind::Boolean(true),
                "false" => TokenKind::Boolean(false),
                _ => TokenKind::Identifier(identifier.into()),
            };
            tokens.push(Token { kind, position });
            continue;
        }
        return Err(ExecutionExpressionError::InvalidToken {
            position,
            message: format!("unsupported character '{}'", char::from(byte)),
        });
    }
    tokens.push(Token {
        kind: TokenKind::End,
        position: expression.len(),
    });
    Ok(tokens)
}

struct Parser<F> {
    tokens: Vec<Token>,
    cursor: usize,
    resolver: F,
}

impl<F> Parser<F>
where
    F: FnMut(&str) -> Option<RuntimeValue>,
{
    fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.cursor].clone();
        self.cursor += 1;
        token
    }

    fn parse_or(&mut self) -> Result<RuntimeValue, ExecutionExpressionError> {
        let mut left = self.parse_and()?;
        while matches!(self.current().kind, TokenKind::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = boolean_binary("||", left, right, |a, b| a || b)?;
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<RuntimeValue, ExecutionExpressionError> {
        let mut left = self.parse_equality()?;
        while matches!(self.current().kind, TokenKind::And) {
            self.advance();
            let right = self.parse_equality()?;
            left = boolean_binary("&&", left, right, |a, b| a && b)?;
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<RuntimeValue, ExecutionExpressionError> {
        let mut left = self.parse_comparison()?;
        loop {
            let operator = match self.current().kind {
                TokenKind::Equal => "==",
                TokenKind::NotEqual => "!=",
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            let equal = values_equal(&left, &right).ok_or_else(|| invalid_binary(
                operator,
                &left,
                &right,
            ))?;
            left = RuntimeValue::Boolean(if operator == "==" { equal } else { !equal });
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<RuntimeValue, ExecutionExpressionError> {
        let mut left = self.parse_additive()?;
        loop {
            let operator = match self.current().kind {
                TokenKind::Less => "<",
                TokenKind::LessEqual => "<=",
                TokenKind::Greater => ">",
                TokenKind::GreaterEqual => ">=",
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            let (a, b) = numeric_pair(operator, &left, &right)?;
            left = RuntimeValue::Boolean(match operator {
                "<" => a < b,
                "<=" => a <= b,
                ">" => a > b,
                _ => a >= b,
            });
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<RuntimeValue, ExecutionExpressionError> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let operator = match self.current().kind {
                TokenKind::Plus => "+",
                TokenKind::Minus => "-",
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = numeric_arithmetic(operator, left, right)?;
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<RuntimeValue, ExecutionExpressionError> {
        let mut left = self.parse_power()?;
        loop {
            let operator = match self.current().kind {
                TokenKind::Star => "*",
                TokenKind::Slash => "/",
                _ => break,
            };
            self.advance();
            let right = self.parse_power()?;
            left = numeric_arithmetic(operator, left, right)?;
        }
        Ok(left)
    }

    fn parse_power(&mut self) -> Result<RuntimeValue, ExecutionExpressionError> {
        let left = self.parse_unary()?;
        if matches!(self.current().kind, TokenKind::Power) {
            self.advance();
            let right = self.parse_power()?;
            return numeric_arithmetic("^", left, right);
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<RuntimeValue, ExecutionExpressionError> {
        if matches!(self.current().kind, TokenKind::Bang) {
            self.advance();
            return match self.parse_unary()? {
                RuntimeValue::Boolean(value) => Ok(RuntimeValue::Boolean(!value)),
                actual => Err(ExecutionExpressionError::InvalidUnaryOperand {
                    operator: "!",
                    actual: actual.kind_name(),
                }),
            };
        }
        if matches!(self.current().kind, TokenKind::Minus) {
            self.advance();
            return match self.parse_unary()? {
                RuntimeValue::Integer(value) => value.checked_neg().map(RuntimeValue::Integer).ok_or(
                    ExecutionExpressionError::NumericOverflow { operator: "-" },
                ),
                RuntimeValue::Real(value) => finite_real(-value),
                actual => Err(ExecutionExpressionError::InvalidUnaryOperand {
                    operator: "-",
                    actual: actual.kind_name(),
                }),
            };
        }
        if matches!(self.current().kind, TokenKind::Plus) {
            self.advance();
            let value = self.parse_unary()?;
            return if is_numeric(&value) {
                Ok(value)
            } else {
                Err(ExecutionExpressionError::InvalidUnaryOperand {
                    operator: "+",
                    actual: value.kind_name(),
                })
            };
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<RuntimeValue, ExecutionExpressionError> {
        let token = self.advance();
        match token.kind {
            TokenKind::Boolean(value) => Ok(RuntimeValue::Boolean(value)),
            TokenKind::Integer(value) => Ok(RuntimeValue::Integer(value)),
            TokenKind::Real(value) => Ok(RuntimeValue::Real(value)),
            TokenKind::Identifier(name) => (self.resolver)(&name)
                .ok_or(ExecutionExpressionError::UnresolvedReference { name }),
            TokenKind::LeftParen => {
                let value = self.parse_or()?;
                if !matches!(self.current().kind, TokenKind::RightParen) {
                    return Err(ExecutionExpressionError::UnexpectedToken {
                        position: self.current().position,
                        expected: "')'",
                    });
                }
                self.advance();
                Ok(value)
            }
            _ => Err(ExecutionExpressionError::UnexpectedToken {
                position: token.position,
                expected: "literal, runtime value reference, unary operator, or '('",
            }),
        }
    }
}

fn boolean_binary(
    operator: &'static str,
    left: RuntimeValue,
    right: RuntimeValue,
    operation: impl FnOnce(bool, bool) -> bool,
) -> Result<RuntimeValue, ExecutionExpressionError> {
    match (left, right) {
        (RuntimeValue::Boolean(left), RuntimeValue::Boolean(right)) => {
            Ok(RuntimeValue::Boolean(operation(left, right)))
        }
        (left, right) => Err(invalid_binary(operator, &left, &right)),
    }
}

fn numeric_arithmetic(
    operator: &'static str,
    left: RuntimeValue,
    right: RuntimeValue,
) -> Result<RuntimeValue, ExecutionExpressionError> {
    if operator != "/"
        && operator != "^"
        && let (RuntimeValue::Integer(left), RuntimeValue::Integer(right)) = (&left, &right)
    {
        let result = match operator {
            "+" => left.checked_add(*right),
            "-" => left.checked_sub(*right),
            "*" => left.checked_mul(*right),
            _ => None,
        };
        return result
            .map(RuntimeValue::Integer)
            .ok_or(ExecutionExpressionError::NumericOverflow { operator });
    }
    let (left, right) = numeric_pair(operator, &left, &right)?;
    if operator == "/" && right == 0.0 {
        return Err(ExecutionExpressionError::DivisionByZero);
    }
    finite_real(match operator {
        "+" => left + right,
        "-" => left - right,
        "*" => left * right,
        "/" => left / right,
        "^" => left.powf(right),
        _ => unreachable!("parser supplies a supported numeric operator"),
    })
}

fn numeric_pair(
    operator: &'static str,
    left: &RuntimeValue,
    right: &RuntimeValue,
) -> Result<(f64, f64), ExecutionExpressionError> {
    let convert = |value: &RuntimeValue| match value {
        RuntimeValue::Integer(value) => Some(*value as f64),
        RuntimeValue::Real(value) => Some(*value),
        _ => None,
    };
    match (convert(left), convert(right)) {
        (Some(left), Some(right)) => Ok((left, right)),
        _ => Err(invalid_binary(operator, left, right)),
    }
}

fn values_equal(left: &RuntimeValue, right: &RuntimeValue) -> Option<bool> {
    match (left, right) {
        (RuntimeValue::Integer(left), RuntimeValue::Real(right)) => {
            Some((*left as f64) == *right)
        }
        (RuntimeValue::Real(left), RuntimeValue::Integer(right)) => {
            Some(*left == (*right as f64))
        }
        (RuntimeValue::Boolean(left), RuntimeValue::Boolean(right)) => Some(left == right),
        (RuntimeValue::Integer(left), RuntimeValue::Integer(right)) => Some(left == right),
        (RuntimeValue::Real(left), RuntimeValue::Real(right)) => Some(left == right),
        (RuntimeValue::Text(left), RuntimeValue::Text(right)) => Some(left == right),
        (RuntimeValue::ElementReference(left), RuntimeValue::ElementReference(right)) => {
            Some(left == right)
        }
        (RuntimeValue::Unset, RuntimeValue::Unset) => Some(true),
        _ => None,
    }
}

fn finite_real(value: f64) -> Result<RuntimeValue, ExecutionExpressionError> {
    if value.is_finite() {
        Ok(RuntimeValue::Real(value))
    } else {
        Err(ExecutionExpressionError::NonFiniteResult)
    }
}

fn is_numeric(value: &RuntimeValue) -> bool {
    matches!(value, RuntimeValue::Integer(_) | RuntimeValue::Real(_))
}

fn invalid_binary(
    operator: &'static str,
    left: &RuntimeValue,
    right: &RuntimeValue,
) -> ExecutionExpressionError {
    ExecutionExpressionError::InvalidBinaryOperands {
        operator,
        left: left.kind_name(),
        right: right.kind_name(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_precedence_boolean_and_runtime_references() {
        let result = evaluate_execution_expression("speed >= 20 && enabled && 2 + 3 * 4 == 14", |name| {
            match name {
                "speed" => Some(RuntimeValue::Real(27.5)),
                "enabled" => Some(RuntimeValue::Boolean(true)),
                _ => None,
            }
        })
        .unwrap();
        assert_eq!(result, RuntimeValue::Boolean(true));
    }

    #[test]
    fn rejects_unresolved_and_unsafe_syntax() {
        assert!(matches!(
            evaluate_execution_expression("missing > 0", |_| None),
            Err(ExecutionExpressionError::UnresolvedReference { .. })
        ));
        assert!(matches!(
            evaluate_execution_expression("x = 1", |_| None),
            Err(ExecutionExpressionError::InvalidToken { .. })
        ));
    }
}
