use std::{
    collections::HashMap,
    str::FromStr,
};

use crate::{
    line::{Line, LineKind},
};

#[derive(Debug)]
pub struct Knot {
    lines: Vec<Line>,
}

pub type LineBuffer = Vec<Line>;

#[derive(Clone, Debug, PartialEq)]
/// What action that is prompted by following a story.
pub enum Next {
    /// Finished with the story.
    Done,
    /// Divert to a new knot with the given name.
    Divert(String),
    // /// Choice for the user.
    // Choice(MultiChoice),
}

impl Knot {
    /// Follow a story while reading every line into a buffer.
    fn follow(&self, buffer: &mut LineBuffer) -> Next {
        for line in &self.lines {
            buffer.push(line.clone());

            match &line.kind {
                LineKind::Divert(name) => return Next::Divert(name.clone()),
                _ => (),
            }
        }

        Next::Done
    }

    /// Follow a story while reading every line into a pure text buffer,
    /// discarding other data.
    fn follow_into_string(&self, buffer: &mut String) -> Next {
        let mut line_buffer = Vec::new();
        let result = self.follow(&mut line_buffer);

        for line in line_buffer {
            buffer.push_str(&line.text);

            if !line.glue_end {
                buffer.push('\n');
            }
        }

        result
    }
}

impl FromStr for Knot {
    type Err = ();

    fn from_str(content: &str) -> Result<Self, Self::Err> {
        let lines = parse_lines(content)?;

        Ok(Knot { 
            lines,
        })
    }
}

fn parse_lines(s: &str) -> Result<Vec<Line>, ()> {
    s.lines().map(|line| Line::from_str(line)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knot_from_plain_text_lines_fully_replicates_them() {
        let text = "\
Hello, world!
Hello?
Hello, are you there?
";

        let knot = Knot::from_str(text).unwrap();

        let mut buffer = String::new();

        assert_eq!(knot.follow_into_string(&mut buffer), Next::Done);
        assert_eq!(buffer, text);
    }

    #[test]
    fn knot_with_divert_shortcuts_at_it() {
        let name = "fool".to_string();

        let pre = "Mrs. Bennet was making a fool of herself.";
        let after = "After Mrs. Bennet left, Elizabet went upstairs to look after Jane.";

        let text = format!(
            "\
{}
-> {}
{}
",
            pre, name, after
        );

        let knot = Knot::from_str(&text).unwrap();

        let mut buffer = String::new();

        assert_eq!(knot.follow_into_string(&mut buffer), Next::Divert(name));
        assert_eq!(buffer.trim_end(), pre);
    }
}
