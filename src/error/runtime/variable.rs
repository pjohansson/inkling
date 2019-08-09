//! Errors from variable assignments or operations.

use std::{error::Error, fmt};

use crate::{
    follow::ChoiceInfo,
    knot::{Address, AddressKind},
    line::Variable,
    node::Stack,
    story::Choice,
};

use std::cmp::Ordering;

impl Error for VariableError {}

#[derive(Clone, Debug)]
/// Error from invalid variable assignments or operations.
pub struct VariableError {
    /// Variable that caused or detected the error.
    pub variable: Variable,
    /// Error variant.
    pub kind: VariableErrorKind,
}

impl VariableError {
    pub(crate) fn from_kind<T: Into<Variable>>(variable: T, kind: VariableErrorKind) -> Self {
        VariableError {
            variable: variable.into(),
            kind,
        }
    }
}

#[derive(Clone, Debug)]
/// Error variant for variable type errors.
pub enum VariableErrorKind {
    /// Divided with or took the remainer from 0.
    DividedByZero {
        /// Zero-valued variable in the operation.
        other: Variable,
        /// Character representation of the operation that caused the error (`/`, `%`).
        operator: char,
    },
    /// Two variables could not be compared to each other like this.
    InvalidComparison {
        /// Other variable in the comparison.
        other: Variable,
        /// Type of comparison betweeen `variable` and `other`.
        comparison: Ordering,
    },
    /// Tried to operate on the variable with an operation that is not allowed for it.
    NonAllowedOperation {
        /// Other variable in the operation.
        other: Variable,
        /// Character representation of operation (`+`, `-`, `*`, `/`, `%`).
        operator: char,
    },
    /// A new variable type was attempted to be assigned to the current variable.
    NonMatchingAssignment {
        /// Variable that was to be assigned but has non-matching type.
        other: Variable,
    },
}

impl fmt::Display for VariableError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use VariableErrorKind::*;

        let variable = &self.variable;

        match &self.kind {
            DividedByZero { other, operator } => write!(
                f,
                "Attempted to divide by 0 in the operation '{:?} {} {:?}",
                variable, operator, other
            ),
            InvalidComparison { other, comparison } => {
                let operator = match comparison {
                    Ordering::Equal => "==",
                    Ordering::Less => ">",
                    Ordering::Greater => "<",
                };

                write!(
                    f,
                    "Cannot compare variable of type '{}' to '{}' using the '{op}' operator \
                     (comparison was: '{:?} {op} {:?}')",
                    variable.variant_string(),
                    other.variant_string(),
                    variable,
                    other,
                    op = operator
                )
            }
            NonAllowedOperation { other, operator } => write!(
                f,
                "Operation '{op}' is not allowed between variables of type '{}' and '{}' \
                 (operation was: '{:?} {op} {:?}')",
                variable.variant_string(),
                other.variant_string(),
                variable,
                other,
                op = operator
            ),
            NonMatchingAssignment { other } => write!(
                f,
                "Cannot assign a value of type '{}' to a variable of type '{}' \
                 (variables cannot change type)",
                other.variant_string(),
                variable.variant_string()
            ),
        }
    }
}
