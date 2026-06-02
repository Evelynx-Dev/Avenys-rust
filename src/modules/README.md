# Built-in Modules

This directory holds Mire's bundled Kioto standard library.

## Layout

```
src/modules/
├── kioto/                # Standard library surface
│   ├── mod.mire          # Aggregator: imports all sections
│   ├── core/
│   │   ├── async/mod.mire
│   │   ├── cpu/mod.mire
│   │   ├── dicts/mod.mire
│   │   ├── env/mod.mire
│   │   ├── fs/mod.mire
│   │   ├── gpu/mod.mire
│   │   ├── lists/mod.mire
│   │   ├── math/mod.mire
│   │   ├── mem/mod.mire
│   │   ├── proc/mod.mire
│   │   ├── strings/mod.mire
│   │   ├── term/mod.mire
│   │   ├── time/mod.mire
│   │   └── ...
│   ├── ext/
│   │   └── ...
└── README.md
```

## How externs work

Kioto modules declare `extern fn` signatures that map directly to `rt_*`
runtime functions or `pal_*` platform functions. The runtime core owns
platform-independent data structures and strings; PAL owns filesystem, process,
time, environment, CPU, memory, GPU, terminal, and I/O behavior.

Higher-level modules stay in Mire where possible. For example, `core/async`
currently exposes task-result helpers plus process-backed `spawn`/`join`
without adding new language syntax.

The read-heavy Kioto surfaces are now biased toward shared references:
`core/strings` and the read paths in `core/lists` borrow `&str`, `&list`, and
`&vec[i64]` so repeated reads avoid accidental moves while still lowering
through the existing `rt_*` runtime ABI.

## Import Management

Dependencies go in `owl.toml` under `[dependencies]`:

```toml
[project]
name = "my_project"
version = "0.1.0"
entry = "code/main.mire"

[dependencies]
kioto = { version = "0.2" }
my-lib = { path = "./lib/my-lib" }
```

Use `mire import <module> [--version <ver>] [--path <path>] [--json]` to add
entries. The default manifest is `owl.toml`.
