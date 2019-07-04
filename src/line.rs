use crate::{
    consts::{GLUE_MARKER, DIVERT_MARKER, TAG_MARKER},
    follow::{Follow, LineBuffer, Next},
};

#[derive(Debug)]
pub struct Line {
    pub text: String,
    pub next: Next,
    pub tags: Vec<String>,
}

impl Follow for Line {
    fn follow(&self, buffer: &mut LineBuffer) -> Next {
        buffer.push(self.into());

        self.next.clone()
    }
}

impl Line {
    pub fn from_string(line: &str) -> Line {
        let mut content = line.to_string();

        let tags = parse_tags(&mut content);
        let divert = parse_divert(&mut content);

        // Diverts always act as glue
        let text = add_line_glue_or_newline(&content, divert.is_some());

        let next = if let Some(name) = divert {
            Next::Divert(name)
        } else {
            Next::Line
        };

        Line { text, next, tags }
    }
}

/// Split diverts off the given line and return it separately if found.
fn parse_divert(line: &mut String) -> Option<String> {
    match line.find(DIVERT_MARKER) {
        Some(i) => {
            let part = line.split_off(i);

            part.trim_start_matches(DIVERT_MARKER)
                .split(DIVERT_MARKER)
                .map(|knot_name| knot_name.trim().to_string())
                .next()
        },
        None => None,
    }
}

/// Split any found tags off the given line and return them separately.
fn parse_tags(line: &mut String) -> Vec<String> {
    match line.find(TAG_MARKER) {
        Some(i) => {
            let part = line.split_off(i);

            part.trim_matches(TAG_MARKER)
                .split(TAG_MARKER)
                .map(|tag| tag.to_string())
                .collect::<Vec<_>>()
        }
        None => Vec::new(),
    }
}

/// If the line has glue, remove the glue marker, retain ending whitespace and do not
/// add a newline character. If it does not have glue, remove all whitespace and add
/// a newline character.
fn add_line_glue_or_newline(line: &str, always_add_glue: bool) -> String {
    let mut text = line.trim_start().to_string();
    let mut add_glue = always_add_glue;

    if let Some(i) = text.rfind(GLUE_MARKER) {
        text.truncate(i);
        add_glue = true;
    }

    if !add_glue {
        text = text.trim_end().to_string();
        text.push('\n');
    }

    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_simple_line() {
        let line = "Hello, world!";
        let mut buffer = String::new();

        assert_eq!(
            Line::from_string(line).follow_into_string(&mut buffer),
            Next::Line
        );
        assert_eq!(buffer.trim(), line);
    }

    #[test]
    fn read_line_adds_newline_character_at_end() {
        let mut buffer = String::new();
        Line::from_string("Hello, world!").follow_into_string(&mut buffer);

        assert!(buffer.ends_with('\n'));
    }

    #[test]
    fn read_line_trims_whitespace() {
        let line = "   Hello, world!   ";
        let trimmed = format!("{}\n", line.trim());

        let mut buffer = String::new();
        Line::from_string(line).follow_into_string(&mut buffer);

        assert_eq!(buffer, trimmed);
    }

    #[test]
    fn read_line_with_glue_retains_end_whitespace_but_not_newline() {
        let line = "Hello, world!";
        let whitespace = "    ";

        let padded_line = format!("   {}{}{}", line, whitespace, GLUE_MARKER);
        let trimmed = format!("{}{}", line.trim_start(), whitespace);

        let mut buffer = String::new();
        Line::from_string(&padded_line).follow_into_string(&mut buffer);

        assert_eq!(buffer, trimmed);
    }

    #[test]
    fn divert_line_returns_knot_name() {
        let name = "knot_name";
        let line = format!("-> {}", name);
        let mut buffer = String::new();

        assert_eq!(
            Line::from_string(&line).follow_into_string(&mut buffer),
            Next::Divert(name.to_string())
        );
        assert_eq!(buffer, "");
    }

    #[test]
    fn embedded_divert_returns_knot_name() {
        let head = "Hello, world!";
        let name = "knot_name";
        let line = format!("{}->{}", head, name);

        let mut buffer = String::new();

        assert_eq!(
            Line::from_string(&line).follow_into_string(&mut buffer),
            Next::Divert(name.to_string())
        );
        assert_eq!(buffer, head);
    }

    #[test]
    fn diverts_in_lines_acts_as_glue() {
        let head = "Hello, world! ";
        let name = "knot_name";
        let line = format!("{}->{}", head, name);

        let mut buffer = String::new();

        assert_eq!(
            Line::from_string(&line).follow_into_string(&mut buffer),
            Next::Divert(name.to_string())
        );
        assert_eq!(buffer, head);
    }

    #[test]
    fn tags_are_not_added_if_none_are_given() {
        let head = "Hello, world! ";
        let name = "knot_name";
        let text = format!("{}->{}", head, name);

        let mut buffer = LineBuffer::new();

        Line::from_string(&text).follow(&mut buffer);
        assert!(buffer[0].tags.is_empty());
    }

    #[test]
    fn multiple_tags_can_be_specified() {
        let head = "Hello, world!";

        let tag1 = "blue colour".to_string();
        let tag2 = "transparent".to_string();
        let tag3 = "italic text".to_string();

        let text = format!(
            "{head}{marker}{}{marker}{}{marker}{}",
            tag1,
            tag2,
            tag3,
            head = head,
            marker = TAG_MARKER
        );

        let mut buffer = LineBuffer::new();
        Line::from_string(&text).follow(&mut buffer);

        let tags = &buffer[0].tags;

        assert_eq!(tags.len(), 3);
        assert_eq!(tags[0], tag1);
        assert_eq!(tags[1], tag2);
        assert_eq!(tags[2], tag3);
    }
}
