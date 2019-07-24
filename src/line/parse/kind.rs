use crate::line::*;

#[derive(Clone, Debug, PartialEq)]
pub enum ParsedLineKind {
    Choice { level: u32, choice_data: FullChoice },
    Gather { level: u32, line: FullLine },
    Line(FullLine),
}

pub fn parse_line_kind(content: &str) -> Result<ParsedLineKind, LineParsingError> {
    if let Some(choice) = parse_choice(content)? {
        Ok(choice)
    } else if let Some(gather) = parse_gather(content)? {
        Ok(gather)
    } else {
        let line = parse_line(content)?;

        Ok(ParsedLineKind::Line(line))
    }
}
