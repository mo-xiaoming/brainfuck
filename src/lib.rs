#![warn(future_incompatible)]
#![warn(rust_2021_compatibility)]
#![warn(missing_debug_implementations)]
#![forbid(overflowing_literals)]

mod utility;

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

pub trait MachineIO {
    fn out_char(&mut self, c: char);
    fn in_char(&mut self) -> char;
    fn flush_all(&mut self);
}

#[derive(Debug)]
pub struct DefaultMachineIO {
    term: console::Term,
}

impl Default for DefaultMachineIO {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultMachineIO {
    fn new() -> Self {
        Self {
            term: console::Term::stdout(),
        }
    }
}

impl MachineIO for DefaultMachineIO {
    fn out_char(&mut self, c: char) {
        print!("{}", c);
    }

    fn in_char(&mut self) -> char {
        self.term.read_char().unwrap()
    }

    fn flush_all(&mut self) {}
}

/// support cell value wrapping and data pointer moving left from the initial point
#[derive(Debug)]
pub struct Machine<IO> {
    cells: Vec<u8>,
    data_ptr: usize,
    instr_ptr: usize,
    io: IO,
}

impl<IO: MachineIO> Machine<IO> {
    pub fn with_io(cell_size: usize, io: IO) -> Self {
        Self {
            cells: vec![0; cell_size],
            data_ptr: Self::reset_data_ptr(cell_size),
            instr_ptr: Self::reset_instr_ptr(),
            io,
        }
    }

    fn reset_data_ptr(cell_size: usize) -> usize {
        cell_size / 2
    }
    fn reset_instr_ptr() -> usize {
        0
    }

    /// must be called between different `eval_` calls,
    /// otherwise the behavior is undefined
    pub fn reset(&mut self) {
        self.cells.iter_mut().for_each(|e| *e = 0);
        self.data_ptr = Self::reset_data_ptr(self.cells.len());
        self.instr_ptr = Self::reset_instr_ptr();
        self.io.flush_all();
    }

    /// . Output the byte at the data pointer.
    fn print(&mut self) {
        self.io
            .out_char(*self.cells.get(self.data_ptr).unwrap() as char);
        self.instr_ptr += 1;
    }

    /// , Accept one byte of input, storing its value in the byte at the data pointer.
    fn read(&mut self) {
        *self.cells.get_mut(self.data_ptr).unwrap() = self.io.in_char() as u8;
        self.instr_ptr += 1;
    }

    /// > Increment the data pointer (to point to the next cell to the right).
    fn inc_data_ptr(&mut self) {
        self.data_ptr = self.data_ptr.wrapping_add(1);
        self.instr_ptr += 1;
    }

    /// < Decrement the data pointer (to point to the next cell to the left).
    fn dec_data_ptr(&mut self) {
        self.data_ptr = self.data_ptr.wrapping_sub(1);
        self.instr_ptr += 1;
    }

    /// + Increment (increase by one) the byte at the data pointer.
    fn inc_cell_value(&mut self) {
        *self.cells.get_mut(self.data_ptr).unwrap() =
            self.cells.get(self.data_ptr).unwrap().wrapping_add(1);
        self.instr_ptr += 1;
    }

    /// - Decrement (decrease by one) the byte at the data pointer.
    fn dec_cell_value(&mut self) {
        *self.cells.get_mut(self.data_ptr).unwrap() =
            self.cells.get(self.data_ptr).unwrap().wrapping_sub(1);
        self.instr_ptr += 1;
    }

    /// [ If the byte at the data pointer is zero, then instead of moving
    ///    the instruction pointer forward to the next command, jump it
    ///    forward to the command after the matching ] command.
    fn start_loop(&mut self, end_ptr: usize) {
        if *self.cells.get(self.data_ptr).unwrap() == 0 {
            self.instr_ptr = end_ptr + 1;
        } else {
            self.instr_ptr += 1;
        }
    }

    /// ] If the byte at the data pointer is nonzero, then instead of
    ///   moving the instruction pointer forward to the next command,
    ///   jump it back to the command after the matching [ command.
    fn end_loop(&mut self, start_ptr: usize) {
        if *self.cells.get(self.data_ptr).unwrap() != 0 {
            self.instr_ptr = start_ptr;
        } else {
            self.instr_ptr += 1;
        }
    }

    pub fn eval_source_file(&mut self, src_file: &SourceFile) {
        use std::collections::HashMap;

        let (start_to_end, end_to_start) = {
            let mut start_to_end = HashMap::<usize, usize>::new();
            let mut end_to_start = HashMap::<usize, usize>::new();

            let mut starts = Vec::with_capacity(10);

            for (idx_in_ucs, uc) in src_file.content.iter().enumerate() {
                let token = src_file.get_token(uc);
                if token == "[" {
                    starts.push(idx_in_ucs);
                } else if token == "]" {
                    let start_idx = starts.pop().unwrap();
                    let existed = start_to_end.insert(start_idx, idx_in_ucs);
                    assert!(existed.is_none());
                    let existed = end_to_start.insert(idx_in_ucs, start_idx);
                    assert!(existed.is_none());
                }
            }

            (start_to_end, end_to_start)
        };

        while self.instr_ptr < src_file.content.len() {
            match src_file.get_token(&src_file.content[self.instr_ptr]) {
                "." => self.print(),
                "," => self.read(),
                ">" => self.inc_data_ptr(),
                "<" => self.dec_data_ptr(),
                "+" => self.inc_cell_value(),
                "-" => self.dec_cell_value(),
                "[" => self.start_loop(*start_to_end.get(&self.instr_ptr).unwrap()),
                "]" => self.end_loop(*end_to_start.get(&self.instr_ptr).unwrap()),
                _ => self.instr_ptr += 1,
            }
        }
    }

    pub fn eval_byte_codes(&mut self, byte_codes: &Vec<ByteCode>) {
        use std::collections::HashMap;

        let (start_to_end, end_to_start) = {
            let mut start_to_end = HashMap::<usize, usize>::new();
            let mut end_to_start = HashMap::<usize, usize>::new();

            let mut starts = Vec::with_capacity(10);

            for (idx, code) in byte_codes.iter().enumerate() {
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
        };

        while self.instr_ptr < byte_codes.len() {
            match byte_codes[self.instr_ptr].kind {
                ByteCodeKind::Write => self.print(),
                ByteCodeKind::Read => self.read(),
                ByteCodeKind::IncPtr => self.inc_data_ptr(),
                ByteCodeKind::DecPtr => self.dec_data_ptr(),
                ByteCodeKind::IncData => self.inc_cell_value(),
                ByteCodeKind::DecData => self.dec_cell_value(),
                ByteCodeKind::JumpIfDataZero => {
                    self.start_loop(*start_to_end.get(&self.instr_ptr).unwrap())
                }
                ByteCodeKind::JumpIfDataNotZero => {
                    self.end_loop(*end_to_start.get(&self.instr_ptr).unwrap())
                }
            }
        }
    }
}

pub fn create_default_machine() -> Machine<DefaultMachineIO> {
    let io = DefaultMachineIO::new();
    Machine::<DefaultMachineIO>::with_io(60_000, io)
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
pub struct SourceFile {
    filename: std::path::PathBuf,
    raw_content: String,
    content: UnicodeChars,
}

fn make_range_for_single_token(
    row: usize,
    column: usize,
    offset: usize,
) -> (SourceFileLocation, SourceFileLocation) {
    (
        SourceFileLocation {
            row,
            column,
            offset,
        },
        SourceFileLocation {
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
    fn from_str<S: AsRef<str>, P: AsRef<Path>>(
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
        fn update(
            kind: ByteCodeKind,
            row: usize,
            col: &mut usize,
            offset: usize,
        ) -> Option<ByteCode> {
            *col += 1;
            Some(ByteCode {
                kind,
                arg: 1,
                range: make_range_for_single_token(row, *col - 1, offset),
            })
        }

        let mut row = 0;
        let mut col = 0;
        self.content
            .iter()
            .flat_map(|uc| match self.get_token(uc) {
                "+" => update(ByteCodeKind::IncData, row, &mut col, uc.idx_in_raw),
                "-" => update(ByteCodeKind::DecData, row, &mut col, uc.idx_in_raw),
                ">" => update(ByteCodeKind::IncPtr, row, &mut col, uc.idx_in_raw),
                "<" => update(ByteCodeKind::DecPtr, row, &mut col, uc.idx_in_raw),
                "[" => update(ByteCodeKind::JumpIfDataZero, row, &mut col, uc.idx_in_raw),
                "]" => update(
                    ByteCodeKind::JumpIfDataNotZero,
                    row,
                    &mut col,
                    uc.idx_in_raw,
                ),
                "." => update(ByteCodeKind::Write, row, &mut col, uc.idx_in_raw),
                "," => update(ByteCodeKind::Read, row, &mut col, uc.idx_in_raw),
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
            .collect()
    }

    pub fn len(&self) -> usize {
        self.content.len()
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    fn get_token(&self, uc: &UnicodeChar) -> &str {
        &self.raw_content[uc.idx_in_raw..uc.idx_in_raw + uc.length]
    }

    fn lex<S: AsRef<str>>(raw: S) -> UnicodeChars {
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

type UnicodeChars = Vec<UnicodeChar>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
enum ByteCodeKind {
    IncPtr,
    DecPtr,
    IncData,
    DecData,
    Read,
    Write,
    JumpIfDataZero,
    JumpIfDataNotZero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
pub struct ByteCode {
    kind: ByteCodeKind,
    arg: u32,
    range: (SourceFileLocation, SourceFileLocation),
}

#[allow(dead_code)]
impl ByteCode {
    fn is_inc_ptr(&self) -> bool {
        self.kind == ByteCodeKind::IncPtr
    }
    fn is_dec_ptr(&self) -> bool {
        self.kind == ByteCodeKind::DecPtr
    }
    fn is_inc_data(&self) -> bool {
        self.kind == ByteCodeKind::IncData
    }
    fn is_dec_data(&self) -> bool {
        self.kind == ByteCodeKind::DecData
    }
    fn is_read(&self) -> bool {
        self.kind == ByteCodeKind::Read
    }
    fn is_write(&self) -> bool {
        self.kind == ByteCodeKind::Write
    }
    fn is_loop_start(&self) -> bool {
        self.kind == ByteCodeKind::JumpIfDataZero
    }
    fn is_loop_end(&self) -> bool {
        self.kind == ByteCodeKind::JumpIfDataNotZero
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
struct SourceFileLocation {
    row: usize,
    column: usize,
    offset: usize,
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions_sorted::assert_eq;

    #[test]
    fn traits() {
        use crate::utility::traits::*;

        is_default_debug(&DefaultMachineIO::default());

        let src_file_error = SourceFileError::FileFailToRead {
            path: PathBuf::default(),
            reason: String::default(),
        };
        is_big_value_enum(&src_file_error);

        is_big_value_struct(&SourceFile::default());

        is_debug(&create_default_machine());

        is_big_value_struct(&UnicodeChar::default());

        is_big_value_struct(&UnicodeChars::default());

        is_debug(&SourceFileIter {
            src_file: &SourceFile::default(),
            idx_in_src: 0,
        });

        is_small_value_enum(&ByteCodeKind::DecData);

        is_small_value_struct(&SourceFileLocation::default());
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
    fn byte_code() {
        let content = r#"[+-,comment.]
<>"#;

        let src_file = SourceFile::from_str(content, "").unwrap();
        let byte_code = src_file.to_byte_codes();
        assert_eq!(
            byte_code,
            vec![
                ByteCode {
                    kind: ByteCodeKind::JumpIfDataZero,
                    arg: 1,
                    range: make_range_for_single_token(0, 0, 0)
                },
                ByteCode {
                    kind: ByteCodeKind::IncData,
                    arg: 1,
                    range: make_range_for_single_token(0, 1, 1)
                },
                ByteCode {
                    kind: ByteCodeKind::DecData,
                    arg: 1,
                    range: make_range_for_single_token(0, 2, 2)
                },
                ByteCode {
                    kind: ByteCodeKind::Read,
                    arg: 1,
                    range: make_range_for_single_token(0, 3, 3)
                },
                ByteCode {
                    kind: ByteCodeKind::Write,
                    arg: 1,
                    range: make_range_for_single_token(0, 11, 11) // skip 7 bytes comment
                },
                ByteCode {
                    kind: ByteCodeKind::JumpIfDataNotZero,
                    arg: 1,
                    range: make_range_for_single_token(0, 12, 12)
                },
                ByteCode {
                    kind: ByteCodeKind::DecPtr,
                    arg: 1,
                    range: make_range_for_single_token(1, 0, 14) // offset 6 is new line
                },
                ByteCode {
                    kind: ByteCodeKind::IncPtr,
                    arg: 1,
                    range: make_range_for_single_token(1, 1, 15)
                },
            ]
        );
    }

    #[test]
    fn is_kind() {
        let mut code = ByteCode {
            kind: ByteCodeKind::IncData,
            arg: 1,
            range: (SourceFileLocation::default(), SourceFileLocation::default()),
        };

        code.kind = ByteCodeKind::DecData;
        assert!(code.is_dec_data());
        code.kind = ByteCodeKind::IncData;
        assert!(code.is_inc_data());
        code.kind = ByteCodeKind::DecPtr;
        assert!(code.is_dec_ptr());
        code.kind = ByteCodeKind::IncPtr;
        assert!(code.is_inc_ptr());
        code.kind = ByteCodeKind::Write;
        assert!(code.is_write());
        code.kind = ByteCodeKind::Read;
        assert!(code.is_read());
        code.kind = ByteCodeKind::JumpIfDataZero;
        assert!(code.is_loop_start());
        code.kind = ByteCodeKind::JumpIfDataNotZero;
        assert!(code.is_loop_end());
    }
}

#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDocTests;
