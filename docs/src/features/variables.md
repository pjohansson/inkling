# Variables

Throughout a story it can be useful to introduce *variables*, which can be declared
and used in the story text. This makes it easy to keep a story consistent and track
a story state.

## Declaring variables

Global variables can be declared in the script using the `VAR` keyword. They
must be declared in the [preamble](structure.md#preamble): before the first knot.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Variable};
# let content = r#"
#
VAR a_float = 1.0
VAR a_int = 2
VAR a_bool = false
VAR a_string = "A String"
VAR a_destination = "-> stairwell"
#
# "#;
# let story = read_story_from_string(content).unwrap();
# assert_eq!(story.get_variable("a_float").unwrap(), Variable::Float(1.0));
# assert_eq!(story.get_variable("a_int").unwrap(), Variable::Int(2));
# assert_eq!(story.get_variable("a_bool").unwrap(), Variable::Bool(false));
# assert_eq!(story.get_variable("a_string").unwrap(), Variable::String("A String".to_string()));
# assert!(story.get_variable("a_destination").is_some());
```

As shown in this example, the variable type is automatically assigned from
the given value. Once assigned, a variable's type cannot be changed.

## Using variables in text

Variables can be inserted into text by enclosing them in curly braces.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Variable};
# let content = r#"
#
VAR time = 11
VAR moon = "gloomy"

The time was {time}. A {moon} moon illuminated the room.
#
# "#;
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.resume(&mut buffer).unwrap();
# assert_eq!(buffer[0].text, "The time was 11. A gloomy moon illuminated the room.\n");
```

## Variable assignment

**It is not currently possible to assign variables in the script. Use `Story::set_variable`.**

## Constant variables

Constant variables, whose values cannot be changed, are declared using the `CONST` keyword.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Variable};
# let content = r#"
#
CONST name = "d'Artagnan" // Constant variable, cannot be modified
VAR rank = "Capitaine"    // Non-constant variable, can be changed
#
# "#;
# let mut story = read_story_from_string(content).unwrap();
# assert_eq!(story.get_variable("name").unwrap(), Variable::from("d'Artagnan"));
# assert!(story.set_variable("name", "Aramis").is_err());
```

## Variable mathematics

## Variable comparisons