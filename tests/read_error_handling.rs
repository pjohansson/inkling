use inkling::error::parse::print_read_error;
use inkling::*;

#[test]
fn all_line_parsing_errors_are_reported_when_printed() {
    let content = "

VAR = 0 // no variable name
VAR variable = 10 // good variable to assert number of errors
VAR bad_variable 0 // no assignment operator

-> root

== root
Let's add a couple more errors.

*+  Choices cannot have both stick and non-sticky markers
*   Nor can they have[] unmatched braces ]

";

    let error = read_story_from_string(content).unwrap_err();

    let error_string = print_read_error(&error).unwrap();
    let error_lines = error_string.lines().collect::<Vec<_>>();

    assert_eq!(error_lines.len(), 4);
}
