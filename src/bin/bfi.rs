use brainfuck::{create_default_machine, SourceFile};

fn main() {
    let mut args = std::env::args();
    let src_file = args.nth(1).unwrap_or_else(|| {
        panic!("expecting a source file");
    });
    assert!(args.next().is_none());

    let mut machine = create_default_machine();
    let src_file = SourceFile::from_file(&src_file).unwrap();
    machine.eval(&src_file);
}
