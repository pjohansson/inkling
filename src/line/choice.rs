use crate::{
    consts::{CHOICE_MARKER, DIVERT_MARKER, STICKY_CHOICE_MARKER},
    error::{LineError, ParseError},
    line::*,
};

#[derive(Clone, Debug, PartialEq)]
/// A single choice in a (usually) set of choices presented to the user.
pub struct FullChoice {
    /// Text presented to the user to represent the choice.
    pub selection_text: FullLine,
    /// Text that the choice produces when selected, replacing the `selection_text` line.
    /// Can be empty, in which case the presented text is removed before the story flow
    /// continues to the next line.
    pub display_text: FullLine,
    /// Conditions that must be fulfilled for the choice to be displayed.
    pub conditions: Vec<Condition>,
    /// By default a choice will be filtered after being visited once. If it is marked
    /// as sticky it will stick around.
    pub is_sticky: bool,
    /// Fallback choices are, in order, automatically followed if no other choices are available.
    pub is_fallback: bool,
}

pub struct FullChoiceBuilder {
    selection_text: FullLine,
    display_text: FullLine,
    conditions: Vec<Condition>,
    is_fallback: bool,
    is_sticky: bool,
}

impl FullChoiceBuilder {
    pub fn from_line(line: FullLine) -> Self {
        FullChoiceBuilder {
            selection_text: line.clone(),
            display_text: line,
            conditions: Vec::new(),
            is_sticky: false,
            is_fallback: false,
        }
    }

    pub fn build(self) -> FullChoice {
        FullChoice {
            selection_text: self.selection_text,
            display_text: self.display_text,
            conditions: self.conditions,
            is_sticky: self.is_sticky,
            is_fallback: self.is_fallback,
        }
    }

    pub fn set_conditions(&mut self, conditions: &[Condition]) {
        self.conditions = conditions.to_vec();
    }

    pub fn set_display_text(&mut self, line: FullLine) {
        self.display_text = line;
    }

    pub fn set_is_fallback(&mut self, fallback: bool) {
        self.is_fallback = fallback;
    }

    pub fn set_is_sticky(&mut self, sticky: bool) {
        self.is_sticky = sticky;
    }

    pub fn set_selection_text(&mut self, line: FullLine) {
        self.selection_text = line;
    }

    #[cfg(test)]
    pub fn is_fallback(mut self) -> Self {
        self.set_is_fallback(true);
        self
    }

    #[cfg(test)]
    pub fn is_sticky(mut self) -> Self {
        self.set_is_sticky(true);
        self
    }

    #[cfg(test)]
    pub fn with_conditions(mut self, conditions: &[Condition]) -> Self {
        self.set_conditions(conditions);
        self
    }

    #[cfg(test)]
    pub fn with_selection_text(mut self, line: FullLine) -> Self {
        self.set_selection_text(line);
        self
    }

    #[cfg(test)]
    pub fn with_display_text(mut self, line: FullLine) -> Self {
        self.set_display_text(line);
        self
    }
}
