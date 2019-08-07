//! Errors from parsing prelude content.

use std::{error::Error, fmt};

use crate::{error::parse::VariableError, utils::MetaData};

impl Error for PreludeError {}

#[derive(Debug)]
pub struct PreludeError {
    /// Line that caused the error.
    pub line: String,
    /// Kind of error.
    pub kind: PreludeErrorKind,
    /// Information about the origin of the line that caused this error.
    pub meta_data: MetaData,
}

#[derive(Debug)]
pub enum PreludeErrorKind {
    /// Could not parse a global variable.
    InvalidVariable(VariableError),
    /// No `=` sign was find in a variable assignment line.
    NoVariableAssignment,
    /// No variable name was found in a variable assignment line.
    NoVariableName,
}

impl_from_error![
    PreludeErrorKind;
    [InvalidVariable, VariableError]
];

impl fmt::Display for PreludeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use PreludeErrorKind::*;

        match &self.kind {
            InvalidVariable(err) => write!(f, "could not parse variable: {}", err),
            NoVariableAssignment => write!(f, "no variable assignment in line"),
            NoVariableName => write!(f, "no variable name in line"),
        }
    }
}
