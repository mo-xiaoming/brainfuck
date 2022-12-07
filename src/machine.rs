use crate::byte_code::{ByteCode, ByteCodeKind};
use crate::machine_io::{DefaultMachineIO, MachineIO};
use crate::source_file::{
    populate_byte_codes_loop_boundaries, LoopCode, SourceFile, SourceFileIter,
};

type CellDataType = u8;

#[derive(Debug)]
pub struct Machine<IO> {
    cells: Vec<CellDataType>,
    data_ptr: usize,
    instr_ptr: usize,
    io: IO,
    #[cfg(feature = "instr_tracing")]
    instr_tracing: crate::utility::tracing::InstructionTracingCollector,
    #[cfg(feature = "instr_timing")]
    instr_timing: crate::utility::timing::InstructionTimingCollector,
}

impl<IO: MachineIO> Machine<IO> {
    #[cfg(any(feature = "instr_timing", feature = "instr_tracing"))]
    const fn codes() -> &'static [&'static str] {
        &[
            "write",
            "read",
            "inc_ptr",
            "dec_ptr",
            "inc_data",
            "dec_data",
            "loop_start",
            "loop_end",
            "get_token",
        ]
    }

    pub fn with_io(cell_size: usize, io: IO) -> Self {
        Self {
            cells: vec![0; cell_size],
            data_ptr: Self::reset_data_ptr(cell_size),
            instr_ptr: Self::reset_instr_ptr(),
            io,
            #[cfg(feature = "instr_tracing")]
            instr_tracing: crate::utility::tracing::InstructionTracingCollector::new(Self::codes()),
            #[cfg(feature = "instr_timing")]
            instr_timing: crate::utility::timing::InstructionTimingCollector::new(Self::codes()),
        }
    }

    fn reset_data_ptr(cell_size: usize) -> usize {
        cell_size / 2
    }

    fn reset_instr_ptr() -> usize {
        0
    }

    fn reset(&mut self) {
        self.cells.iter_mut().for_each(|e| *e = 0);
        self.data_ptr = Self::reset_data_ptr(self.cells.len());
        self.instr_ptr = Self::reset_instr_ptr();
        self.io.flush_all();
    }

    /// . Output `arg` bytes at the data pointer.
    fn write(&mut self, arg: usize) {
        #[cfg(feature = "instr_tracing")]
        self.instr_tracing.add("write");

        #[cfg(feature = "instr_timing")]
        let _t = self.instr_timing.start("write");

        self.io
            .out_char_n_times(*self.cells.get(self.data_ptr).unwrap() as char, arg);
        self.instr_ptr += 1;
    }

    /// , Accept one byte of input, storing its value in the byte at the data pointer.
    fn read(&mut self, _arg: usize) {
        #[cfg(feature = "instr_tracing")]
        self.instr_tracing.add("read");

        #[cfg(feature = "instr_timing")]
        let _t = self.instr_timing.start("read");

        *self.cells.get_mut(self.data_ptr).unwrap() = self.io.in_char() as CellDataType;
        self.instr_ptr += 1;
    }

    /// > Increment the data pointer (to point to the next `arg` cells to the right).
    fn inc_ptr(&mut self, arg: usize) {
        #[cfg(feature = "instr_tracing")]
        self.instr_tracing.add("inc_ptr");

        #[cfg(feature = "instr_timing")]
        let _t = self.instr_timing.start("inc_ptr");

        self.data_ptr = self.data_ptr.wrapping_add(arg);
        self.instr_ptr += 1;
    }

    /// < Decrement the data pointer (to point to the previous `arg` cells to the left).
    fn dec_ptr(&mut self, arg: usize) {
        #[cfg(feature = "instr_tracing")]
        self.instr_tracing.add("dec_ptr");

        #[cfg(feature = "instr_timing")]
        let _t = self.instr_timing.start("dec_ptr");

        self.data_ptr = self.data_ptr.wrapping_sub(arg);
        self.instr_ptr += 1;
    }

    /// + Increment (increase by `arg`) the byte at the data pointer.
    fn inc_data(&mut self, arg: usize) {
        #[cfg(feature = "instr_tracing")]
        self.instr_tracing.add("inc_data");

        #[cfg(feature = "instr_timing")]
        let _t = self.instr_timing.start("inc_data");

        *self.cells.get_mut(self.data_ptr).unwrap() = self
            .cells
            .get(self.data_ptr)
            .unwrap()
            .wrapping_add(arg as CellDataType);
        self.instr_ptr += 1;
    }

    /// - Decrement (decrease by `arg`) the byte at the data pointer.
    fn dec_data(&mut self, arg: usize) {
        #[cfg(feature = "instr_tracing")]
        self.instr_tracing.add("dec_data");

        #[cfg(feature = "instr_timing")]
        let _t = self.instr_timing.start("dec_data");

        *self.cells.get_mut(self.data_ptr).unwrap() = self
            .cells
            .get(self.data_ptr)
            .unwrap()
            .wrapping_sub(arg as CellDataType);
        self.instr_ptr += 1;
    }

    /// [ If the byte at the data pointer is zero, then instead of moving
    ///    the instruction pointer forward to the next command, jump it
    ///    forward to the command after the matching ] command.
    fn loop_start_jump_if_data_zero(&mut self, end_ptr: usize) {
        #[cfg(feature = "instr_tracing")]
        self.instr_tracing.add("loop_start");

        #[cfg(feature = "instr_timing")]
        let _t = self.instr_timing.start("loop_start");

        if *self.cells.get(self.data_ptr).unwrap() == 0 {
            self.instr_ptr = end_ptr + 1;
        } else {
            self.instr_ptr += 1;
        }
    }

    /// ] If the byte at the data pointer is nonzero, then instead of
    ///   moving the instruction pointer forward to the next command,
    ///   jump it back to the command after the matching [ command.
    fn loop_end_jump_if_data_not_zero(&mut self, start_ptr: usize) {
        #[cfg(feature = "instr_tracing")]
        self.instr_tracing.add("loop_end");

        #[cfg(feature = "instr_timing")]
        let _t = self.instr_timing.start("loop_end");

        if *self.cells.get(self.data_ptr).unwrap() != 0 {
            self.instr_ptr = start_ptr;
        } else {
            self.instr_ptr += 1;
        }
    }

    pub fn eval_source_file(&mut self, src_file: &SourceFile) {
        self.reset();

        let (start_to_end, end_to_start) =
            populate_byte_codes_loop_boundaries(RawSourceCodes { src_file }.into_iter());

        while self.instr_ptr < src_file.len() {
            match {
                #[cfg(feature = "instr_tracing")]
                self.instr_tracing.add("get_token");

                #[cfg(feature = "instr_timing")]
                let _t = self.instr_timing.start("get_token");

                src_file.at_instr_ptr(self.instr_ptr).uc.as_str()
            } {
                "." => self.write(1),
                "," => self.read(1),
                ">" => self.inc_ptr(1),
                "<" => self.dec_ptr(1),
                "+" => self.inc_data(1),
                "-" => self.dec_data(1),
                "[" => {
                    self.loop_start_jump_if_data_zero(*start_to_end.get(&self.instr_ptr).unwrap())
                }
                "]" => {
                    self.loop_end_jump_if_data_not_zero(*end_to_start.get(&self.instr_ptr).unwrap())
                }
                _ => self.instr_ptr += 1,
            }
        }

        #[cfg(feature = "instr_timing")]
        eprintln!("{}", self.instr_timing.finalize_to_string());

        #[cfg(feature = "instr_tracing")]
        eprintln!("{}", self.instr_tracing.finalize_to_string());
    }

    pub fn eval_byte_codes(&mut self, byte_codes: &[ByteCode]) {
        self.reset();

        while self.instr_ptr < byte_codes.len() {
            match {
                #[cfg(feature = "instr_tracing")]
                self.instr_tracing.add("get_token");

                #[cfg(feature = "instr_timing")]
                let _t = self.instr_timing.start("get_token");

                &byte_codes[self.instr_ptr]
            } {
                ByteCode {
                    kind: ByteCodeKind::Write,
                    arg,
                    ..
                } => self.write(*arg),
                ByteCode {
                    kind: ByteCodeKind::Read,
                    arg,
                    ..
                } => self.read(*arg),
                ByteCode {
                    kind: ByteCodeKind::IncPtr,
                    arg,
                    ..
                } => self.inc_ptr(*arg),
                ByteCode {
                    kind: ByteCodeKind::DecPtr,
                    arg,
                    ..
                } => self.dec_ptr(*arg),
                ByteCode {
                    kind: ByteCodeKind::IncData,
                    arg,
                    ..
                } => self.inc_data(*arg),
                ByteCode {
                    kind: ByteCodeKind::DecData,
                    arg,
                    ..
                } => self.dec_data(*arg),
                ByteCode {
                    kind: ByteCodeKind::LoopStartJumpIfDataZero,
                    arg,
                    ..
                } => self.loop_start_jump_if_data_zero(*arg),
                ByteCode {
                    kind: ByteCodeKind::LoopEndJumpIfDataNotZero,
                    arg,
                    ..
                } => self.loop_end_jump_if_data_not_zero(*arg),
            }
        }

        #[cfg(feature = "instr_timing")]
        eprintln!("{}", self.instr_timing.finalize_to_string());

        #[cfg(feature = "instr_tracing")]
        eprintln!("{}", self.instr_tracing.finalize_to_string());
    }
}

pub fn create_default_machine() -> Machine<DefaultMachineIO> {
    let io = DefaultMachineIO::new();
    Machine::<DefaultMachineIO>::with_io(60_000, io)
}

struct RawSourceCodes<'src_file> {
    src_file: &'src_file SourceFile,
}

impl<'a> LoopCode for &'a str {
    fn is_loop_start(&self) -> bool {
        self == &"["
    }

    fn is_loop_end(&self) -> bool {
        self == &"]"
    }
}

struct LoopCodeIter<'src_file> {
    it: SourceFileIter<'src_file>,
}

impl<'src_file> Iterator for LoopCodeIter<'src_file> {
    type Item = &'src_file str;

    fn next(&mut self) -> Option<Self::Item> {
        self.it.next().map(|uc| uc.uc.as_str())
    }
}

impl<'src_file> IntoIterator for RawSourceCodes<'src_file> {
    type Item = &'src_file str;

    type IntoIter = LoopCodeIter<'src_file>;

    fn into_iter(self) -> Self::IntoIter {
        LoopCodeIter {
            it: self.src_file.iter(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn traits() {
        use crate::utility::traits::*;

        is_debug(&create_default_machine());
    }
}
