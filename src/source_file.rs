use crate::{
    byte_code::{ByteCode, ByteCodeKind},
    utility::populate_loop_boundaries,
};
use smol_str::SmolStr;
use std::path::{Path, PathBuf};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
pub enum SourceFileError {
    FileFailToRead { path: PathBuf, reason: String },
}
impl std::fmt::Display for SourceFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceFileError::FileFailToRead { path, reason } => {
                write!(f, "failed to read {}, {}", path.display(), reason)
            }
        }
    }
}
impl std::error::Error for SourceFileError {}

impl<'a> IntoIterator for &'a SourceFile {
    type Item = &'a RawToken;

    type IntoIter = <&'a RawTokens as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.content.iter()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
pub struct RawToken {
    idx_in_raw: usize,
    pub(crate) uc: SmolStr,
}

pub type RawTokens = Vec<RawToken>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
pub struct SourceFile {
    filename: PathBuf,
    raw_content: String,
    content: RawTokens,
}
#[cfg(test)]
pub(crate) fn make_mock_src_file() -> SourceFile {
    SourceFile {
        filename: std::path::PathBuf::new(),
        raw_content: String::new(),
        content: RawTokens::new(),
    }
}

fn make_range_for_token(
    src_file: &SourceFile,
    row: usize,
    column: usize,
    offset: usize,
) -> (SourceFileLocation, SourceFileLocation) {
    (
        SourceFileLocation {
            src_file,
            row,
            column,
            offset,
        },
        SourceFileLocation {
            src_file,
            row,
            column: column + 1,
            offset: offset + 1,
        },
    )
}

impl SourceFile {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, SourceFileError> {
        let raw = std::fs::read_to_string(&path).map_err(|e| SourceFileError::FileFailToRead {
            path: path.as_ref().to_path_buf(),
            reason: e.to_string(),
        })?;
        Self::from_str(raw, path)
    }
    pub(crate) fn from_str<S: AsRef<str>, P: AsRef<Path>>(
        s: S,
        pseudo_filename: P,
    ) -> Result<Self, SourceFileError> {
        let content = Self::lex(s.as_ref());
        Ok(Self {
            filename: pseudo_filename.as_ref().to_path_buf(),
            raw_content: s.as_ref().to_owned(),
            content,
        })
    }

    pub fn to_byte_codes(&self) -> Vec<ByteCode> {
        fn make_byte_code<'src_file, 'c>(
            this: &'src_file SourceFile,
            arg: usize,
            kind: ByteCodeKind,
            row: usize,
            col: &'c mut usize,
            offset: usize,
        ) -> ByteCode<'src_file> {
            *col += arg;
            ByteCode {
                kind,
                arg,
                range: make_range_for_token(this, row, *col - arg, offset),
            }
        }

        let mut row = 0;
        let mut col = 0;
        let mut idx = 0;
        let mut byte_codes = Vec::with_capacity(self.len());
        let symbols = std::collections::HashMap::from([
            ("+", ByteCodeKind::IncData),
            ("-", ByteCodeKind::DecData),
            (">", ByteCodeKind::IncPtr),
            ("<", ByteCodeKind::DecPtr),
            (".", ByteCodeKind::Write),
            (",", ByteCodeKind::Read),
        ]);
        while idx < self.content.len() {
            let cur_offset = self.content[idx].idx_in_raw;
            match self.content[idx].uc.as_str() {
                s if symbols.contains_key(s) => {
                    let arg = self.content[idx..]
                        .iter()
                        .position(|e| e.uc != s)
                        .unwrap_or(1);
                    byte_codes.push(make_byte_code(
                        self,
                        arg,
                        *symbols.get(s).unwrap(),
                        row,
                        &mut col,
                        cur_offset,
                    ));
                    idx += arg;
                }
                "[" => {
                    byte_codes.push(make_byte_code(
                        self,
                        1,
                        ByteCodeKind::LoopStartJumpIfDataZero,
                        row,
                        &mut col,
                        cur_offset,
                    ));
                    idx += 1;
                }
                "]" => {
                    byte_codes.push(make_byte_code(
                        self,
                        1,
                        ByteCodeKind::LoopEndJumpIfDataNotZero,
                        row,
                        &mut col,
                        cur_offset,
                    ));
                    idx += 1;
                }
                "\r\n" | "\n" => {
                    row += 1;
                    col = 0;
                    idx += 1;
                }
                _ => {
                    col += 1;
                    idx += 1;
                }
            }
        }

        let loop_matches = populate_loop_boundaries(byte_codes.iter()).unwrap();

        for (
            i,
            ByteCode {
                kind, ref mut arg, ..
            },
        ) in byte_codes.iter_mut().enumerate()
        {
            match kind {
                ByteCodeKind::LoopStartJumpIfDataZero => *arg = loop_matches.get_matching_end(i),
                ByteCodeKind::LoopEndJumpIfDataNotZero => *arg = loop_matches.get_matching_start(i),
                _ => (),
            }
        }

        byte_codes
    }

    pub fn len(&self) -> usize {
        self.content.len()
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub(crate) fn at_instr_ptr(&self, instr_ptr: usize) -> &RawToken {
        &self.content[instr_ptr]
    }

    pub fn lex<S: AsRef<str>>(raw: S) -> RawTokens {
        UnicodeSegmentation::grapheme_indices(raw.as_ref(), true)
            .map(|(idx, uc)| RawToken {
                idx_in_raw: idx,
                uc: SmolStr::from(uc),
            })
            .collect()
    }

    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
pub(crate) struct SourceFileLocation<'src_file> {
    src_file: &'src_file SourceFile,
    row: usize,
    column: usize,
    offset: usize,
}
#[cfg(test)]
pub(crate) fn make_mock_src_file_loc(src_file: &SourceFile) -> SourceFileLocation {
    SourceFileLocation {
        src_file,
        row: 0,
        column: 0,
        offset: 0,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn traits() {
        use crate::utility::traits::*;

        let src_file_error = SourceFileError::FileFailToRead {
            path: PathBuf::from("abc"),
            reason: String::from("xyz"),
        };
        is_big_value_enum(&src_file_error);
        is_display(&src_file_error);
        let se = src_file_error.to_string();
        assert!(se.contains("abc") && se.contains("xyz"));

        let src_file = make_mock_src_file();

        is_big_value_struct_but_no_default(&src_file);

        is_big_value_struct(&RawToken::default());

        is_big_value_struct(&RawTokens::default());

        is_small_value_struct_but_no_default(&make_mock_src_file_loc(&src_file));
    }

    #[test]
    fn error_when_source_file_does_not_exist() {
        assert!(SourceFile::new("I hope it doesn't exist").is_err());
    }

    #[test]
    fn unicode() {
        let content = r#".a̐éö̲.
[+-]"#;

        let src_file = SourceFile::lex(content);
        assert_eq!(10, src_file.len());
        let mut s = String::with_capacity(20);
        src_file.iter().fold(&mut s, |acc, x| {
            acc.push_str(x.uc.as_str());
            acc
        });
        assert_eq!(s, content);
    }

    #[test]
    fn src_file_to_byte_codes() {
        let content = r#"[+-,comment.]
<>"#;

        let src_file = SourceFile::from_str(content, "").unwrap();
        let byte_code = src_file.to_byte_codes();
        assert_eq!(
            byte_code,
            vec![
                ByteCode {
                    kind: ByteCodeKind::LoopStartJumpIfDataZero,
                    arg: 5,
                    range: make_range_for_token(&src_file, 0, 0, 0)
                },
                ByteCode {
                    kind: ByteCodeKind::IncData,
                    arg: 1,
                    range: make_range_for_token(&src_file, 0, 1, 1)
                },
                ByteCode {
                    kind: ByteCodeKind::DecData,
                    arg: 1,
                    range: make_range_for_token(&src_file, 0, 2, 2)
                },
                ByteCode {
                    kind: ByteCodeKind::Read,
                    arg: 1,
                    range: make_range_for_token(&src_file, 0, 3, 3)
                },
                ByteCode {
                    kind: ByteCodeKind::Write,
                    arg: 1,
                    range: make_range_for_token(&src_file, 0, 11, 11) // skip 7 bytes comment
                },
                ByteCode {
                    kind: ByteCodeKind::LoopEndJumpIfDataNotZero,
                    arg: 0,
                    range: make_range_for_token(&src_file, 0, 12, 12)
                },
                ByteCode {
                    kind: ByteCodeKind::DecPtr,
                    arg: 1,
                    range: make_range_for_token(&src_file, 1, 0, 14) // offset 6 is new line
                },
                ByteCode {
                    kind: ByteCodeKind::IncPtr,
                    arg: 1,
                    range: make_range_for_token(&src_file, 1, 1, 15)
                },
            ]
        );
    }
}
