use inkling::error::ReadErrorKind;
use inkling::*;

#[test]
fn knot_names_in_diverts_are_validated() {
    let content = "
    
== duckburg
-> bin

== money_bin
-> END

";

    match read_story_from_string(content) {
        Err(ReadErrorKind::InvalidAddress(..)) => (),
        _ => panic!(),
    }
}

#[test]
fn stitch_names_in_diverts_are_validated() {
    let content = "

-> duckburg.bin
    
== duckburg
= money_bin
-> END

";

    match read_story_from_string(content) {
        Err(ReadErrorKind::InvalidAddress(..)) => (),
        _ => panic!(),
    }
}

#[test]
fn stitch_names_to_relative_addresses_are_validated() {
    let content = "

== duckburg
Welcome to Duck Burg!
-> bin

= money_bin
-> END

";

    match read_story_from_string(content) {
        Err(ReadErrorKind::InvalidAddress(..)) => (),
        _ => panic!(),
    }
}

#[test]
fn diverts_in_choice_text_are_validated() {
    let content = "

== duckburg
Welcome to Duck Burg!
*   [Money bin] -> bin

= money_bin
-> END

";

    match read_story_from_string(content) {
        Err(ReadErrorKind::InvalidAddress(..)) => (),
        _ => panic!(),
    }
}

#[test]
fn diverts_in_alternative_sequences_are_validated() {
    let content = "

== duckburg
Welcome to Duck Burg! {We live here.|We headed to Uncle Scrooge's money bin. -> bin}

== money_bin
-> END

";

    match read_story_from_string(content) {
        Err(ReadErrorKind::InvalidAddress(..)) => (),
        _ => panic!(),
    }
}

#[test]
fn diverts_in_nested_branches_are_validated() {
    let content = "

== duckburg
*   Money bin
    We headed to the money bin. -> bin

== money_bin
-> END

";

    match read_story_from_string(content) {
        Err(ReadErrorKind::InvalidAddress(..)) => (),
        _ => panic!(),
    }
}

#[test]
fn condition_addresses_are_validated() {
    let content = "

== duckburg
*   {bin} But we had already visited the money bin.
*   -> END

== money_bin
-> END

";

    match read_story_from_string(content) {
        Err(ReadErrorKind::InvalidAddress(..)) => (),
        _ => panic!(),
    }
}
