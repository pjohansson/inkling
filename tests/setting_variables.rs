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

    story.start().unwrap();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer[0].text,
        "The latest measurement is 3.6 Röntgen.\n"
    );
}

#[test]
fn global_variables_can_be_changed_from_the_caller() {
    let content = "

VAR value = 3.6
VAR unit = \"Röntgen\"
VAR is_hazardous = false

The latest measurement is {value} {unit}. 

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    story.set_variable("value", 15000.0).unwrap();

    story.start().unwrap();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer[0].text,
        "The latest measurement is 15000 Röntgen.\n"
    );
}

#[test]
fn variables_can_be_used_in_conditions() {
    let content = "

VAR value = 3.6
VAR threshold = 10
VAR unit = \"Röntgen\"

-> root

== root

The latest measurement is {value} {unit}. {value < threshold: Not terrible, not great. | Oh no.}

+   [Redo measurement] -> root

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    story.start().unwrap();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer[0].text,
        "The latest measurement is 3.6 Röntgen. Not terrible, not great.\n"
    );

    story.set_variable("value", 15000.0).unwrap();

    line_buffer.clear();
    story.make_choice(0).unwrap();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer[0].text,
        "The latest measurement is 15000 Röntgen. Oh no.\n"
    );
}

#[test]
fn variables_can_be_changed_and_influence_the_story_flow_in_conditions() {
    let content = "

VAR value = 3.6
VAR unit = \"Röntgen\"
VAR is_hazardous = false

-> root

== root

The latest measurement is {value} {unit}. {not is_hazardous: Not terrible, not great. | Oh no.}

+   [Redo measurement] -> root

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    story.start().unwrap();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer[0].text,
        "The latest measurement is 3.6 Röntgen. Not terrible, not great.\n"
    );

    story.set_variable("value", 15000.0).unwrap();
    story.set_variable("is_hazardous", true).unwrap();

    line_buffer.clear();
    story.make_choice(0).unwrap();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer[0].text,
        "The latest measurement is 15000 Röntgen. Oh no.\n"
    );
}
