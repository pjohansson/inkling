# Story structure

## Knots

A story can be divided into different sections, called *knots* in `ink`. This division
is invisible to the user but makes it easier to write and reason about the story
in production.

A knot is denoted by beginning the line with at least two (2) `=` signs followed by
a name for the knot. On the following lines, the story text can resume.

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content = r"
#
== stairwell
I made my way down the empty stairwell.
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# assert!(story.move_to("stairwell", None).is_ok());
```

The name ('stairwell' in the previous example) cannot contain spaces or non-alphanumeric
symbols. Optionally, it may be followed by more `=` signs, which are not necessary but may
make it easier to identify knots in the document. This is identical to the previous
example:

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content = r"
#
=== stairwell ===
I made my way down the empty stairwell.
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# assert!(story.move_to("stairwell", None).is_ok());
```

### Non-latin characters
Knot names support any character as long as they are
[alphanumeric](https://doc.rust-lang.org/std/primitive.char.html#method.is_alphanumeric)
according to the `Rust` language specification. This seems to include all languages
which are recognized by UTF-8. Thus, knots (and any identifer) may contain e.g.
Chinese, Japanese, Arabic, Cyrillic and other characters. Do let us know if you
find any exceptions.

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content = r"
#
=== عقدة ===
These

=== 매듭 ===
are

=== गांठ ===
all

=== 結 ===
allowed.
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# assert!(story.move_to("عقدة", None).is_ok());
# assert!(story.move_to("매듭", None).is_ok());
# assert!(story.move_to("गांठ", None).is_ok());
# assert!(story.move_to("結", None).is_ok());
```

## Stitches
Knots may be further subdivided into *stitches*. These are denoted by single `=` markers.

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content = r"
#
=== garden ===
= entrance
A pale moonlight illuminated the garden.

= well
The well stank of stagnant water. Is that an eel I see at the bottom?
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# assert!(story.move_to("garden", Some("entrance")).is_ok());
# assert!(story.move_to("garden", Some("well")).is_ok());
```

## Diverts
*Diverts* are used to move to different parts of the story. A divert to a *knot* moves
the story to continue from there. They are designated with the `->` marker followed
by the destination.

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content = r"
# -> stairwell
#
=== stairwell ===
The stairs creaked as I descended.
-> lower_floor

=== garden ===
A pale moonlight illuminated the garden as I entered it.
-> END

=== lower_floor ===
On the bottom I found an unlocked door.
-> garden
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.start().unwrap();
# story.resume(&mut buffer).unwrap();
# assert_eq!(story.get_current_location().unwrap(), ("garden".to_string(), None));
```

Diverts are automatically followed as they are encountered.

### Diverts to stitches

Stitches inside knots can be diverted to using `knot.stitch` as a destination:

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content = r"
#
-> garden.entrance
#
# === garden ===
# = well
# Unreachable.
# = entrance
# A pale moonlight illuminated the garden.
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.start().unwrap();
# story.resume(&mut buffer).unwrap();
# assert!(buffer[0].text.starts_with("A pale moonlight illuminated the garden."));
```

Stitches within the same knot can be diverted to with only the stitch name:

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content = r"
# -> garden
#
=== garden ===
-> well

= entrance
A pale moonlight illuminated the garden.

= well
The well stank of stagnant water.
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.start().unwrap();
# story.resume(&mut buffer).unwrap();
# assert!(buffer[0].text.starts_with("The well stank of stagnant water."));
```

### Ending the story with `-> END`
`END` is a destination that signifies that the story has come to, well, an end. Use
`-> END` diverts for such occasions. An `ink` story is not complete unless all
branches one way or another leads to an `-> END` divert: ending a story should
be intentional.

## Diverts in choices

A common use of branches is to divert to other knots.

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content = r"
#
*   [Descend stairs] -> lower_floor
*   [Return to desk]
    I sighed, wearily, and returned to my room.
    -> desk

=== desk ===
As I sat by my desk, I noticed that my notebook had gone missing.

=== lower_floor ===
On the bottom I found an unlocked door.
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.start().unwrap();
# story.resume(&mut buffer).unwrap();
# story.make_choice(1).unwrap();
# story.resume(&mut buffer).unwrap();
# assert_eq!(story.get_current_location().unwrap(), ("desk".to_string(), None));
```
