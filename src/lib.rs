use std::collections::HashMap;

#[derive(Debug)]
struct Story {
    knots: HashMap<String, Knot>,
}

#[derive(Debug)]
struct Knot {
    lines: Vec<Line>,
}

#[derive(Debug)]
struct Line {
    text: String,
    next: Follow,
}

#[derive(Debug)]
enum End {
    Next,
}

#[derive(Debug)]
enum Follow {
    Line(Box<Line>),
    Divert(String),
}

impl Line {
    fn from_string<T: Into<String>>(line: T) -> Line {
        unimplemented!();
    }

    fn follow(&self) -> Follow {
        unimplemented!();
    }
}

impl Knot {
    fn from_string<T: Into<String>>(text: T) -> Knot {
        unimplemented!();
    }

    fn follow(&self) -> String {
        unimplemented!();
    }
}

#[derive(Debug)]
struct Divert(String);

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn read_lines_from

    #[test]
    fn read_simple_knot_from_text() {
        let text = "\
Hello, world!
Hello?
Hello, are you there?\
";

        let knot = Knot::from_string(text);
        assert_eq!(knot.follow(), text);
    }
}
