# Implemented Ink features

This chapter contains a list of the `Ink` features which are available in `inkling`.

More information about these features can be found in 
[Inkle's guide to writing with Ink](https://github.com/inkle/ink/blob/master/Documentation/WritingWithInk.md),
which is a better guide showing how to write a script. 

However, not everything in that guide can be done with `inkling`, since it is not
completely compatible with the original implementation. This is partly 
the reason for this document, which shows which features are *guaranteed* to work.
All examples shown here are accompanied under the hood by tests which assert that 
the result is what it should be.

Examples in this chapter show how the features are written in plain `.ink` 
text files (although the file names do not have to end with `.ink`). Text inside of 
these files will appear like this:

```rust
# let content = "
Example text in a file to be read.
# ";
```

Major features which are not yet implemented are listed on the [missing features](missing-features.md) page.
