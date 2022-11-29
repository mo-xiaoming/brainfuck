use crate::byte_code::{ByteCode, ByteCodeKind};
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

#[derive(Debug)]
pub struct SourceFileIter<'a> {
    src_file: &'a SourceFile,
    idx_in_src: usize,
}

impl<'a> Iterator for SourceFileIter<'a> {
    type Item = &'a UnicodeChar;
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx_in_src == self.src_file.len() {
            None
        } else {
            self.idx_in_src += 1;
            Some(&self.src_file.content[self.idx_in_src - 1])
        }
    }
}

impl<'a> IntoIterator for &'a SourceFile {
    type Item = &'a UnicodeChar;

    type IntoIter = SourceFileIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            src_file: self,
            idx_in_src: 0,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
pub struct UnicodeChar {
    idx_in_raw: usize,
    length: usize,
}

pub type UnicodeChars = Vec<UnicodeChar>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
pub struct SourceFile {
    filename: PathBuf,
    raw_content: String,
    content: UnicodeChars,
}

fn make_range_for_single_token(
    src_file: &SourceFile,
    row: usize,
    column: usize,
    offset: usize,
) -> (SourceFileLocation<'_>, SourceFileLocation<'_>) {
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

pub(crate) trait LoopCode {
    fn is_loop_start(&self) -> bool;
    fn is_loop_end(&self) -> bool;
}

impl<'a> LoopCode for &'a ByteCode<'a> {
    fn is_loop_start(&self) -> bool {
        self.kind == ByteCodeKind::LoopStartJumpIfDataZero
    }

    fn is_loop_end(&self) -> bool {
        self.kind == ByteCodeKind::LoopEndJumpIfDataNotZero
    }
}

pub(crate) fn populate_byte_codes_loop_boundaries<I>(
    codes: I,
) -> (
    std::collections::HashMap<usize, usize>,
    std::collections::HashMap<usize, usize>,
)
where
    I: Iterator,
    <I as Iterator>::Item: LoopCode,
{
    use std::collections::HashMap;

    let mut start_to_end = HashMap::<usize, usize>::new();
    let mut end_to_start = HashMap::<usize, usize>::new();

    let mut starts = Vec::with_capacity(10);

    for (idx, code) in codes.enumerate() {
        if code.is_loop_start() {
            starts.push(idx);
        } else if code.is_loop_end() {
            let start_idx = starts.pop().unwrap();
            let existed = start_to_end.insert(start_idx, idx);
            assert!(existed.is_none());
            let existed = end_to_start.insert(idx, start_idx);
            assert!(existed.is_none());
        }
    }
    (start_to_end, end_to_start)
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
        fn update<'src_file, 'c>(
            this: &'src_file SourceFile,
            kind: ByteCodeKind,
            row: usize,
            col: &'c mut usize,
            offset: usize,
        ) -> Option<ByteCode<'src_file>> {
            *col += 1;
            Some(ByteCode {
                kind,
                arg: 1,
                range: make_range_for_single_token(this, row, *col - 1, offset),
            })
        }

        let mut row = 0;
        let mut col = 0;
        let mut byte_codes = self
            .iter()
            .flat_map(|uc| match self.get_token(uc) {
                "+" => update(self, ByteCodeKind::IncData, row, &mut col, uc.idx_in_raw),
                "-" => update(self, ByteCodeKind::DecData, row, &mut col, uc.idx_in_raw),
                ">" => update(self, ByteCodeKind::IncPtr, row, &mut col, uc.idx_in_raw),
                "<" => update(self, ByteCodeKind::DecPtr, row, &mut col, uc.idx_in_raw),
                "[" => update(
                    self,
                    ByteCodeKind::LoopStartJumpIfDataZero,
                    row,
                    &mut col,
                    uc.idx_in_raw,
                ),
                "]" => update(
                    self,
                    ByteCodeKind::LoopEndJumpIfDataNotZero,
                    row,
                    &mut col,
                    uc.idx_in_raw,
                ),
                "." => update(self, ByteCodeKind::Write, row, &mut col, uc.idx_in_raw),
                "," => update(self, ByteCodeKind::Read, row, &mut col, uc.idx_in_raw),
                "\n" | "\r\n" => {
                    row += 1;
                    col = 0;
                    None
                }
                _ => {
                    col += 1;
                    None
                }
            })
            .collect::<Vec<_>>();

        let (start_to_end, end_to_start) = populate_byte_codes_loop_boundaries(byte_codes.iter());
        for (
            i,
            ByteCode {
                kind, ref mut arg, ..
            },
        ) in byte_codes.iter_mut().enumerate()
        {
            match kind {
                ByteCodeKind::LoopStartJumpIfDataZero => *arg = *start_to_end.get(&i).unwrap(),
                ByteCodeKind::LoopEndJumpIfDataNotZero => *arg = *end_to_start.get(&i).unwrap(),
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

    pub(crate) fn at_instr_ptr(&self, instr_ptr: usize) -> &UnicodeChar {
        &self.content[instr_ptr]
    }

    pub(crate) fn get_token(&self, uc: &UnicodeChar) -> &str {
        &self.raw_content[uc.idx_in_raw..uc.idx_in_raw + uc.length]
    }

    pub fn lex<S: AsRef<str>>(raw: S) -> UnicodeChars {
        UnicodeSegmentation::grapheme_indices(raw.as_ref(), true)
            .map(|(idx, uc)| UnicodeChar {
                idx_in_raw: idx,
                length: uc.len(),
            })
            .collect()
    }

    pub fn iter(&self) -> SourceFileIter<'_> {
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
mod test {
    use super::*;
    use crate::source_file::SourceFile;
    use pretty_assertions_sorted::assert_eq;

    #[test]
    fn traits() {
        use crate::utility::traits::*;

        let src_file_error = SourceFileError::FileFailToRead {
            path: PathBuf::default(),
            reason: String::default(),
        };
        is_big_value_enum(&src_file_error);

        let src_file = SourceFile {
            filename: std::path::PathBuf::new(),
            raw_content: String::new(),
            content: UnicodeChars::default(),
        };

        is_big_value_struct_but_no_default(&src_file);

        is_big_value_struct(&UnicodeChar::default());

        is_big_value_struct(&UnicodeChars::default());

        is_debug(&SourceFileIter {
            src_file: &src_file,
            idx_in_src: 0,
        });

        is_small_value_struct_but_no_default(&SourceFileLocation {
            src_file: &src_file,
            row: 0,
            column: 0,
            offset: 0,
        });
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
            acc.push_str(&content[x.idx_in_raw..x.idx_in_raw + x.length]);
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
                    range: make_range_for_single_token(&src_file, 0, 0, 0)
                },
                ByteCode {
                    kind: ByteCodeKind::IncData,
                    arg: 1,
                    range: make_range_for_single_token(&src_file, 0, 1, 1)
                },
                ByteCode {
                    kind: ByteCodeKind::DecData,
                    arg: 1,
                    range: make_range_for_single_token(&src_file, 0, 2, 2)
                },
                ByteCode {
                    kind: ByteCodeKind::Read,
                    arg: 1,
                    range: make_range_for_single_token(&src_file, 0, 3, 3)
                },
                ByteCode {
                    kind: ByteCodeKind::Write,
                    arg: 1,
                    range: make_range_for_single_token(&src_file, 0, 11, 11) // skip 7 bytes comment
                },
                ByteCode {
                    kind: ByteCodeKind::LoopEndJumpIfDataNotZero,
                    arg: 0,
                    range: make_range_for_single_token(&src_file, 0, 12, 12)
                },
                ByteCode {
                    kind: ByteCodeKind::DecPtr,
                    arg: 1,
                    range: make_range_for_single_token(&src_file, 1, 0, 14) // offset 6 is new line
                },
                ByteCode {
                    kind: ByteCodeKind::IncPtr,
                    arg: 1,
                    range: make_range_for_single_token(&src_file, 1, 1, 15)
                },
            ]
        );
    }
}
