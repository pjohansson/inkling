use crate::{consts::TODO_COMMENT_MARKER, error::MetaData};
use std::fmt;

#[derive(Clone, Debug, Default)]
pub struct Logger {
    /// To-do comments.
    pub todo_comments: Vec<LogMessage>,
    /// Non-fatal errors and incompatibilities.
    pub warnings: Vec<LogMessage>,
}

#[allow(dead_code)]
impl Logger {
    pub(crate) fn add_todo(&mut self, comment: &str, meta_data: &MetaData) {
        let without_marker = comment
            .trim_start()
            .trim_start_matches(TODO_COMMENT_MARKER)
            .trim();

        let message = MessageKind::Todo(without_marker.to_string());

        self.todo_comments
            .push(LogMessage::with_kind(message, meta_data));
    }

    pub(crate) fn add_warning(&mut self, warning: Warning, meta_data: &MetaData) {
        self.warnings.push(LogMessage::with_kind(
            MessageKind::Warning(warning),
            meta_data,
        ));
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq))]
///
pub struct LogMessage {
    /// Type of message that was logged.
    pub message: MessageKind,
    /// Meta data of where the message originated from.
    pub meta_data: MetaData,
}

impl LogMessage {
    pub(crate) fn with_kind(message: MessageKind, meta_data: &MetaData) -> Self {
        LogMessage {
            message,
            meta_data: meta_data.clone(),
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq))]
/// Type of log message with content.
pub enum MessageKind {
    /// Todo comment.
    Todo(String),
    /// Non-fatal error or incompatibility.
    Warning(Warning),
}

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq))]
/// A detected non-fatal error or incompatibility.
pub enum Warning {
    /// Found a shuffle sequence but the `random` feature is not enabled.
    ShuffleSequenceNoRandom,
}

impl fmt::Display for LogMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let kind = match &self.message {
            MessageKind::Todo(_) => "TODO",
            MessageKind::Warning(_) => "WARNING",
        };

        write!(f, "[{}] {}: {}", self.meta_data, kind, self.message)
    }
}

impl fmt::Display for MessageKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use MessageKind::*;

        match self {
            Todo(comment) => write!(f, "{}", comment),
            Warning(warning) => write!(f, "{}", warning),
        }
    }
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Warning::*;

        match self {
            ShuffleSequenceNoRandom => write!(
                f,
                "found a shuffle sequence but the `random` feature is not enabled: \
                 changed it to a cycle sequence (fix: compile `inkling` with the \
                 `random` feature)"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todo_comment_messages_are_printed_with_marker() {
        let message = LogMessage::with_kind(MessageKind::Todo("".to_string()), &MetaData::from(2));

        assert!(format!("{}", message).contains("TODO"));
    }

    #[test]
    fn warning_messages_are_printed_with_marker() {
        let warning = Warning::ShuffleSequenceNoRandom;
        let message = LogMessage::with_kind(MessageKind::Warning(warning), &MetaData::from(2));

        assert!(format!("{}", message).contains("WARNING"));
    }
}
