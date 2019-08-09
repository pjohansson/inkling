//! Errors from parsing prelude content.

use std::{error::Error, fmt};

use crate::error::{
    parse::variable::VariableError,
    utils::{write_line_information, MetaData},
};

#[derive(Debug)]
/// Error from parsing a line in the prelude.
pub struct PreludeError {
    /// Line that caused the error.
    pub line: String,
    /// Kind of error.
    pub kind: PreludeErrorKind,
    /// Information about the origin of the line that caused this error.
    pub meta_data: MetaData,
}

#[derive(Debug)]
/// Variant of error from parsing the prelude.
pub enum PreludeErrorKind {
    /// Could not parse a global variable.
    InvalidVariable(VariableError),
    /// No `=` sign was find in a variable assignment line.
    NoVariableAssignment,
    /// No variable name was found in a variable assignment line.
    NoVariableName,
}

impl Error for PreludeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.kind)
    }
}

impl Error for PreludeErrorKind {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self {
            PreludeErrorKind::InvalidVariable(err) => Some(err),
            _ => None,
        }
    }
}

impl_from_error![
    PreludeErrorKind;
    [InvalidVariable, VariableError]
];

impl fmt::Display for PreludeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write_line_information(f, &self.meta_data)?;
        write!(f, "{}", self.kind)
    }
}

impl fmt::Display for PreludeErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use PreludeErrorKind::*;

        match &self {
            InvalidVariable(err) => write!(f, "could not parse variable: {}", err),
            NoVariableAssignment => write!(f, "no variable assignment ('=') in line"),
            NoVariableName => write!(f, "no variable name in line"),
        }
    }
}
