use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct Span {
    pub file: String,
    pub start: u32,
    pub end: u32,
    pub line: u32,
    pub col: u32,
}

impl Span {
    pub fn new(file: impl Into<String>, start: u32, end: u32, line: u32, col: u32) -> Self {
        Self {
            file: file.into(),
            start,
            end,
            line,
            col,
        }
    }
}
