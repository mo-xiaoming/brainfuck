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

    fn flush_all(&mut self) {
        self.out_buf.clear();
    }
}

#[test]
fn it_works() {
    let mut io = DebugMachineIO {
        out_buf: String::new(),
    };
    let mut machine = Machine::<&mut DebugMachineIO>::with_io(30_000, &mut io);
    let src_file = SourceFile::from_file("tests/artifacts/print_12345.bf").unwrap();
    src_file.eval_on(&mut machine);
    assert_eq!("12345", io.out_buf);
}
