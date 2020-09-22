# Story structure

Story scripts can be divided into different sections, to which the story can diverge.
This section introduces how to create these sections and move to them in the text.

## Knots

A story can be divided into different sections, called *knots* in `Ink`. This division
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
# story.resume(&mut buffer).unwrap();
# story.make_choice(1).unwrap();
# story.resume(&mut buffer).unwrap();
# assert_eq!(story.get_current_location().unwrap(), ("desk".to_string(), None));
```

## Revisiting content and choices

With diverts we can easily return to previously visited knots and stitches. When 
this happens, the text is reevaluated to reflect the current state of the story
(see the sections on [conditional content](conditional-content.md) and 
[alternating sequences](sequences.md) for more information).

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content = r"
#
=== table ===
You are seated at the table.

*   [Order a cup of tea] 
    A waiter returns with a steaming hot cup of tea. 
    -> table
*   [Leave]
    You leave the café.
#
# ";
# assert!(read_story_from_string(content).is_ok());
```

### Once-only and sticky choices

Any set of branching choices will also be reevaluated. There are two types of 
choices, denoted by if they begin with `*` or `+` markers:

 *  `*` marks *once-only* choices, which can only be picked once
 *  `+` marks *sticky* choices, which can be picked any number of times

In short, *once-only* choices are removed from the choice list if they are picked.
*Sticky* choices will remain. This has to be kept in mind if the branch might be 
revisited during the story.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Prompt};
# let content = r"
# -> loop
#
=== loop ===
*   This choice can only be picked once -> loop
+   This choice is always here -> loop
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# match story.resume(&mut buffer).unwrap() {
#   Prompt::Choice(choices) => {
#       assert_eq!(choices.len(), 2);
#       assert_eq!(&choices[0].text, "This choice can only be picked once");
#   }
#   _ => unreachable!()
# }
# story.make_choice(0).unwrap();
# match story.resume(&mut buffer).unwrap() {
#   Prompt::Choice(choices) => {
#       assert_eq!(choices.len(), 1);
#       assert_eq!(&choices[0].text, "This choice is always here");
#   }
#   _ => unreachable!()
# }
# story.make_choice(0).unwrap();
# match story.resume(&mut buffer).unwrap() {
#   Prompt::Choice(choices) => {
#       assert_eq!(choices.len(), 1);
#       assert_eq!(&choices[0].text, "This choice is always here");
#   }
#   _ => unreachable!()
# }
```

### Running out of choices

Since once-only choices are removed it is possible for a branching choice point
to run out of choices. **This will result in an 
[error](https://docs.rs/inkling/latest/inkling/enum.InklingError.html#variant.OutOfChoices) 
being returned from `inkling` at runtime.**

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Prompt};
# let content = r"
# -> thrice_fail
#
=== thrice_fail ===
The third time we visit this we are out of choices and an error is returned.

*   First choice -> thrice_fail
*   Second choice -> thrice_fail
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.resume(&mut buffer).unwrap();
# story.make_choice(0).unwrap();
# story.resume(&mut buffer).unwrap();
# story.make_choice(0).unwrap();
# assert!(story.resume(&mut buffer).is_err());
```

So be careful when writing branching choices using only once-only markers. Is there a 
risk that you will return to it multiple times? 

### Fallback choices

There is a fallback option available for running out of choices. If no regular (sticky 
or once-only) choices are left to present for the user, `inkling` will look for a 
*fallback* choice and automatically follow it.

This can only be a single choice and is marked by being a choice *without choice text*,
which is to say that it starts with a divert `->` marker.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Prompt};
# let content = r"
# -> twice_fail
#
=== twice_fail ===
The second time we visit this we are out of regular choices.
We then use the fallback.

*   First choice -> twice_fail
*   -> fallback

=== fallback ===
We escaped the loop!
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# match story.resume(&mut buffer).unwrap() {
#   Prompt::Choice(choices) => {
#       assert_eq!(choices.len(), 1);
#       assert_eq!(&choices[0].text, "First choice");
#   }
#   _ => unreachable!()
# }
# story.make_choice(0).unwrap();
# story.resume(&mut buffer).unwrap(); 
# assert_eq!(&buffer.last().unwrap().text, "We escaped the loop!\n");
```

The fallback content can contain text by putting it on a new line directly after 
the divert marker.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Prompt};
# let content = r"
# -> write_article
#
=== write_article ===
*   [Write abstract] -> write_article
*   [Write main text] -> write_article
*   [Write summary] -> write_article
*   -> 
    The article is finished.
    -> submit_article

=== submit_article ===
You submit it to your editor. Wow, writing is easy!
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.resume(&mut buffer).unwrap(); 
# story.make_choice(0).unwrap();
# story.resume(&mut buffer).unwrap(); 
# story.make_choice(0).unwrap();
# story.resume(&mut buffer).unwrap(); 
# story.make_choice(0).unwrap();
# story.resume(&mut buffer).unwrap(); 
# assert_eq!(&buffer[0].text, "The article is finished.\n");
# assert!(&buffer[1].text.starts_with("You submit it to your editor."));
```

Fallback choices can also be sticky. If they are not they will also be consumed after
use. Again, ensure that you are sure that branches with non-sticky fallback choices
will not be returned to multiple times.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Prompt};
# let content = r"
#
=== once_only_fallback ===
This will return an error if the fallback choice is used twice.
*   -> once_only_fallback 

=== sticky_fallback ===
# {sticky_fallback > 4 : -> END} // exit once we have returned here a few times
This sticky fallback choice can be use any number of times.
+   -> sticky_fallback 
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.move_to("once_only_fallback", None).unwrap();
# assert!(story.resume(&mut buffer).is_err());
# story.move_to("sticky_fallback", None).unwrap();
# story.resume(&mut buffer).unwrap(); 
# assert!(story.resume(&mut buffer).is_ok());
```

## Gather points

When creating a set of choices, you can return (or, *gather*) all of the branches to 
a single path after they have gone through their content. This is done using 
*gather points.*

To return the branches, add a gather marker `-` at a new line after the branches.

In the following example, regardless of whether the player heads to the garden 
or the kitchen, they return to their room. There, they are presented with the next choice.

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content = r"
#
*   [Head into the garden]
    The chirp of crickets greet you as you enter the garden.
*   [Move to the kitchen]
    A crackling fireplace illuminates the dark room.
-   A while later, you return to your room.
*   [Lay in bed]
*   [Sit at table]
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut story_other = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.resume(&mut buffer).unwrap();
# story.make_choice(0).unwrap();
# story.resume(&mut buffer).unwrap();
# assert_eq!(buffer.len(), 2);
# assert_eq!(&buffer[0].text, "The chirp of crickets greet you as you enter the garden.\n");
# assert_eq!(&buffer[1].text, "A while later, you return to your room.\n");
# buffer.clear();
# story_other.resume(&mut buffer).unwrap();
# story_other.make_choice(1).unwrap();
# story_other.resume(&mut buffer).unwrap();
# assert_eq!(buffer.len(), 2);
# assert_eq!(&buffer[0].text, "A crackling fireplace illuminates the dark room.\n");
# assert_eq!(&buffer[1].text, "A while later, you return to your room.\n");
```

### Nested gather points

Gathers can be performed for any nested level of choices. Simply add the corresponding 
number of gather markers `-` below.

In this example, both inner choices 1.1 and 1.2 will gather at 1.1. Inner choices 2.1 
and 2.2 at gather 2.1. Then finally, both outer choices 1 and 2 at gather point 1.

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content = r"
#
*   Choice 1
    * *     Choice 1.1
    * *     Choice 1.2
    - -     Gather 1.1
*   Choice 2
    * *     Choice 2.1
    * *     Choice 2.2
    - -     Gather 2.1
-   Gather 1
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut story_other = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.resume(&mut buffer).unwrap();
# story.make_choice(1).unwrap();
# story.resume(&mut buffer).unwrap();
# story.make_choice(0).unwrap();
# story.resume(&mut buffer).unwrap();
# assert_eq!(buffer.len(), 4);
# assert_eq!(&buffer[0].text, "Choice 2\n");
# assert_eq!(&buffer[1].text, "Choice 2.1\n");
# assert_eq!(&buffer[2].text, "Gather 2.1\n");
# assert_eq!(&buffer[3].text, "Gather 1\n");
# buffer.clear();
# story_other.resume(&mut buffer).unwrap();
# story_other.make_choice(0).unwrap();
# story_other.resume(&mut buffer).unwrap();
# story_other.make_choice(1).unwrap();
# story_other.resume(&mut buffer).unwrap();
# assert_eq!(buffer.len(), 4);
# assert_eq!(&buffer[0].text, "Choice 1\n");
# assert_eq!(&buffer[1].text, "Choice 1.2\n");
# assert_eq!(&buffer[2].text, "Gather 1.1\n");
# assert_eq!(&buffer[3].text, "Gather 1\n");
```

## Preamble

The script is divided into a *preamble* and the story *content*. The preamble contains
[variable declarations](variables.md), [metadata](metadata.md) and inclusions of other 
documents. The content comes afterwards and can refer to declarations in the preamble.

The end of the preamble in a script is marked by the first line of text or story content.
This can be a divert to the introductory scene.

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content = r#"
#
// Global story tags are declared in the preamble
## title: Inkling 
## author: Petter Johansson

// ... as are global variables
CONST name = "d'Artagnan"
VAR rank = "Capitaine"

// First line of story content comes here, which ends the preamble declaration
-> introduction 

=== introduction ===
I opened my notebook to a blank page, pen in hand.
# "#;
# let story = read_story_from_string(content).unwrap();
# let tags = story.get_story_tags();
# assert_eq!(&tags[0], "title: Inkling");
# assert_eq!(&tags[1], "author: Petter Johansson");
# assert!(story.get_variable("name").is_ok());
# assert!(story.get_variable("rank").is_ok());
```