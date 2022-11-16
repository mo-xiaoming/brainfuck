use brainfuck::{Machine, MachineIO, SourceFile};

#[derive(Debug)]
struct DebugMachineIO {
    out_buf: String,
}

impl MachineIO for &mut DebugMachineIO {
    fn out_char(&mut self, c: char) {
        self.out_buf.push(c);
    }

    fn in_char(&mut self) -> char {
        todo!()
    }
}

#[test]
fn print_0() {
    let mut io = DebugMachineIO {
        out_buf: String::new(),
    };
    let mut machine = Machine::<&mut DebugMachineIO>::with_io(&mut io);
    let src_file = SourceFile::from_file("tests/artifacts/print_0.bf").unwrap();
    src_file.eval_on(&mut machine);
    assert_eq!("12345", io.out_buf);
}
