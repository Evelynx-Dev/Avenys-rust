# August 2026 Changelog

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

## [3.24.30] - 2026-08-26 (Zero-cost runtime: dependency collector + inlined bounds/div + string optimization)

### Added

- **RuntimeTier config** (`[c] runtime = "full" | "minimal" | "none"`):
  configurable in `owl.toml` to control which runtime/PAL C sources are
  compiled. `Full` (default) compiles everything; `Minimal` compiles all
  runtime + PAL sources (demand-driven declarations); `None` compiles no
  runtime/PAL C sources (freestanding mode).
- **Dependency Collector** (`collect_used_symbols`): scans generated LLVM IR
  for `call @pal_*` and `call @rt_*` patterns, returning the set of
  actually-used symbols. Replaces the old dual-mechanism approach.
- **Demand-driven PAL declarations** (`filter_pal_decls`): in `Minimal` or
  `None` mode, only emits `declare` statements for PAL symbols that appear
  in the IR. `Full` mode emits all declarations for backward-compatibility.
- **`runtime = "none"` enforcement**: compilation fails with a clear error
  listing missing symbols if any `rt_*` calls are found when tier is None.
- **`@[no_main]` file attribute**: respected in the build pipeline to skip
  `@main` wrapper generation (freestanding mode).
- **String concat constant folding**: `"foo" + "bar"` is folded to `"foobar"`
  at compile time, eliminating the `rt_string_concat` call.
- **String literal `len()` elision**: `len("hello")` is lowered to `i64 5`
  directly, eliminating the `rt_strings_len` call.
- **Division/remainder inline at MIR level**: integer `/` and `%` operators
  are now lowered to multi-block MIR with a div-by-zero check (`ICmp(r == 0)`
  → `BrCond` to panic or ok block) followed by native `sdiv`/`srem` in
  codegen. Eliminates `rt_div_i64`/`rt_rem_i64` function call overhead.
- **Bounds-check inline at MIR level**: array/vector/list index expressions
  now emit `ICmp(index < 0)` + `ICmp(index >= upper_bound)` with conditional
  branches to a panic block (`rt_panic_loc("index out of bounds")`) or an ok
  block (GEP + Load). Eliminates `rt_check_bounds_i64` function call overhead.
- **Runtime symbol→C-file mapping** (`runtime_symbol_to_c_file`,
  `minimal_runtime_c_files`): maps `rt_*` symbols to their C source files,
  ready for future selective `.c` compilation in `Minimal` tier.
- **`[c] target` config**: configurable LLVM target triple for
  cross-compilation (e.g. `[c] target = "aarch64-unknown-linux-gnu"`).
  PAL platform directory is auto-selected from the target triple
  (`pal/linux`, `pal/darwin`, `pal/windows`, `pal/freebsd`).
- **Freestanding mode fix** (`runtime = "none"`): `@[no_main]` attribute
  now detected on fn-level annotations (not just file_attributes).
  Linker flags (`-lm`, `-lssl`, `-lcrypto`, `-lsodium`) skipped for
  `RuntimeTier::None`. Verified: 15KB binary, only `libc` linked.
- **R3.2 Selective .c compilation for minimal tier**: `runtime = "minimal"`
  now filters both runtime and PAL C files to only those actually used.
  A dependency graph resolves transitive C→C dependencies (e.g. `strings.c`
  → `vecs.c` → `strings.c`). Linker flags for crypto libs
  (`-lssl`, `-lcrypto`, `-lsodium`) are only passed when PAL .c files are
  actually compiled. Verified: string-only program → 4 .c files (was 15),
  links only `libm` + `libc`.
- **R4 Option unboxed (Maybe[T] → {i1 tag, T value})**: `Maybe[T]` types now
  use zero-cost unboxed representation `{i1 tag, T value}` instead of
  pointer + runtime functions. `Some(x)` and `None` lower to inline struct
  construction (`InsertValue`/`ExtractValue`). The `?` operator on `Maybe`
  extracts tag/value via MIR `ICmp` + `ExtractValue` — no runtime calls.
  Removed 16 `rt_maybe_*` symbols from builtins and abi_map.toml.
  Verified: all test suites pass (368/368 + consumers).

### Changed

- **While-loop lowering fix**: `lower/stmt.rs` now uses `self.current_block`
  (not hardcoded `cond_block`) for the `BrCond` terminator, because the
  condition expression may create intermediate blocks (e.g., from division
  inline or bounds-check inline). Without this fix, the while loop's
  terminator would overwrite the check's terminator, producing undefined
  values in LLVM IR.
- **Module visibility**: `build_support` and `config` modules changed to
  `pub(crate)`; `RuntimeTier` re-exported from `lib.rs` and `avens/mod.rs`.

### Deprecated

- `rt_div_i64`, `rt_rem_i64`, `rt_check_bounds_i64` are still declared in
  builtins for backward-compatibility but are no longer called by the
  compiler for new code. They will be removed in a future major version.

---

## [3.24.29] - 2026-08-24 (PAL filesystem backend + runtime fixes)

### Added

- **PAL filesystem backend implementations**: `linux_fs_exists`,
  `linux_fs_mkdir`, `linux_fs_rmdir`, `linux_fs_unlink`, and `linux_fs_remove`
  are now implemented in `pal_linux.c` and wired into the `linux_ops` table.
  These were previously declared in `pal_core.h` but had no backend
  implementations — the ops table fields were NULL. All five functions are
  gated behind `PAL_ALLOW_UNSANDBOXED` like the other `pal_fs_*` primitives.

### Fixed

- **`rt_math_abs_i64` overflow**: `INT64_MIN` now returns `INT64_MIN` instead of UB.
- **`rt_string_concat` NUL handling**: ensures result is NUL-terminated when both inputs are.
- **`rt_string_substr` bounds**: clamps `start`/`len` to source length instead of UB.
- **`rt_vecs_filter_ptr`**: fixed pointer-size bug (was casting `int64_t` to `void*`).

### Changed

- **Version**: 3.24.28 → 3.24.29

---

## [3.24.28] - 2026-08-21 (Arena allocator replaces per-string refcount)

### Added

- **Arena allocator** (`rt_managed_arena`): replaces per-string `malloc`/`free` with
  a single 1GB `mmap` + 9 size-class free lists (16/32/64/128/256/512/1024/2048/4096).
  Strings are allocated from the arena; on `refs==0` they return to the free list.
  Eliminates allocator contention and fragmentation.
- **`MirOp::Drop(MirValue)`**: explicit drop in MIR for deterministic cleanup.
  Borrowck/lowering inserts `Drop` on reassignment, scope exit, return.
  Codegen lowers `Drop` per tier: `Full`=no-op, `Minimal`=`rt_managed_free`,
  `None`=`free`.
- **Free-list size classes**: 9 buckets, blocks stored in data area (not header)
  to avoid corruption. `rt_managed_free` adds to size-class free list when `refs==0`.
  `rt_managed_cleanup_all` uses `munmap`.

### Changed

- **Memory management**: all string/struct allocations go through arena.
- **Optimizer passes** updated for `MirOp::Drop`/`Concat`: hash, DCE, simplify, inline, wrapper.

### Fixed

- **`rt_managed_free` double-free**: free-list only receives blocks when `refs==0`.

---

## [3.24.27] - 2026-08-06 (PAL FS removal API: pal_root_remove / pal_fs_remove)

### Added
- **`PAL_ERR_NOT_EMPTY`** appended to `pal_error_code_t` (append-only).
- **`pal_root_remove(pal_root_t, const char *rel_path)`** capability primitive:
  resolves PARENT with `RESOLVE_NO_SYMLINKS` only, then `fstatat` with
  `AT_SYMLINK_NOFOLLOW` + `unlinkat` (dirs with `AT_REMOVEDIR`).
- **`pal_fs_remove(path)`** host-only, gated behind `PAL_ALLOW_UNSANDBOXED`.
- **`pal_core_errno_map`**: `ENOENT/ENOTDIR`→NOT_FOUND, `EACCES/EPERM/ELOOP/EXDEV`
  →PERMISSION, `ENOTEMPTY/EEXIST`→NOT_EMPTY, `EISDIR/EINVAL/ENAMETOOLONG`
  →INVALID, `EBUSY`→BUSY, `ENOMEM`→NO_MEM, default→IO.
- **Linux discovery**: `RESOLVE_BENEATH` returns `EXDEV` when relative path crosses
  mount point; `linux_root_remove` uses `RESOLVE_NO_SYMLINKS` only.
- **Error-code fidelity fix**: dispatch functions now use `pal_core_errno_map(errno)`
  instead of hard-coded `PAL_ERR_IO`. `pal_root_open` on missing parent now reports
  `NOT_FOUND`.

### kioto
- `fs::remove(path)` (1:1 capability wrapper), `fs::remove_all(path)` (recursion),
  `fs::last_error()` → PAL error code.

### Tests
- kioto `tests/fs_remove.mire` — 10 tests: single file/empty-dir, missing→NOT_FOUND,
  non-empty→NOT_EMPTY(11), nested `remove_all`, `remove_all` on file, 4 symlink-escape cases.

### Verification
- All test batteries green: avenys 368/0, kioto 4/4, MireData 10/10, stress 40 benches.

---

## [3.24.26] - 2026-08-05 (shell migration: proc::run::shell removed)

### Removed
- **`proc::run::shell(cmd)`** removed from kioto — last Mire surface invoking `/bin/sh -c`.

### Added
- **`proc::run::output_cwd(cmd, args, cwd, merge_err)`**, `proc::run::last_exit()`,
  `proc::run::read_line()` to kioto.
- **`rt_proc_capture_argv2`**, `rt_proc_last_exit()`, `rt_read_tty()` to C runtime.

### Changed
- **`PAL_ALLOW_LEGACY_SHELL` default flipped to `0`** in `pal.h` — shell functions
  (`pal_proc_system`, `pal_proc_capture`, `pal_proc_capture_output`) compiled out.
- **owl tool migrated**: all shell consumers → argv-only (`cd &&` → `output_cwd`,
  `2>/dev/null` → explicit, `read ans < /dev/tty` → `read_line()`).
- **Test suite rewritten**: 5 test files use `proc::run::output`/`output_cwd` only.
- **Language regression test**: `pal_proc_shell_echo` → `pal_proc_argv_echo`.

### Verification
- All green: avenys 368/0, kioto 3/3, MireData 10/10, owl 16/16, stress 40 benches.

---

## [3.24.25] - 2026-08-05 (WAL cache concurrency hardening)

### Fixed
- **WAL filename collision**: now `{timestamp_ms}-{pid}-{seq}.wal` with `O_EXCL` retry.
- **Corrupt WAL no longer hard-fails**: drops single file, replays decoded records.
- **Owner-only WAL cleanup**: `save()` removes only this instance's WAL files.
  Age-pruned abandoned files (`WAL_GRACE_SECS=60`, `BLOB_GRACE_SECS=30`).
- **Cold-cache wipe race**: init serialized with exclusive `create_dir` lock
  (`wait_for_init_lock` + `InitLockGuard`, 30s stale timeout).
- **Atomic meta/blob writes**: all go through `atomic_write` (temp + rename).
- **Test harness pollution**: persist CLEAN program; inject harness only in codegen copy.
- **Build cache key**: includes `test_mode` so test/normal builds don't collide.

### Added
- **Unit tests**: `wal_filenames_are_collision_free_across_writes`,
  `replay_wal_ignores_corrupt_and_truncated_files`,
  `prune_stale_wal_removes_only_old_files`,
  `concurrent_caches_share_one_cache_dir_without_corruption` (8 threads × 16 rounds),
  `build_cache_distinguishes_test_and_normal_builds`.

### Verification
- `cargo test --release` green ×3 consecutive; lib ×14 consecutive.
- `mire test -j 8` → 3/3; `owl test -j 8` fresh cache → 10/10; `owl run` correct binary.
- Stress: 60+60 kioto runs `-j 8` 0 failed, WAL dir empty after.

---

## [3.24.24] - 2026-08-04 (.method() syntax for builtin collections)

### Fixed
- **`.method()` syntax** for `v.len()`, `v.get::i64(1)`, `m.len()`, `s.len()`.
- Root cause: parser normalizes `::`→`.`; `canonical_fn_name` also `::`→`.`.
  `builtin_method_target` was producing `::` names → `functions.get("vec::len")` = None.
- **Overloaded methods**: `v.get::i64(1)` parses as `v.get.i64`; Call-arm extracts base
  (`get`) and delegates type-suffix to `builtin_method_target`.
- **Format strings**: `format!("vec.get::{e}")` → `format!("vec.get.{e}")`.

### Tests
- All 5 `collections_method` tests pass.

### Documentation
- `SYNTAX.md` v3.24.24: §9.7 `.method()` syntax, §19 Macros (security, hygiene).

---

## [3.24.23] - 2026-08-02 (incremental roundtrip + HOF externs + stale-test migration)

### Fixed
- **`incremental::hashing` roundtrip**: `version.txt` parse bug — reads line 1 as tag,
  last line as version; wipe only on mismatch.
- **Meta key reconstruction**: `load_*_metas` now uses persisted `key` field (added
  `#[serde(default)] key: String` to `FileMeta`/`AnalysisMeta`/`BuildMeta`/`MirMeta`).
- **`benchmark_smoke`**: added 5 missing HOF externs (`rt_list_create`, `rt_list_len`,
  `rt_list_push_i64`, `rt_list_push_ptr`, `rt_lists_get_i64`).
- **11 stale `language_regressions.rs` tests** migrated to current kioto API:
  `lists.*` → `vec::*`, `dicts.*` → `map::*`, `env.get` → `env::var`,
  `fs.mkdir/rmdir` → `fs::dir::create/remove`, etc.

### Verification
- `cargo test --release` FULLY GREEN: lib 170/170, language_regressions 151/151,
  compiler_benchmarks 1/1, golden 1/1, abi_consistency 1/1, pal_conformance 8/8.
- kioto `mire test` → 1/1; MireData `owl test -j 1` → 9/9; `owl run` OK.

---

## [3.24.22] - 2026-08-01 (PAL v4 hardening: sandbox isolation, ownership, struct-return audit)

### Added
- **P1**: Absolute-path ops (`pal_fs_exists/mkdir/rmdir/unlink/read_file`) behind
  `PAL_ALLOW_UNSANDBOXED` guard. Banner: "UNSANDBOXED — bypass root capabilities".
- **P2**: Ownership conventions documented in `pal.h` + `PAL-ABI.md`:
  `[PAL-OWNED]` (caller frees with `pal_free`), `[BORROWED]` (static), `[WRITE-INTO]`.
- **P3**: Struct-return audit — `pal_dir_next`/`pal_channel_recv` removed from
  `abi_map.toml`/builtins (Mire codegen has no struct-return support; use bridges).
- **P4**: Legacy shell (`pal_proc_system/capture/capture_output`) behind
  `PAL_ALLOW_LEGACY_SHELL` (default 0). `pal_proc_capture_output` returns NULL on error.
- **P5**: Crypto error codes unified to `pal_error_code_t`; `pal_crypto.h` includes `pal.h`.
- **P6**: Scaled criteria to entire PAL surface.

### Verification
- All test batteries green: avenys 368/0, kioto 4/4, owl 16/16, MireData 10/10.