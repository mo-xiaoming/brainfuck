use std::fs;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub enum SourceFileError {
    FileFailToRead { path: String, reason: String },
}
impl std::fmt::Display for SourceFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceFileError::FileFailToRead { path, reason } => {
                write!(f, "failed to read {}, {}", path, reason)
            }
        }
    }
}
impl std::error::Error for SourceFileError {}

pub fn read_source(path: &str) -> Result<String, SourceFileError> {
    fs::read_to_string(path).map_err(|e| SourceFileError::FileFailToRead {
        path: path.to_owned(),
        reason: e.to_string(),
    })
}

#[derive(Debug)]
pub struct UnicodeChar<'source> {
    pub idx_in_source: usize,
    pub unicode_char: &'source str,
}

type UnicodeChars<'source> = Vec<UnicodeChar<'source>>;

pub fn split_source_to_ucs(content: &str) -> UnicodeChars {
    return UnicodeSegmentation::grapheme_indices(content, true)
        .map(|(idx, uc)| UnicodeChar {
            idx_in_source: idx,
            unicode_char: uc,
        })
        .collect();
}

#[derive(Debug, PartialEq, Eq)]
pub enum Token {
    Increase { idx_in_ucs: usize },
    Decrease { idx_in_ucs: usize },
    Next { idx_in_ucs: usize },
    Prev { idx_in_ucs: usize },
    LoopBegin { idx_in_ucs: usize },
    LoopEnd { idx_in_ucs: usize },
    Read { idx_in_ucs: usize },
    Write { idx_in_ucs: usize },
}

pub fn lex(chars: &[UnicodeChar]) -> Vec<Token> {
    return chars
        .iter()
        .enumerate()
        .flat_map(|(idx, uc)| match uc.unicode_char {
            "." => Some(Token::Write { idx_in_ucs: idx }),
            "," => Some(Token::Read { idx_in_ucs: idx }),
            ">" => Some(Token::Next { idx_in_ucs: idx }),
            "<" => Some(Token::Prev { idx_in_ucs: idx }),
            "+" => Some(Token::Increase { idx_in_ucs: idx }),
            "-" => Some(Token::Decrease { idx_in_ucs: idx }),
            "[" => Some(Token::LoopBegin { idx_in_ucs: idx }),
            "]" => Some(Token::LoopEnd { idx_in_ucs: idx }),
            _ => None,
        })
        .collect();
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn error_when_source_file_does_not_exist() {
        assert!(read_source("I hope it doesn't exist").is_err());
    }

    #[test]
    fn unicode() {
        let content = r#".a̐éö̲.
[+<->]"#;

        let ucs = split_source_to_ucs(content);
        let got = lex(&ucs);
        let expected = vec![
            Token::Write { idx_in_ucs: 0 },
            Token::Write { idx_in_ucs: 4 },
            Token::LoopBegin { idx_in_ucs: 6 },
            Token::Increase { idx_in_ucs: 7 },
            Token::Prev { idx_in_ucs: 8 },
            Token::Decrease { idx_in_ucs: 9 },
            Token::Next { idx_in_ucs: 10 },
            Token::LoopEnd { idx_in_ucs: 11 },
        ];
        assert_eq!(got, expected);
    }
}

#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDocTests;
