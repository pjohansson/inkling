use crate::follow::*;
use crate::line::*;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
pub struct LineChunk {
    pub conditions: Vec<Condition>,
    pub items: Vec<Content>,
}

impl Process for LineChunk {
    fn process(&mut self, buffer: &mut String) -> Result<Next, ProcessError> {
        for item in self.items.iter_mut() {
            let result = item.process(buffer)?;

            if let Next::Divert(..) = result {
                return Ok(result);
            }
        }

        Ok(Next::Done)
    }
}

pub struct LineBuilder {
    items: Vec<Content>,
}

impl LineBuilder {
    pub fn new() -> Self {
        LineBuilder { items: Vec::new() }
    }

    pub fn build(self) -> LineChunk {
        LineChunk {
            conditions: Vec::new(),
            items: self.items,
        }
    }

    pub fn add_pure_text(&mut self, text: &str) {
        self.add_item(Content::PureText(text.to_string()));
    }

    pub fn add_divert(&mut self, address: &str) {
        self.add_item(Content::Divert(address.to_string()));
    }

    pub fn add_item(&mut self, item: Content) {
        self.items.push(item);
    }

    pub fn with_divert(mut self, address: &str) -> Self {
        self.with_item(Content::Divert(address.to_string()))
    }

    pub fn with_item(mut self, item: Content) -> Self {
        self.items.push(item);
        self
    }

    pub fn with_pure_text(mut self, text: &str) -> Self {
        self.with_item(Content::PureText(text.to_string()))
    }

    // #[cfg(test)]
    pub fn with_text(mut self, text: &str) -> Result<Self, LineParsingError> {
        let chunk = parse_chunk(text)?;

        for item in chunk.items {
            self.add_item(item);
        }

        Ok(self)
    }
}
