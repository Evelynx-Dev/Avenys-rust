# June 2026 Changelog

## [3.11.28] - 2026-06-18

### Fixed
- Trivial dereference: `&T → T` where both map to `ptr` at LLVM level (e.g. `str`,
  `list`, `map`) no longer emits a spurious `load ptr, ptr`. This fixes SIGSEGV
  in kioto wrapper functions like `env.get`, `env.cwd`, `env.args` that use
  `*key` to unwrap `&str → str`.
- Standalone file loading: `mire check/build file.mire` without an `owl.toml`
  project now resolves imports (e.g. `load kioto`) instead of only parsing the
  source file. The `load_shallow_program` fast-path was replaced with a full
  `ImportResolver` rooted at the source file's parent directory.

---

## [3.11.27] - 2026-06-16

### Infrastructure
- First build marker: initial packages published to mire-lang/libs (kioto, owl)
- Build-based compatibility boundary established for Avenys B1.001
- libs repo: https://github.com/mire-lang/libs.git

---

## [3.11.26] - 2026-06-16

### Fixed
- Closure capture codegen: insertion order of capture GEP instructions corrected
  (moved before `lower_function_body` so captured variables are visible to the
  closure body as local variables).
- Closure env struct types now propagated to `MirProgram.struct_types` so LLVM
  GEP can resolve the struct layout.
- Removed dead-code warnings (5 unused functions/imports across hashing, semantic,
  and typeck modules).

---

## [3.11.25] - 2026-06-15

### Changed
- MIR codegen: extern-function wrappers are now generated on demand only for
  extern functions that are actually referenced as values (direct calls, `call`
  builtin targets, or stored in function-typed variables). This significantly
  reduces LLVM IR size and link time for programs that import kioto's large
  extern-function surface.

### Fixed
- Restored MIR function inlining (was temporarily disabled during profiling).

---

## [3.11.24] - 2026-06-15

### Added
- MIR: first-class function-value foundation. `MirValue::FunctionRef` is now
  emitted for closures and resolved to a concrete `@fn_...` symbol, and every
  mire function (including generated closure functions) receives an implicit
  `env_ptr` parameter so indirect calls can pass a null environment pointer.
- MIR lowerer: closure literals are now lowered into standalone functions
  (`closure_N`) instead of being expanded inline at every `call(...)` site.
- MIR lowerer: `lists.map`, `lists.filter`, and `lists.fold` are now lowered
  into loops that call the supplied closure function for each element.
- MIR codegen: wrappers are generated for `extern fn` declarations so they
  share the mire `env_ptr` calling convention and can be used as function
  values. (Fixes `callback_call_extern_function_value_alias_runs_end_to_end`.)

### Fixed
- MIR inliner: parameter mapping now substitutes `MirValue::Param` with the
  caller's argument value and no longer allocates duplicate parameter slots,
  so inlining small functions (including short closures) produces correct IR.
- MIR codegen: any block whose terminator is still `Unreachable` after
  lowering/inlining now emits a default return instead of LLVM `unreachable`,
  preventing control from falling through into blocks appended later (e.g.
  inliner continuation blocks).

### Tests
- Full regression suite is green: 140 passed / 0 failed.

---

## [3.11.23] - 2026-06-15

### Fixed
- Kioto `core/dicts`: `rt_dicts_get()` and the `get` wrapper now declare a
  `:str` return type so the runtime's `void*` result is propagated instead of
  being discarded as a void call. (Fixes
  `kioto_async_ready_value_compiles_and_runs`.)

---

## [3.11.22] - 2026-06-15

### Fixed
- MIR lowerer: bare `len(...)` is now dispatched to the correct runtime length
  helper based on the argument type (`rt_strings_len`, `rt_list_len`, or
  `rt_dicts_len`) instead of relying on ambiguous bare-name resolution that
  could pick `strings.len` for a list. (Fixes
  `borrowck_moves_in_if_else_are_tracked_per_branch`.)
- Build pipeline: added LLVM declarations for `rt_strings_len()` and
  `rt_dicts_len()`.

---

## [3.11.21] - 2026-06-15

### Fixed
- MIR caching: `MirFunction::compute_hash()` now hashes function signature, all
  instruction opcodes, operands, constants, types, and terminator values. The
  previous implementation only hashed block IDs and result-temp IDs, causing
  the MIR program cache to return stale IR when source changes altered only
  literals or constants. (Fixes
  `incremental_recompile_keeps_enum_match_string_result_consistent`.)

---

## [3.11.20] - 2026-06-15

### Fixed
- Type checker: `contains(...)` and `strings.contains(...)` on non-string
  collections now emit a `Backend` error instead of silently compiling to an
  unimplemented call. (Fixes
  `backend_rejects_unimplemented_contains_instead_of_returning_silent_false`.)

---

## [3.11.19] - 2026-06-15

### Fixed
- MIR lowerer: void calls no longer allocate a result temp. A call whose
  return type is `()` now emits a `None`-result `Call` instruction and yields
  `Const(None)`. This prevents undefined-value errors when a void call is used
  in a return expression (e.g. `return rt_dicts_get(...)` in kioto wrappers).
- MIR codegen: result temps allocated for instructions without an explicit MIR
  result are now offset into a high-ID space (`%t100000+`) so they can never
  collide with MIR-defined temps like `%t0..%tN`.
- Runtime safety: integer division and modulo now call `rt_div_i64()` /
  `rt_rem_i64()`, which panic with "division by zero" on zero divisors. (Fixes
  `runtime_division_by_zero_exits_with_error`.)
- Runtime safety: array/list indexing now calls `rt_check_bounds_i64()` before
  the access, which panics with "index out of bounds" when the index is outside
  `[0, len)`. (Fixes `runtime_out_of_bounds_exits_with_error`.)

### Added
- Runtime: `src/runtime/safety.c` with `rt_panic_division_by_zero()`,
  `rt_panic_out_of_bounds()`, `rt_div_i64()`, `rt_rem_i64()`, and
  `rt_check_bounds_i64()`.
- Build pipeline: LLVM declarations for the new runtime safety helpers.

---

## [3.11.18] - 2026-06-15

### Fixed
- MIR lowerer: dict literal rendering — `dasu()` / `str()` / `print()` now wrap
  map/dict arguments with `rt_dict_to_string()` so nested maps print as strings
  instead of raw struct bytes. (Fixes `nested_map_string_render_executes_-
  without_runtime_errors`, `syntax_reference_prototype_compiles_and_runs`.)
- MIR lowerer: nested map `value_kind` — dict literals with map values now call
  the new runtime helper `rt_dicts_set_with_kind()` so the runtime knows the
  value is a map and can recursively stringify it.
- MIR lowerer: signed integer modulo — `%` was falling through to `Add` because
  `SRem` was missing from the MIR op set. Added `MirOp::SRem`, lowered `%` to
  it, and wired it through codegen, inlining, and optimization passes. (Fixes
  `signed_integer_division_and_remainder_match_runtime_expectations`.)
- MIR lowerer: `for` loop list element indexing — list layout is `[len, elem0,
  elem1, ...]`, but the loop used the raw index as the GEP index, reading the
  length field as the first element. The index is now offset by 1. (Fixes
  `secondary_for_loop_binding_compiles_and_uses_index`.)
- Runtime: `rt_strings_split()` rewrote empty-segment handling; it no longer
  uses `strtok()` (which drops empties) and now appends a trailing empty
  segment when the input ends with the separator. (Fixes
  `strings_split_preserves_empty_segments`.)
- MIR lowerer: inline closure calls — `call((x) => ..., ...)` with a closure
  literal callee now lowers the closure body inline in the caller, binding
  parameters and preserving captured variables. (Fixes
  `callback_call_closure_with_capture_runs_end_to_end`.)
- Type checker: generic nominal type argument parsing — `Box[T]` now parses `T`
  as `DataType::Generic("T")` instead of `Unknown`, so instance method dispatch
  can bind generic parameters to concrete types. (Fixes
  `generic_impl_method_codegen_builds_for_concrete_type`.)
- MIR codegen / build pipeline: `DataType::Generic` now maps to `i64` in LLVM IR
  so generic struct fields and method returns use a concrete scalar type.

### Added
- Runtime: `rt_dict_to_string()` and `rt_dicts_set_with_kind()` helpers.
- MIR op: `SRem` for signed remainder.

### Changed
- Test suite: added `struct_types: HashMap::new()` to `MirProgram` initializers
  in unit tests so the test target compiles.
- Compiler is now warning-free (unused `program`/`method` parameters fixed).

---

## [3.11.17] - 2026-06-14

### Fixed
- MIR lowerer: `self` parameter type in impl methods — `self` was parsed with
  `DataType::Unknown` but the lowerer needs `StructNamed(type_name)` for
  `get_struct_name` to resolve field accesses. Now overridden during method
  lowering. (Fixes `impl_method_can_mutate_self_field_and_run`,
  `implicit_self_method_return_still_runs`.)
- MIR lowerer: instance method dispatch — `Expression::Call` with dot-qualified
  names (e.g. `p.distance()`) now resolves the receiver variable's type, looks
  up the method in `method_map`, rewrites the call target to the qualified
  function name (`Point.distance`), and prepends the receiver as the first
  argument. (Fixes `instance_method_call_resolves_and_compiles`.)

### Added
- MIR program metadata: `MirProgram.method_map` field (`HashMap<String,
  HashMap<String, String>>`), extracted from `Statement::Impl` in
  `extract_method_map()` and passed to `MirLower` for instance method dispatch.

### Changed
- MIR lowerer: match arm pattern handling — `EnumVariantPath` patterns now
  match against discriminant (previously fell through to `_ => Br(cs)` which
  always selected the first arm). (Fixes
  `enum_match_without_default_returns_second_variant_string`.)
- MIR codegen: `EnumNamed` data type maps to `i64` in LLVM IR instead of `ptr`,
  fixing return type mismatch when functions return enum types.

---

## [3.11.16] - 2026-06-14

### Fixed
- MIR codegen: enum type mapping — `EnumNamed` data types now map to `i64` in
  LLVM IR, matching the discriminant representation, eliminating type mismatch
  errors (`ret i64` vs `ptr`) when functions return enum types.

### Added
- MIR program metadata: `MirProgram.enum_types` field (`HashMap<String,
  Vec<(String, usize)>>`), extracted from `Statement::Enum` in
  `extract_enum_types()` and passed to both lowerer and codegen.
- MIR program metadata: `MirProgram.bare_to_qualified` field (`HashMap<String,
  String>`), extracted from IR qualified name map in `extract_bare_name_map()`,
  enabling bare-name to IR-qualified name resolution.

### Changed
- MIR lowerer: `Expression::EnumVariantPath` now returns the correct integer
  discriminant instead of `Const(Int(0))`, and match arm patterns for both
  `EnumVariant` and `EnumVariantPath` compare against the real discriminant.
  (Fixes `enum_match_without_default_returns_second_variant_string`,
  test count 118/140.)
- MIR lowerer: bare-name resolution — `Expression::Call` resolves unqualified
  function names (`main`) to their IR-qualified symbols (`fn_main`) via the
  `bare_to_qualified` map before emitting the `Call` op.

---

## [3.11.15] - 2026-06-14

### Fixed
- MIR lowerer: `Expression::Match` lowering rewritten — pattern literals now
  use `BrCond` with `ICmp (Eq, match_val, literal)` branching to the matching
  case block or the next check block, and case bodies are lowered into their
  own blocks (not the default block). Dead code / infinite loop eliminated.
  (Fixes `match_expression_with_default_infers_string_branch_type_and_compiles`,
  test count 111/140.)

---

## [3.11.13] - 2026-06-13

### Fixed
- MIR codegen: temp ID space collision — `tmp_extra` (`%e` prefix) and
  `tmp_result` (`%t{mir_id}`) now use separate counters, preventing MIR
  result temps from aliasing LLVM extra temps.
- MIR codegen: struct metadata pipeline — `struct_types` collected from
  `Statement::Type` during lowering and threaded through `MirProgram` /
  `MirLower` / `LlvmCtx` so field lookup works in codegen.
- MIR codegen: GEP fallback now uses `struct_name` directly as LLVM type
  (instead of hardcoded `"ptr"`), plus `struct:` prefix for explicit struct
  types, enabling array element GEP via element-type strings.
- MIR codegen: `Trunc` op now emits `trunc <src> <val> to <dst>` instead of
  falling through to the no-op catch-all.
- MIR codegen: `Array` and `Slice` data types mapped to `[N x T]` / element
  type in `llvm_type_str`, enabling stack-allocated arrays of the correct size.
- MIR codegen: `And`, `Or` on `i1` values now work correctly (no `icmp ne`
  double-negation).
- Type checker: `referenced_type_for_expr` now unwraps `Ref { inner }` /
  `RefMut { inner }` when falling back to `lookup_var`, fixing `*value`
  returning the reference type instead of the referenced type for function
  parameters of `&T` type.

### Added
- MIR lowerer: `Expression::Index` lowered as GEP + Load (array reads).
- MIR lowerer: `Expression::Reference` lowered as pointer return (skips Load).
- MIR lowerer: `Expression::Dereference` lowered as Load from pointer.
- MIR lowerer: `Expression::UnaryOp` lowered (negation `-` as `Sub(0, x)`,
  logical not `!` as `ICmp(Eq, x, false)`).
- MIR lowerer: `Expression::List` for `Array` data type — stack-allocates the
  array and initializes each element (used in `let x = [a b c] :arr[T N]`).
- MIR lowerer: `Statement::For` lowered as while-loop with iterator alloca,
  index counter, `rt_list_len` call, GEP + Load for element access, and index
  variable registration.
- MIR lowerer: `Statement::Unsafe` lowered by forwarding body statements.
- MIR lowerer: `AssignmentTarget::Index` — GEP + Store for array writes
  (e.g. `arr at i = val`).
- MIR lowerer: `AssignmentTarget::Field` — load struct heap pointer, GEP +
  Store for struct field writes (e.g. `obj.field = val`).
- MIR lowerer: `get_target_elem_type()` helper to extract the LLVM element
  type from a variable's `Array`/`Vector`/`Slice` data type.
- MIR lowerer: `llvm_elem_type_str()` standalone function for LLVM type
  strings from `DataType`.
- Struct metadata: `MirProgram.struct_types` field (`HashMap<String,
  Vec<(String, DataType)>>`), extracted from `Statement::Type` in
  `extract_struct_types()` and passed to both lowerer and codegen.

---

## [3.11.12] - 2026-06-04

### Fixed
- MIR codegen: struct constructor functions (`@Stack`, `@str`, etc.) are
  generated automatically from `Statement::Type` definitions, enabling struct
  field initialization in the MIR pipeline.
- MIR codegen: `__if_expr` builtin expanded to proper MIR control flow
  (Alloca + BrCond + Store/Load across then/else/end blocks), fixing
  `if`-expression codegen in the MIR path.
- MIR codegen: `@main` entry-point wrapper (`define i32 @main(i32, ptr)`)
  emitted when `@fn_main` is defined, so the MIR pipeline produces runnable
  binaries instead of linker errors.
- MIR codegen: runtime declarations (`@dasu`, `@.fmt_*` globals, `@.argc`,
  `@.argv`) auto-inserted when absent, fixing "use of undefined value" errors
  for built-in I/O.
- MIR codegen: `@dasu` declaration check uses `"declare @"` prefix instead of
  bare `"@dasu"` to distinguish declarations from call sites.
- MIR lowerer: `new_block()` now uses `self.func.blocks.len()` instead of
  `self.next_block`, fixing block ID mismatch that caused `__if_expr` blocks
  to alias the entry block and be eliminated by the optimizer.
- `compile_binary_from_ir` in toolchain: pass object files before `-x ir -`
  so clang auto-detects `.o` files instead of trying to parse them as LLVM IR.

### Added
- Hierarchical load paths: `load kioto::math::basic` resolves through
  `owl.toml [exports]` recursively. `Statement::Load.path` is now `Vec<String>`.
- `module <name>` declaration for intra-package identity (block-style body
  removed — was dead code).
- `use <name>` (bare identifier, no parens) for linking submodules within a
  package via `[exports]`.
- `owl.toml [exports]` section maps export names to `.mire` files or
  sub-directories with their own `owl.toml` for hierarchical resolution.
- `owl.toml [bootstrap]` section with `BootstrapConfig { std_package, std_entry }`,
  defaulting to `kioto` / `"mod.mire"`.
- `MireProject` fields `name`, `version`, `entry` made `#[serde(default)]`
  for minimal manifests. `#[serde(alias = "owl")]` on `project` field for
  backward compat.
- `mire validate` command: validates `owl.toml` dependencies + exports.
- `mire owl add <name> --path <p>|--version <v>`: adds a dependency entry.
- `mire owl remove <name>`: removes a dependency entry.
- `load_exports()` / `resolve_export_path()` in manifest module.
- `resolve_package(name)` in loader: resolves solely through manifest
  `[dependencies]` — no filesystem probing.
- `resolve_load_path(&[segments])` chains through `[exports]` recursively.
- Kioto unbundled: `src/modules/kioto/` deleted; code lives in standalone
  `../mire-kioto/` package with its own `owl.toml` and per-submodule `owl.toml`
  files.
- 10 new parser tests for hierarchical load, `module`, `use` disambiguation.

### Changed
- Module loading is now purely package-based: `load` only resolves through
  `owl.toml [dependencies]`. No heuristic filesystem resolution, no OWL_HOME
  probing, no relative paths.
- `Statement::Load.is_local` removed from AST — one resolve path only.
- `Statement::Module { name, body }` simplified to `Statement::Module { name }`.
- `import` keyword removed entirely from lexer, parser, AST, and CLI.
  Legacy `import` fails to parse with "unexpected token" error.
- `--allow-legacy-imports` flag, `allow_legacy_imports` warning config, and
  `MIRE_ALLOW_LEGACY_IMPORTS` env var removed.
- `mire import` CLI command removed. Use `owl add` / `owl remove` instead.
- `load std` standardized to `load kioto` everywhere.
- `load ./path` is invalid syntax (removed completely, not just warned).
- `owl.toml` manifests normalized to `[project]` section (no duplicate `[owl]`).
- Loader reduced from 1920 to 1561 lines: 7 heuristic functions removed,
  3 new resolution functions added.
- `mire-kioto/` submodules converted to `module <name>` + `use <name>`.
- `mire/owl.toml` created with `kioto = { path = "../mire-kioto" }`.

### Fixed
- Module loading now rewrites internal references inside prefixed modules, so
  recursive calls and cross-references remain valid after namespacing.

---

## [3.11.10] - 2026-06-02

### Added
- Regression coverage for recursive `load X` modules and root-level local
  module discovery.

### Fixed
- Module loading now rewrites internal references inside prefixed modules, so
  recursive calls and cross-references remain valid after namespacing.
- Local `load` diagnostics now describe the actual `load` path instead of the
  legacy wording.

---

## [3.11.9] - 2026-06-02

### Changed
- Public manifest dependency types were renamed from `MireImports` / `MireImportEntry`
  to `MireDependencies` / `MireDependency`, with `load_manifest_dependencies`
  as the new loader helper name.
- Documentation in Mire and `mire-docs/` was aligned with the current `load`
  keyword, `--owl-home`, and `[dependencies]` naming.

---

## [3.11.8] - 2026-06-02

### Added
- `load helper` now auto-discovers local root modules from the project root
  using direct-file and module-directory candidates.
- Regression coverage for root-level module discovery and legacy import
  deprecation warnings.

### Changed
- `import` remains as a legacy alias for `load`, with analyzer support for
  deprecation warnings and the `--allow-legacy-imports` escape hatch.

---

## [3.11.7] - 2026-06-02

### Added
- New `load` module keyword, with `import` retained as a legacy alias during
  the migration window.
- `--allow-legacy-imports` CLI flag to silence legacy import warnings when
  working with old sources.

### Changed
- Module loading is now surfaced as `Statement::Load` in the compiler AST and
  analysis pipeline, with legacy imports tracked separately for warnings.
- Built-in module documentation now calls out `load` as the preferred source
  keyword.

---

## [3.11.6] - 2026-06-02

### Fixed
- Bundled Kioto resolution no longer hijacks `import kioto: (...)`; local
  `kioto` modules now win again while `std` keeps using the package-aware
  bundled/Owl lookup.

---

## [3.11.5] - 2026-06-02

### Added
- Package-aware Kioto resolution now prefers the configured Owl cache root via
  `--owl-home` before bundled fallbacks, and `mire test` honors the same flag.
- `owl.toml` manifests now serialize dependencies under `[dependencies]` while
  still reading older `[imports]` manifests for backward compatibility.
- Regression coverage for Owl-home package resolution.

### Changed
- `ImportMode::Legacy` and the `--import-mode` CLI flag were removed; the build
  pipeline now uses the single reachable import mode everywhere.
- Kioto module resolution now routes `std`, `strings`, `lists`, `math`, and
  the other bundled submodules through package-aware lookup instead of the old
  `std_entry_path()` / `resolve_module_path()` fallback chain.
- `mire import` writes `[dependencies]` in `owl.toml` instead of `[imports]`.

---

## [3.11.4] - 2026-06-02

### Added
- Kioto `math` core module now split into submodules (`basic`, `stats`, `random`)
  with a shared `_externs.mire` for runtime extern declarations.
- Runtime C helpers for `rt_math_random`, `rt_math_random_range`.
- Multi-level namespace resolution in the type checker: names like
  `math.complex.new` are now resolved by iteratively stripping namespace
  prefixes (`math.complex.new` → `complex.new` → `new`) until a match is
  found in the flat function table.

### Changed
- `src/modules/kioto/ext/iter/mod.mire` rewritten to lower through `rt_lists_*`
  externs directly instead of calling `lists.len`, avoiding a name-resolution
  collision between the `lists` and `strings` modules in certain import
  configurations.
- `decimal.mire` now uses `rt_strings_*` externs directly for string
  manipulation (strip, substr, index_of, pad_left), removing its dependency
  on `import ./../strings` and the associated name-resolution instability.

### Fixed
- `rt_string_to_i64` implemented in `strings.c` and declared in `runtime.h`,
  fixing a linker error when `decimal.mire` is loaded.
- `strip_root_namespace` in the type checker now iterates across multiple
  dot-separated prefixes instead of stopping at the first level, so
  e.g. `kioto::fs::read` resolves correctly even after intermediate
  namespace components are stripped.

---

## [3.11.3] - 2026-06-02

### Added
- Kioto `strings` and `lists` now use reference-based read paths for shared
  bindings, with runtime C helpers for `strings.index_of`, `strings.repeat`,
  `lists.contains`, `lists.index_of`, `lists.reverse`, and `lists.unique`.
- Regression coverage for Kioto reference APIs on `strings` and `lists`.
- Manifest dependencies now serialize as `[dependencies]` with backward
  compatibility for existing `[imports]` manifests.

### Changed
- `strings.repeat` now lowers through `rt_strings_repeat` instead of an inline
  Mire loop, avoiding reference-vs-value type drift in the wrapper layer.
- `lists` read-only wrappers now borrow `&list` / `&vec[i64]` where appropriate
  so repeated reads do not consume the source binding.
- CLI import resolution now routes Kioto through package-aware resolution with
  `--owl-home` support and no `legacy` import mode.
- Kioto module docs now reflect the runtime-backed `strings` and `lists`
  surface.