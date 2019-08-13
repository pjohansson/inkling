//! Errors from validating the content of a story that was successfully parsed.
//!
//! Validation is done in a separate step, after a story has been successfully parsed.
//! This pass will check for errors in expressions, conditions, naming and assignments
//! throughout the entire story. Any invalid types or names will yield a
//! [`ValidationError`][crate::error::parse::validate::ValidationError].

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

#[derive(Debug)]
/// Collection of errors encountered when validating a story.
pub struct ValidationError {
    /// Errors from invalid addresses to knots, stitchs or variables.
    pub invalid_address_errors: Vec<InvalidAddressError>,
    /// Errors from name space collisions between knots, stitches and variables.
    ///
    /// Stitches or global variables may not have the same name as any knot in the story. Nor may
    /// stitches have the same name as any global variable.
    ///
    /// This is to ensure that addresses are well determined. Internal addresses to stitches
    /// within knots can exclude the knot name, meaning that if a stitch and knot share a name
    /// it will not be clear which of the two an address refers to. The same problem goes for
    /// global variables.
    pub name_space_errors: Vec<NameSpaceCollision>,
    /// Errors from expressions and conditions containing invalid variables.
    ///
    /// Expressions must evaluate to a valid variable. For example, mathematics can only include
    /// numerical variables and will produce a numerical variable. If an expression in the story
    /// tries to add a string to an integer an error will be yielded in this collection.
    ///
    /// Conditions likewise cannot compare different types to each other. Such errors will also
    /// be collected in this set.
    ///
    /// See [`Variable`][crate::line::Variable] for more information about valid operations
    /// and comparisons between variables.
    pub variable_errors: Vec<InvalidVariableExpression>,
}

impl ValidationError {
    /// Construct an empty error.
    pub(crate) fn new() -> Self {
        ValidationError {
            invalid_address_errors: Vec::new(),
            name_space_errors: Vec::new(),
            variable_errors: Vec::new(),
        }
    }

    /// Assert whether no errors have been added to the set.
    pub(crate) fn is_empty(&self) -> bool {
        self.num_errors() == 0
    }

    /// Get the number of errors in the set.
    pub(crate) fn num_errors(&self) -> usize {
        self.invalid_address_errors.len()
            + self.name_space_errors.len()
            + self.variable_errors.len()
    }
}

#[derive(Debug)]
/// Error type for invalid variables inside expressions and conditions.
pub struct InvalidVariableExpression {
    /// Whether the error is in a condition or expression.
    pub expression_kind: ExpressionKind,
    /// Variant of error that was encountered.
    pub kind: InvalidVariableExpressionError,
    /// Information about the origin of the line containing this error.
    pub meta_data: MetaData,
}

#[derive(Debug)]
/// Kind of encountered invalid expression.
pub enum ExpressionKind {
    Condition,
    Expression,
}

#[derive(Debug)]
/// Error variant for invalid variables inside expressions and conditions.
pub enum InvalidVariableExpressionError {
    /// An invalid variable assignment, comparison or operation caused the error.
    ///
    /// Most if not all invalid errors should be of this type.
    VariableError(VariableError),
    /// Other errors inside the validated item.
    ///
    /// Represents that something that is not a simple variable type or invalid address caused
    /// the error. This is likely due to some bad assumptions inside `inkling` itself.
    Internal(InklingError),
}

#[derive(Debug)]
/// Error type for name space collisions.
pub struct NameSpaceCollision {
    /// Shared name of the two items.
    pub name: String,
    /// Kind of item that encountered the collision.
    pub from_kind: CollisionKind,
    /// Information about the origin of the line that encountered the collision
    pub from_meta_data: MetaData,
    /// Kind of item that was already present in the story.
    pub to_kind: CollisionKind,
    /// Information about the origin of the line with the item that was already present.
    pub to_meta_data: MetaData,
}

#[derive(Clone, Copy, Debug)]
/// Kind of item that encountered a name space collision.
pub enum CollisionKind {
    Knot,
    Stitch,
    Variable,
}

/// Print every error that was encountered as a separate line.
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

impl Error for ValidationError {}

impl Error for NameSpaceCollision {}

impl Error for InvalidVariableExpression {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            InvalidVariableExpressionError::Internal(err) => Some(err),
            InvalidVariableExpressionError::VariableError(err) => Some(err),
        }
    }
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
        write!(
            f,
            "Encountered {} invalid address, {} name space collision \
             and {} invalid variable errors during validation",
            self.invalid_address_errors.len(),
            self.name_space_errors.len(),
            self.variable_errors.len()
        )
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
