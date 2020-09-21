# Design intent

This library has been written with the intent to create a simple usage loop.

```rust,ignore
while let Prompt::Choice(choices) = story.resume(&mut buffer)? {
    // Process text, show it to the player, then present the encountered
    // choices to them and resume.
    let i = select_choice(&choices)?;
    story.make_choice(i)?;
}
```

The loop will finish when `Prompt::Done` is returned from the `resume` call, 
signaling the end of the story. Here errors are returned through the standard
`?` operator, which further simplifies the loop.

Of course, this pattern may not suit your application.