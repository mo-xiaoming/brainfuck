use std::error::Error;

use brainfuck::{parse, read_source, split_source_to_ucs};

fn main() -> Result<(), Box<dyn Error>> {
    let content = read_source("abc.bf")?;
    let ucs = split_source_to_ucs(&content);
    let tokens = parse(&ucs);
    println!("{:#?}", tokens);

    Ok(())
}

