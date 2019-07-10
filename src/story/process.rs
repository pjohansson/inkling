//! Process lines to their final form, which will be displayed to the user.

use crate::{
    follow::LineDataBuffer,
    line::{Choice, LineData},
};

use super::story::{Line, LineBuffer};

/// Process full `LineData` lines to their final state: remove empty lines, add newlines
/// unless glue is present.
pub fn process_buffer(into_buffer: &mut LineBuffer, from_buffer: LineDataBuffer) {
    let mut iter = from_buffer
        .into_iter()
        .filter(|line| !line.text.trim().is_empty())
        .peekable();

    while let Some(mut line) = iter.next() {
        add_line_ending(&mut line, iter.peek());

        into_buffer.push(Line {
            text: line.text,
            tags: line.tags,
        });
    }
}

/// Prepared the choices with the text that will be displayed to the user.
/// Preserve line tags in case processing is desired. Choices are filtered
/// based on a set condition (currently: visited or not, unless sticky).
pub fn prepare_choices_for_user(choices: &[Choice]) -> Vec<Line> {
    choices
        .iter()
        .filter(|choice| choice.is_sticky || choice.num_visited == 0)
        .map(|choice| Line {
            text: choice.displayed.text.clone(),
            tags: choice.displayed.tags.clone(),
        })
        .collect()
}

/// Add a newline character if the line is not glued to the next. Retain only a single
/// whitespace between the lines if they are glued.
fn add_line_ending(line: &mut LineData, next_line: Option<&LineData>) {
    let glue = next_line
        .map(|next_line| line.glue_end || next_line.glue_start)
        .unwrap_or(false);

    let whitespace = glue && {
        next_line
            .map(|next_line| line.text.ends_with(' ') || next_line.text.starts_with(' '))
            .unwrap_or(false)
    };

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

    use crate::line::{
        tests::{ChoiceBuilder, LineBuilder},
        LineKind,
    };

    #[test]
    fn processing_line_buffer_removes_empty_lines() {
        let text = "Mr. and Mrs. Doubtfire";
        let buffer = vec![
            LineBuilder::new(text).build(),
            LineBuilder::new("").build(),
            LineBuilder::new(text).build(),
        ];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert_eq!(processed.len(), 2);
        assert_eq!(processed[0].text.trim(), text);
        assert_eq!(processed[1].text.trim(), text);
    }

    #[test]
    fn processing_line_buffer_adds_newlines_if_no_glue() {
        let text = "Mr. and Mrs. Doubtfire";
        let buffer = vec![
            LineBuilder::new(text).build(),
            LineBuilder::new(text).build(),
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
            LineBuilder::new(text).with_glue_end().build(),
            LineBuilder::new(text).build(),
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
            LineBuilder::new(text).build(),
            LineBuilder::new(text).with_glue_start().build(),
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
            LineBuilder::new(text).build(),
            LineBuilder::new("").build(),
            LineBuilder::new(text).with_glue_start().build(),
        ];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert!(!processed[0].text.ends_with('\n'));
        assert!(processed[1].text.ends_with('\n'));
    }

    #[test]
    fn processing_line_buffer_sets_newline_on_last_line_regardless_of_glue() {
        let text = "Mr. and Mrs. Doubtfire";
        let buffer = vec![LineBuilder::new(text).with_glue_end().build()];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert!(processed[0].text.ends_with('\n'));
    }

    #[test]
    fn processing_line_buffer_keeps_single_whitespace_between_lines_with_glue() {
        let buffer = vec![
            LineBuilder::new("Ends with whitespace before glue, ")
                .with_glue_end()
                .build(),
            LineBuilder::new(" starts with whitespace after glue")
                .with_glue_start()
                .build(),
        ];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert!(processed[0].text.ends_with(' '));
        assert!(!processed[1].text.starts_with(' '));
    }

    #[test]
    fn processing_line_buffer_preserves_tags() {
        let text = "Mr. and Mrs. Doubtfire";
        let tags = vec!["tag 1".to_string(), "tag 2".to_string()];

        let buffer = vec![LineBuilder::new(text).with_tags(tags.clone()).build()];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert_eq!(processed[0].tags, tags);
    }

    #[test]
    fn preparing_choices_returns_selection_text_lines() {
        let displayed1 = LineBuilder::new("Choice 1").build();
        let displayed2 = LineBuilder::new("Choice 2").build();

        let choices = vec![
            ChoiceBuilder::empty()
                .with_displayed(displayed1.clone())
                .build(),
            ChoiceBuilder::empty()
                .with_displayed(displayed2.clone())
                .build(),
        ];

        let displayed_choices = prepare_choices_for_user(&choices);

        assert_eq!(displayed_choices.len(), 2);
        assert_eq!(displayed_choices[0].text, displayed1.text);
        assert_eq!(displayed_choices[1].text, displayed2.text);
    }

    #[test]
    fn preparing_choices_preserves_tags() {
        let tags = vec!["tag 1".to_string(), "tag 2".to_string()];
        let line = LineBuilder::new("Choice with tags")
            .with_tags(tags.clone())
            .build();

        let choices = vec![ChoiceBuilder::empty().with_displayed(line).build()];

        let displayed_choices = prepare_choices_for_user(&choices);

        assert_eq!(displayed_choices[0].tags, tags);
    }

    #[test]
    fn preparing_choices_filters_choices_which_have_been_visited_for_non_sticky_lines() {
        let kept_line = LineBuilder::new("Kept").build();
        let removed_line = LineBuilder::new("Removed").build();

        let choices = vec![
            ChoiceBuilder::empty()
                .with_displayed(kept_line.clone())
                .build(),
            ChoiceBuilder::empty()
                .with_displayed(removed_line.clone())
                .with_num_visited(1)
                .build(),
            ChoiceBuilder::empty()
                .with_displayed(kept_line.clone())
                .build(),
        ];

        let displayed_choices = prepare_choices_for_user(&choices);

        assert_eq!(displayed_choices.len(), 2);
        assert_eq!(&displayed_choices[0].text, "Kept");
        assert_eq!(&displayed_choices[1].text, "Kept");
    }

    #[test]
    fn preparing_choices_does_not_filter_visited_sticky_lines() {
        let kept_line = LineBuilder::new("Kept").build();
        let removed_line = LineBuilder::new("Removed").build();

        let choices = vec![
            ChoiceBuilder::empty()
                .with_displayed(kept_line.clone())
                .build(),
            ChoiceBuilder::empty()
                .with_displayed(removed_line.clone())
                .with_num_visited(1)
                .build(),
            ChoiceBuilder::empty()
                .with_displayed(kept_line.clone())
                .with_num_visited(1)
                .is_sticky()
                .build(),
        ];

        let displayed_choices = prepare_choices_for_user(&choices);

        assert_eq!(displayed_choices.len(), 2);
        assert_eq!(&displayed_choices[0].text, "Kept");
        assert_eq!(&displayed_choices[1].text, "Kept");
    }
}
