# Avenys v3.24.32

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

## Install

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
curl -fsSL https://raw.githubusercontent.com/mire-lang/Avenys-rust/main/install/install.sh | sh -s -- --tag-compiler v3.24.32 --tag-kioto v2.4.8
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
LLVM IR generation ──► opt (O0-O3) ──► clang ──► Native binary
```

**Stage 1 — Frontend:** The lexer tokenizes UTF-8 source, the parser builds an AST. The type checker infers and verifies every expression with real fixed-width types (i8..i128, u8..u128, f32/f64, bool, char, str). The borrow checker enforces ownership: no use-after-move, no mutation during shared borrows, no dangling references.

**Stage 2 — MIR:** The AST lowers to a Mid-level Intermediate Representation. Optimization passes run to fixed point: constant folding, copy propagation, dead code elimination, branch folding, block merging, inlining, strength reduction, and more. New in v3.24+: division/remainder inlining (`sdiv`/`srem`), bounds-check inlining, string concat flattening (`MirOp::Concat`), and `MirOp::Drop` for explicit resource management.

**Stage 3 — Codegen:** MIR translates to LLVM IR text. The compiler invokes LLVM's `opt` for further optimization (O1-O3/Os/Oz), then `clang` links the IR with the C runtime (`src/runtime/`) and PAL objects (`src/pal/linux/`) into a native binary. Runtime tier controls what's linked:
- `Full` (default): all runtime + PAL symbols
- `Minimal`: demand-driven — only used `rt_*`/`pal_*` symbols
- `None`: freestanding — no runtime/PAL, user provides `_start`

**Incremental compilation:** On rebuild, a fingerprint (source hash + dependency graph) is checked. If unchanged, returns in single-digit milliseconds. On partial changes, only affected units are re-analyzed. Cache uses WAL (write-ahead log) for crash-safe persistence.

---

## The mire language


[Full syntax reference →](./SYNTAX.md)


---

## The language at a glance

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
│ ├── lexer/ # UTF-8 source scanning and tokenization
│ ├── parser/ # Recursive descent parser and AST
│ ├── compiler/ # Type checker, borrow checker, semantic analysis
│ │ └── mir/ # MIR lowering, optimization, and LLVM codegen
│ ├── avens/ # Build pipeline, codegen, CLI integration
│ ├── incremental/ # Incremental cache (LRU, WAL, fingerprinting)
│ ├── loader/ # Module resolution and symbol renaming
│ └── pal/ # PAL ABI, core dispatch, and Linux host adapter
├── install/ # Installation script
├── tests/ # Integration tests + compiler benchmarks
├── docs/ # CHANGELOG, error codes, architecture docs
└── SYNTAX.md # Complete language reference
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

## CLI

```bash
# Use owl for all project work
owl build [--release] [-O<0-3|s|z>]
owl run [FILE] [--release] [-O<0-3|s|z>] [-- <args>]
owl test [--verbose] [--no-run] [-j N]
owl check
owl debug [FILE] [--tokens] [--ast] [--ir] [--run]

# Compiler development only (use mire directly)
# mire build [file] [--release] [-O<0-3|s|z>]
# mire run [file] [--release] [-O<0-3|s|z>] [-- <args>]
# mire check [file] [--show-warn] [-W <code>] [--deny <code>]
# mire debug [file] [--tokens] [--ast] [--ir]
# mire test [paths...] [--no-run] [--verbose] [--show-warn] [-O<0-3|s|z>] [-r] [-d]
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

| Document | Description |
|----------|-------------|
| [SYNTAX.md](./SYNTAX.md) | Complete language reference with examples |
| [docs/FFI.md](./docs/FFI.md) | Foreign Function Interface reference |
| [docs/LIBRARIES.md](./docs/LIBRARIES.md) | Module loading, owl.toml, and exports |
| [docs/PAL-ABI.md](./docs/PAL-ABI.md) | Platform Abstraction Layer architecture |
| [docs/ERROR_CODES.md](./docs/ERROR_CODES.md) | All compiler error and warning codes |
| [docs/CHANGELOG.md](./docs/CHANGELOG.md) | Version history |
| [docs/mir-pipeline.md](./docs/mir-pipeline.md) | MIR design and optimization passes |
| [docs/incremental-design.md](./docs/incremental-design.md) | Cache architecture and fingerprinting |

---

## License

GNU General Public License v3.0