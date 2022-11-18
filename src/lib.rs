#![warn(future_incompatible)]
#![warn(rust_2021_compatibility)]
#![warn(missing_debug_implementations)]
#![forbid(overflowing_literals)]

use std::path::{Path, PathBuf};
use unicode_segmentation::UnicodeSegmentation;
mod utility;

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
            data_ptr: cell_size / 2,
            instr_ptr: 0,
            io,
        }
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
        self.data_ptr = self.data_ptr.wrapping_add(1); //.unwrap();
        self.instr_ptr += 1;
    }

    /// < Decrement the data pointer (to point to the next cell to the left).
    fn dec_data_ptr(&mut self) {
        self.data_ptr = self.data_ptr.wrapping_sub(1); //.unwrap();
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

    pub fn eval(&mut self, src_file: &SourceFile) {
        use std::collections::HashMap;

        let (start_to_end, end_to_start) = {
            let mut start_to_end = HashMap::<usize, usize>::new();
            let mut end_to_start = HashMap::<usize, usize>::new();

            let mut starts = Vec::with_capacity(10);

            for (idx_in_ucs, UnicodeChar { unicode, .. }) in src_file.content.iter().enumerate() {
                if unicode == "[" {
                    starts.push(idx_in_ucs);
                } else if unicode == "]" {
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
            match src_file.content[self.instr_ptr].unicode.as_ref() {
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
}

pub fn create_default_machine() -> Machine<DefaultMachineIO> {
    let io = DefaultMachineIO::new();
    Machine::<DefaultMachineIO>::with_io(60_000, io)
}

#[derive(Debug, Default)]
pub struct SourceFile {
    content: UnicodeChars,
}

impl SourceFile {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, SourceFileError> {
        let raw = std::fs::read_to_string(&path).map_err(|e| SourceFileError::FileFailToRead {
            path: path.as_ref().to_path_buf(),
            reason: e.to_string(),
        })?;
        Ok(Self::from_content(raw))
    }

    pub fn len(&self) -> usize {
        self.content.len()
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub fn from_content<S: AsRef<str>>(content: S) -> Self {
        Self {
            content: UnicodeSegmentation::grapheme_indices(content.as_ref(), true)
                .map(|(idx, uc)| UnicodeChar {
                    idx_in_raw: idx,
                    unicode: uc.to_owned(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Default)]
pub struct UnicodeChar {
    pub idx_in_raw: usize,
    pub unicode: String,
}

type UnicodeChars = Vec<UnicodeChar>;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn traits() {
        use crate::utility::traits::*;

        is_default_debug(&DefaultMachineIO::default());

        let src_file_error = SourceFileError::FileFailToRead {
            path: PathBuf::default(),
            reason: String::default(),
        };
        is_big_value_enum(&src_file_error);

        is_default_debug(&SourceFile::default());

        is_debug(&create_default_machine());

        is_default_debug(&SourceFile::default());

        is_default_debug(&UnicodeChar::default());

        is_default_debug(&UnicodeChars::default());
    }

    #[test]
    fn error_when_source_file_does_not_exist() {
        assert!(SourceFile::from_file("I hope it doesn't exist").is_err());
    }

    #[test]
    fn unicode() {
        let content = r#".a̐éö̲.
[+-]"#;

        let src_file = SourceFile::from_content(content);
        assert_eq!(10, src_file.len());
    }
}

#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDocTests;
