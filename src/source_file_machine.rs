use crate::machine_io::{DefaultMachineIO, MachineIO};
use crate::source_file::SourceFile;

/// support cell value wrapping and data pointer moving left from the initial point
#[derive(Debug)]
pub struct SourceFileMachine<IO> {
    cells: Vec<u8>,
    data_ptr: usize,
    instr_ptr: usize,
    io: IO,
}

impl<IO: MachineIO> SourceFileMachine<IO> {
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

    /// must be called between different `eval` calls,
    /// otherwise the behavior is undefined
    fn reset(&mut self) {
        self.cells.iter_mut().for_each(|e| *e = 0);
        self.data_ptr = Self::reset_data_ptr(self.cells.len());
        self.instr_ptr = Self::reset_instr_ptr();
        self.io.flush_all();
    }

    /// . Output the byte at the data pointer.
    fn write(&mut self) {
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
    fn inc_ptr(&mut self) {
        self.data_ptr = self.data_ptr.wrapping_add(1);
        self.instr_ptr += 1;
    }

    /// < Decrement the data pointer (to point to the next cell to the left).
    fn dec_ptr(&mut self) {
        self.data_ptr = self.data_ptr.wrapping_sub(1);
        self.instr_ptr += 1;
    }

    /// + Increment (increase by one) the byte at the data pointer.
    fn inc_data(&mut self) {
        *self.cells.get_mut(self.data_ptr).unwrap() =
            self.cells.get(self.data_ptr).unwrap().wrapping_add(1);
        self.instr_ptr += 1;
    }

    /// - Decrement (decrease by one) the byte at the data pointer.
    fn dec_data(&mut self) {
        *self.cells.get_mut(self.data_ptr).unwrap() =
            self.cells.get(self.data_ptr).unwrap().wrapping_sub(1);
        self.instr_ptr += 1;
    }

    /// [ If the byte at the data pointer is zero, then instead of moving
    ///    the instruction pointer forward to the next command, jump it
    ///    forward to the command after the matching ] command.
    fn jump_if_data_not_zero(&mut self, end_ptr: usize) {
        if *self.cells.get(self.data_ptr).unwrap() == 0 {
            self.instr_ptr = end_ptr + 1;
        } else {
            self.instr_ptr += 1;
        }
    }

    /// ] If the byte at the data pointer is nonzero, then instead of
    ///   moving the instruction pointer forward to the next command,
    ///   jump it back to the command after the matching [ command.
    fn jump_if_data_zero(&mut self, start_ptr: usize) {
        if *self.cells.get(self.data_ptr).unwrap() != 0 {
            self.instr_ptr = start_ptr;
        } else {
            self.instr_ptr += 1;
        }
    }
}
