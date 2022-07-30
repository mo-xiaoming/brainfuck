use std::error::Error;

use brainfuck::{read_source, Lexer};

fn main() -> Result<(), Box<dyn Error>> {
    let content = read_source("abc.bf")?;
    let lexer = Lexer::new(&content);

    Ok(())
}