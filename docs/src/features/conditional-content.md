# Conditional content

`Ink` provides many methods for varying the text content of a line or the choices 
presented to a user.

## Choice conditions

The easiest way to gate which choices are presented to the user is to check if they have 
visited a knot in the story. This is done by preceding the choice with the knot name
enclosed by curly braces, for example `{knot}`. 

In the following example, the first choice is only presented if the player has previously 
visited the knot with name `tea_house`.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Prompt};
# let content = r#"
# -> choice
# === choice ===
# 
+   {tea_house} "Yes, I saw them at 'Au thé à la menthe.'"
+   "No, I have not met them."
#
# === tea_house ===
# -> choice
# "#;
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# match story.resume(&mut buffer).unwrap() {
#   Prompt::Choice(choices) => {
#       assert_eq!(choices.len(), 1);
#       assert_eq!(choices[0].text, r#""No, I have not met them.""#);
#   }
#   _ => unreachable!()
# }
# story.move_to("tea_house", None).unwrap();
# match story.resume(&mut buffer).unwrap() {
#   Prompt::Choice(choices) => {
#       assert_eq!(choices.len(), 2);
#       assert_eq!(choices[0].text, r#""Yes, I saw them at 'Au thé à la menthe.'""#);
#   }
#   _ => unreachable!()
# }
```

Under the hood, `inkling` resolves this by translating `{tea_house}` as a variable
whose value is the number of times the knot has been visited. It then asserts 
whether that value is "true", which in `Ink` is whether it is non-zero. Thus, `{tea_house}` 
is an implicit form of writing the explicit condition `{tea_house != 0}`.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Prompt};
# let content = r#"
# -> choice
# === choice ===
# 
+   {tea_house != 0} "Yes, I saw them at 'Au thé à la menthe.'"
+   "No, I have not met them."
#
# === tea_house ===
# -> choice
# "#;
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# match story.resume(&mut buffer).unwrap() {
#   Prompt::Choice(choices) => {
#       assert_eq!(choices.len(), 1);
#       assert_eq!(choices[0].text, r#""No, I have not met them.""#);
#   }
#   _ => unreachable!()
# }
# story.move_to("tea_house", None).unwrap();
# match story.resume(&mut buffer).unwrap() {
#   Prompt::Choice(choices) => {
#       assert_eq!(choices.len(), 2);
#       assert_eq!(choices[0].text, r#""Yes, I saw them at 'Au thé à la menthe.'""#);
#   }
#   _ => unreachable!()
# }
```

Knowing this, we can of course also test these conditions using other types 
of [variables](variables.md). 

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Prompt};
# let content = r"
#
VAR visited_château = true
VAR coins = 3

+   {visited_château} You recognize the bellboy.
+   {coins > 2} [Tip the bellboy]
+   {coins <= 2} [You cannot afford entry]
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# match story.resume(&mut buffer).unwrap() {
#   Prompt::Choice(choices) => {
#       assert_eq!(choices.len(), 2);
#       assert_eq!(choices[0].text, "You recognize the bellboy.");
#       assert_eq!(choices[1].text, "Tip the bellboy");
#   }
#   _ => unreachable!()
# }
```

### Multiple conditions

Multiple conditions can be tested at once by supplying them one after another.
All must be true for the choice to be presented. In this example, the first and third
choices will be presented.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Prompt};
# let content = r"
#
VAR visited_château = true
VAR coins = 6

+   {visited_château} {coins > 5} Purchase the painting
+   {not visited_château} {coins > 5} Your wallet itches but you see nothing of interest.
+   Leave the exhibit
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# match story.resume(&mut buffer).unwrap() {
#   Prompt::Choice(choices) => {
#       assert_eq!(choices.len(), 2);
#       assert_eq!(choices[0].text, "Purchase the painting");
#       assert_eq!(choices[1].text, "Leave the exhibit");
#   }
#   _ => unreachable!()
# }
```

### Beginning choices with variables instead of conditions

Finally, in case you want the choice text to begin with a variable, "escape" the first
curly brace by prepending it with a `\` character. This is so that `inkling` will know 
to write the variable as text, not evaluate it as a condition. This is an unfortunate
quirk of the very compact `Ink` scripting language, where curly braces play many roles.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Prompt};
# let content = r#"
#
VAR mentor = "Evan"

+   \{mentor}, your mentor, greets you
#
# "#;
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# match story.resume(&mut buffer).unwrap() {
#   Prompt::Choice(choices) => {
#       assert_eq!(choices[0].text, "Evan, your mentor, greets you");
#   }
#   _ => unreachable!()
# }
```

For more information about which types of comparisons are supported, see the section
on [variable comparisons](variables.md#variable-comparisons).

## Text conditions

Conditions for displaying text are very similar to how conditions work for choices, 
but work on this format: `{condition: process this if true | otherwise process this}`. 
A colon `:` follows the condition, the content is inside the braces and an optional 
`|` marker marks content to show if the condition is not true.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Prompt};
# let content = r"
# 
VAR visited_château = false
VAR coins = 3

You {visited_château: recognize a painting | see nothing of interest}.
{coins < 5: You cannot afford anything.}
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.resume(&mut buffer).unwrap();
# assert_eq!(&buffer[0].text, "You see nothing of interest.\n");
# assert_eq!(&buffer[1].text, "You cannot afford anything.\n");
```

```plain
You see nothing of interest.
You cannot afford anything.
```

Again, see the section on [variable comparisons](variables.md#variable-comparisons)
for more information about how conditions can be tested.

### Nesting conditions

Conditions can naturally be nested inside of conditional content:

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Prompt};
# let content = r"
# 
VAR met_evan = true
VAR met_austin = false

{met_evan: Yes, I met with Evan {met_austin: and | but not} Austin}.
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.resume(&mut buffer).unwrap();
# assert_eq!(&buffer[0].text, "Yes, I met with Evan but not Austin.\n");
```

```plain
Yes, I met with Evan but not Austin.
```

### Diverts inside conditions

Content inside of conditions can divert to other knots.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Prompt};
# let content = r"
# 
VAR met_evan = true

{met_evan: Evan takes you to his home. -> château | -> END }

=== château ===
The car ride takes a few hours.
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.resume(&mut buffer).unwrap();
# assert_eq!(&buffer[0].text, "Evan takes you to his home.\n");
# assert_eq!(&buffer[1].text, "The car ride takes a few hours.\n");
```

```plain
Evan takes you to his home.
The car ride takes a few hours.
```
