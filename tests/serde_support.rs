#[cfg(not(feature = "serde_support"))]
#[test]
fn serde_support_tests() {
    panic!("Feature `serde_support` must be activated to run these integration tests");
}

#[cfg(all(feature = "serde_support", test))]
pub mod tests {
    use inkling::*;
    use serde_json;

    #[test]
    fn serialization_saves_current_state() {
        let content = "

== passage ==

A crossing! Which path do you take?

+   Left -> torch
+   Right -> dark_room
    
== dark_room ==
You enter a dark room.

+   {torch} Use your torch to light the way forward. 
+   Head back.
-> passage

== torch ==
In a small chamber further in you find a torch. 
You head back.
-> passage

";

        let mut story = read_story_from_string(content).unwrap();
        let mut line_buffer = Vec::new();

        let choices = story
            .start(&mut line_buffer)
            .unwrap()
            .get_choices()
            .unwrap();

        let serialized_without_torch = serde_json::to_string(&story).unwrap();
        let mut state_without_torch: Story =
            serde_json::from_str(&serialized_without_torch).unwrap();

        let choices_without_torch = state_without_torch
            .resume_with_choice(&choices[1], &mut line_buffer)
            .unwrap()
            .get_choices()
            .unwrap();

        let choices = story
            .resume_with_choice(&choices[0], &mut line_buffer)
            .unwrap()
            .get_choices()
            .unwrap();

        let serialized_with_torch = serde_json::to_string(&story).unwrap();
        let mut state_with_torch: Story = serde_json::from_str(&serialized_with_torch).unwrap();

        let choices_with_torch = state_with_torch
            .resume_with_choice(&choices[1], &mut line_buffer)
            .unwrap()
            .get_choices()
            .unwrap();

        assert_eq!(choices_without_torch.len(), 1);
        assert_eq!(choices_with_torch.len(), 2);
    }
}
