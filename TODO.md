# TODO

Versioned copy of the workspace TODO. Keep this in sync with `../TODO.md`.

## Language gaps and failures

- [ ] Complete closures with real captures
  - Current state: closures lower to standalone functions, but captures are still empty.
  - Needed: captured environment representation, type propagation, codegen/runtime plumbing, and tests.

- [ ] Finish pipeline support end to end
  - Real syntax in this tree: `=>` and safe pipeline `=>?`.
  - Needed: keep parser/type checker/codegen behavior aligned, and remove legacy fallback paths when the lowering is fully stable.

- [ ] Complete enum variants with payloads
  - Current state: bare variants resolve correctly, payload variants still lose the payload binding path in some stages.
  - Needed: bind payloads through semantic, type checking, lowering, and codegen.

- [ ] Enforce function visibility across files and modules
  - Current state: private functions are still callable from other files in the temporary smoke test.
  - Needed: make visibility rules explicit and reject cross-module access to private functions while keeping same-file private access working.

- [ ] Decide and implement `try`, `ok`, `err`
  - Current state: parser/typeck already know about these forms, but the backend path is not complete.
  - Decision needed: either fully support them as first-class ergonomic sugar, or keep them as a deliberate non-goal if the language prefers explicit `match` and result handling.
  - If implemented: support `?`, `ok(...)`, and `err(...)` consistently across MIR and LLVM backends.

- [ ] Support dict and map literals end to end
  - Current state: parser and type checker understand them, but lowering/backend support is incomplete.
  - Needed: allocation strategy, key/value typing, runtime representation, and mutation semantics.

## Runtime and backend

- [ ] Remove remaining legacy lowering paths
  - Inline closure expansion still exists as a special-case path.
  - Goal: route function values through one representation instead of maintaining ad hoc call paths.

- [ ] Keep MIR and Avenys codegen feature parity
  - Any feature added in one backend should be wired in the other or explicitly marked unsupported.
  - This applies especially to closures, pipeline stages, enum payloads, and result helpers.

- [ ] Continue tightening build cleanliness
  - Keep `cargo check`, `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and the test suite green.
  - Preserve no-warning builds unless a warning is intentionally documented.

## Box plan

- [ ] Add a proper `box` feature to Mire
  - Candidate syntax: `box::T(value)` or a close variant that fits the existing namespaced style.
  - Goal: make heap-owned boxed values a first-class, well-supported language feature.

- [ ] Define the box data model
  - Need a clear runtime representation, ownership rules, and destructor behavior.
  - Box values should be useful in structs, returns, collections, and function arguments.

- [ ] Add parser and type-checker support for box
  - Decide whether `box` is a constructor, a type wrapper, or both.
  - Make diagnostics precise when the syntax or type is wrong.

- [ ] Add lowering and backend support for box
  - Generate the correct heap allocation and pointer handling in MIR and LLVM backends.
  - Ensure box operations do not create memory leaks or double-free paths.

- [ ] Add box-specific warnings
  - Use the existing warning system in `docs/ERROR_CODES.md` instead of inventing a new channel.
  - Good fits are performance/style/memory warnings, such as unnecessary allocation, suspicious discard, or unclear ownership.
  - If needed, add dedicated warnings for box misuse, but keep them in the same diagnostic model as the rest of the compiler.

## Diagnostics

- [ ] Expand lint-style warnings where useful
  - The compiler already has warning categories for unused, type, performance, style, complexity, logic, memory, and deprecated code.
  - Treat these as the project equivalent of "clippy-like" feedback.
  - Add new warning codes only when they improve code quality without blocking correctness.

## Documentation

- [ ] Keep docs synchronized with actual behavior
  - Update `docs/mir-pipeline.md`, `docs/LIBRARIES.md`, `docs/CHANGELOG.md`, and syntax docs when behavior changes.
  - Avoid decorative markers; use plain text statuses only.
