# Mire

Mire is a compiled, statically typed programming language with ownership-oriented memory safety checks and an LLVM-based backend.

Current compiler crate version: `3.11.1`.

## Status

- Active backend: Avenys
- Compiler pipeline: lexer, parser, type checker, semantic analysis, borrow checker, LLVM lowering
- Incremental compilation: enabled (cache, reuse, LRU pruning)
- Optimization profiles: `debug/release` + `-O0/-O1/-O2/-O3/-Os/-Oz`
- Public CLI surface: `run`, `build`, `check`, `debug`, `test`, `import`
- Standard library (`std/` / Kioto): provides fs, env, strings, lists, dicts, time, cpu, mem, proc, async, gpu, term, math, and io via direct `rt_*` / `pal_*` externs.
- LLVM codegen emits `rt_*` / `pal_*` calls directly — the old `@mire_*` symbols are gone.
- PAL (Platform Abstraction Layer): `src/pal/` with linux backend. WASM backend in progress.
- Runtime core: `src/runtime/` — platform-independent managed strings, lists, dicts.
- TOML-based import management: `owl.toml` `[imports]` section with `mire import` CLI command.

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
mire import <module> [options]            # Add import to owl.toml
mire test [paths...] [options]            # Compile/run .mire tests
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

# Add a module dependency
mire import kioto --version 0.2
mire import ./local-lib --path lib/local-lib
mire import kioto --json
```

## Documentation

- Language syntax (canonical): [SYNTAX.md](./SYNTAX.md)
- Changelog: [CHANGELOG.md](./CHANGELOG.md)
- Test suite guide: [tests/README.md](./tests/README.md)
- Built-in modules: [src/modules/README.md](./src/modules/README.md)

## License

GNU General Public License v3.0
