//! Utilities for story content.

use crate::story::types::LineBuffer;

/// Read all text from lines in a buffer into a single string and return it.
///
/// # Examples
/// ```
/// # use inkling::{copy_lines_into_string, read_story_from_string};
/// let content = "\
/// Gamle gode Väinämöinen
/// rustade sig nu att resa
/// bort till kyligare trakter
/// till de dunkla Nordanlanden.
/// ";
///
/// let mut story = read_story_from_string(content).unwrap();
/// let mut line_buffer = Vec::new();
///
/// story.resume(&mut line_buffer);
///
/// let text = copy_lines_into_string(&line_buffer);
/// assert_eq!(&text, content);
/// ```
pub fn copy_lines_into_string(line_buffer: &LineBuffer) -> String {
    line_buffer
        .iter()
        .map(|line| line.text.clone())
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::story::Line;

    #[test]
    fn string_from_line_buffer_joins_without_extra_newlines() {
        let lines = vec![
            Line {
                text: "Start of line, ".to_string(),
                tags: Vec::new(),
            },
            Line {
                text: "end of line without new lines".to_string(),
                tags: Vec::new(),
            },
        ];

        assert_eq!(
            &copy_lines_into_string(&lines),
            "Start of line, end of line without new lines"
        );
    }
}
