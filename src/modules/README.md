# Built-in Modules Source Layout

This directory contains built-in module definitions used by Mire.

## Layout

```
src/modules/
├── std/                  # Standard Library (canonical)
│   ├── mod.mire          # Aggregator: import __std_all__
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
│   ├── lib.mire
│   ├── cpu.mire
│   ├── dicts.mire
│   ├── env.mire
│   ├── fs.mire
│   ├── iter.mire
│   ├── lists.mire
│   ├── math.mire
│   ├── maybe.mire
│   ├── mem.mire
│   ├── proc.mire
│   ├── result.mire
│   ├── strings.mire
│   ├── term.mire
│   ├── time.mire
│   ├── tuple.mire
│   └── types.mire
└── README.md             # This file
```

## Conventions

- `src/modules/std/<section>/mod.mire` — Standard Library section modules
- `src/modules/std/mod.mire` — std aggregator (imports all sections)
- All `__kioto_*` extern functions are declared in `std/` and implemented in C (`src/avens/runtime_support.c`)

## Import Management

Dependencies can be declared in `owl.toml` under `[imports]`:

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
