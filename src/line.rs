use crate::{
    consts::GLUE_MARKER,
    follow::{Follow, Next},
};

#[derive(Debug)]
pub struct Line {
    text: String,
    next: Next,
}

impl Line {
    pub fn from_string(line: &str) -> Line {
        let parts = line.split("->").collect::<Vec<_>>();

        let line_has_divert = parts.len() > 1;

        let text = with_line_glue_or_newline(parts[0], line_has_divert);

        let next = if line_has_divert {
            let name = parts[1].trim().to_string();
            Next::Divert(name)
        } else {
            Next::Done
        };

        Line {
            text,
            next,
        }
    }
}

/// If the line has glue, remove the glue marker, retain ending whitespace and do not
/// add a newline character. If it does not have glue, remove all whitespace and add
/// a newline character.
fn with_line_glue_or_newline(line: &str, always_add_glue: bool) -> String {
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

impl Follow for Line {
    fn follow(&self, buffer: &mut String) -> Next {
        buffer.push_str(&self.text);

        self.next.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_simple_line() {
        let line = "Hello, world!";
        let mut buffer = String::new();

        assert_eq!(Line::from_string(line).follow(&mut buffer), Next::Done);
        assert_eq!(buffer.trim(), line);
    }

    #[test]
    fn read_line_adds_newline_character_at_end() {
        let mut buffer = String::new();
        Line::from_string("Hello, world!").follow(&mut buffer);

        assert!(buffer.ends_with('\n'));
    }

    #[test]
    fn read_line_trims_whitespace() {
        let line = "   Hello, world!   ";
        let trimmed = format!("{}\n", line.trim());

        let mut buffer = String::new();
        Line::from_string(line).follow(&mut buffer);

        assert_eq!(buffer, trimmed);
    }

    #[test]
    fn read_line_with_glue_retains_end_whitespace_but_not_newline() {
        let line = "Hello, world!";
        let whitespace = "    ";

        let padded_line = format!("   {}{}{}", line, whitespace, GLUE_MARKER);
        let trimmed = format!("{}{}", line.trim_start(), whitespace);

        let mut buffer = String::new();
        Line::from_string(&padded_line).follow(&mut buffer);

        assert_eq!(buffer, trimmed);
    }

    #[test]
    fn divert_line_returns_knot_name() {
        let name = "knot_name";
        let line = format!("-> {}", name);
        let mut buffer = String::new();

        assert_eq!(Line::from_string(&line).follow(&mut buffer), Next::Divert(name.to_string()));
        assert_eq!(buffer, "");
    }

    #[test]
    fn embedded_divert_returns_knot_name() {
        let head = "Hello, world!";
        let name = "knot_name";
        let line = format!("{}->{}", head, name);

        let mut buffer = String::new();

        assert_eq!(Line::from_string(&line).follow(&mut buffer), Next::Divert(name.to_string()));
        assert_eq!(buffer, head);
    }

    #[test]
    fn diverts_in_lines_acts_as_glue() {
        let head = "Hello, world! ";
        let name = "knot_name";
        let line = format!("{}->{}", head, name);

        let mut buffer = String::new();

        assert_eq!(Line::from_string(&line).follow(&mut buffer), Next::Divert(name.to_string()));
        assert_eq!(buffer, head);
    }
}
