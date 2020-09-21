# Story metadata

Information about the story, knots or even individual lines can be marked with *tags*. All tags
begin with the `#` marker.

Tags are stored as pure strings and can thus be of any form you want. `inkling` assigns no
meaning to them on its own, it's for you as the user to decide how to treat them.

## Global tags

Tags in the preamble are global story tags. Here you can typically mark up metadata for the script.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Variable};
# let content = r#"
#
## title: Inkling
## author: Petter Johansson
#
# "#;
# let story = read_story_from_string(content).unwrap();
# let tags = story.get_story_tags();
# assert_eq!(&tags[0], "title: Inkling");
# assert_eq!(&tags[1], "author: Petter Johansson");
```

## Knot tags

Tags encountered in a knot before any content is parsed as tags belonging to that knot.

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content = r#"
#
=== stairwell ===
## sound: blowing_wind.ogg
## dark, quiet, dangerous
I made my way down the empty stairwell.
#
# "#;
# let story = read_story_from_string(content).unwrap();
# let tags = story.get_knot_tags("stairwell").unwrap();
# assert_eq!(&tags[0], "sound: blowing_wind.ogg");
# assert_eq!(&tags[1], "dark, quiet, dangerous");
```

## Line tags

Lines can be tagged by adding the tag after the line content. Multiple tags can
be set, separated by additional '#' markers.

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content = r#"
#
A pale moonlight illuminated the garden. # sound: crickets.ogg
The well stank of stagnant water. # smell, fall # sound: water_drip.ogg
#
# "#;
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.start().unwrap();
# story.resume(&mut buffer).unwrap();
# let tags1 = &buffer[0].tags;
# let tags2 = &buffer[1].tags;
# assert_eq!(&tags1[0], "sound: crickets.ogg");
# assert_eq!(&tags2[0], "smell, fall");
# assert_eq!(&tags2[1], "sound: water_drip.ogg");
```

Tags can also be added to choice lines.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Prompt};
# let content = r#"
#
*   I made my way to the well. # sound: footsteps.ogg
#
# "#;
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.start().unwrap();
# match story.resume(&mut buffer).unwrap() {
#   Prompt::Choice(choices) => {
#       assert_eq!(choices[0].tags[0], "sound: footsteps.ogg");
#   }
#   _ => unreachable!()
# }
```
