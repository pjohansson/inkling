use crate::{
    consts::TODO_COMMENT_MARKER,
    error::MetaData,
    log::{LogMessage, MessageKind, Warning},
};

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
