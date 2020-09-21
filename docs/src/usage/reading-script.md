# Reading a script

Let us now introduce how to move through a script with `inkling`. We will reuse the script from the [short introductory example](../introduction/example.md):

```rust
let content = r#"
A single candle flickered by my side.
Pen in hand I procured a blank letter.

*   "Dear Guillaume"
    Sparing the more unfavorable details from him, I requested his aid.

*   "To the Fiendish Impostor"
"#;
```

## Reading text content

To parse your script into a story, `inkling` provides the [`read_story_from_string`][read_story_from_string]
function. It takes the script (as a string) and from that returns a [`Story`][Story] 
object in a `Result`.

Again, we parse it into a story:

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Story};
# let content = r#"
# A single candle flickered by my side.
# Pen in hand I procured a blank letter.
# 
# *   "Dear Guillaume"
#     Sparing the more unfavorable details from him, I requested his aid.
# 
# *   "To the Fiendish Impostor"
# "#;
let mut story: Story = read_story_from_string(&content).unwrap();

// Mark the story as ready by calling `start`
story.start();
```

### Aside: The `Story` object

[`Story`][Story] contains the entire parsed script in a form that is ready to be used. 
It implements a wealth of methods which can be used to go through it or modify its 
state by setting variables and changing locations. Look through the documentation
for the object for more information about these methods.

## Starting the story

To start the story we must supply a [buffer][LineBuffer] which it can add text lines into.
The story will then proceed until a set of choices is encountered, which the user has 
to select from.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Story};
# let content = r#"
# A single candle flickered by my side.
# Pen in hand I procured a blank letter.
# 
# *   "Dear Guillaume"
#     Sparing the more unfavorable details from him, I requested his aid.
# 
# *   "To the Fiendish Impostor"
# "#;
# let mut story: Story = read_story_from_string(&content).unwrap();
# story.start();
use inkling::Line;

// Buffer which the text lines will be added to
let mut line_buffer: Vec<Line> = Vec::new();

// Begin the story by calling `resume`
let result = story.resume(&mut line_buffer).unwrap();

// The two first lines have now been added to the buffer
assert_eq!(line_buffer[0].text, "A single candle flickered by my side.\n");
assert_eq!(line_buffer[1].text, "Pen in hand I procured a blank letter.\n");
assert_eq!(line_buffer.len(), 2);
```

Note that the lines end with newline characters to denote that they are separate 
paragraphs. Oh, and the text lines are of type [`Line`][Line], which contains
two fields: `text` (seen above) and `tags` for [tags](../features/metadata.md#line-tags) 
which are associated with the line. 

## Encountering choices

The story returned once it encountered the choice of whom to pen a letter to.
This set of choices is present in the returned object, which is an `enum`
of type [`Prompt`][Prompt]. We can access the choices (which are of type 
[`Choice`][Choice]) through this object.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Story, Prompt};
# let content = r#"
# A single candle flickered by my side.
# Pen in hand I procured a blank letter.
# 
# *   "Dear Guillaume"
#     Sparing the more unfavorable details from him, I requested his aid.
# 
# *   "To the Fiendish Impostor"
# "#;
# let mut story: Story = read_story_from_string(&content).unwrap();
# story.start();
# let mut line_buffer = Vec::new();
# let result = story.resume(&mut line_buffer).unwrap();
match result {
    Prompt::Choice(choices) => {
        assert_eq!(choices[0].text, r#""Dear Guillaume""#);
        assert_eq!(choices[1].text, r#""To the Fiendish Impostor""#);
        assert_eq!(choices.len(), 2);
    }
    Done => (),
}
```

To continue the story we use the `make_choice` method with an index corresponding
to `Choice` made.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Story, Prompt};
# let content = r#"
# A single candle flickered by my side.
# Pen in hand I procured a blank letter.
# 
# *   "Dear Guillaume"
#     Sparing the more unfavorable details from him, I requested his aid.
# 
# *   "To the Fiendish Impostor"
# "#;
# let mut story: Story = read_story_from_string(&content).unwrap();
# story.start();
# let mut line_buffer = Vec::new();
# let result = story.resume(&mut line_buffer).unwrap();
story.make_choice(0).unwrap();

let result = story.resume(&mut line_buffer).unwrap();

assert!(line_buffer[2].text.starts_with(r#""Dear Guillaume""#));
assert!(line_buffer[3].text.starts_with("Sparing the more unfavorable details"));
```

Note that `inkling` does not clear the supplied buffer when resuming the story. 
That task is trusted to you, if you need to, by running `line_buffer.clear()`.

## Summary

*   Parse the story using [`read_story_from_string`][read_story_from_string]
*   Move through it with [`resume`][resume], which adds text to a buffer
*   Use [`make_choice`][make_choice] to select a choice when hitting a branch, 
    then [`resume`][resume] again
*   Key objects: [`Story`][Story], [`Line`][Line], [`Choice`][Choice]
    and [`Prompt`][Prompt]

[Choice]: https://docs.rs/inkling/latest/inkling/struct.Choice.html
[Line]: https://docs.rs/inkling/latest/inkling/struct.Line.html
[LineBuffer]: https://docs.rs/inkling/latest/inkling/type.LineBuffer.html
[Story]: https://docs.rs/inkling/latest/inkling/struct.Story.html
[Prompt]: https://docs.rs/inkling/latest/inkling/enum.Prompt.html
[read_story_from_string]: https://docs.rs/inkling/latest/inkling/fn.read_story_from_string.html
[make_choice]: https://docs.rs/inkling/latest/inkling/struct.Story.html#method.make_choice
[resume]: https://docs.rs/inkling/latest/inkling/struct.Story.html#method.resume