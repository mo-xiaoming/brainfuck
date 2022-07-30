use std::fs;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct Lexer<'source> {
    chars: Vec<(usize, &'source str)>,
}

#[derive(Debug)]
pub enum LexerError {
    FileFailToRead { path: String },
}
impl std::fmt::Display for LexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for LexerError {}

pub fn read_source(path: &str) -> Result<String, LexerError> {
    fs::read_to_string(path).or_else(|_| {
        Err(LexerError::FileFailToRead {
            path: path.to_owned(),
        })
    })
}

#[derive(Debug, PartialEq, Eq)]
pub enum Token {
    Increase,
    Decrease,
    Next,
    Prev,
    Loop,
    Read,
    Write,
}

impl<'source> Lexer<'source> {
    pub fn new(content: &'source str) -> Self {
        Self {
            chars: UnicodeSegmentation::grapheme_indices(content, true)
                .collect::<Vec<(usize, &str)>>(),
        }
    }

    pub fn next_token(&mut self) -> Option<Token> {
        return None;
    }
}

#[cfg(test)]
mod test {
    use super::read_source;

    #[test]
    fn error_when_source_file_does_not_exist() {
        assert!(read_source("I hope it doesn't exist").is_err());
    }
}

#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDocTests;
