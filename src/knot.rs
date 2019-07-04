use crate::{
    follow::{Follow, Next},
    line::Line
};

#[derive(Debug)]
pub struct Knot {
    lines: Vec<Line>,
}

impl Knot {
    fn from_string<T: Into<String>>(text: T) -> Knot {
        Knot {
            lines: text.into().lines().map(|line| Line::from_string(line)).collect()
        }
    }
}

impl Follow for Knot {
    fn follow(&self, buffer: &mut String) -> Next {
        self.lines.iter().for_each(|line| {
            line.follow(buffer);
        });

        Next::Done
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

        assert_eq!(knot.follow(&mut buffer), Next::Done);
        assert_eq!(buffer, text);
    }
}
