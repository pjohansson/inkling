# Basic elements

These are the basic features needed to write a story with `Ink` and `inkling`.

## Text

Plain text is the most basic element of a story. It is written in the story text as regular lines.

```rust
# let content = r"
#
I opened my notebook to a blank page, pen in hand.
#
# ";
```

Text is separated into paragraphs by being on different lines.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Prompt};
# let content = r"
#
My hand moved towards the canvas.
The cold draft made a shudder run through my body.
A dark blot spread from where my pen was resting.
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.resume(&mut buffer).unwrap();
# assert_eq!(buffer[0].text, "My hand moved towards the canvas.\n");
# assert_eq!(buffer[1].text, "The cold draft made a shudder run through my body.\n");
# assert_eq!(buffer[2].text, "A dark blot spread from where my pen was resting.\n");
```

Those three lines will be returned from `inkling` as separate lines, each ending with
a newline character.

### Glue

If you want to remove the newline character from in between lines, you can use the `<>` 
marker which signifies *glue*. This:

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content = r"
#
This line will <>
be glued to this, without creating a new paragraph.
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.resume(&mut buffer).unwrap();
# assert!(!buffer[0].text.ends_with("\n"));
```

Becomes:

```plain
This line will be glued to this, without creating a new paragraph.
```

as will this, since glue can be put at either end:

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content = r"
#
This line will 
<> be glued to this, without creating a new paragraph.
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.resume(&mut buffer).unwrap();
# assert!(!buffer[0].text.ends_with("\n"));
```

For these examples glue doesn't do much, but it will be more useful once we introduce 
story structure features. Keep it in mind until then.

### Comments

The text file can contain comments, which will be ignored by `inkling` as it parses the story.
To write a comment, preceed the line with `//`.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Prompt};
# let content = r"
#
The cold could not be ignored.
// Unlike this line, which will be 
As will the end of this. // removed comment at end of line
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.resume(&mut buffer).unwrap();
# assert_eq!(buffer[0].text, "The cold could not be ignored.\n");
# assert_eq!(buffer[1].text, "As will the end of this.\n");
```

Note that multiline comments with `/*` and `*/` are ***not*** currently supported.

## Branching story paths

To mark a choice in a branching story, use the `*` marker.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Prompt};
# let content = r"
#
*   Choice 1
*   Choice 2 
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# match story.resume(&mut buffer).unwrap() {
#   Prompt::Choice(choices) => {
#       assert_eq!(choices[0].text, "Choice 1");
#       assert_eq!(choices[1].text, "Choice 2");
#   } 
#   _ => unreachable!()
# }
```

(The `+` marker can also be used, which results in a different behavior
if the tree is visited again. [More on this later.](structure.md#once-only-and-sticky-choices))

When `inkling` encounters one or more lines beginning with this marker, the options will 
be collected and returned to the user to make a choice.

After making a choice, the story proceeds from lines below the choice. So this story:

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Prompt};
# let content = r#"
#
A noise rang from the door.
*   "Hello?" I shouted.
    "Who's there?"
*   I rose from the desk and walked over.
#
# "#;
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# match story.resume(&mut buffer).unwrap() {
#   Prompt::Choice(choices) => {
#       assert_eq!(choices[0].text, r#""Hello?" I shouted."#);
#       assert_eq!(choices[1].text, r"I rose from the desk and walked over.");
#   } 
#   _ => unreachable!()
# }
# story.make_choice(0).unwrap();
# story.resume(&mut buffer).unwrap();
# assert!(buffer[0].text.starts_with(r#"A noise rang from the door."#));
# assert!(buffer[1].text.starts_with(r#""Hello?" I shouted."#));
# assert!(buffer[2].text.starts_with(r#""Who's there?""#));
```

results in this "game" for the user (in this case picking the first option):

```plain
A noise rang from the door.
 1: "Hello?" I shouted.
 2: I rose from the desk and walked over.

> 1
"Hello?" I shouted.
"Who's there?"
```

### Removing choice text from output

As the previous example show, by default, the choice text will be added to the 
text presented to the user. Text encased in square brackets `[]` will, however, 
be ignored. Building on the previous example: 

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Prompt};
# let content = r#"
#
*   ["Hello?" I shouted.]
    "Who's there?"
#
# "#;
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# match story.resume(&mut buffer).unwrap() {
#   Prompt::Choice(choices) => {
#       assert_eq!(choices[0].text, r#""Hello?" I shouted."#);
#   } 
#   _ => unreachable!()
# }
# story.make_choice(0).unwrap();
# story.resume(&mut buffer).unwrap();
# assert!(buffer[0].text.starts_with(r#""Who's there?""#));
```

```plain
 1: "Hello?" I shouted.

> 1
"Who's there?"
```

Note how the choice text is not printed below the selection.

### Advanced: mixing choice and presented text

The square brackets also acts as a divider between choice and presented text. Any 
text *after* the square brackets will not appear in the choice text. Text *before*
the brackets will appear in both choice and output text. This makes it easy to 
build a simple choice text into a more presentable sentence for the story:

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Prompt};
# let content = r#"
#
*   "Hello[?"]," I shouted. "Who's there?"
#
# "#;
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# match story.resume(&mut buffer).unwrap() {
#   Prompt::Choice(choices) => {
#       assert_eq!(choices[0].text, r#""Hello?""#);
#   } 
#   _ => unreachable!()
# }
# story.make_choice(0).unwrap();
# story.resume(&mut buffer).unwrap();
# assert!(buffer[0].text.starts_with(r#""Hello," I shouted. "Who's there?""#));
```

```plain
 1: "Hello?"

> 1
"Hello," I shouted. "Who's there?"
```

### Nested dialogue options

Dialogue branches can be nested, more or less infinitely. Just add extra `*` markers
to specify the depth.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Prompt};
# let content = r"
#
*   Choice 1
    * *     Choice 1.1
    * *     Choice 1.2
            * * *   Choice 1.2.1
*   Choice 2
    * *     Choice 2.1
    * *     Choice 2.2
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.resume(&mut buffer).unwrap();
# story.make_choice(0).unwrap();
# match story.resume(&mut buffer).unwrap() {
#   Prompt::Choice(choices) => {
#       assert_eq!(&choices[0].text, "Choice 1.1");
#       assert_eq!(&choices[1].text, "Choice 1.2");
#       story.make_choice(1).unwrap();
#       match story.resume(&mut buffer).unwrap() {
#           Prompt::Choice(choices) => {
#               assert_eq!(&choices[0].text, "Choice 1.2.1");
#           }
#           _ => unreachable!()
#       }
#   }
#   _ => unreachable!()
# }
```

Any extra whitespace is just for readability. The previous example produces the exact 
same tree as this, much less readable, example:

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content_nowhitespace = r"
#
*Choice 1
**Choice 1.1
**Choice 1.2
***Choice 1.2.1
*Choice 2
**Choice 2.1
**Choice 2.2
#
# ";
# let content_whitespace = r"
#
# *   Choice 1
#     * *     Choice 1.1
#     * *     Choice 1.2
#             * * *   Choice 1.2.1
# *   Choice 2
#     * *     Choice 2.1
#     * *     Choice 2.2
#
# ";
#
# let story_nowhitespace = read_story_from_string(content_nowhitespace).unwrap();
# let story_whitespace = read_story_from_string(content_whitespace).unwrap();
# assert_eq!(format!("{:?}", story_nowhitespace), format!("{:?}", story_whitespace));
```

