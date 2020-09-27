use crate::{
    consts::TODO_COMMENT_MARKER,
    error::MetaData,
    log::{LogMessage, MessageKind, Warning},
};

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default)]
#[cfg_attr(test, derive(PartialEq))]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Log of warnings and to-do comments of the current script.
///
/// Use `Logger::iter()` to iterate over the warning messages. All messages implement `Display`
/// which means that printing the errors to any sort of string buffer or file is trivial.
///
/// # Examples
/// ```
/// # use inkling::read_story_from_string;
/// # let content = "Story content.";
/// let story = read_story_from_string(content).unwrap();
///
/// for msg in story.get_log().iter() {
///     eprintln!("{}", msg);
/// }
/// ```
pub struct Logger {
    /// To-do comments.
    pub todo_comments: Vec<LogMessage>,
    /// Non-fatal errors and incompatibilities.
    pub warnings: Vec<LogMessage>,
}

#[allow(dead_code)]
impl Logger {
    /// Return whether or not the log has any entries.
    pub fn has_entries(&self) -> bool {
        !self.todo_comments.is_empty() || !self.warnings.is_empty()
    }

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

/****************************
 * Iterator implementations *
 ****************************/

impl Logger {
    /// Iterate over the logged messages.
    ///
    /// The iterator visits the messages in the order of their line numbers.
    pub fn iter(&self) -> LoggerIter {
        LoggerIter {
            todo_comments: self.todo_comments.iter().peekable(),
            warnings: self.warnings.iter().peekable(),
        }
    }
}

impl IntoIterator for Logger {
    type Item = LogMessage;
    type IntoIter = LoggerIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        LoggerIntoIter {
            todo_comments: self.todo_comments.into_iter().peekable(),
            warnings: self.warnings.into_iter().peekable(),
        }
    }
}

pub struct LoggerIntoIter {
    todo_comments: std::iter::Peekable<std::vec::IntoIter<LogMessage>>,
    warnings: std::iter::Peekable<std::vec::IntoIter<LogMessage>>,
}

impl Iterator for LoggerIntoIter {
    type Item = LogMessage;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.todo_comments.peek(), self.warnings.peek()) {
            (Some(msg_todo), Some(msg_warn)) => {
                if msg_todo.meta_data.line() < msg_warn.meta_data.line() {
                    self.todo_comments.next()
                } else {
                    self.warnings.next()
                }
            }
            _ => self.todo_comments.next().or(self.warnings.next()),
        }
    }
}

pub struct LoggerIter<'a> {
    todo_comments: std::iter::Peekable<std::slice::Iter<'a, LogMessage>>,
    warnings: std::iter::Peekable<std::slice::Iter<'a, LogMessage>>,
}

impl<'a> Iterator for LoggerIter<'a> {
    type Item = &'a LogMessage;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.todo_comments.peek(), self.warnings.peek()) {
            (Some(msg_todo), Some(msg_warn)) => {
                if msg_todo.meta_data.line() < msg_warn.meta_data.line() {
                    self.todo_comments.next()
                } else {
                    self.warnings.next()
                }
            }
            _ => self.todo_comments.next().or(self.warnings.next()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iterating_through_log_yields_todo_comments_in_order() {
        let mut logger = Logger::default();

        logger.add_todo("Comment 1", &MetaData::from(0));
        logger.add_todo("Comment 2", &MetaData::from(1));
        logger.add_todo("Comment 3", &MetaData::from(2));
        logger.add_todo("Comment 4", &MetaData::from(3));

        let logged_messages = logger.todo_comments.clone();
        let iter_messages = logger.iter().cloned().collect::<Vec<_>>();

        assert_eq!(iter_messages, logged_messages);
    }

    #[test]
    fn iterating_through_log_yields_warnings_in_order() {
        let mut logger = Logger::default();

        logger.add_warning(Warning::ShuffleSequenceNoRandom, &MetaData::from(0));
        logger.add_warning(Warning::ShuffleSequenceNoRandom, &MetaData::from(1));

        let logged_messages = logger.warnings.clone();
        let iter_messages = logger.iter().cloned().collect::<Vec<_>>();

        assert_eq!(iter_messages, logged_messages);
    }

    #[test]
    fn iterating_through_log_yields_comments_and_warnings_in_line_index_order() {
        let mut logger = Logger::default();

        // `MetaData::line_index` determines order
        logger.add_todo("Comment 1", &MetaData::from(1));
        logger.add_todo("Comment 2", &MetaData::from(2));
        logger.add_warning(Warning::ShuffleSequenceNoRandom, &MetaData::from(0));
        logger.add_warning(Warning::ShuffleSequenceNoRandom, &MetaData::from(3));

        let todo_comments = logger.todo_comments.clone();
        let warnings = logger.warnings.clone();

        let mut iter = logger.iter().cloned();

        assert_eq!(iter.next().unwrap(), warnings[0]);
        assert_eq!(iter.next().unwrap(), todo_comments[0]);
        assert_eq!(iter.next().unwrap(), todo_comments[1]);
        assert_eq!(iter.next().unwrap(), warnings[1]);
        assert!(iter.next().is_none());
    }

    #[test]
    fn into_iter_yields_items_in_same_order_as_iter() {
        let mut logger = Logger::default();

        logger.add_todo("Comment 1", &MetaData::from(1));
        logger.add_todo("Comment 2", &MetaData::from(2));
        logger.add_warning(Warning::ShuffleSequenceNoRandom, &MetaData::from(0));
        logger.add_warning(Warning::ShuffleSequenceNoRandom, &MetaData::from(3));

        let iter_messages = logger.iter().cloned().collect::<Vec<_>>();
        let into_iter_messages = logger.into_iter().collect::<Vec<_>>();

        assert_eq!(into_iter_messages.len(), 4);
        assert_eq!(into_iter_messages, iter_messages);
    }

    #[test]
    fn logger_has_entries_if_todo_or_warnings_list_has() {
        let mut logger = Logger::default();
        assert!(!logger.has_entries());

        logger.add_todo("Comment 1", &MetaData::from(1));
        assert!(logger.has_entries());

        logger.todo_comments.clear();
        assert!(!logger.has_entries());

        logger.add_warning(Warning::ShuffleSequenceNoRandom, &MetaData::from(0));
        assert!(logger.has_entries());

        logger.warnings.clear();
        assert!(!logger.has_entries());

        logger.add_todo("Comment 1", &MetaData::from(1));
        logger.add_warning(Warning::ShuffleSequenceNoRandom, &MetaData::from(0));
        assert!(logger.has_entries());
    }
}
