use crate::{
    follow::{Follow, LineBuffer, Next},
    line::Line,
};

#[derive(Debug)]
pub struct Knot {
    lines: Vec<Line>,
}

impl Knot {
    pub fn from_string<T: Into<String>>(text: T) -> Knot {
        Knot {
            lines: text
                .into()
                .lines()
                .map(|line| Line::from_string(line))
                .collect(),
        }
    }
}

impl Follow for Knot {
    fn follow(&self, buffer: &mut LineBuffer) -> Next {
        for line in &self.lines {
            match line.follow(buffer) {
                Next::Divert(name) => {
                    return Next::Divert(name);
                }
                _ => (),
            }
        }

        Next::Line
    }
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

        let knot = Knot::from_string(text);

        let mut buffer = String::new();

        assert_eq!(knot.follow_into_string(&mut buffer), Next::Line);
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

        let knot = Knot::from_string(text);

        let mut buffer = String::new();

        assert_eq!(knot.follow_into_string(&mut buffer), Next::Divert(name));
        assert_eq!(buffer.trim(), pre);
    }
}
