# July 2026 Changelog

## [3.24.21] - 2026-07-31 (OOP expansion: inheritance, skills, async, proc)

### Added
- **Struct inheritance (`extends`)**: A struct can extend a parent struct and
  inherit its fields. Constructor accepts both parent and child fields (named
  or positional). Inherited fields are accessible from local variables and via
  `self` inside impl blocks. Single inheritance only. (`SYNTAX.md §10.5`)
- **Skill inheritance (`super`)**: A skill can extend another skill, inheriting
  its method signatures. The implementing type must provide all methods from
  both the parent and child skills. (`SYNTAX.md §12.5`)
- **Enum match inside impl blocks**: `match self { ... }` inside `impl Enum { }`
  blocks now correctly lowers to discriminant checks and case blocks instead of
  being silently dropped.
- **`async` module** (`kioto::core::async`): Added `Task` struct, `ready()`,
  `value()`, `spawn()`, `wait()` for async task/future pattern and process
  spawning via PAL.
- **`proc::shell(cmd)`** (`kioto::core::proc`): Captures command output via
  `rt_proc_capture_output` (PAL-based, managed memory).

### Fixed
- **`strings.replace` → `strings.replace.all`**: Updated test to match the
  current kioto namespace API.
- **`owl_build_run_info_cycle`**: Uses `mire` instead of stale `owl` binary.

### Changed
- **Version**: 3.24.20 → 3.24.21

---

## [3.24.20] - 2026-07-28 (Avenys test runner isolation)

### Fixed
- Test harness sources are generated outside the incremental cache, so cache
  cleanup cannot remove a source while a test worker compiles it.
- Recursive test discovery ignores `.git`, `.cache`, `target`, and `bin`
  artifacts instead of treating generated sources as tests.

---

## [3.24.19] - 2026-07-28 (Avenys documentation and PAL contract cleanup)

### Changed
- Updated README and SYNTAX to the current compiler version, module layout, Unicode terminology, and `mu` unit value.
- Rewrote the current PAL ABI reference around the actual `{index, generation}` handles and `src/pal/pal.h` contract.
- Removed obsolete v3 ABI inventory and the superseded PAL archive; the current symbol catalogue is `docs/abi_map.toml`.
- Updated library and PAL documentation to describe Core dispatch and the current host adapter instead of removed TLS, WASM-stub, and legacy backend files.

---

## [3.24.18] - 2026-07-28 (Avenys UTF-8 traversal correction)

### Fixed
- Corrected UTF-8 codepoint boundary traversal for length, substring, and index operations.
- Invalid, truncated, overlong, surrogate, and out-of-range byte sequences are treated as individual invalid bytes instead of being misread as valid codepoints.
- Clarified that the runtime's case conversion helpers are not a complete Unicode case-mapping implementation.

---

## [3.24.17] - 2026-07-28 (Avenys runtime, cache, and lexer stabilization)

### Changed
- Extracted lexer keyword classification into `lexer/keywords.rs` and restored keyword token spans to the identifier start position.
- Made incremental cache WAL replay, metadata writes, cleanup, and flush failures explicit instead of silently discarding I/O errors.
- Repaired managed-string hash-table probe clusters after deletion and prevented duplicate or partially registered entries.
- Added overflow checks to runtime list allocation and growth paths, plus safe handling for null list operands and invalid scalar element sizes.
- Removed an unused UTF-8 cache helper and made the POSIX monotonic clock declaration explicit for strict C11 builds.

---

## [3.24.16] - 2026-07-28 (Avenys build pipeline modularization)

### Changed
- Split runtime declarations, struct constructor generation, C object caching, cfg filtering, and test harness injection into `avens/build_support.rs`.
- Reduced `avens/build_pipeline.rs` to build-phase orchestration and result handling.

---

## [3.24.15] - 2026-07-28 (Avenys loader modularization)

### Changed
- Split expression, query-operation, and enum-variant renaming into `loader/rename_expression.rs`.
- Kept module prefixing, declaration renaming, scope tracking, and type-name handling in `loader/rename.rs`.

---

## [3.24.14] - 2026-07-28 (Avenys MIR modularization)

### Changed
- Split MIR list operations (`map`, `filter`, `fold`) into `compiler/mir/lower/expr_collections.rs`.
- Split literal lowering, element-type lookup, and scalar conversion emission into `compiler/mir/lower/expr_values.rs`.
- Kept `lower/expr.rs` focused on expression dispatch and scalar expression lowering.

---

## [3.24.13] - 2026-07-28 (Avenys type checking)

### Changed
- Moved the type-checker test suite to `compiler/typeck_tests.rs`; production type checking no longer shares a 900-line test block in `typeck.rs`.

---

## [3.24.12] - 2026-07-28 (Avenys parser modularization)

### Changed
- Split conditional statements, loops, `find`, and `unsafe` parsing into `parser/statement_control.rs`.
- Reduced `parser/statements.rs` below 700 lines while preserving statement dispatch behavior.

---

## [3.24.11] - 2026-07-28 (Avenys parser modularization)

### Changed
- Split function, nominal type, struct, type alias, and skill declarations into `parser/statement_declarations.rs`.
- Kept statement dispatch and control-flow parsing in `parser/statements.rs`.

---

## [3.24.10] - 2026-07-28 (Avenys parser modularization)

### Changed
- Split expression precedence and unary parsing into `parser/expression_precedence.rs`.
- Split primary expressions, closures, lifecycle expressions, and conditional expressions into `parser/expression_primary.rs`.
- Reduced `parser/expressions.rs` to postfix, calls, literals, interpolation, and assignment-target responsibilities.

---

## [3.24.9] - 2026-07-28 (Avenys diagnostics)

### Fixed
- The loop string-concatenation warning now requires the identifier's actual `str` type instead of treating every identifier as a string.

---

## [3.24.8] - 2026-07-28 (Avenys stabilization)

### Changed
- Separated persistent cache records and metadata types from cache coordination logic without changing the on-disk format.

---

## [3.24.7] - 2026-07-28 (Avenys stabilization)

### Changed
- Separated borrow-checker scope and binding state management into its own module.
- Separated warning diagnostic emission and `deny(unsafe)` validation from AST scanning.
- Preserved existing cache, parser, and diagnostic behavior while reducing compiler cross-responsibility.

---

## [3.24.6] - 2026-07-28 (Avenys modularization)

### Changed
- Moved `ErrorKind` to diagnostic metadata and help resolution in `src/error/kind.rs`, keeping error construction focused on lifecycle and context.
- Renamed the parser's `imports.rs` unit to `loads.rs`; it contains only `load` and `load!` parsing.
- `mu` is the sole unit type/literal spelling in the parser; the obsolete `none` compatibility diagnostic was removed.

---

## [3.24.5] - 2026-07-27 (Avenys stabilization)

### Fixed
- C source discovery now propagates filesystem errors instead of silently producing an incomplete runtime link set.
- `rt_build_argv` validates inputs and arithmetic, checks every allocation, and releases partially built argv arrays on failure.
- Process cleanup no longer kills or waits on a process after it has already been reaped.

### Changed
- C source collection is recursive and owned by the toolchain module rather than the build pipeline.
- PAL conformance checks resolve implementations from the current PAL Core/runtime tree, matching the PAL v4 layout.

---

## [3.24.4] - 2026-07-27 (PAL v4 Handle Safety & Kioto proc/fs Fixes)

### Fixed
- **PAL slot table recycling**: `pal_core_reserve` now uses `in_use` flag for the free
  list instead of generation wraparound. `pal_core_release` clears `in_use` without
  resetting `generation`, eliminating ABA-style handle reuse bugs.
- **`pal_core_validate(slot, generation, type)`** added as the canonical handle validity
  check — verifies `in_use && generation match && type match`. Wired up in all 30+
  handle-based dispatch functions in `pal_dispatch.c`.
- **`pal_dir_next` ABI mismatch** (memory corruption on every directory iteration):
  The C function `pal_dir_next(pal_dir_t dir)` returns `pal_dir_entry_t` (259 bytes)
  by value, requiring a hidden pointer on x86-64 SysV ABI. Kioto's Mire FFI declared it
  as `(dir :i64, entry :&str) :i64` — completely wrong return model and ghost parameter.
  Added `pal_dir_next_into(pal_dir_t dir, pal_dir_entry_t *out)` (struct-pointer out-param)
  and `pal_dir_next_name(dir, out_buf, cap)` (Kioto-friendly name-only helper) to
  `pal_dispatch.c` and `pal.h`. Updated Kioto `fs.mod.mire` to use `pal_dir_next_name`.
- **`proc.spawn` was using shell via `pal_proc_system`**, ignoring `args` and leaking
  shell injection surface. Now uses `pal_proc_create(argv, PAL_SPAWN_WAIT, ...)` via
  `rt_build_argv(cmd, args)` — proper argv construction, no shell, blocking exit code.
- **`proc.wait` was calling `pal_proc_wait_pid` with a PAL handle** instead of PID.
  Fixed to use `pal_proc_wait(pal_process_t proc)` (handle-based wait).
- **`pal_proc_create` ABI mismatch** — Kioto declared `argv :&str` (`const char *`) but
  C expects `const char **`. Added `rt_build_argv(cmd, args_vec, argc_out)` runtime helper
  that marshals `vec[str]` → `char **argv` with NULL terminator, plus `rt_free_argv` for cleanup.
- **Kioto `fs.join`/`fs.dir`/`fs.name`/`fs.ext`**: replaced broken builtins `concat(a,b)`
  and `substr(s,i,n)` with `rt_string_concat` and `rt_strings_substr` respectively.
  Also added `"concat"` and `"substr"` to the MIR lowerer's `builtin_names` protection list.
- **Dead code cleanup**: Removed unused `g_proc_buf[65536]` and dead `pal_slot_t *s`
  in `pal_dispatch.c`.

### Changed
- **Kioto version**: 2.3.2 → 2.4.0

---

## [3.24.3] - 2026-07-26 (Nested Function Flattening)

### Added
- **Nested function definitions**: Functions can now be defined inside other
  function bodies. A flattening pass automatically promotes nested `fn`
  declarations to top-level with `parent::child` naming:
  - Single level: `pub fn unwrap: () { pub fn i64: ... }` → `unwrap::i64`
  - Multi-level: `unwrap::i64::or` via deeper nesting
  - Parent functions with ONLY nested fn children become empty namespace anchors
  - Mixed bodies supported — parent keeps executable statements, children promoted
  - Flat (`pub fn unwrap::i64:`) and nested styles coexist (backward compatible)
  - Flattening runs inside `parse()` — all parse paths get it automatically
  - Visibility modifiers (`pub`/`fn`) preserved through flattening
- **Loader prefix-group fallback**: `load mire::maybe::unwrap` (3+ segments)
  falls back to loading the parent module (`mire::maybe`) and filtering exports
  by prefix (`unwrap::`), enabling targeted imports of grouped functions.
- **7 unit tests** in `src/parser/flatten.rs` + **8 integration tests** in
  `tests/nested_functions.rs`

### Changed
- **`mire::maybe` stdlib module**: Rewritten with nested function grouping
  (`some`, `is`, `unwrap` groups). All 27 functions preserved.

---

## [3.24.2] - 2026-07-26 (Stdlib Consolidation & Crypto Removal)

### Removed
- **`mire::math` stdlib module**: Removed 32 functions (pi, sin, cos, tan, sqrt, pow,
  sum, mean, range, random, etc.) — exact duplicates of `kioto::math`. Use
  `load kioto::math` instead.
- **`mire::io` stdlib module**: Removed 4 functions (print, println, input,
  input_prompt) — duplicates of `kioto::proc`. Use `load kioto::proc` instead.
- **`crypto::` builtins from avenys**: Removed SHA-256, HMAC-SHA256, base64, and hex
  encode/decode builtins (`crypto.c`, LLVM extern declarations, ABI map entries,
  SYNTAX.md documentation). Kioto's `kioto::crypto` module provides the full crypto
  API. Three byte/file utility helpers (`rt_crypto_byte_at`, `rt_read_bytes`,
  `rt_write_bytes`) retained in runtime.

### Changed
- **`mire` stdlib collections**: kioto `lists`/`dicts` removed in favor of
  `mire::vec` / `mire::map` (see kioto CHANGELOG 2.4.3).

---

## [3.24.22] - 2026-07-31 (STL loader: transitive path dependencies)

### Added
- **Transitive path dependencies**: `loader/load.rs` absolutizes relative
  dependency paths against the declaring package root, so a `load` deep inside a
  dependency resolves against its own root, not the top-level consumer's.

### Changed
- **Version**: 3.24.21 → 3.24.22