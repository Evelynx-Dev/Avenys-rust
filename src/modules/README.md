# Built-in Modules

This directory holds Mire's standard library (`std/`).

## Layout

```
src/modules/
├── std/                  # Standard library (canonical)
│   ├── mod.mire          # Aggregator: imports all sections
│   ├── cpu/mod.mire
│   ├── dicts/mod.mire
│   ├── env/mod.mire
│   ├── fs/mod.mire
│   ├── gpu/mod.mire
│   ├── lists/mod.mire
│   ├── math/mod.mire
│   ├── mem/mod.mire
│   ├── proc/mod.mire
│   ├── strings/mod.mire
│   ├── term/mod.mire
│   └── time/mod.mire
├── kioto/                # DEPRECATED — use std/ instead
│   └── ...
└── README.md
```

## How externs work

Each `std/<section>/mod.mire` declares `extern fn` signatures that map to
C functions prefixed `__kioto_*`. Those are implemented in
`src/runtime/kioto_exports.c`, which delegates to `rt_*` (runtime core) or
`pal_*` (platform layer) under the hood.

This is a temporary bridge. Eventually the `std/` modules will call `rt_*`
and `pal_*` directly, at which point `kioto_exports.c` gets deleted.

## Import Management

Dependencies go in `owl.toml` under `[imports]`:

```toml
[project]
name = "my_project"
version = "0.1.0"
entry = "code/main.mire"

[imports]
kioto = { version = "0.2" }
my-lib = { path = "./lib/my-lib" }
```

Use `mire import <module> [--version <ver>] [--path <path>]` to add entries.
