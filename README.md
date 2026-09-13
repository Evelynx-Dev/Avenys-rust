# Avenys v3.24.36

**A compiled, ownership-aware systems language with an LLVM backend.**

Avenys is a statically typed programming language designed for clarity and control.
It gives you structs, enums, generics, closures, pattern matching, and a borrow
checker that tracks ownership at compile time — no garbage collector, no runtime
overhead beyond what you ask for.

The compiler, Avenys, translates Mire source through a multi-stage pipeline into
native binaries via LLVM. It ships with Kioto, a standard library covering
strings, collections, math, filesystem, processes, and more.

---

## Important: Always Use owl CLI

**Always use `owl` CLI (package manager). Never use `mire` CLI directly except for compiler development.**

| Task | Use this | NOT this |
|------|----------|----------|
| Create project | `owl new myproject` | — |
| Build | `owl build` | `mire build` |
| Run | `owl run` | `mire run` |
| Test | `owl test` | `mire test` |
| Check | `owl check` | `mire check` |
| Debug | `owl debug` | `mire debug` |
| Dependencies | `owl load <name>` | — |
| Package install | `owl install <name>` | — |

The `mire` CLI is the raw compiler interface; `owl` handles project management,
dependency resolution, build caching, tests, and delegates to `mire`
internally. Use `owl` for all day-to-day work.

---

## CLI boundary: compiler versus Owl

Avenys exposes only the compiler operations `build`, `run`, `test`, and
`debug`. Project creation, dependency installation, registries, lockfiles,
checks, and upgrades belong to Owl. Owl resolves dependencies first and passes
their directories explicitly to Avenys with `--lib-dir`.

Every compiler command that consumes a source file accepts an explicit input
file. When no output or cache path is supplied, standalone compilation uses
`<source-directory>/bin/{debug,release}` and `<source-directory>/bin/.cache`;
project builds use the project's `bin/` directory.

```bash
mire build src/main.mire \
  --lib-dir ~/.owl/libs \
  --output-dir bin/release \
  --cache-dir bin/.cache \
  --target x86_64-unknown-linux-gnu \
  --runtime full
```

Relevant path flags are:

- `--lib-dir <path>`: explicit package directory; repeat through a
  colon-separated list when Owl has several resolved dependency roots.
- `-o, --output <file>`: exact executable or library output.
- `--output-dir <dir>`: output directory when the filename should be inferred.
- `--cache-dir <dir>` (alias `--cache`): incremental cache location.

The native C-object cache is stored below `<cache-dir>/cobjects` when this
flag is used, so a build can be relocated or cleaned as one unit.

These paths are compiler inputs, not dependency management. Avenys does not
install packages, consult registries, or choose a project entry from a
manifest.

## New in v3.24.36

### LLVM object pipeline and reproducible Mire tests

Mire IR is lowered to native objects with `llc` before linking. Static
libraries use `llvm-ar`, shared libraries use `ld.lld`, and executables use
Clang only for target-aware CRT/libc linking. The modular Mire suite can be
run with reproducible per-family metrics:

```bash
mire test --log -j 8 --lib-dir ~/.owl/libs
```

Logs are overwritten under `tests/log/` on every execution.

## New in v3.24.35

### Explicit build paths

Added `--cache-dir`/`--cache` and `--output-dir`, plus deterministic standalone
defaults under `bin/`. `--lib-dir` now supports multiple search roots supplied
by Owl, including `~` expansion.

### Dependency boundary

Package resolution no longer reads the consumer's `[dependencies]` table.
Owl constructs the resolved search path and passes it to Avenys. Package export
metadata remains readable so the language's `load` and namespace semantics can
be checked safely.

## New in v3.24.34

### Shared Library Support

```bash
# Build a shared library (.so)
owl build --crate-type cdylib

# Build a static library (.a)
owl build --crate-type staticlib
```

Or configure in `owl.toml`:

```toml
[build]
crate-type = "cdylib"   # or "staticlib"
```

Produces `.so` (shared) or `.a` (static) libraries with all symbols exported for `dlopen`/`dlsym` use.

### Debug CLI Help

```bash
mire debug --help
```

Shows all debug options: `--tokens`, `--ast`, `--ir`, `--run`, plus build profiles and output options.

### Build Configuration Fix

`crate-type` in `owl.toml [build]` now correctly parses the hyphenated key:

```toml
[build]
crate-type = "cdylib"   # or "staticlib", "bin"
```

The install script supports modular installation of the Mire toolchain components:
- **owl** — package manager
- **mire** — compiler (Avenys)
- **kioto** — standard library

All install methods use the same script from `install/install.sh`.

### Linux (x86_64)

#### Option 1: Quick full install (recommended)

```bash
# Installs: owl + kioto + mire compiler
curl -fsSL https://raw.githubusercontent.com/mire-lang/Avenys-rust/main/install/install.sh | sh
```

#### Option 2: Auditable install

```bash
# Review the script before running
curl -fsSL https://raw.githubusercontent.com/mire-lang/Avenys-rust/main/install/install.sh -o install.sh
less install.sh
chmod +x install.sh && ./install.sh
```

#### Option 3: User-local install (no sudo)

```bash
curl -fsSL https://raw.githubusercontent.com/mire-lang/Avenys-rust/main/install/install.sh | sh -s -- --prefix ~/.local
```

### Install options

```bash
# Owl + Kioto stdlib (default)
curl -fsSL https://raw.githubusercontent.com/mire-lang/Avenys-rust/main/install/install.sh | sh

# Owl + Kioto + Mire compiler
curl -fsSL https://raw.githubusercontent.com/mire-lang/Avenys-rust/main/install/install.sh | sh -s -- --compiler

# Mire compiler only (no owl, no kioto)
curl -fsSL https://raw.githubusercontent.com/mire-lang/Avenys-rust/main/install/install.sh | sh -s -- --compiler-only

# Kioto stdlib only (sets up ~/.owl/modules/kioto/)
curl -fsSL https://raw.githubusercontent.com/mire-lang/Avenys-rust/main/install/install.sh | sh -s -- --kioto-only

# Owl only (package manager, no kioto, no compiler)
curl -fsSL https://raw.githubusercontent.com/mire-lang/Avenys-rust/main/install/install.sh | sh -s -- --owl-only

# Auditable install (review script first)
curl -fsSL https://raw.githubusercontent.com/mire-lang/Avenys-rust/main/install/install.sh -o install.sh
less install.sh
chmod +x install.sh && ./install.sh --compiler

# User-local install (no sudo, installs to ~/.local/bin)
curl -fsSL https://raw.githubusercontent.com/mire-lang/Avenys-rust/main/install/install.sh | sh -s -- --prefix ~/.local --compiler

# Specific versions
curl -fsSL https://raw.githubusercontent.com/mire-lang/Avenys-rust/main/install/install.sh | sh -s -- --tag-compiler v3.24.33 --tag-kioto v2.4.8
```

### Prerequisites

The script installs these automatically via your package manager:
- `curl`, `tar`, `clang`, `llvm`, `libssl`, `libsdl2`

Or install them manually:

```bash
# Debian/Ubuntu
sudo apt install curl tar clang llvm-dev libssl-dev libsdl2-dev

# Arch Linux
sudo pacman -S curl tar clang llvm openssl sdl2

# Fedora/RHEL
sudo dnf install curl tar clang llvm-devel openssl-devel SDL2-devel
```

### Post-install

```bash
# Restart your shell or source your profile
source ~/.bashrc # or ~/.zshrc

# Verify installation
owl --version
mire --version

# Create a new project
owl new myproject
cd myproject
owl info
owl run
```

---

## Quick start (owl CLI)

### Using owl (recommended for all day-to-day work)

```bash
# Create a new project
owl new myproject
cd myproject
owl run

# Build, run, test
owl build
owl run
owl test
owl check
owl debug
```

### Compiler development (build from source)

```bash
# Clone and build the compiler
git clone https://github.com/mire-lang/Avenys-rust
cd Avenys-rust
cargo build --release

# Run tests
cargo test

# Install locally (optional)
cp target/release/mire ~/.local/bin/mire
mire --version
```

### Using mire directly (compiler development only)

```bash
# Build and run a program
mire build hello.mire
mire run hello.mire

# Test
mire test
mire check hello.mire
mire debug hello.mire --ir
```

---

## How it works

Every Mire program passes through the following pipeline:

```
Source (.mire)
  │
  ▼
Lexer ──► Parser ──► Type checker ──► Borrow checker
  │
  ▼
MIR lowering ──► MIR optimization (to fixed point)
  │
  ▼
LLVM IR generation ──► opt (O0-O3) ──► LLVM object/archive tools ──► artifact
```

**Stage 1 — Frontend:** The lexer tokenizes UTF-8 source, the parser builds an AST. The type checker infers and verifies every expression with real fixed-width types (i8..i128, u8..u128, f32/f64, bool, char, str). The borrow checker enforces ownership: no use-after-move, no mutation during shared borrows, no dangling references.

**Stage 2 — MIR:** The AST lowers to a Mid-level Intermediate Representation. Optimization passes run to fixed point: constant folding, copy propagation, dead code elimination, branch folding, block merging, inlining, strength reduction, and more. New in v3.24+: division/remainder inlining (`sdiv`/`srem`), bounds-check inlining, string concat flattening (`MirOp::Concat`), and `MirOp::Drop` for explicit resource management.

**Stage 3 — Codegen:** MIR translates to LLVM IR text. The compiler invokes LLVM's `opt` for further optimization (O1-O3/Os/Oz), then `llc` lowers Mire IR to objects. Static Mire libraries use `llvm-ar`; PAL/runtime C objects remain compiled by the configured C compiler. Clang is retained only as the target-aware CRT/libc/native-library link driver for executables. Runtime tier controls what's linked:
- `Full` (default): all runtime + PAL symbols
- `Minimal`: demand-driven — only used `rt_*`/`pal_*` symbols
- `None`: freestanding — no runtime/PAL, user provides `_start`

For `wasm32-wasip1`, Avenys uses the WASI SDK when `WASI_SDK_PATH` or
`WASI_SYSROOT` is set. `wasm32-unknown-unknown` is a no-libc freestanding
target; host services must be supplied as imports. The opt-in Docker fixtures
are run with `MIRE_WASM_DOCKER_TESTS=1 tests/wasm_docker.sh`.

**Incremental compilation:** On rebuild, a fingerprint (source hash + dependency graph) is checked. If unchanged, returns in single-digit milliseconds. On partial changes, only affected units are re-analyzed. Cache uses WAL (write-ahead log) for crash-safe persistence.

---

## The Mire Language

[Full syntax reference →](./SYNTAX.md)

---

## The Language at a Glance

```mire
// Functions with inferred or explicit return types
fn fib: (n: i64) :i64 {
  if n <= 1 { return n }
  return fib(n - 1) + fib(n - 2)
}

// Structs and methods
struct Point { x: i64, y: i64 }

impl Point {
  fn dist: (self) :f64 {
    return sqrt((self.x * self.x + self.y * self.y) :f64)
  }
}

// Struct inheritance (extends)
pub struct Animal { name: str }
pub struct Dog extends Animal { breed: str }

impl Dog {
  fn greet: (self) :str {
    return self.name + " the " + self.breed
  }
}

// Skills (traits) with inheritance (super)
pub skill Greeter { fn greet: (self) :str }
pub skill Named super Greeter { fn get_name: (self) :str }

// Enums with pattern matching
enum Option[T] { None, Some(value: T) }

pub fn main: () {
  set p = Point::new(3, 4)
  set d = p.dist()
  use dasu("Distance: {d}")

  set dog = (Dog name: "Rex" breed: "Husky")
  use dasu(dog.greet())
}
```

[Full syntax reference →](./SYNTAX.md)

---

## Project structure

```
avenys/
├── src/
│ ├── lexer/        # UTF-8 source scanning and tokenization
│ ├── parser/       # Recursive descent parser and AST
│ ├── compiler/     # Type checker, borrow checker, semantic analysis
│ │ └── mir/        # MIR lowering, optimization, and LLVM codegen
│ ├── avens/        # Build pipeline, codegen, CLI integration
│ ├── incremental/  # Incremental cache (LRU, WAL, fingerprinting)
│ ├── loader/       # Module resolution and symbol renaming
│ └── pal/          # PAL ABI, core dispatch, and Linux host adapter
├── install/        # Installation script
├── tests/          # Integration tests + compiler benchmarks
├── docs/           # CHANGELOG, error codes, architecture docs
├── examples/       # Example programs
└── SYNTAX.md       # Complete language reference
```

---

## Standard library (Kioto)

Kioto lives at `~/.owl/libs/kioto/` and provides:

| Module | What it does |
|--------|-------------|
| `strings` | upper/lower, split/join, replace, trim, pad, substr, from, copy, repeat |
| `math` | trig, log, powers, statistics, random, complex numbers, decimal (basic, stats, complex, decimal, random) |
| `fs` | read, write, exists, mkdir, remove/remove_all, is_file, path/root/dir/file handles, last_error |
| `env` | var, cwd, args |
| `proc` | create, spawn (argv-safe, no shell), output_cwd, wait, kill, stdio channels, last_exit, read_line |
| `async` | channel (send/recv/close), task (ready/value), spawn/wait |
| `time` | now::ms/now::ns, elapsed/mark, sleep |
| `mem` / `cpu` | system resource queries |
| `net` | socket (connect/send/recv/close), listener (bind/accept/close) |
| `log` / `cli` | logging and CLI parsing |
| `crypto` | SHA-256/512, hex/base64, CSPRNG, Ed25519 |
| `sdl3` | joystick, gamepad, haptic, sensor, render, font_ttf, events, structs |

`fs::remove` removes a single entry (file, symlink, or empty dir) without ever
following symlinks; `fs::remove_all` recursively removes a tree (still
symlink-safe — external targets are never entered). On failure,
`fs::last_error()` returns the PAL error code (`11` = directory not empty).
See `kioto/README.md` for examples.

Dynamic collections (`vec`, `map`) are provided by the `mire` standard library,
loaded with `load mire::vec` / `load mire::map`. See `mire/README.md`.

### Runtime tiers

The `[c] runtime` setting in `owl.toml` controls what gets linked:

| Tier | C files | Libraries | Use case |
|------|---------|-----------|----------|
| `full` (default) | 15 runtime + PAL | `-lm -lssl -lcrypto -lsodium -lc` | Full stdlib, backward compatible |
| `minimal` | demand-driven (1-6) | `-lm -lc` (+ crypto if PAL used) | Size optimization, embeddings |
| `none` | 0 | none (static) | Freestanding, kernels, bare metal |

With `runtime = "none"` + `nostartfiles = true` + `nostdlib = true` you get a
~14KB statically linked binary with **zero external dependencies**.

---

## New in v3.24.33

### Load/Use syntax enforcement (E0025 / E0026)

Two new error codes enforce correct module call syntax:

| Code | Trigger | Fix |
|------|---------|-----|
| **E0025** | Using `use!` with `load` (package) modules | Call directly: `str::from_i64(42)` |
| **E0026** | Calling `load!` modules without `use!` | Wrap: `use! math::add(1, 2)` |

```mire
// Package load (owl.toml dependency) — direct calls
load mire::str
pub fn main: () {
  set x = str::from_i64(42)  // ✓ direct call
  // set x = use! str::from_i64(42)  // ✗ E0025
}

// Local load! — mandatory use!
load! /math
pub fn main: () {
  set x = use! math::add(1, 2)  // ✓ mandatory use!
  // set x = math::add(1, 2)  // ✗ E0026
}
```

### Runtime tiers documented
- **Full** (default): All 15 runtime + PAL C files, full libs
- **Minimal**: Demand-driven C compilation, only `-lm -lc` (+ crypto if PAL)
- **None**: Freestanding, no runtime/PAL, `nostartfiles` + `nostdlib`

### WASM/WASI support
- `wasm32-wasip1`: WASI Preview 1 with sysroot from `WASI_SDK_PATH`
- `wasm32-unknown-unknown`: Freestanding, host-provided imports only
- Docker tests: `MIRE_WASM_DOCKER_TESTS=1 tests/wasm_docker.sh`

### Minimal runtime strings
`strings_minimal.c` — POSIX-free string operations (no `clock_gettime`/`clock`)

---

## CLI

```bash
# Use owl for all project work
owl build [--release] [-O<0-3|s|z>]
owl run [FILE] [--release] [-O<0-3|s|z>] [-- <args>]
owl test [--verbose] [--no-run] [-j N]
owl debug [FILE] [--tokens] [--ast] [--ir] [--run]

# Compiler development only (use mire directly)
# mire build [file] [--release] [-O<0-3|s|z>]
# mire run [file] [--release] [-O<0-3|s|z>] [-- <args>]
# mire debug [file] [--tokens] [--ast] [--ir]
# mire test [paths...] [--no-run] [--verbose] [--show-warn] [-O<0-3|s|z>] [-r] [-d]

# Native compiler link/runtime controls are available on build/run/debug:
# --target <triple>  -L, --link <dir>  -l, --link-lib <name>
# --libt <bin|static|shared>  --runtime <full|minimal|none>
```

Warnings are **off by default**. Enable with:
- `--show-warn` — show all warnings (summary)
- `--position` — show per-file warning locations
- `-W <code>` — promote a specific warning to error (e.g. `-W W0001`)
- `--deny <code>` — deny a specific warning code
- `--warnings-as-errors` — deny all default warnings (W0001–W0005, W0034, W0039)
- `--no-warn <category>` — suppress warnings by category (e.g. `--no-warn Unused`)

---

## Documentation

### Core References
| Document | Description |
|----------|-------------|
| [SYNTAX.md](./SYNTAX.md) | Complete language reference with examples |
| [docs/Changelog.md](./docs/Changelog.md) | **Monolithic changelog** (all versions) |

### ABI & Runtime
| Document | Description |
|----------|-------------|
| [docs/ABI/README.md](./docs/ABI/README.md) | Mire ABI v4 specification |
| [docs/PAL/README.md](./docs/PAL/README.md) | PAL v4 Platform Abstraction Layer |
| [docs/RT/README.md](./docs/RT/README.md) | Runtime tiers (full/minimal/none) |
| [docs/libs/README.md](./docs/libs/README.md) | Building libraries (kioto/mire model) |

### Compiler Architecture
| Document | Description |
|----------|-------------|
| [docs/compiler/README.md](./docs/compiler/README.md) | Compiler pipeline & internals |
| [docs/mir-pipeline.md](./docs/mir-pipeline.md) | MIR design and optimization passes |
| [docs/incremental-design.md](./docs/incremental-design.md) | Cache architecture and fingerprinting |

### Platform Support
| Document | Description |
|----------|-------------|
| [docs/WASM/README.md](./docs/WASM/README.md) | WebAssembly & WASI support |
| [docs/WASM/runtime-requirements.md](./docs/WASM/runtime-requirements.md) | WASM runtime specifics |

### Language Syntax (Modular)
| Document | Description |
|----------|-------------|
| [docs/syntax/program-structure/README.md](./docs/syntax/program-structure/README.md) | Program entry, comments, attributes |
| [docs/syntax/variables/README.md](./docs/syntax/variables/README.md) | Declaration, mutability, constants |
| [docs/syntax/types/README.md](./docs/syntax/types/README.md) | Primitive and composite types |
| [docs/syntax/memory-ownership/README.md](./docs/syntax/memory-ownership/README.md) | Move/borrow semantics |
| [docs/syntax/fn/README.md](./docs/syntax/fn/README.md) | Functions, methods, generics |
| [docs/syntax/closures/README.md](./docs/syntax/closures/README.md) | Anonymous functions |
| [docs/syntax/control-flow/README.md](./docs/syntax/control-flow/README.md) | if, loops, match |
| [docs/syntax/pattern-matching/README.md](./docs/syntax/pattern-matching/README.md) | match patterns |
| [docs/syntax/pipeline/README.md](./docs/syntax/pipeline/README.md) | Pipeline operators |
| [docs/syntax/poo/README.md](./docs/syntax/poo/README.md) | Structs, inheritance, methods |
| [docs/syntax/enums/README.md](./docs/syntax/enums/README.md) | Enum declarations |
| [docs/syntax/skills/README.md](./docs/syntax/skills/README.md) | Traits/interfaces |
| [docs/syntax/generics/README.md](./docs/syntax/generics/README.md) | Generic types |
| [docs/syntax/collections/README.md](./docs/syntax/collections/README.md) | vec, map, array, Box |
| [docs/syntax/strings/README.md](./docs/syntax/strings/README.md) | String operations |
| [docs/syntax/operators/README.md](./docs/syntax/operators/README.md) | All operators & precedence |
| [docs/syntax/error-handling/README.md](./docs/syntax/error-handling/README.md) | Result, Maybe, panic |
| [docs/syntax/modules/README.md](./docs/syntax/modules/README.md) | load, load!, exports |
| [docs/syntax/ffi/README.md](./docs/syntax/ffi/README.md) | C FFI |
| [docs/syntax/macros/README.md](./docs/syntax/macros/README.md) | Macro system |
| [docs/syntax/builtins-io/README.md](./docs/syntax/builtins-io/README.md) | dasu, ireru, proc I/O |
| [docs/syntax/testing/README.md](./docs/syntax/testing/README.md) | Testing framework |

### Additional References
| Document | Description |
|----------|-------------|
| [docs/FFI.md](./docs/FFI.md) | Foreign Function Interface |
| [docs/LIBRARIES.md](./docs/LIBRARIES.md) | Module loading, owl.toml, exports |
| [docs/ERROR_CODES.md](./docs/ERROR_CODES.md) | Error code reference |

---

## ABI Verification

The PAL ABI v4 is validated through:
- `cargo test --release --test abi_consistency` — 8/8 pass
- `cargo test --release --test pal_conformance` — 8/8 pass
- Symbol count: 126 symbols (128 compiler-emitted minus 2 struct-return)
- All documented symbols match compiler emission and C implementation

---

## License

GNU General Public License v3.0
