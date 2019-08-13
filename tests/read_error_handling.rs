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

#[test]
fn all_address_validation_errors_are_returned() {
    let content = "\

VAR variable = 10

== root
= stitch

Here we want to use variable and knot addresses that will be invalid.
In a condition: {unknown: Invalid!}
In an expression: {unknown}
In a nested calculation: {2 + (3 * unknown)}
As a divert: -> unknown

*   Divert in a choice: -> one
*   Variable in a choice: {two}
*   {three} As a choice condition

As a bad stitch label: -> other_knot.stitch

== other_knot
Addressing stitch in other knot: -> stitch

";

    let error = read_story_from_string(content).unwrap_err();

    let error_string = print_read_error(&error).unwrap();
    let error_lines = error_string.lines().collect::<Vec<_>>();

    assert_eq!(error_lines.len(), 9);
}

#[test]
fn name_space_collision_errors_are_yielded() {
    let content = "\

VAR variable = 10
VAR knot = 2

== knot
= variable
Line one.

";

    let error = read_story_from_string(content).unwrap_err();

    let error_string = print_read_error(&error).unwrap();
    let error_lines = error_string.lines().collect::<Vec<_>>();

    assert_eq!(error_lines.len(), 2);
}

#[test]
fn invalid_expression_and_condition_errors_are_yielded() {
    let content = "\

VAR int = 2

{true + int} is not an allowed operation. {\"str\" + int > 0: Neither is this.}
{\"string\" == true: True | False} is also an invalid comparison between string and boolean.

*   {\"string\" == true} Invalid comparisons are checked in choice conditions.
    And in lines belonging to a branch: {int == true: True}

Of course text after branching points is verified: {2 + \"string\"}

As are items inside alternative sequences: {{1 + true} | {2 + true} | {3 + true}}

Bad nested expressions are validated: {1 + (2 + (3 + \"string\"))}.

== knot
And in all knots! {2 + true}.

";

    let error = read_story_from_string(content).unwrap_err();

    let error_string = print_read_error(&error).unwrap();
    let error_lines = error_string.lines().collect::<Vec<_>>();

    assert_eq!(error_lines.len(), 11);
}
