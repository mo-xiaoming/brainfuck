use brainfuck::{machine::Machine, machine_io::MachineIO, source_file::SourceFile};
use serde::Deserialize;

#[derive(Debug)]
struct DebugMachineIO {
    out_buf: std::cell::RefCell<String>,
}

impl MachineIO for &DebugMachineIO {
    fn out_char_n_times(&mut self, c: char, n: usize) {
        self.out_buf.borrow_mut().push_str(&c.to_string().repeat(n));
    }

    fn in_char(&mut self) -> char {
        todo!()
    }

    fn flush_all(&mut self) {
        self.out_buf.borrow_mut().clear();
    }
}

#[derive(Debug, Deserialize)]
struct Test {
    src_file: String,
    output: String,
}

#[test]
fn it_works() {
    use pretty_assertions_sorted::assert_eq;
    use std::path::Path;

    let test_base_dir = Path::new("tests/artifacts");

    let json = std::fs::read_to_string(test_base_dir.join("oracles.json")).unwrap();
    let tests: Vec<Test> = serde_json::from_str(&json).unwrap();
    let io = &DebugMachineIO {
        out_buf: std::cell::RefCell::new(String::new()),
    };
    for t in &tests {
        let mut machine = Machine::with_io(30_000, io);
        let src_file = SourceFile::new(test_base_dir.join(&t.src_file)).unwrap();
        machine.eval_source_file(&src_file);

        let output = std::fs::read_to_string(test_base_dir.join(&t.output)).unwrap();
        assert_eq!(
            output,
            io.out_buf.borrow().as_ref(),
            "source file eval failed on {}",
            t.src_file
        );

        let byte_codes = src_file.to_byte_codes();
        machine.eval_byte_codes(&byte_codes);
        assert_eq!(
            output,
            io.out_buf.borrow().as_ref(),
            "byte codes eval failed on {}",
            t.src_file
        );
    }
}
