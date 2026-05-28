# Mire

Mire is a compiled, statically typed programming language with ownership-oriented memory safety checks and an LLVM-based backend.

Current compiler crate version: `3.8.0`.

## Status

- Active backend: Avenys
- Compiler pipeline: lexer, parser, type checker, semantic analysis, borrow checker, LLVM lowering
- Incremental compilation: enabled (cache, reuse, LRU pruning)
- Optimization profiles: `debug/release` + `-O0/-O1/-O2/-O3/-Os/-Oz`
- Public CLI surface: `run`, `build`, `check`, `debug`
- Callback/FFI contract: `call(...)` only lowers callbacks with inferable signature metadata; opaque `:function` dynamic calls are rejected at compile time.

## Quick Start

```bash
cargo build --release
cargo test
```

## CLI

```bash
mire run [file] [options] [-- args]      # Compile and run
mire build [file] [options]               # Compile to binary
mire check [file] [options]               # Type-check without codegen
mire debug [file] [options]               # Debug compilation
```

Default profile is `debug` (`-O0`). Use `--release` or `-O2` for optimized builds.

## Examples

```bash
# Hello world
mire run tests/level/beginner/01_hello_world.mire

# Run with args
mire run tests/complex/algorithms/01_sum_loop.mire -- --ms

# Build a release binary
mire build my_program.mire --release
```

## Documentation

- Language syntax (canonical): [SYNTAX.md](./SYNTAX.md)
- Changelog: [CHANGELOG.md](./CHANGELOG.md)
- Test suite guide: [tests/README.md](./tests/README.md)
- Built-in modules: [src/modules/README.md](./src/modules/README.md)

## License

GNU General Public License v3.0
