# [Brainfuck](https://en.wikipedia.org/wiki/Brainfuck) with jit, llvm c api as backend

[![CI](https://github.com/mo-xiaoming/brainfuck/actions/workflows/build.yml/badge.svg)](https://github.com/mo-xiaoming/brainfuck/actions/workflows/build.yml)
[![codecov](https://codecov.io/gh/mo-xiaoming/brainfuck/branch/main/graph/badge.svg?token=04MMF2MJGH)](https://codecov.io/gh/mo-xiaoming/brainfuck)

## TODO

- [ ] add optimizations at byte code level
  - [ ] peephole
  - [ ] too many to be listed
- [ ] add llvm c api backend to jit it
- [ ] add better error handling, currently there is none besides panicking

## Examples

### As a Library

```rust
use brainfuck::{machine::create_default_machine, source_file::SourceFile};

fn main() {
    let mut machine = create_default_machine();
    let src_file = SourceFile::new("tests/artifacts/hello_world_1.bf").unwrap();
    machine.eval_source_file(&src_file);  // OUTPUT: Hello World!
    let byte_codes = src_file.to_byte_codes();
    machine.eval_byte_codes(&byte_codes); // OUTPUT: Hello World!
}
```

### As an Interpreter

```text
$ cargo build --release
$ target/release/bfi tests/artifacts/hello_world_1.bf
Hello World!
```

## Benchmark

### Baseline

End to end, doesn't do anything indirectly, encapsule no objects. Interpreting source file takes $\color{green}46$ seconds

```text
$ /usr/bin/time ./target/release/bfi tests/artifacts/mandelbrot.bf
46.71user 0.00system 0:46.71elapsed 99%CPU (0avgtext+0avgdata 2852maxresident)k
0inputs+0outputs (0major+301minor)pagefaults 0swaps
```

### New Baseline

With interpreting source file directly, eval time is $\color{green}133$ seconds

### Convert source file to byte codes

Evaluating byte codes, eval time down to $\color{yellow}29$ seconds

### Compress byte codes

With "compressed" byte codes, byte codes eval time down to $\color{yellow}14$ seconds. "Compressed" means to transform codes like `>>>>` to somthing like `4>`.

### Store tokens directly instead of slicing source file

Change `UnicodeChar` from storing source file index to `SmolStr`, by doing this, we don't need to do slicing each time to get tokens.

- Source file evaluation time down to $\color{green}78$ seconds
- Byte code evaluation time down to $\color{yellow}10$ seconds

### Simplify SourceFile iterator

By changing the iterator to a more direct call to `&UnicodeChars::IntoIter`,

- Source file evaluation time down to $\color{green}66$ seconds
- Byte code evaluation time down to $\color{yellow}9$ seconds
