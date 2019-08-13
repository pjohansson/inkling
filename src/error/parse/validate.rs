use crate::error::{
    parse::address::InvalidAddressError,
    runtime::variable::VariableError,
    utils::{write_line_information, MetaData},
    InklingError,
};

use std::{
    error::Error,
    fmt::{self, Write},
};

impl Error for ValidationError {}

#[derive(Debug)]
pub struct ValidationError {
    pub invalid_address_errors: Vec<InvalidAddressError>,
    pub name_space_errors: Vec<NameSpaceCollision>,
    pub variable_errors: Vec<InvalidVariableExpression>,
}

impl ValidationError {
    pub fn new() -> Self {
        ValidationError {
            invalid_address_errors: Vec::new(),
            name_space_errors: Vec::new(),
            variable_errors: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.invalid_address_errors.is_empty()
            && self.name_space_errors.is_empty()
            && self.variable_errors.is_empty()
    }

    pub fn num_errors(&self) -> usize {
        self.invalid_address_errors.len()
            + self.name_space_errors.len()
            + self.variable_errors.len()
    }
}

#[derive(Debug)]
pub struct InvalidVariableExpression {
    pub expression_kind: ExpressionKind,
    pub kind: InvalidVariableExpressionError,
    pub meta_data: MetaData,
}

#[derive(Debug)]
pub enum ExpressionKind {
    Condition,
    Expression,
}

#[derive(Debug)]
pub enum InvalidVariableExpressionError {
    Internal(InklingError),
    VariableError(VariableError),
}

#[derive(Debug)]
pub struct NameSpaceCollision {
    pub name: String,
    pub from_kind: CollisionKind,
    pub from_meta_data: MetaData,
    pub to_kind: CollisionKind,
    pub to_meta_data: MetaData,
}

#[derive(Clone, Copy, Debug)]
pub enum CollisionKind {
    Knot,
    Stitch,
    Variable,
}

pub(super) fn print_validation_error(error: &ValidationError) -> Result<String, fmt::Error> {
    let mut buffer = String::new();

    for err in &error.invalid_address_errors {
        write!(&mut buffer, "{}\n", err)?;
    }

    for err in &error.name_space_errors {
        write!(&mut buffer, "{}\n", err)?;
    }

    for err in &error.variable_errors {
        write!(&mut buffer, "{}\n", err)?;
    }

    Ok(buffer)
}

impl From<InklingError> for InvalidVariableExpressionError {
    fn from(err: InklingError) -> Self {
        match err {
            InklingError::VariableError(err) => InvalidVariableExpressionError::VariableError(err),
            _ => InvalidVariableExpressionError::Internal(err),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unimplemented!();
    }
}

impl fmt::Display for InvalidVariableExpression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write_line_information(f, &self.meta_data)?;

        match &self.kind {
            InvalidVariableExpressionError::VariableError(err) => {
                write!(f, "Invalid {}: {}", &self.expression_kind, err)
            }
            InvalidVariableExpressionError::Internal(err) => write!(
                f,
                "Unknown internal inconsistency in {}: {}\n",
                &self.expression_kind, err
            ),
        }
    }
}

impl fmt::Display for ExpressionKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            ExpressionKind::Condition => write!(f, "condition"),
            ExpressionKind::Expression => write!(f, "expression"),
        }
    }
}

impl fmt::Display for NameSpaceCollision {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write_line_information(f, &self.from_meta_data)?;

        write!(
            f,
            "Name space collision for {} of name '{}' which is also defined as a {} at {}",
            self.from_kind, self.name, self.to_kind, self.to_meta_data
        )
    }
}

impl fmt::Display for CollisionKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            CollisionKind::Knot => write!(f, "knot"),
            CollisionKind::Stitch => write!(f, "stitch"),
            CollisionKind::Variable => write!(f, "global variable"),
        }
    }
}
