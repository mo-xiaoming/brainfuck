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
}

#[derive(Debug)]
pub struct DefaultMachineIO {
    term: console::Term,
}

#[allow(clippy::new_without_default)]
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
}

#[derive(Debug)]
pub struct Machine<IO> {
    tape: Vec<u8>,
    mem_ptr: usize,
    idx_in_src: usize,
    io: IO,
    loop_starts: Vec<usize>,
}

impl<IO: MachineIO> Machine<IO> {
    pub fn with_io(io: IO) -> Self {
        Self {
            tape: vec![0; 30_000],
            mem_ptr: 0,
            idx_in_src: 0,
            io,
            loop_starts: vec![],
        }
    }

    fn print(&mut self) -> usize {
        self.io
            .out_char(*self.tape.get(self.mem_ptr).unwrap() as char);
        self.idx_in_src += 1;
        self.idx_in_src
    }

    fn read(&mut self) -> usize {
        *self.tape.get_mut(self.mem_ptr).unwrap() = self.io.in_char() as u8;
        self.idx_in_src += 1;
        self.idx_in_src
    }

    fn next_mem_slot(&mut self) -> usize {
        self.mem_ptr = self.mem_ptr.checked_add(1).unwrap();
        self.idx_in_src += 1;
        self.idx_in_src
    }

    fn prev_mem_slot(&mut self) -> usize {
        self.mem_ptr = self.mem_ptr.checked_sub(1).unwrap();
        self.idx_in_src += 1;
        self.idx_in_src
    }

    fn inc_mem_value(&mut self) -> usize {
        *self.tape.get_mut(self.mem_ptr).unwrap() = self
            .tape
            .get(self.mem_ptr)
            .unwrap()
            .checked_add(1)
            .unwrap_or_else(|| {
                panic!(
                    "at {} on value {} add 1 failed, pc: {}",
                    self.mem_ptr, self.tape[self.mem_ptr], self.idx_in_src
                );
            });
        self.idx_in_src += 1;
        self.idx_in_src
    }

    fn dec_mem_value(&mut self) -> usize {
        *self.tape.get_mut(self.mem_ptr).unwrap() = self
            .tape
            .get(self.mem_ptr)
            .unwrap()
            .checked_sub(1)
            .unwrap_or_else(|| {
                panic!(
                    "at {} on value {} sub 1 failed, pc: {}",
                    self.mem_ptr, self.tape[self.mem_ptr], self.idx_in_src
                );
            });
        self.idx_in_src += 1;
        self.idx_in_src
    }

    fn start_loop(&mut self) -> usize {
        self.loop_starts.push(self.idx_in_src);
        self.idx_in_src += 1;
        self.idx_in_src
    }

    fn end_loop(&mut self) -> usize {
        let start_pc = self.loop_starts.pop().unwrap();
        if *self.tape.get(self.mem_ptr).unwrap() == 0 {
            self.idx_in_src += 1;
        } else {
            self.idx_in_src = start_pc;
        }
        self.idx_in_src
    }

    fn set_pc(&mut self, idx_in_src: usize) -> usize {
        self.idx_in_src = idx_in_src;
        self.idx_in_src
    }

    fn eof(&self) {
        assert!(self.loop_starts.is_empty());
    }
}

pub fn create_default_machine() -> Machine<DefaultMachineIO> {
    let io = DefaultMachineIO::new();
    Machine::<DefaultMachineIO>::with_io(io)
}

#[derive(Debug, Default)]
pub struct SourceFile {
    ucs: UnicodeChars,
}

impl SourceFile {
    pub fn from_file<P: AsRef<Path> + Copy>(path: P) -> Result<Self, SourceFileError> {
        let raw = std::fs::read_to_string(path).map_err(|e| SourceFileError::FileFailToRead {
            path: path.as_ref().to_path_buf(),
            reason: e.to_string(),
        })?;
        Ok(Self::from_content(raw))
    }

    pub fn len(&self) -> usize {
        self.ucs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ucs.is_empty()
    }

    pub fn from_content<S: AsRef<str>>(content: S) -> Self {
        Self {
            ucs: UnicodeSegmentation::grapheme_indices(content.as_ref(), true)
                .map(|(idx, uc)| UnicodeChar {
                    idx_in_raw: idx,
                    unicode_char: uc.to_owned(),
                })
                .collect(),
        }
    }

    pub fn eval_on<IO: MachineIO>(&self, machine: &mut Machine<IO>) {
        let mut idx_in_src = 0;

        while idx_in_src < self.ucs.len() {
            idx_in_src = match self.ucs[idx_in_src].unicode_char.as_ref() {
                "." => machine.print(),
                "," => machine.read(),
                ">" => machine.next_mem_slot(),
                "<" => machine.prev_mem_slot(),
                "+" => machine.inc_mem_value(),
                "-" => machine.dec_mem_value(),
                "[" => machine.start_loop(),
                "]" => machine.end_loop(),
                _ => machine.set_pc(idx_in_src + 1),
            };
        }
        machine.eof();
    }
}

#[derive(Debug)]
pub struct UnicodeChar {
    pub idx_in_raw: usize,
    pub unicode_char: String,
}

type UnicodeChars = Vec<UnicodeChar>;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn traits() {
        use crate::utility::traits::{is_big_value_enum, is_default_debug};

        let src_file_error = SourceFileError::FileFailToRead {
            path: PathBuf::default(),
            reason: String::default(),
        };
        is_big_value_enum(&src_file_error);

        let src_file = SourceFile::default();
        is_default_debug(&src_file);
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
