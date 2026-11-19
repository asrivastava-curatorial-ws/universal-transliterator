#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;

pub const MAX_COMPOSITION_BYTES: usize = 4096;
pub const MAX_CANDIDATES: usize = 64;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    pub value: String,
    pub selected: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineSnapshot {
    pub preedit: String,
    pub raw_input: String,
    pub cursor: usize,
    pub segments: Vec<Segment>,
    pub candidates: Vec<String>,
    pub highlighted: usize,
    pub page: usize,
    pub page_size: usize,
    pub committed: Option<String>,
    pub composing: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineKey {
    Char(char),
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    PageUp,
    PageDown,
    Space,
    Enter,
    Escape,
    Candidate(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineError {
    message: String,
}

impl EngineError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn public_message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for EngineError {}

pub trait CompositionEngine {
    fn name(&self) -> &'static str;
    fn reset(&mut self) -> Result<EngineSnapshot, EngineError>;
    fn process(&mut self, key: EngineKey) -> Result<EngineSnapshot, EngineError>;
    fn snapshot(&mut self) -> Result<EngineSnapshot, EngineError>;
}

pub fn bounded_candidates<I>(values: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    values.into_iter().take(MAX_CANDIDATES).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_limit_is_enforced() {
        let values = (0..100).map(|n| n.to_string());
        assert_eq!(bounded_candidates(values).len(), MAX_CANDIDATES);
    }
}
