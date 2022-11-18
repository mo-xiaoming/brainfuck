# [Brainfuck](https://en.wikipedia.org/wiki/Brainfuck) with jit, llvm c api as backend

[![CI](https://github.com/mo-xiaoming/brainfuck/actions/workflows/build.yml/badge.svg)](https://github.com/mo-xiaoming/brainfuck/actions/workflows/build.yml)
[![codecov](https://codecov.io/gh/mo-xiaoming/brainfuck/branch/main/graph/badge.svg?token=04MMF2MJGH)](https://codecov.io/gh/mo-xiaoming/brainfuck)

## Examples

### As a Library

```rust
use brainfuck::{create_default_machine, SourceFile};

fn main() {
    let mut machine = create_default_machine();
    let src_file = SourceFile::from_file("tests/artifacts/hello_world_1.bf").unwrap();
    machine.eval(&src_file); // OUTPUT: Hello World!
}
```

### As an interpreter

```text
$ cargo build --release
$ target/release/bfi tests/artifacts/hello_world_1.bf
Hello World!
```
