# Changelog

All notable changes to Mire are documented in this file.

## [3.10.0] - 2026-05-30

### Added
- Kioto ABI v1 closed: all `__kioto_*` extern functions declared in `std/` and implemented in C.
  - 3 new C wrappers: `__kioto_lists_first`, `__kioto_lists_last`, `__kioto_lists_is_empty`.
  - `__kioto_strings_strip` extern added to `std/strings/mod.mire`.
  - Filled remaining proc externs: `run`, `exec`, `shell`, `wait`, `kill`, `exit`, `exists` with function bodies.
- TOML-based import system: `owl.toml` now supports `[imports]` section.
  - New CLI command: `mire import <module> [--version <ver>] [--path <path>]`.
  - `ImportResolver` checks manifest imports before filesystem resolution.
  - New types: `MireImports`, `MireImportEntry` (Simple, WithPath, PathOnly).
  - New functions: `write_manifest`, `load_manifest_imports`.
- `kioto/` modules marked as deprecated — std/ is the canonical module source.

### Changed
- `std/proc/mod.mire` now links to Kioto ABI C externs.
- `MireManifest` extended with optional `imports: MireImports` field.
- Import resolution order: manifest imports > project local > owl home > bundled.

## [3.9.1] - 2026-05-26

### Fixed
- `__kioto_dicts_merge` fixed: was iterating `a` entries instead of `b`, and accessing non-existent `entries[i].key_kind`.
- `__kioto_cpu_loadavg` replaced `getloadavg()` with `/proc/loadavg` read (removes `_GNU_SOURCE` dependency).
- Cleaned redundant `extern` declarations in `__kioto_time_elapsed_ns` / `__kioto_cpu_elapsed_ns`.

### Added
- `match` advanced pattern support (compiler-internal, no syntax breaks):
  - or-patterns (`p1 | p2`)
  - guard patterns (`when <bool-expr>`)
  - numeric range patterns (`a..b`, inclusive)
- Parser support for `result[T E]` and `result[T]` type surface.

### Changed
- Match pattern bindings/validation now understand wrapped patterns used by guards and or-patterns.
- Avenys match lowering now evaluates guard/or/range conditions directly during branch selection.
- Internal `DataType::Result` now stores both `ok` and `err` channels (`Result { ok, err }`), with `result[T]` defaulting `err` to `str`.

### Tests
- Added regressions:
  - `match_supports_or_patterns_and_numeric_ranges`
  - `match_guard_when_is_supported`
  - `match_guard_requires_bool_condition`
  - `parser_accepts_result_type_with_two_slots`
  - `parser_accepts_result_type_with_default_error_slot`

## [3.8.13] - 2026-05-25

### Changed
- Borrow checker now enforces move semantics on pass-by-value call arguments for non-copy types (e.g. `vec`, `map`, `str`, enums/structs), rejecting later use-after-move.
- Incremental build cache keys/fingerprints now include `import_mode` (`legacy|reachable`) to isolate cache reuse paths between global import strategies.

### Tests
- Added regression `borrowck_moves_non_copy_value_when_passing_by_value`.
- Callback regression subset remains green after ownership hardening.

### Performance
- Re-ran `benchmarks/p0_import_reachability_bench.sh` after cache-key isolation:
  - `legacy` warm avg: `55.75ms`, `10018KB` RSS peak
  - `reachable` warm avg: `36.00ms`, `9266KB` RSS peak

### Apps
- `mire-apps` compile sweep now passes `26/26` after syntax/typing cleanup in enum/map/flow samples.

## [3.8.12] - 2026-05-25

### Changed
- Type checker now rejects `call(...)` when callback is typed as `:function` but signature metadata cannot be inferred (instead of compiling with unresolved `Unknown` contract).
- Callback e2e matrix was tightened: dynamic opaque callback cases are now expected compile errors.

### Fixed
- Added backend defense-in-depth for `call(...)` dynamic lowering:
  - reject identifier callbacks without inferable signature metadata,
  - reject dynamic callback lowering when return type remains unresolved.

### Tests
- `language_regressions` callback subset updated:
  - valid: named/external/closure/function-value alias callbacks,
  - invalid: opaque `:function` param/return-value paths without inferable signature.
- Re-ran `benchmarks/p0_import_reachability_bench.sh`:
  - `legacy` warm avg: `47.75ms`, `9966KB` RSS peak
  - `reachable` warm avg: `55.75ms`, `9275KB` RSS peak
  - `reachable` lowers memory footprint in this run, with a small wall-time overhead on warm incremental rebuilds.

## [3.8.11] - 2026-05-25

### Added
- CLI flag `--import-mode legacy|reachable` (default `legacy`) for controlled rollout of global import reachability.

### Changed
- In `reachable` mode, `import modulo` (global, sin selección) now infers used symbols from importer dependencies and only loads reachable exports plus required private transitive dependencies.
- Build pipeline propagates `import_mode` into loader resolution, enabling compile-time pruning without syntax changes.

### Tests
- Added regression `global_local_import_reachable_mode_loads_only_used_symbols` (compares `legacy` vs `reachable` behavior).

## [3.8.10] - 2026-05-25

### Changed
- P0 reachability completada para imports selectivos: `import x: (...)` ahora mantiene cierre transitivo de dependencias intramódulo, evitando arrastrar exports no usados y preservando dependencias privadas requeridas.
- Loader usa grafo de dependencias de statements para construir el set alcanzable por símbolo importado.

### Added
- Regresión `local_import_selected_symbol_keeps_private_dependencies` para garantizar que imports selectivos incluyan helpers privados requeridos por símbolos públicos.

## [3.8.9] - 2026-05-25

### Added
- Modularización P6: `src/compiler/borrowck/mod.rs` (1480→1319 lines). Extraído `check_expression` + `expression_location` a `borrowck_expressions.rs` (166 lines).

## [3.8.8] - 2026-05-25

### Added
- Modularización P5: `src/incremental/mod.rs` (1595→951 lines). Extraída serialización binaria a `serialize.rs` (507 lines) y utilidades a `utils.rs` (141 lines).

## [3.8.7] - 2026-05-25

### Added
- Modularización P4: `src/compiler/typeck.rs` (1814→1236 lines). Extraído `check_expression` (~580 líneas) a `typeck/typeck_check_expression.rs` (586 lines).

## [3.8.6] - 2026-05-25

### Added
- Modularización P2: `src/avens/llvm_functions.rs` (2559→1782 lines). Extraídos 55 métodos `compile_*` + helpers de closure a `llvm_builtins.rs` (776 lines).
- Modularización P3: `src/avens/llvm_collections.rs` (2203→1380 lines). Extraídas operaciones de lista (10 métodos) a `llvm_lists.rs` (583 lines) y de dict (5 métodos) a `llvm_dicts.rs` (250 lines).

## [3.8.5] - 2026-05-25

### Added
- Modularización P1: `src/parser/mod.rs` (3387→749 lines). Extraídos `statements.rs` (598), `expressions.rs` (1432), `helpers.rs` (434).
- Kioto ABI v1 Fase 2A: wrappers `__kioto_lists_*` y `__kioto_dicts_*` en `runtime_support.c` (10+8 funciones C).
- Módulos Kioto `lists.mire` y `dicts.mire` expandidos con wrappers ABI (len, push_i64, concat, remove, delete, clear).
- New builtin dispatchers en `infer_collection_call` para `lists.pop`, `lists.first`, `lists.last`, `lists.is_empty`, `lists.append` con inferencia de tipos genérica correcta.
- LLVM codegen directo para `lists.pop`, `lists.first`, `lists.last`, `lists.is_empty`, `lists.append`.
- Symlinks `src/modules/kioto/*.mire` → `mire-kioto/modules/*.mire`.

### Fixed
- `unify_types`/`is_assignable` ahora manejan `DataType::None` (funciones sin `:Type` podían fallar con "return type mismatch").
- `check_function_statement` trata `None` como `Unknown` para inferencia de tipo de retorno.
- Backend LLVM registra `extern fn` en `user_functions` (antes declaraba pero no generaba `call`).

## [3.8.4] - 2026-05-23

### Changed
- Compiler modularization phase 2:
  - parser import parsing moved to `src/parser/imports.rs`
  - parser type parsing and generic type-parameter helpers moved to `src/parser/types.rs`
  - type checker builtin registry/import helpers moved to `src/compiler/typeck/typeck_builtins.rs`
- `src/parser/mod.rs` and `src/compiler/typeck.rs` reduced as orchestration layers while preserving behavior.

## [3.8.3] - 2026-05-23

### Changed
- Compiler modularization phase 1 (separation of responsibilities, no syntax/semantics changes):
  - parser lifecycle parsing moved to `src/parser/lifecycle.rs`
  - parser pipeline self-placeholder logic moved to `src/parser/pipeline.rs`
  - type checker return-flow helpers moved to `src/compiler/typeck/typeck_returns.rs`
- Updated planning documentation to reflect completed modularization phase and next extraction steps.

## [3.8.2] - 2026-05-23

### Changed
- Removed legacy AST/runtime variants and dead compiler branches:
  - `Statement::{Class, Trait, Code, AddLib, DmireTable, DmireColumn, DmireDlist}`
  - `MireValue::{Object, Trait, Instance}`
- Cleaned parser/type checker/borrow checker/semantic model/backend/incremental paths to stop matching and hashing removed legacy nodes.
- Simplified and updated internal tests that depended on legacy class/code units.
- Updated planning docs (`todo.md`) with current completion status and remaining high-impact performance tasks.

## [3.8.1] - 2026-05-23

### Changed
- Parser cleanup for syntax coherence:
  - removed `none` keyword parsing; `mu` is now the only unit literal/type keyword.
  - removed user-facing parsing paths for legacy `trait` and `code` statements.
- Lexer now reports `?` as an explicit reserved/unsupported token instead of silently keeping a dormant token path.
- Documentation alignment pass:
  - `SYNTAX.md` updated to use `skill` examples and `mu` type reference.
  - `docs/deprecated-cleanup.md` rewritten to reflect current real status (done vs internal pending).
  - `todo.md` updated with active focus on `Performance & Optimizations` and `Quality of Life`.

## [3.8.0] - 2026-05-22

### Changed
- Kioto and Owl code paths migrated to canonical namespace syntax using `::`.
- Parser namespace member recognition extended to support keyword-like member names in paths
  (e.g. `env::set`, `dicts::set`) without conflicting with statement parsing.
- Version bump: `3.7.0` -> `3.8.0`.

## [3.7.0] - 2026-05-21

### Added
- Namespace call compatibility for Rust-style module paths:
  - parser accepts chained `::` calls like `kioto::fs::read(...)`
  - type checker resolves root-qualified aliases to imported module symbols when needed
  - backend call resolution mirrors the same alias fallback for codegen compatibility

### Changed
- Module namespace syntax is now dual-compatible:
  - recommended: `module::submodule::fn(...)`
  - backwards-compatible: `submodule.fn(...)`
- Added parser and language regression tests for double-colon namespace calls.
- Version bump: `3.6.0` -> `3.7.0`.

## [3.6.0] - 2026-05-18

### Changed
- Generic call validation is now strict for non-generic functions:
  - explicit type arguments on non-generic functions now produce a type error
  - avoids silent acceptance of invalid generic syntax at call-site
- Added regression coverage for the non-generic explicit type-args rejection path.
- Version bump: `3.5.0` -> `3.6.0`.

## [3.5.0] - 2026-05-18

### Added
- Backend monomorph call symbol wrappers for generic calls:
  - call-site type arguments now produce stable specialized LLVM symbols
  - wrappers are emitted once per signature and forward to canonical function bodies

### Changed
- Generic nominal normalization in backend (`Type[T]` / `Type[i64]` -> `Type`) for:
  - struct constructor lookup
  - struct field/member resolution
  - impl method metadata and dispatch (`b.get()` on generic nominal receivers)
- Added E2E regression for generic impl method codegen/build path.
- Version bump: `3.4.0` -> `3.5.0`.

## [3.4.0] - 2026-05-18

### Added
- Generic impl method resolution for concrete receiver types:
  - `impl[T] Box[T] { fn get: (self) :T { ... } }`
  - calls like `b.get()` now resolve correctly when `b` is `Box[i64]`.
- Nominal generic type propagation in constructor typing (`Box[i64](...)` keeps concrete nominal type).

### Changed
- Member access/type checking now normalizes nominal generic owners when resolving fields/methods.
- Additional regression coverage for generic impl method resolution.
- Version bump: `3.3.0` -> `3.4.0`.

## [3.3.0] - 2026-05-18

### Added
- Generic `impl` headers in syntax:
  - `impl[T] Box[T] { ... }`
- Generic trait bounds in function type parameters:
  - `fn print_it[T: Show]: (x :T) { ... }`
- Type checker enforcement for generic bounds:
  - validates declared bound trait existence
  - rejects call sites where inferred/explicit generic type does not satisfy bound

### Changed
- AST expanded:
  - `Statement::Function.type_param_bounds`
  - `Statement::Impl.type_params`
  - `Statement::Impl.type_param_bounds`
- Incremental hashing updated to account for new generic metadata.
- Version bump: `3.2.0` -> `3.3.0`.

## [3.2.0] - 2026-05-18

### Added
- Nominal generic type support in parser/type checker:
  - generic type declarations: `type Box[T] { ... }`, `enum Option[T] { ... }`
  - generic nominal type usage in annotations: `Box[i64]`, `Option[i64]`
  - generic constructor/variant call paths:
    - `Box[i64](...)`
    - `Option[i64].Some(...)`
- Type checker generic substitution for:
  - constructor field type validation
  - enum payload validation and match payload bindings

### Changed
- AST expanded:
  - `Statement::Type.type_params`
  - `Statement::Enum.type_params`
- Version bump: `3.1.0` -> `3.2.0`.

## [3.1.0] - 2026-05-18

### Added
- Function-level generics syntax in parser/type system:
  - generic declarations: `fn identity[T]: (x :T) :T { ... }`
  - explicit call arguments: `identity[i64](42)`
  - inferred call arguments: `identity("ok")`
- Generic type node support in AST (`DataType::Generic`).
- Type checker generic argument resolution for function calls (explicit + inferred) with
  consistency validation.
- Parser support for `find` keyword and lowering was retained stable while extending syntax.

### Changed
- `Expression::Call` now carries `type_args` in AST.
- `Statement::Function` now carries `type_params` in AST.
- Incremental hashing updated to include generic function parameters and call type arguments.
- Version bump: `3.0.0` -> `3.1.0`.

## [3.0.0] - 2026-05-18

### Added
- `find` control-flow statement is now fully implemented end-to-end:
  - lexer keyword support (`find`)
  - parser support (`find <item> in <iterable> { ... }`)
  - backend lowering support (no longer rejected as backend limitation)
- New regression coverage for:
  - `find` parse/lower/compile path
  - `const` bindings + compound assignment analysis path

### Changed
- Version bump: `2.9.0` -> `3.0.0`.

## [2.9.0] - 2026-05-18

### Added
- Lifecycle syntax parsing and analysis surface:
  - `new::(...)`
  - `own::(...)`
  - `move::(...) to target`
  - `drop::(...)`
- New AST variants for lifecycle operations: `Statement::New`, `Statement::Own`.
- New warning diagnostic codes `W0028`–`W0033` for explicit ownership guidance in Owl/check workflows.

### Changed
- Removed legacy `vec![T]` type syntax support from parser; canonical vector type is now `vec[T]`.
- Migrated in-repo Mire sources/docs to `vec[T]` notation.
- Extended lexer keyword set for lifecycle and ownership helpers: `new`, `own`, `move`, `drop`.
- Incremental hashing/dependency tracking now covers lifecycle statements.
- `match` validation now rejects duplicate enum arms and enforces exhaustive coverage for enum matches without `_`.
- `match` parser now rejects multiple default (`_`) arms and rejects cases declared after default.
- Lifecycle type rules hardened:
  - `new::` now validates stack-construction targets (`arr/vec/map`).
  - `own::` now validates heap-allocatable targets and reports precise type errors.
- Version bump: `2.8.0` → `2.9.0`.

## [2.8.0] - 2026-05-11

### Changed
- Diagnostic precision hardening extended across type checker, borrow checker and backend context propagation.
- Warning anchoring now prioritizes real source positions and suppresses non-source/internal diagnostics to avoid misleading `1:1` reports.
- Improved compiler/Owl integration stability by surfacing and fixing multiple real ownership and symbol-collision issues discovered from Owl full-build workflows.
- Version bump: `2.7.0` → `2.8.0`.

## [2.7.0] - 2026-05-11

### Added
- New backend optimization level model (`OptLevel`) with `-O0/-O1/-O2/-O3/-Os/-Oz`.
- New roadmap document: `docs/roadmap.md` consolidating completed compiler work and pending Owl integration.

### Changed
- CLI reduced to four commands only: `run`, `build`, `check`, `debug`.
- Default compilation profile switched to `debug` (`-O0`) for faster feedback.
- Build fingerprint now includes optimization level to guarantee cache correctness.
- LLVM `opt` and `clang` flags are now driven by selected optimization level.
- Borrow checker ownership diagnostics now report contextual line/column based on active statement/expression instead of defaulting to `1:1`.
- Type checker/backend diagnostic propagation now reanchors default-position errors to active AST context when available.
- Warning diagnostics (`unused variable/function/import`) now resolve concrete source positions more reliably and skip non-source internal symbols instead of emitting misleading `1:1`.
- Version bump: `2.6.0` → `2.7.0`.

## [2.6.0] - 2026-05-11

### Added
- Unified diagnostic system (`src/error/diagnostic.rs`, `src/error/format.rs`):
  - `Diagnostic` struct with `Severity`, `DiagnosticCode` (E0001–E0015, W0001–W0027), `Label`, `Suggestion`.
  - `format_diagnostic()` with source context, colors, labels, notes, help.
  - `WarningFilter` enum (`Default`, `All`, `Codes`).
- New CLI command `mire check <file>` for analysis-only mode without binary generation.
- New CLI flags: `--warn-all`, `-W <Wxxxx>`, `--deny <Wxxxx>`.
- `analyze_program_with_warnings()` in pipeline with `WarningConfig` (filter + deny).
- Warning tests in `tests/warnings/`.
- Integration guide `docs/owl-diagnostics.md` for Owl tooling.

### Changed
- `MireError` refactored to wrap `Diagnostic` (backward-compatible API).
- `MssError` mapped to diagnostic codes E0007–E0013.
- Warnings rewritten to emit `Diagnostic` instead of `Warning` struct.
- Unused variable/function tracking now works (previously silent).
- `BuildOptions` now includes `warning_filter` + `deny_warnings`.
- Docs updated: `docs/cli.md`, `docs/diagnostic-system.md`, `MORE/0004-diagnostic-system.md`.
- Version bump: `2.5.6` → `2.6.0`.

## [2.5.6] - 2026-05-10

### Fixed
- Updated integration test `extern_and_inline_asm_declarations_parse_and_compile` to use assembly templates compatible with real LLVM inline asm emission.

### Changed
- Patch semver bump: `2.5.5` -> `2.5.6`.

## [2.5.5] - 2026-05-10

### Added
- Runtime string helpers in `src/avens/runtime_support.c`:
  - `mire_strings_contains`
  - `mire_strings_substr`
  - `mire_strings_repeat`
  - `mire_strings_pad_left`
  - `mire_strings_pad_right`
  - `mire_list_pop_i64`
- Backend handlers in `src/avens/mod.rs` for:
  - `list.pop`
  - `contains` / `strings.contains`
  - `strings.substr`
  - `strings.pad_left`
  - `strings.pad_right`
  - `strings.repeat`

### Changed
- Hardened string memory operations (`concat`, `append_owned`, `replace`) with overflow guards and safer capacity math.
- `for` lowering now supports list/vector/slice iteration in addition to `range(...)`.
- `match` pointer comparisons now distinguish string vs non-string pointer semantics:
  - strings -> `strcmp`
  - struct/enum/pointer values -> pointer equality
- Backend now accepts `extern fn` declarations by emitting LLVM `declare` signatures.
- Backend now emits minimal inline `asm` via LLVM `asm sideeffect`.
- Warning set expanded with missing warning codes (`W001`, `W006`, `W010`, `W013`, `W034`, `W036`, `W037`, `W039`, `W043`-`W051`).

### Fixed
- `sqrt(...)` lowering now calls `libm` `sqrt(double)`.
- `Drop` and `Move` statements now lower with concrete backend behavior.

### Added
- Created `std.mire` - Standard Library de Mire con todas las funciones estándar organizadas por categorías:
  - MATH: abs, min, max, sum, clamp, range, round, floor, ceil
  - LISTS: len, push, pop, append, remove, delete, clear, join, contains, index_of, first, last, slice, concat, flatten, reverse, sort, unique, is_empty, map, filter, fold
  - STRINGS: upper, lower, strip, split, replace, contains, startswith, endswith, len, trim, ltrim, rtrim, substr, pad_left, pad_right, repeat, is_empty
  - DICTS: len, keys, values, has, get, set, remove, delete, entries, merge, is_empty
  - TIME: unix_ms, unix_ns, since_ms, since_ns, mark, elapsed, elapsed_ms, elapsed_ns, sleep_ms, sleep_ns
  - TERM: style, hr, clear
  - MEM: used, total, free, available, percent, process, snapshot, format
  - CPU: time_ns, time_ms, mark, elapsed, elapsed_ms, elapsed_ns, count, freq_mhz, cycles_est, loadavg, snapshot
  - GPU: available, snapshot
  - FS: read, write, append, exists, size, copy, move, drop, list, mkdir, rmdir, join, dir, name, ext
  - ENV: get, set, all, args, cwd, chdir
  - PROC: run, spawn, pipe, shell, read, write, on, exit, err, exec, exec_bg, kill, wait, exists

### Changed
- Repository hygiene improvements: standard `.gitignore` added to prevent committing build artifacts (`target/`, `bin/*`, `benchmarks/build/`).
- Removed tracked generated artifacts from git index (`target/`, `benchmarks/build/`, `bin/debug`, `bin/release`) to keep repository lean.
- Syntax documentation standardized into a single canonical file: `SYNTAX.md`.
- `README.md` rewritten for current project status and clearer onboarding.
- Incremental cache format updated to `v5` to reduce in-memory and on-disk metadata duplication.
- Incremental cache blob store now auto-compacts when sparse to avoid unbounded growth under frequent invalidations.
- `stable_statement_hash` now hashes in streaming mode (no intermediate JSON `Vec<u8>` allocation).
- Type checker source context no longer relies on thread-local state; diagnostics now use explicit checker context.

### Fixed
- Backend lowering coverage expanded in Avenys (`src/avens/mod.rs`):
  - Frontend-only statements now handled explicitly as no-op in codegen instead of generic backend failure (`Type`, `Skill`, `Code`, `Class`, `Trait`, `Impl`, `Enum`, `AddLib`, `Module`, `Dmire*`, `Query`, `Find`, `Drop`, `Move`).
  - Added lowering for literal compound forms in `compile_expr`: `Literal::List`, `Literal::Dict`, `Literal::Tuple`.
  - Added qualified string builtin wiring: `strings.contains`, `strings.concat`, `strings.len`, `strings.strip`, `strings.ltrim`, `strings.rtrim`, `strings.is_empty`.
  - Improved unknown function diagnostics from "does not yet lower call" to explicit "unknown function".
  - Expanded `map_type` support for previously unmapped frontend types (`Function`, `Db`, `Datetime`, `Box`, `DynTrait`, `Result`) by mapping to pointer backend representation.
  - Expanded runtime kind classification for struct/enum/function/result families.
- Removed unused constant in incremental cache module (`src/incremental.rs`) to keep warnings clean.
- Simplified enum top-level scan patterns in parser (`src/parser/mod.rs`) by collapsing nested match/if branches without behavior changes.
- Reduced unnecessary `Program` cloning in loader parse/cache path (`src/loader.rs`).
- Added borrow-check regression coverage for impl-method local-reference escapes.
- Lexer token column tracking now preserves start-column for compound operators, improving diagnostic accuracy.
- Runtime CPU MHz cache is now thread-safe via C11 atomics.
- Parser and closure-lowering paths now propagate numeric/closure compilation errors instead of silently defaulting to zero values.
- Avenys numeric lowering now emits real floating-point arithmetic/comparison (`fadd/fsub/fmul/fdiv/frem/fcmp`) when operands are float-typed.
- Type checking now preserves nested vector element types across assignability checks and rejects incompatible inner element pushes (for example `vec[vec[i64]]` + `vec[str]`).
- `for item, index in range(...)` is now fully supported end-to-end (parser/AST/scopes/type-checking/backend lowering).
- Lexer/parser now support prefixed integer literals (`0b`, `0o`, `0x`) and normalize them safely to integer values.
- Raw string literals with hash delimiters are now supported (`r"..."`, `r#"..."#`, `r##"..."##`).
- Added `char` type and character literals; chars are represented as Unicode scalar values (`u32`) in the type system.
- Added frontend support for `unsafe { ... }` blocks (lexer/parser + semantic/type/borrow integration) and backend lowering by compiling contained statements.
- Added frontend support for `extern lib ...` and `extern fn ... lib ...` declarations, including FFI pointer-shape parsing (`*const/*mut`) as scalar `i64` in the current type model.
- Added frontend support for `asm { ... }` blocks and AST preservation; current Avenys backend accepts them as no-op (no target-specific IR emission yet).
- Extended Unicode case conversion: `to_upper`/`to_lower` now handle full Latin-1 supplement range (0xC0-0xFF).
- Fixed memory leak in dict format: nested map values now properly copied instead of returning managed pointer directly.
- Reference mutability semantics: `&x` now infers mutability from original binding (`mut` → mutable ref, otherwise shared), explicit `&mut x` rejected for immutable bindings.

## [2.2.0]

### Added
- Incremental compilation cache improvements (binary cache container, LRU pruning, partial analysis reuse).
- Broader support across structs, enums, methods, collections, and diagnostics.

### Changed
- Avenys backend is the active compiled path.
- Significant maturity improvements in type checking and ownership checking.

## [2.0.0]

### Changed
- Breaking syntax update from v1.x.
- Explicit `self` required for instance methods.
- Associated/static methods use `Type::method(...)`.
- Enum vs impl path behavior clarified.

## [1.0.3]

### Added
- Struct support and method dispatch improvements.
- Field access and enum payload matching fixes.

## [1.0.0]

### Added
- First stable syntax family and compiler baseline.
