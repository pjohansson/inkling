# Alternating sequences

`Ink` comes with several methods which vary the content of a line every time it is
seen. These are known as *alternating sequences* of content.

## Sequences

Alternative sequences can be declared in a line using curly braces and `|` separators.
Every time the line is revisited, the next piece will be presented, allowing us to
write something like this:

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content = r"
# -> continue
# === continue ===
#
The train had arrived {in Mannheim|in Heidelberg|at its final stop}.
#
# + [Continue] -> continue
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.resume(&mut buffer).unwrap();
# assert_eq!(&buffer.last().unwrap().text, "The train had arrived in Mannheim.\n");
# story.make_choice(0).unwrap();
# story.resume(&mut buffer).unwrap();
# assert_eq!(&buffer.last().unwrap().text, "The train had arrived in Heidelberg.\n");
# story.make_choice(0).unwrap();
# story.resume(&mut buffer).unwrap();
# assert_eq!(&buffer.last().unwrap().text, "The train had arrived at its final stop.\n");
# story.make_choice(0).unwrap();
# story.resume(&mut buffer).unwrap();
# assert_eq!(&buffer.last().unwrap().text, "The train had arrived at its final stop.\n");
```

When [revisiting](structure.md#revisiting-content-and-choices) this line, it will go through the
alternatives in order, then repeat the final value after reaching it. In `Ink` terms,
this is called a *stopping sequence*.

```plain
The train had arrived in Mannheim.
The train had arrived in Heidelberg.
The train had arrived at its final stop.
The train had arrived at its final stop.
```

### Cycle sequences
*Cycle sequences* repeat the entire sequence after reaching the final piece. They are
denoted by starting the first alternative with a `&` marker.

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content = r"
# -> continue
# === continue ===
#
Today is a {&Monday|Tuesday|Wednesday|Thursday|Friday|Saturday|Sunday}.
#
# + [Continue] -> continue
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# for day in ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday", "Monday"].iter() {
#   story.resume(&mut buffer).unwrap();
#   assert_eq!(buffer.last().unwrap().text, format!("Today is a {}.\n", day));
#   story.make_choice(0).unwrap();
# }
```

```plain
Today is a Monday.
Today is a Tuesday.
Today is a Wednesday.
[...]
Today is a Sunday.
Today is a Monday.
```

### Once-only sequences
*Once-only sequences* goes through all alternatives in order and then produce nothing.
They are denoted by starting the first alternative with a `!` marker.

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content = r"
# -> continue
# === continue ===
#
I met with Anirudh{! for the first time| for the second time}.
#
# + [Continue] -> continue
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.resume(&mut buffer).unwrap();
# assert_eq!(&buffer.last().unwrap().text, "I met with Anirudh for the first time.\n");
# story.make_choice(0).unwrap();
# story.resume(&mut buffer).unwrap();
# assert_eq!(&buffer.last().unwrap().text, "I met with Anirudh for the second time.\n");
# story.make_choice(0).unwrap();
# story.resume(&mut buffer).unwrap();
# assert_eq!(&buffer.last().unwrap().text, "I met with Anirudh.\n");
```

```plain
I met with Anirudh for the first time.
I met with Anirudh for the second time.
I met with Anirudh.
```

### Shuffle sequences
*Shuffle sequences* go through the alternatives in a random order, then shuffle the
alternatives and deal them again. They are denoted by starting the first alternative
with a `~` marker.

Note that these are only random if `inkling` has been compiled with the `random` feature.
Otherwise they mimic the behavior of cycle sequences.

```rust
# let content = r"
# -> continue
# === continue ===
#
I was dealt a Jack of {~hearts|spades|diamonds|clubs}.
#
# + [Continue] -> continue
# ";
```

```plain
I was dealt a Jack of diamonds.
I was dealt a Jack of spades.
I was dealt a Jack of hearts.
I was dealt a Jack of clubs.
```

## Nested alternatives

Alternatives can of course hide even more alternatives. How would we otherwise have any fun in life?

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content = r"
# -> continue
# === continue ===
#
I {&{strode|walked} hastily|waltzed {gracefully|clumsily}} into the room.
#
# + [Continue] -> continue
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.resume(&mut buffer).unwrap();
# assert_eq!(&buffer.last().unwrap().text, "I strode hastily into the room.\n");
# story.make_choice(0).unwrap();
# story.resume(&mut buffer).unwrap();
# assert_eq!(&buffer.last().unwrap().text, "I waltzed gracefully into the room.\n");
# story.make_choice(0).unwrap();
# story.resume(&mut buffer).unwrap();
# assert_eq!(&buffer.last().unwrap().text, "I walked hastily into the room.\n");
# story.make_choice(0).unwrap();
# story.resume(&mut buffer).unwrap();
# assert_eq!(&buffer.last().unwrap().text, "I waltzed clumsily into the room.\n");
```

```plain
I strode hastily into the room.
I waltzed gracefully into the room.
I walked hastily into the room.
I waltzed clumsily into the room.
```

## Diverts in alternatives

We can use diverts inside of alternatives to alternatively trigger different parts
of the story.

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content = r"
# -> continue
# === continue ===
#
The {first|next} time I saw the door it was {locked. -> locked_door|open. -> open_door}

=== locked_door ===
I had to return another day.
# -> continue

=== open_door ===
In the doorway stood a thin figure.
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.resume(&mut buffer).unwrap();
# assert_eq!(&buffer[0].text, "The first time I saw the door it was locked.\n");
# assert_eq!(&buffer[1].text, "I had to return another day.\n");
# assert_eq!(&buffer[2].text, "The next time I saw the door it was open.\n");
# assert_eq!(&buffer[3].text, "In the doorway stood a thin figure.\n");
```

```plain
The first time I saw the door it was locked.
I had to return another day.
The next time I saw the door it was open.
In the doorway stood a thin figure.
```

Here's where [glue](basic.md#glue) can come in handy. By adding glue before
the divert we can continue the same paragraph in a new knot. Building on the
previous example:

```rust
# extern crate inkling;
# use inkling::read_story_from_string;
# let content = r"
# -> continue
# === continue ===
#
The {first|next} time I saw the door it was {locked. -> locked_door|open. -> open_door}

=== locked_door ===
<> I had to return another day.
# -> continue

=== open_door ===
<> In the doorway stood a thin figure.
#
# ";
# let mut story = read_story_from_string(content).unwrap();
# let mut buffer = Vec::new();
# story.resume(&mut buffer).unwrap();
# assert_eq!(&buffer[0].text, "The first time I saw the door it was locked. ");
# assert_eq!(&buffer[1].text, "I had to return another day.\n");
# assert_eq!(&buffer[2].text, "The next time I saw the door it was open. ");
# assert_eq!(&buffer[3].text, "In the doorway stood a thin figure.\n");
```

```plain
The first time I saw the door it was locked. I had to return another day.
The next time I saw the door it was open. In the doorway stood a thin figure.
```