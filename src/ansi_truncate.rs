use std::fmt::Display;

use grid_ui::{
    grid::Alignment,
    process::DrawProcess,
    trim::{TrimStrategy, TrimmedText},
};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct AnsiTruncate {
    pub left: &'static str,
    pub right: &'static str,
    pub extra_length: usize,
}
impl AnsiTruncate {
    pub fn new(left: &'static str, right: &'static str) -> AnsiTruncate {
        AnsiTruncate {
            left,
            right,
            extra_length: 0,
        }
    }
}
impl Display for AnsiTruncate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl TrimStrategy for AnsiTruncate {
    type Input = String;
    fn trim(&mut self, text: String, chunk: &DrawProcess, _: Alignment) -> Vec<TrimmedText> {
        let blank_space = " ".graphemes(true).cycle();
        let orig = text
            .graphemes(true)
            .chain(blank_space)
            .take(chunk.width() + self.extra_length)
            .collect::<String>();
        let res = format!(
            "{}{}{}",
            self.left.to_string(),
            orig,
            self.right.to_string()
        );
        self.extra_length = 0; // resets extra length.
        vec![TrimmedText(res)]
    }
    fn back(&mut self, text: Vec<TrimmedText>, _: &DrawProcess, _: Alignment) -> Self::Input {
        text.into_iter().next().expect("Safe unwrap").0
    }
}
