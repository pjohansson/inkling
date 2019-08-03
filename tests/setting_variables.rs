use inkling::*;

#[test]
fn global_variables_are_parsed_when_the_story_is_read() {
    let content = "

VAR value = 3.6
VAR unit = \"Röntgen\"
VAR is_hazardous = false

The latest measurement is {value} {unit}. 

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    story.start(&mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer[0].text,
        "The latest measurement is 3.6 Röntgen.\n"
    );
}

#[test]
fn global_variables_can_be_changed_from_the_valler() {
    let content = "

VAR value = 3.6
VAR unit = \"Röntgen\"
VAR is_hazardous = false

The latest measurement is {value} {unit}. 

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    story.set_variable("value", 15000.0).unwrap();

    story.start(&mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer[0].text,
        "The latest measurement is 15000 Röntgen.\n"
    );
}
