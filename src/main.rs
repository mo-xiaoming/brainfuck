use std::error::Error;

use brainfuck::{lex, read_source, split_source_to_ucs};

fn main() -> Result<(), Box<dyn Error>> {
    let content = read_source("abc.bf")?;
    let ucs = split_source_to_ucs(&content);
    let tokens = lex(&ucs);
    println!("{:#?}", tokens);

    Ok(())
}

