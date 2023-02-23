use crate::{
    byte_code::{ByteCode, ByteCodeKind},
    utility::{populate_loop_boundaries, ExtraParen},
};
use smol_str::SmolStr;
use std::path::{Path, PathBuf};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
pub enum UcSourceFileError<'src_file> {
    FileFailToRead {
        path: PathBuf,
        reason: String,
    },
    UnmatchedParen {
        src_file: &'src_file UcSourceFile,
        details: ExtraParen,
    },
}
impl<'src_file> std::fmt::Display for UcSourceFileError<'src_file> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileFailToRead { path, reason } => {
                write!(f, "failed to read {}, {}", path.display(), reason)
            }
            Self::UnmatchedParen {
                src_file: _src_file,
                details: _details,
            } => {
                write!(f, "unmatched paren")
            }
        }
    }
}
impl<'src_file> std::error::Error for UcSourceFileError<'src_file> {}

impl<'a> IntoIterator for &'a UcSourceFile {
    type Item = &'a UcToken;

    type IntoIter = <&'a UcTokens as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.uc_content.iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
pub struct UcToken {
    idx_in_raw: RawContentIndex,
    pub(crate) uc: SmolStr,
}
#[cfg(test)]
fn make_mock_raw_token() -> UcToken {
    UcToken {
        idx_in_raw: RawContentIndex::new(0),
        uc: SmolStr::default(),
    }
}

pub type UcTokens = Vec<UcToken>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
pub(crate) struct RawContentIndex(usize);

impl RawContentIndex {
    pub(crate) fn new(n: usize) -> Self {
        Self(n)
    }
    pub(crate) fn inc_from(self, n: usize) -> Self {
        Self(self.0 + n)
    }
}

impl std::ops::Index<UcContentIndex> for UcSourceFile {
    type Output = UcToken;

    fn index(&self, index: UcContentIndex) -> &Self::Output {
        &self.uc_content[index.0]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
pub(crate) struct UcContentIndex(usize);

impl UcContentIndex {
    pub(crate) fn new(n: usize) -> Self {
        Self(n)
    }
    pub(crate) fn inc_from(self, n: usize) -> Self {
        Self(self.0 + n)
    }
    pub(crate) fn get(&self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
pub struct UcSourceFile {
    filename: PathBuf,
    raw_content: String,
    uc_content: UcTokens,
}
#[cfg(test)]
pub(crate) fn make_mock_src_file() -> UcSourceFile {
    UcSourceFile {
        filename: std::path::PathBuf::new(),
        raw_content: String::new(),
        uc_content: UcTokens::new(),
    }
}

impl UcSourceFile {
    pub fn new<'src_file, P: AsRef<Path>>(path: P) -> Result<Self, UcSourceFileError<'src_file>> {
        let raw =
            std::fs::read_to_string(&path).map_err(|e| UcSourceFileError::FileFailToRead {
                path: path.as_ref().to_path_buf(),
                reason: e.to_string(),
            })?;
        Ok(Self::from_str(raw, path))
    }
    pub(crate) fn from_str<S: AsRef<str>, P: AsRef<Path>>(s: S, pseudo_filename: P) -> Self {
        Self {
            filename: pseudo_filename.as_ref().to_path_buf(),
            raw_content: s.as_ref().to_owned(),
            uc_content: Self::lex(s.as_ref()),
        }
    }

    fn lex<S: AsRef<str>>(raw: S) -> UcTokens {
        UnicodeSegmentation::grapheme_indices(raw.as_ref(), true)
            .map(|(idx, uc)| UcToken {
                idx_in_raw: RawContentIndex::new(idx),
                uc: SmolStr::from(uc),
            })
            .collect()
    }

    /*
        fn get_diagnostics<'src_file>(&'src_file self, range: &'src_file UcSourceFileRange<'src_file>) {
            let start_offset = range.start.offset;
            let end_offset = range.end.offset;
            assert_eq!(end_offset, start_offset + 1); // only one char token in bf
        }
    */
    pub fn to_byte_codes(&self) -> Result<Vec<ByteCode>, UcSourceFileError> {
        let mut byte_codes = Vec::with_capacity(self.len());
        let symbols = std::collections::HashMap::from([
            ("+", ByteCodeKind::IncData),
            ("-", ByteCodeKind::DecData),
            (">", ByteCodeKind::IncPtr),
            ("<", ByteCodeKind::DecPtr),
            (".", ByteCodeKind::Write),
            (",", ByteCodeKind::Read),
        ]);

        let mut idx_in_ucs = UcContentIndex::new(0);
        while idx_in_ucs != UcContentIndex::new(self.len()) {
            let idx_in_raw = self[idx_in_ucs].idx_in_raw;
            let idx_in_ucs_fwd = match self[idx_in_ucs].uc.as_str() {
                s if symbols.contains_key(s) => {
                    // TODO: not happy with exposing uc_content
                    let arg = self.uc_content[idx_in_ucs.0..]
                        .iter()
                        .position(|e| e.uc != s)
                        .unwrap_or(1);
                    byte_codes.push(ByteCode::make_non_jump_code(
                        *symbols.get(s).unwrap(),
                        idx_in_raw,
                        arg,
                    ));
                    arg
                }
                "[" => {
                    byte_codes.push(ByteCode::make_uninit_jump_code(
                        ByteCodeKind::LoopStartJumpIfDataZero,
                        idx_in_raw,
                    ));
                    1
                }
                "]" => {
                    byte_codes.push(ByteCode::make_uninit_jump_code(
                        ByteCodeKind::LoopEndJumpIfDataNotZero,
                        idx_in_raw,
                    ));
                    1
                }
                _ => 1,
            };
            idx_in_ucs = UcContentIndex::inc_from(idx_in_ucs, idx_in_ucs_fwd);
        }

        let loop_matches = populate_loop_boundaries(byte_codes.iter()).map_err(|e| {
            UcSourceFileError::UnmatchedParen {
                src_file: self,
                details: e,
            }
        })?;

        for (idx_in_ucs, bc) in byte_codes.iter_mut().enumerate() {
            match bc.kind {
                ByteCodeKind::LoopStartJumpIfDataZero => bc.correct_jump(UcContentIndex::new(
                    loop_matches.get_matching_end(idx_in_ucs),
                )),
                ByteCodeKind::LoopEndJumpIfDataNotZero => bc.correct_jump(UcContentIndex::new(
                    loop_matches.get_matching_start(idx_in_ucs),
                )),
                _ => (),
            }
        }

        Ok(byte_codes)
    }

    pub fn len(&self) -> usize {
        self.uc_content.len()
    }

    pub fn is_empty(&self) -> bool {
        self.uc_content.is_empty()
    }

    pub(crate) fn at_instr_ptr(&self, instr_ptr: usize) -> &UcToken {
        &self.uc_content[instr_ptr]
    }

    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn traits() {
        use crate::utility::traits::*;

        let src_file_error = UcSourceFileError::FileFailToRead {
            path: PathBuf::from("abc"),
            reason: String::from("xyz"),
        };
        is_big_error(&src_file_error);
        let se = src_file_error.to_string();
        assert!(se.contains("abc") && se.contains("xyz"));

        let src_file = make_mock_src_file();

        is_big_value_struct_but_no_default(&src_file);

        is_big_value_struct_but_no_default(&make_mock_raw_token());

        is_big_value_struct_but_no_default(&make_mock_raw_token());

        is_small_value_struct_but_no_default(&RawContentIndex::new(0));
        is_small_value_struct_but_no_default(&UcContentIndex::new(0));
    }

    #[test]
    fn error_when_source_file_does_not_exist() {
        assert!(UcSourceFile::new("I hope it doesn't exist").is_err());
    }

    #[test]
    fn unicode() {
        let content = r#".a̐éö̲.
[+-]"#;

        let src_file = UcSourceFile::lex(content);
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
        use pretty_assertions_sorted::assert_eq;

        let content = r#"[++-,comment.]
<<<<>"#;

        let src_file = &UcSourceFile::from_str(content, "");
        let byte_code = src_file.to_byte_codes().unwrap();
        assert_eq!(
            byte_code,
            vec![
                {
                    let mut bc = ByteCode::make_uninit_jump_code(
                        ByteCodeKind::LoopStartJumpIfDataZero,
                        RawContentIndex::new(0),
                    );
                    bc.correct_jump(UcContentIndex::new(5));
                    bc
                },
                ByteCode::make_non_jump_code(ByteCodeKind::IncData, RawContentIndex::new(1), 2),
                ByteCode::make_non_jump_code(ByteCodeKind::DecData, RawContentIndex::new(3), 1),
                ByteCode::make_non_jump_code(ByteCodeKind::Read, RawContentIndex::new(4), 1),
                // skip 7 bytes comment
                ByteCode::make_non_jump_code(ByteCodeKind::Write, RawContentIndex::new(12), 1),
                {
                    let mut bc = ByteCode::make_uninit_jump_code(
                        ByteCodeKind::LoopEndJumpIfDataNotZero,
                        RawContentIndex::new(13),
                    );
                    bc.correct_jump(UcContentIndex::new(0));
                    bc
                },
                ByteCode::make_non_jump_code(ByteCodeKind::DecPtr, RawContentIndex::new(15), 4),
                ByteCode::make_non_jump_code(ByteCodeKind::IncPtr, RawContentIndex::new(19), 1),
            ]
        );
    }
}
