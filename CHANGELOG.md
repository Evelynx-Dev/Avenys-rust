# Changelog

All notable changes to Avenys will be documented in this file.

## 4.3.0 - 2026-09-26
### Added
- **`bits::<T>(x)` bit-level reinterpretation intrinsic** — a same-width scalar
  bitcast, lowered straight to the existing `MirOp::BitCast`, so the emitted IR
  is a native LLVM `bitcast` with no runtime call and no C builtin.
  `DataType::bit_width()` reports the real storage width of plain scalars
  (i8..i128, u8..u128, f32, f64) and `None` for bool, char, str and containers.
  `bitcast_target()` resolves the `bits::T` / `bits.T` namespace suffix to a
  `DataType` and distinguishes "not a bitcast" from "bitcast with a bad target",
  so the type checker can reject a wrong argument count, a non-bitcastable
  target, a non-scalar source and any width mismatch with a spanned diagnostic.
  Tests cover the f64 roundtrip, zero bits, identity, denormal patterns, NaN
  payload preservation and the i32/f32 width pair.

> This is new language surface, hence the minor bump rather than a patch.

### Fixed
- **Float division folded to `0.0` during constant folding** — `SDiv` dispatched
  on an `is_float` flag but its constant-folding arm ignored both operands and
  returned `0.0` for every float division, so a program that divided two
  literals such as `1.0 / 2.0` had the division erased at compile time and
  printed `0`. The arm now folds to `x / y`. Covered by four cases in
  `src/compiler/mir/optimize/mod.rs`: exact halves, signed zero, and operands
  that are themselves constant-folded expressions.
- **A test build inherited the project's artifact** — `mire test` on a project
  declaring `artifact = "shared"` built a shared object, so the runner had no
  executable to run and reported every test file `ok` having executed no
  assertion. It reached every library package, and a green from that path was a
  build success rather than a test result. `normalize_test_build_options()` now
  forces an executable and lifts a `none` runtime tier to `minimal`, once at the
  entry point and before any consumer of those fields branches on them, so the
  guarantee holds however the compiler is driven. A normal build is untouched:
  packages can still publish the artifact they declare.
- **A failing test file could report zero failures** — the summary counts
  `@[test]` declarations, so a script-style test declaring none (a plain `main`
  asserting through a helper) summed to nothing and printed `Failed: 0`
  directly beneath a `FAILED` line. The exit code was always correct, which is
  why it went unnoticed: only the number a human reads was wrong. A failing unit
  now counts as at least one failure, in both the summary and the per-family
  logs.
- **The runner never executed script-style tests, and never noticed a crash** —
  two independent gaps, both of which turned a real failure into a green. A test
  file that declares no `@[test]` and just runs assertions from `main` was only
  compiled, so its assertions were never reached; and a non-golden unit was
  judged solely on its output text, so a test binary that segfaulted printed no
  `[FAIL]` line and was reported as a pass. The exit status is now honoured, and
  a file under a configured test directory, or named explicitly on the command
  line, runs even when it declares no `@[test]`. `should_skip_run()` keeps the
  one case that should not execute: a program in the swept tree that is not a
  test path stays `ok (compiled)` and has no run-time effect, so compiling
  anything cannot turn into a side effect. On this correction four genuine
  compiler bugs became visible for the first time, all previously masked by the
  two gaps above; each is reduced to a self-contained repro and recorded in the
  affected project's tests:
  - a function whose parameter and return type are both `anything` crashes when
    called with a concrete value;
  - the pipeline operator applied to a lambda, `5 => (x => double(x))`, crashes;
  - reading a field of a struct-valued field, `o.inner.value`, returns the wrong
    value when it does not crash outright;
  - a fixed array of structs that carry a managed string field crashes, while
    the same array over a struct with only an integer field is fine.
  A further gap is a type-safety hole rather than a code-generation one: `dasu`
  has type `None` yet lowers to `0`, so an `i64 == str` comparison survives type
  checking and reaches an `inttoptr` plus `strcmp` at run time.
- **`tests/Operators/Pipeline` asserted through `dasu` against a string** — the
  case did not test the pipeline, and the comparison above is what let it
  compile. It now checks the actual result value and adds an independent
  arithmetic pipeline case.

## 4.2.2 - 2026-09-25
### Fixed
- **Compiler ownership bug**: the borrow checker treated `Str` as a non-copy
  type, producing false "Use after move" errors when a string was passed to a
  function such as `strings::concat`. `Str` is now a copy type
  (`src/compiler/borrowck/mod.rs`).

> The release note first published in 4.2.1 announced this fix, but the code
> change only shipped in 4.2.2. The 4.2.1 bump carried no source change.

## 4.2.1 - 2026-09-25
### Documentation
- **PAL API** (`src/pal/pal.h`): documented the ownership conventions
  (`[PAL-OWNED]`, `[BORROWED]`, `[WRITE-INTO]`), the thread-local
  `pal_last_error_message()` error getter, capability-based removal
  `pal_root_remove`, and the safe FFI variants `pal_dir_next_into` /
  `pal_dir_next_name`. Clarified that UNSANDBOXED functions are internal-only.
- Added the 4.2.0 changelog entry.

## 4.2.0 - 2026-09-22
### Documentation
- **PAL API documented**: Added ownership conventions (`[PAL-OWNED]`, `[BORROWED]`, `[WRITE-INTO]`), thread-local error getter `pal_last_error_message()`, capability-based removal `pal_root_remove`, safe FFI variants `pal_dir_next_into` and `pal_dir_next_name`. Clarified UNSANDBOXED functions are internal-only.


### Added
- **Multi-arch installer** (`install/install.sh`): detects the host release
  triple from `uname -m`, overridable via `--arch` / `MIRE_ARCH`, and pulls
  per-architecture release archives (`mire-compiler-<triple>.tar.gz`,
  `owl-<triple>.tar.gz`) with the x86_64 legacy fallback name. Supported
  triples: x86_64, aarch64, riscv64.
- **`--check` (read-only audit)**: validates distro, libc (glibc >= 2.39),
  detected arch, package manager and LLVM (>= 18) and exits 0/1 without
  installing or downloading.
- **`--build-from-source`**: fetches avenys-rust, ensures rustc >= 1.85 and
  builds the compiler against the host LLVM (for older glibc, different libc
  variants, custom LLVM, or architectures without prebuilt archives).
  `SOURCE_URL` / `SOURCE_REF` / `MIRE_RUSTUP_URL` override the defaults.
- **`--docker` fallback**: installs inside the `mire-lang/toolchain`
  container (GNU glibc >= 2.39), for distros without official support.
- **Multi-arch release pipeline** (`.github/workflows/release.yml`):
  x86_64/aarch64 compiler matrix plus an experimental riscv64 build under
  QEMU (greater `continue-on-error`; never blocks the release).
- **Toolchain container** (`docker/toolchain.Dockerfile` +
  `.github/workflows/docker.yml`): publishes a full LLVM/Clang toolchain
  image to GHCR on release.

### Documentation
- README install section rewritten: architecture detection (`--arch`),
  `--check` audit, `--build-from-source`, `--docker`, install options, and
  release archive table.
- All `.md` docs cleaned to standard ASCII/markdown (no emojis or non-ASCII
  symbols), keeping only project-structure trees in tree(1) style.

## 4.1.1 - 2026-09-18

### Fixed
- **`[c]` user cflags are C-only flags**: `c_defs.cflags` (e.g. `-I...`
  include paths forwarded by Owl from a project's `[c] include`) were also
  passed to the `llc` invocations that lower Mire IR. `llc` rejects C driver
  flags and exits immediately, killing the IR pipe (`Failed to stream LLVM IR
  into llc: Broken pipe`). `cflags` now reach only `clang` (C compilation and
  link) and project C object compilation; `llc` is driven solely by the
  configured `--target`/`-mtriple`.

## 4.1.0 - 2026-09-17

### Fixed
- **Ownership / memory leak (runtime)**: the `Drop` of a variable now loads the
  value held by the slot and releases that, instead of freeing the address of
  the stack slot (a silent no-op). Reassigned owned strings and collections are
  now actually freed. As a consequence a growing string is no longer quadratic
  in memory (an internal 10k-iteration concatenation probe dropped from
  ~808 MB to ~1.6 MB).
- **Retain on borrow**: `set y = x` where `x` is an owned pointer now emits
  `rt_managed_retain` for the borrowed source, so the alias stays valid after
  the original variable is reassigned or dropped.
- **Concat operand release**: `rt_string_concat` copies its operands, so fresh
  call-result operands of a string `+` are released after the concatenation.
- **Range arity**: `range`/`to` dispatches by argument count: 1 arg ->
  `rt_math_range_i64`, 2 args -> `rt_math_range_between_i64`, 3 args ->
  `rt_math_range_step_i64`. LLVM declarations added.
- **Void return codegen**: functions and `Ret(None)` now emit `ret void` for
  `()` functions instead of `ret null`-style defaults.
- **`@[attr]` after an expression**: an `at`/`@` token that starts a new line
  is treated as a top-level attribute (`@[test]`), not as an outdented index
  expression.
- **Function-name shadowing**: declaring a local with the same name as a
  function is now a declaration, not a reassignment to the function symbol.
- **Minimal runtime tier / PAL bridging**: when a retained runtime `.c` file
  bridges into PAL at the C level (`helpers.c`, `thread.c`), the PAL sources are
  now compiled in even when the IR symbol scan finds no PAL use.
- **Compiler bugs discovered** (7 real codegen bugs): implicit-return-0,
  `(expr):f64` call-arg, `::`-generic-enum, generic-impl constructors,
  nested-fn captures, bounded skill-dispatch, `=`-labelled args ignoring order.
- **Comment syntax**: documentation comments normalized from `#` to `//` line
  comments (Mire `#` comments are a lexical error). Block comments `/! !/`
  preserved.
- **Type ascription**: removed the invalid `set u = () :mu` unit-type example
  from the types docs; updated the Unit/Void table row description.

### Added
- **`pal_file_chmod` PAL capability**: added the `fs_chmod` field to
  `pal_ops_t`, implemented `linux_fs_chmod` using
  `chmod(path, strtol(mode, NULL, 8))`, and added a WASI stub. Enables
  `fs::permission` in Kioto.
- **File crypto runtime helpers** (`helpers.c`, declared in `runtime.h`):
  `rt_crypto_sha256_file_hex`, `rt_crypto_sha512_file_hex`,
  `rt_crypto_base64_file`, `rt_crypto_ed25519_verify_b64`,
  `rt_crypto_ed25519_verify_file`, and `rt_crypto_ed25519_pubkey_raw_b64`,
  with shared base64 encode/decode and binary-safe file reading.
- **`src/runtime/bytes.c`**: raw buffer allocation, little-endian readers/
  writers and blend helpers extracted from `helpers.c` so the minimal runtime
  tier can use them (math.c, FFI bindings) without the PAL-dependent file.

### Changed
- Documentation reorganized into self-contained sections under `docs/`: ABI,
  CLI, compiler, errors, FAQ, libraries, PAL, runtime, syntax (23 topics) and
  WASM; root changelog and `docs/README.md` index added.
- `owl.toml` build manifest declares `runtime = "full"`.
- Documentation coherence updates across 22 syntax sub-topics to align with
  actual compiler behavior; `collections`, `enums`, `control-flow` and
  `pattern-matching` READMEs had broken syntax claims corrected (unit type,
  type-before-`=`, `len(arr)` on arrays, match `=>` arrows, index-loops,
  find-in-collection).

### Deprecated
- None

## 4.0.0
- Initial release of Avenys v4.0.0 with PAL v4 hardening, range arity dispatch, and documentation coherence initiative.
- **Owl-managed compiler boundary**: the compiler is a restricted interface;
  project discovery, dependencies, registries and lockfiles belong to Owl,
  which passes a generated config with `--config` (`runtime = "minimal"`,
  `target x86_64-unknown-linux-gnu`, `artifact = "bin"` defaults). The old
  `--libt` spelling was removed.
- **LLVM object pipeline**: Mire IR is lowered to native objects with `llc`;
  static libraries use `llvm-ar`, shared libraries use `ld.lld`, executables
  use Clang only for target-aware CRT/libc linking.
- **Explicit build paths**: `--cache-dir`/`--cache` and `--output-dir`, with
  deterministic standalone defaults under `bin/`; `--lib-dir` supports multiple
  search roots and `~` expansion. Package resolution no longer reads the
  consumer's `[dependencies]` table.
- **Shared libraries**: `crate-type = "cdylib"` / `"staticlib"` in `owl.toml`
  (or `--crate-type` on the CLI) produce `.so`/`.a` with all symbols exported.
- **Debug CLI help**: `mire debug --help` documents `--tokens`, `--ast`,
  `--ir`, `--run`, profiles and output options.
- **Load/use enforcement (E0025 / E0026)**: `use!` is forbidden on `load`
  (package) modules (call directly); mandatory on `load!` (local) modules.
- **Runtime tiers documented**: `full`/`minimal`/`none` C-file and library
  sets, with demand-driven minimal compilation.
- **WASM/WASI targets**: `wasm32-wasip1` (WASI SDK sysroot) and
  `wasm32-unknown-unknown` (no-libc freestanding).
- `strings_minimal.c`: POSIX-free string operations for the minimal tier.
