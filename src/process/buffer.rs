use crate::{
    follow::{LineDataBuffer, LineText},
    story::{Line, LineBuffer},
};

/// Process internal lines to a user-ready state.
pub fn process_buffer(into_buffer: &mut LineBuffer, from_buffer: LineDataBuffer) {
    let mut iter = from_buffer
        .into_iter()
        .filter(|line| !line.text.trim().is_empty())
        .peekable();

    while let Some(mut line) = iter.next() {
        let (glue, whitespace) = check_for_whitespace_and_glue(&line, iter.peek());

        trim_extra_whitespace(&mut line);
        add_line_ending(&mut line, glue, whitespace);

        into_buffer.push(Line {
            text: line.text,
            tags: line.tags,
        });
    }
}

/// Check whether the line is glued to the next and if so whether it ends with a blank space.
fn check_for_whitespace_and_glue(line: &LineText, next_line: Option<&LineText>) -> (bool, bool) {
    let glue = next_line
        .map(|next_line| line.glue_end || next_line.glue_begin)
        .unwrap_or(false);

    let whitespace = glue && {
        next_line
            .map(|next_line| line.text.ends_with(' ') || next_line.text.starts_with(' '))
            .unwrap_or(false)
    };

    (glue, whitespace)
}

/// Trim multiple whitespace characters between words.
fn trim_extra_whitespace(line: &mut LineText) {
    let trimmed = line.text.split_whitespace().collect::<Vec<_>>().join(" ");

    line.text = trimmed;
}

/// Add a newline character to the current line if it is not glued to the next.
///
/// Ensures that only a single whitespace remains between the lines if they are glued.
fn add_line_ending(line: &mut LineText, glue: bool, whitespace: bool) {
    if !glue || whitespace {
        let mut text = line.text.trim().to_string();

        if whitespace {
            text.push(' ');
        }

        if !glue {
            text.push('\n');
        }

        line.text = text;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::follow::LineTextBuilder;

    #[test]
    fn processing_line_buffer_removes_empty_lines() {
        let text = "Mr. and Mrs. Doubtfire";

        let buffer = vec![
            LineTextBuilder::from_string(text).build(),
            LineTextBuilder::from_string("").build(),
            LineTextBuilder::from_string(text).build(),
        ];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert_eq!(processed.len(), 2);
        assert_eq!(processed[0].text.trim(), text);
        assert_eq!(processed[1].text.trim(), text);
    }

    #[test]
    fn processing_line_buffer_trims_extra_whitespace() {
        let buffer = vec![
            LineTextBuilder::from_string("    Hello, World!    ").build(),
            LineTextBuilder::from_string("    Hello right back at you!  ").build(),
        ];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert_eq!(processed.len(), 2);
        assert_eq!(processed[0].text.trim(), "Hello, World!");
        assert_eq!(processed[1].text.trim(), "Hello right back at you!");
    }

    #[test]
    fn processing_line_buffer_adds_newlines_if_no_glue() {
        let text = "Mr. and Mrs. Doubtfire";

        let buffer = vec![
            LineTextBuilder::from_string(text).build(),
            LineTextBuilder::from_string(text).build(),
        ];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert!(processed[0].text.ends_with('\n'));
        assert!(processed[1].text.ends_with('\n'));
    }

    #[test]
    fn processing_line_buffer_removes_newlines_between_lines_with_glue_end_on_first() {
        let text = "Mr. and Mrs. Doubtfire";

        let buffer = vec![
            LineTextBuilder::from_string(text).with_glue_end().build(),
            LineTextBuilder::from_string(text).build(),
        ];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert!(!processed[0].text.ends_with('\n'));
        assert!(processed[1].text.ends_with('\n'));
    }

    #[test]
    fn processing_line_buffer_removes_newlines_between_lines_with_glue_start_on_second() {
        let text = "Mr. and Mrs. Doubtfire";

        let buffer = vec![
            LineTextBuilder::from_string(text).build(),
            LineTextBuilder::from_string(text).with_glue_begin().build(),
        ];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert!(!processed[0].text.ends_with('\n'));
        assert!(processed[1].text.ends_with('\n'));
    }

    #[test]
    fn processing_line_buffer_with_glue_works_across_empty_lines() {
        let text = "Mr. and Mrs. Doubtfire";

        let buffer = vec![
            LineTextBuilder::from_string(text).build(),
            LineTextBuilder::from_string("").build(),
            LineTextBuilder::from_string(text).with_glue_begin().build(),
        ];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert!(!processed[0].text.ends_with('\n'));
        assert!(processed[1].text.ends_with('\n'));
    }

    #[test]
    fn processing_line_buffer_sets_newline_on_last_line_regardless_of_glue() {
        let line = LineTextBuilder::from_string("Mr. and Mrs. Doubtfire")
            .with_glue_end()
            .build();

        let buffer = vec![line];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert!(processed[0].text.ends_with('\n'));
    }

    #[test]
    fn processing_line_buffer_keeps_single_whitespace_between_lines_with_glue() {
        let line1 = LineTextBuilder::from_string("Ends with whitespace before glue, ")
            .with_glue_end()
            .build();
        let line2 = LineTextBuilder::from_string(" starts with whitespace after glue")
            .with_glue_begin()
            .build();

        let buffer = vec![line1, line2];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert!(processed[0].text.ends_with(' '));
        assert!(!processed[1].text.starts_with(' '));
    }

    #[test]
    fn processing_line_buffer_preserves_tags() {
        let text = "Mr. and Mrs. Doubtfire";
        let tags = vec!["tag 1".to_string(), "tag 2".to_string()];

        let line = LineTextBuilder::from_string(text).with_tags(&tags).build();

        let buffer = vec![line];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert_eq!(processed[0].tags, tags);
    }

    #[test]
    fn only_single_whitespaces_are_left_between_words_after_processing() {
        let text = "A line    with   just    enough   whitespace";
        let line = LineTextBuilder::from_string(text).build();

        let buffer = vec![line];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert_eq!(&processed[0].text, "A line with just enough whitespace\n");
    }
}
