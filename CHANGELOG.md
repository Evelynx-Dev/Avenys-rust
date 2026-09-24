# Changelog

All notable changes to Avenys will be documented in this file.

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
