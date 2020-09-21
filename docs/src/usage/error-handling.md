# Dealing with errors

## Errors from reading the `Story`

When parsing the `Story` using `read_story_from_string`, its content is validated.
This means that `inkling` goes through it and looks for errors. These errors include
things like invalid knot or variable declarations, using invalid names for variables in
assignments and conditions, wrongly typed conditions, and much more.

If any error is encountered during this validation step, the function returns
a [`ReadError`][ReadError] which contains a list of all the errors it found. The helper 
function [`print_read_error`][print_read_error] exists to write a description of all 
errors and where they were found into a single string, which can be written to a log file.

## Runtime errors

Once a story is started, returned errors will be of [`InklingError`][InklingError] type.

[InklingError]: https://docs.rs/inkling/latest/inkling/enum.InklingError.html
[ReadError]: https://docs.rs/inkling/latest/inkling/error/enum.ReadError.html
[print_read_error]: https://docs.rs/inkling/latest/inkling/error/parse/fn.print_read_error.html