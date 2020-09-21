# Helper functions

This page lists miscellaneous functions to deal with `inkling` data.

## Text handling 

*   [`copy_lines_into_string`][copy_lines_into_string] takes a buffer of `Line` objects 
    and joins the text into a single string which is returned

## Read error handling

*   [`print_read_error`][print_read_error] creates a string with the information of all
    errors that were encountered when parsing a story

[print_read_error]: https://docs.rs/inkling/latest/inkling/error/parse/fn.print_read_error.html
[copy_lines_into_string]: https://docs.rs/inkling/latest/inkling/fn.copy_lines_into_string.html