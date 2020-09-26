use crate::{consts::TODO_COMMENT_MARKER, error::MetaData};

#[derive(Clone, Debug)]
pub struct Message {
    pub message: String,
    pub meta_data: MetaData,
}

impl Message {
    pub fn new(msg: &str, meta_data: &MetaData) -> Self {
        Message {
            message: msg.to_string(),
            meta_data: meta_data.clone(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Logger {
    pub todo_comments: Vec<Message>,
    pub warnings: Vec<Message>,
}

#[allow(dead_code)]
impl Logger {
    pub(crate) fn add_todo(&mut self, comment: &str, meta_data: &MetaData) {
        let without_marker = comment
            .trim_start()
            .trim_start_matches(TODO_COMMENT_MARKER)
            .trim();

        self.todo_comments
            .push(Message::new(without_marker, meta_data));
    }

    pub(crate) fn add_warning(&mut self, warning: &str, meta_data: &MetaData) {
        self.warnings.push(Message::new(warning, meta_data));
    }
}
