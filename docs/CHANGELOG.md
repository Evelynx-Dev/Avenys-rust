# Changelog

All notable changes to Mire are documented in this file.

This is the **main changelog** — for detailed history, see the monthly archives:
- [August 2026](Aug-changelog.md) — v3.24.22 through v3.24.32
- [July 2026](Jul-changelog.md) — v3.24.2 through v3.24.21
- [June 2026](Jun-changelog.md) — v3.11.3 through v3.11.28
- [May 2026](May-changelog.md) — v2.2.0 through v3.11.2

---

## [3.24.32] - 2026-08-31 (Phase 2: MirOp::Concat + rt_string_concat_n, Phase 3: vec ops, Phase 4: true no-libc freestanding)

### Added
- **`MirOp::Concat(Vec<MirValue>)`** MIR op: flattens `a + b + c + d` string chains into a single concat operation
- **`rt_string_concat_n(Vec<MirValue>)`** runtime: single allocation + sequential memcpy for n-way string concat (replaces pairwise `rt_string_concat` calls)
- **`__attribute__((noinline))`** on `rt_string_concat_n` and `rt_vecs_clone` to ensure runtime symbols are emitted
- **vec operations** in `mire/core/vec/mod.mire` and C runtime `avenys/src/runtime/vecs.c`:
  - `filter::i64`, `filter::str`
  - `map::i64_i64`, `map::i64_str`, `map::str_str`
  - `fold::i64`, `fold::str`
  - `find::i64`, `find::str`
  - `partition::i64`
  - `chunk`, `window`, `binary_search`
  - `clone` (via `rt_vecs_clone`)
- **True no-libc freestanding** (`runtime = "none"`, `nostartfiles = true`, `nostdlib = true`):
  - `-nostartfiles` flag (skips crt1.o, crti.o, crtn.o)
  - `-nostdlib` flag (skips libc, libgcc, libstdc++)
  - `-ffreestanding` flag
  - Validated: 14KB statically-linked binary with no external dependencies
- **`[c] target` config**: LLVM target triple override (e.g. `aarch64-unknown-linux-gnu`)
- **`[c] cflags` config**: pass custom compiler flags to clang
- **Crypto linker gating**: `-lssl -lcrypto -lsodium` only linked when PAL .c files are actually compiled

### Changed
- **Phase 1A/1B**: `MirOp::Drop` + arena `mmap` + size-class free lists (committed in 3.24.31 baseline)
- **Division/remainder inline** at MIR level (3.24.30): `sdiv`/`srem` native codegen
- **Bounds-check inline** at MIR level (3.24.30): multi-block MIR for index checks
- **Selective .c compilation** for `minimal` tier (R3.2): `minimal_runtime_c_files` filters C sources by used symbols
- **Moved `src/runtime` C collection** inside tier check (only collected for `Full`/`Minimal`, not `None`)

### Fixed
- `rt_vecs_filter_ptr` pointer-size bug: was casting `int64_t` to `void*` (now stores as `int64_t`)
- `while` lowering: uses `self.current_block` for BrCond target (fixes intermediate blocks from inlined division)

---

## [3.24.31] - 2026-08-29 (Derive: @[derive(...)] with text-level expansion + real spans)

### Added
- **`@[derive(...)]` attribute** for structs: `Default`, `Clone`, `PartialEq`, `Debug`
- **Text-level source expansion** (`expand_derives_source`): generates `impl` blocks as Mire source text, spliced before parsing. This gives **real spans** — errors in derived code point to actual source lines in the user's file.
- **Derive expansion in loader** (`load_or_parse_file`): runs on the entry file before reachable-import selection, so generated `impl` blocks participate in dependency candidate collection (e.g. `str::copy` is reachable-selected when `Clone` is derived).
- **Generators**:
  - `Default` → `fn default: () :T`
  - `Clone` → `fn clone: (self) :T` (uses `str::copy` for `str` fields)
  - `PartialEq` → `fn equals: (self other :&T) :bool`
  - `Debug` → `fn to_string: (self) :str` (parenthesised format, no `{}` interpolation)
- **Multiple derives**: `@[derive(Clone, PartialEq, Debug)]` — order independent
- **Attribute consumed** after expansion; rest of compiler is unaware derive exists
- **Spans preserved**: errors in derived `impl` blocks show exact line/column in user's file

### Changed
- Moved derive expansion from `build_pipeline` (post-load) to `loader` (pre-parse). Generated `impl` blocks now participate in `collect_program_dependency_candidates` for correct reachable-import selection.
- `Statement::Type` AST gains `line`, `column`, `end_line`, `end_column` fields for precise splice positions.

### Limitations
- Structs only (enums future work)
- `Clone` requires `str::copy` → needs `load mire::str`
- `Clone` on `vec`/`map` fields fails (no `vec::clone`/`map::clone` in stdlib)
- `Debug` uses parenthesised format `Name(f: v, ...)`; no `{}` string interpolation

---

*For complete history, see the [monthly archive files](Aug-changelog.md).*