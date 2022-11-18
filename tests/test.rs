use brainfuck::{Machine, MachineIO, SourceFile};
use serde::Deserialize;

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

#[derive(Debug, Deserialize)]
struct Test {
    src_file: String,
    output: String,
}

#[test]
fn it_works() {
    use std::path::Path;

    let test_base_dir = Path::new("tests/artifacts");

    let json = std::fs::read_to_string(test_base_dir.join("oracles.json")).unwrap();
    let tests: Vec<Test> = serde_json::from_str(&json).unwrap();
    let mut io = DebugMachineIO {
        out_buf: String::new(),
    };
    for t in &tests {
        let mut machine = Machine::<&mut DebugMachineIO>::with_io(30_000, &mut io);
        let src_file = SourceFile::from_file(test_base_dir.join(&t.src_file)).unwrap();
        machine.eval(&src_file);

        let output = std::fs::read_to_string(test_base_dir.join(&t.output)).unwrap();
        assert_eq!(output, io.out_buf, "failed on {}", t.src_file);

        (&mut io).flush_all();
    }
}
