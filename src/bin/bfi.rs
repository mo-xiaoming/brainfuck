use brainfuck::{machine::create_default_machine, source_file::SourceFile};

fn main() {
    let mut args = std::env::args();
    let src_file = args.nth(1).unwrap_or_else(|| {
        panic!("expecting a source file");
    });
    assert!(args.next().is_none());

    let mut machine = create_default_machine();
    let src_file = SourceFile::new(src_file).unwrap();
    let start = std::time::Instant::now();
    machine.eval_source_file(&src_file);
    eprintln!("eval source file: {}", start.elapsed().as_secs());

    let byte_codes = src_file.to_byte_codes();
    let start = std::time::Instant::now();
    machine.eval_byte_codes(&byte_codes);
    eprintln!("eval byte codes: {}", start.elapsed().as_secs());
}
