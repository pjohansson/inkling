# Inspecting the log

When the `Story` is [parsed][read_story_from_string] it goes through all lines and scenes in the script, 
inspecting them for inconsistencies and notes. If any errors are encountered the parsing fails, 
and an [error](./error-handling.md#errors-from-reading-the-story) is returned. 

However, some inconsistencies are not very serious and do not raise an error. Such
issues are instead added to a [log][log], which you can inspect after the parsing has 
finished. You can then decide whether any yielded warning is sufficient for further investigation.

We recommend that you always check this log and inspect its messages after parsing a story. Besides
regular warnings, it will also contain reminders of any [to-do comment](../features/metadata.md#to-do-comments) 
you have added to the story, which may merit having a look at.

The log supports standard iterator operations, which makes it simple to walk through all warnings
and to-do comments at once.

```rust
# extern crate inkling;
# use inkling::{read_story_from_string, Story};
# let content = r#"
# TODO: Should these names be in variables?
# A single candle flickered by my side.
# Pen in hand I procured a blank letter.
# 
# *   "Dear Guillaume"
#     Sparing the more unfavorable details from him, I requested his aid.
# 
# *   "To the Fiendish Impostor"
# "#;
let mut story: Story = read_story_from_string(&content).unwrap();

let log = story.get_log();

// Print all warnings and comments to standard error for inspection
for message in log.iter() {
    eprintln!("{}", message);
}
#
# assert_eq!(log.todo_comments.len(), 1);
```

[log]: https://docs.rs/inkling/latest/inkling/struct.Story.html#structfield.log
[Story]: https://docs.rs/inkling/latest/inkling/struct.Story.html
[read_story_from_string]: https://docs.rs/inkling/latest/inkling/fn.read_story_from_string.html