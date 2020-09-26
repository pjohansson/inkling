//! Choice which branches the story.

use crate::{
    error::{parse::validate::ValidationError, utils::MetaData},
    knot::Address,
    line::{Condition, InternalLine},
    story::{
        validate::{ValidateContent, ValidationData},
        Logger,
    },
};

use std::sync::{Arc, Mutex};

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};
use std::ops::DerefMut;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// A single choice in a (usually) set of choices presented to the user.
pub struct InternalChoice {
    /// Text presented to the user to represent the choice.
    ///
    /// This is a reference counted object because of how we process the sets of encountered
    /// choices. When encountered inside the node during a follow, all choices in a set are
    /// collected and sent further up in the stack. They are then processed before displaying
    /// to the user.
    ///
    /// This is different from how regular lines are processed to their final form, which
    /// is done during the follow as the lines are encountered by the `Process` trait.
    ///
    /// In theory, we could rewrite the choice code to process them at collection, but
    /// that would mess a bit with how the nodes are processing the data: changing their
    /// responsibility from just finding the content, to checking conditions for which
    /// choices will be available and so on.
    ///
    /// Instead, we use a pointer with internal mutability and send that further up the stack.
    /// This means that any processing of choices further up will affect the data in the node,
    /// meaning that for example alternative sequences will be updated if the choice was seen.
    pub selection_text: Arc<Mutex<InternalLine>>,
    /// Text that will be added to the output line buffer if the choice is selected.
    ///
    /// This will be added to the buffer before the rest of the lines from the selected
    /// branch will be followed and processed.
    ///
    /// Can be empty.
    pub display_text: InternalLine,
    /// ConditionKinds that must be fulfilled for the choice to be displayed.
    pub condition: Option<Condition>,
    /// By default a choice will be filtered after being visited once. If it is marked
    /// as sticky it will stick around.
    pub is_sticky: bool,
    /// Fallback choices are, in order, automatically followed if no other choices are available.
    pub is_fallback: bool,
    /// Information about the origin of this choice in the story file or text.
    pub meta_data: MetaData,
}

impl PartialEq for InternalChoice {
    fn eq(&self, rhs: &InternalChoice) -> bool {
        let left_line = (*self.selection_text.lock().unwrap()).clone();
        let right_line = (*rhs.selection_text.lock().unwrap()).clone();

        left_line == right_line
            && self.display_text == rhs.display_text
            && self.condition == rhs.condition
            && self.is_sticky == rhs.is_sticky
            && self.is_fallback == rhs.is_fallback
            && self.meta_data == rhs.meta_data
    }
}

impl ValidateContent for InternalChoice {
    fn validate(
        &mut self,
        error: &mut ValidationError,
        log: &mut Logger,
        current_location: &Address,
        _: &MetaData,
        data: &ValidationData,
    ) {
        let num_address_errors = error.invalid_address_errors.len();

        self.selection_text.lock().unwrap().deref_mut().validate(
            error,
            log,
            current_location,
            &self.meta_data,
            data,
        );

        // If address errors were found in the selection part of this line they may be repeated
        // in the display part. Since they are parsed from the same line we raise an error for
        // it either way, so any independent errors that are purely in the display part
        // are not essential to report until we know that the selection part is good. Thus,
        // if (but *only* if) we found such an error in the selection part we skip validation in
        // the display part. Otherwise multiple errors about the same part may be raised.
        if num_address_errors == error.invalid_address_errors.len() {
            self.display_text
                .validate(error, log, current_location, &self.meta_data, data);
        }

        if let Some(ref mut condition) = self.condition {
            condition.validate(error, log, current_location, &self.meta_data, data);
        }
    }
}

/// Builder for constructing an `InternalChoice`.
///
/// For testing purposes this struct implement additional functions when
/// the `test` profile is activated. These functions are not meant to be used internally
/// except by tests, since they do not perform any validation of the content.
///
/// # Notes
///  *  Tags can be set to the builder, in which case they are set to both
///     the `selection_text` and `display_text` items.
pub struct InternalChoiceBuilder {
    selection_text: InternalLine,
    display_text: InternalLine,
    condition: Option<Condition>,
    is_fallback: bool,
    is_sticky: bool,
    tags: Option<Vec<String>>,
}

impl InternalChoiceBuilder {
    /// Construct the builder with a line of text.
    ///
    /// The given line is set as both the `selection_text` and `display_text` items.
    pub fn from_line(line: InternalLine) -> Self {
        InternalChoiceBuilder {
            selection_text: line.clone(),
            display_text: line,
            condition: None,
            is_sticky: false,
            is_fallback: false,
            tags: None,
        }
    }

    /// Finalize the `InternalChoice` and return it.
    ///
    /// If tags have been set they are set as the tags for both the `selection_text`
    /// and `display_text` lines.
    pub fn build(mut self) -> InternalChoice {
        if let Some(tags) = self.tags {
            self.display_text.tags = tags.clone();
            self.selection_text.tags = tags.clone();
        }

        let meta_data = self.display_text.meta_data.clone();

        InternalChoice {
            selection_text: Arc::new(Mutex::new(self.selection_text)),
            display_text: self.display_text,
            condition: self.condition,
            is_sticky: self.is_sticky,
            is_fallback: self.is_fallback,
            meta_data,
        }
    }

    /// Set a list of conditions for the choice.
    pub fn set_condition(&mut self, condition: &Condition) {
        self.condition.replace(condition.clone());
    }

    #[cfg(test)]
    /// Set the `display_text` line.
    pub fn set_display_text(&mut self, line: InternalLine) {
        self.display_text = line;
    }

    /// Set whether or not the choice is a fallback.
    pub fn set_is_fallback(&mut self, fallback: bool) {
        self.is_fallback = fallback;
    }

    /// Set the `selection_text` line.
    pub fn set_selection_text(&mut self, line: InternalLine) {
        self.selection_text = line;
    }

    #[cfg(test)]
    /// Construct the builder with a line of pure text.
    ///
    /// Uses `InternalLine::from_string` to create the line which is set to both `selection_text`
    /// and `display_text`.
    pub fn from_string(line: &str) -> Self {
        Self::from_line(InternalLine::from_string(line))
    }

    #[cfg(test)]
    /// Construct the builder with a line of pure text for the `selection_text` item.
    ///
    /// The `display_text` line will be empty.
    pub fn from_selection_string(line: &str) -> Self {
        let empty = InternalLine::from_string("");
        Self::from_string(line).with_display_text(empty)
    }

    #[cfg(test)]
    /// Set `is_fallback` to true.
    pub fn is_fallback(mut self) -> Self {
        self.is_fallback = true;
        self
    }

    #[cfg(test)]
    /// Set `is_sticky` to true.
    pub fn is_sticky(mut self) -> Self {
        self.is_sticky = true;
        self
    }

    #[cfg(test)]
    /// Set the `display_text` item to the given line.
    pub fn with_condition(mut self, condition: &Condition) -> Self {
        self.set_condition(condition);
        self
    }

    #[cfg(test)]
    /// Set the `display_text` item to the given line.
    pub fn with_display_text(mut self, line: InternalLine) -> Self {
        self.set_display_text(line);
        self
    }

    #[cfg(test)]
    /// Set tags to the choice.
    pub fn with_tags(mut self, tags: &[String]) -> Self {
        self.tags.replace(tags.to_vec());
        self
    }
}
