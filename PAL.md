# PAL — Platform Abstraction Layer

## Architecture

```
Mire Program
     │
     ▼
┌─────────────────────┐
│   Kioto (Mire)      │  Higher-level logic (reverse, unique, filter, ...)
│   Core + Extensions  │  Implemented in Mire where possible
└─────────┬───────────┘
          │ extern fn __kioto_*
          ▼
┌─────────────────────┐
│   Runtime Core      │  Platform-independent data structures
│   (C)               │  managed, strings, lists, dicts
└─────────┬───────────┘
          │ pal_*()
          ▼
┌─────────────────────┐
│   PAL (C)           │  Platform Abstraction Layer
│   linux/  win32/    │  fs, env, proc, time, cpu, mem, gpu, term
└─────────┬───────────┘
          │ OS calls
          ▼
┌─────────────────────┐
│   Operating System  │  Linux, Windows, WASM, ...
└─────────────────────┘
```

## Goal

Replace the monolithic `runtime_support.c` with a clean separation:

- **Runtime Core** — data structures (managed strings, lists, dicts). Pure C, no OS calls.
- **PAL** — OS/platform interface (fs, env, proc, time, cpu, mem, gpu, term). Platform-specific backends.
- **Kioto** — higher-level logic in Mire (reverse, unique, contains, repeat, ...).

## Directory Layout

```
src/
  runtime/                  # Platform-independent
    runtime.h               # Umbrella header
    managed.h / managed.c   # Ref-counted strings, GC
    strings.h / strings.c   # String primitives (concat, split, substr, len, cmp)
    lists.h / lists.c       # List primitives (len, get, set, push, pop, remove, clear)
    dicts.h / dicts.c       # Dict primitives (len, get, set, has, keys, values, remove)
    kioto_abi.c             # __kioto_* wrappers → runtime_*() / pal_*()

  pal/
    pal.h                   # Umbrella + platform selection (#ifdef)
    pal_fs.h / linux/pal_fs.c
    pal_env.h / linux/pal_env.c
    pal_proc.h / linux/pal_proc.c
    pal_time.h / linux/pal_time.c
    pal_cpu.h / linux/pal_cpu.c
    pal_mem.h / linux/pal_mem.c
    pal_gpu.h / linux/pal_gpu.c
    pal_term.h / linux/pal_term.c
    win32/                  # Future
    wasm/                   # Future
```

## Runtime Core (platform-independent)

| Module | File | Functions |
|--------|------|-----------|
| Managed | `managed.h/c` | `managed_alloc`, `managed_ref`, `managed_unref`, `managed_from_cstr`, `managed_from_slice`, `managed_len`, `managed_data` |
| Strings | `strings.h/c` | `concat`, `split`, `substr`, `len`, `cmp`, `to_upper`, `to_lower`, `trim`, `replace`, `contains`, `starts_with`, `ends_with`, `pad_left`, `pad_right` |
| Lists | `lists.h/c` | `len`, `get_i64`, `get_ptr`, `set_i64`, `set_ptr`, `push_i64`, `push_ptr`, `pop`, `remove`, `clear`, `concat` |
| Dicts | `dicts.h/c` | `len`, `get`, `set`, `has`, `keys`, `values`, `remove`, `is_empty` |

## PAL (platform-dependent)

| Module | Header | Functions |
|--------|--------|-----------|
| FS | `pal_fs.h` | `read`, `write`, `append`, `exists`, `is_dir`, `size`, `copy`, `move`, `drop`, `list`, `mkdir`, `rmdir`, `join`, `dir`, `name`, `ext` |
| Env | `pal_env.h` | `get`, `set`, `all`, `args`, `cwd`, `chdir` |
| Proc | `pal_proc.h` | `run`, `spawn`, `shell`, `wait`, `kill`, `exit`, `exists`, `pipe`, `stdin`, `read`, `write` |
| Time | `pal_time.h` | `unix_ms`, `unix_ns`, `sleep_ms`, `sleep_ns`, `mark`, `elapsed_ms`, `elapsed_ns` |
| CPU | `pal_cpu.h` | `time_ns`, `time_ms`, `mark`, `elapsed_ms`, `elapsed_ns`, `count`, `freq_mhz`, `cycles_est`, `loadavg`, `snapshot` |
| Mem | `pal_mem.h` | `used`, `total`, `free`, `available`, `percent`, `process`, `snapshot`, `format` |
| GPU | `pal_gpu.h` | `available`, `snapshot` |
| Term | `pal_term.h` | `style`, `hr`, `clear` |

## Implementation Phases

### Phase 0 — Fix clippy warnings
- `src/loader.rs:545` — collapse nested `if`
- `src/loader.rs:637` — remove unnecessary borrow

### Phase A — Split runtime_support.c → Runtime Core + PAL
- A1–A4: Create `src/runtime/{managed,strings,lists,dicts}.h/.c`
- A5–A13: Create `src/pal/{fs,env,proc,time,cpu,mem,gpu,term}.h` + `linux/*.c`
- A14: Create `src/runtime/kioto_abi.c` (thin shim mapping `__kioto_*` → `pal_*` / `runtime_*`)
- A15: Update `toolchain.rs` to compile `src/runtime/*.c` + `src/pal/linux/*.c`
- A16: **Delete `src/avens/runtime_support.c`**

### Phase B — Move logic from C to Mire (kioto)
- Implement `lists.reverse`, `lists.unique`, `lists.contains`, `lists.index_of` in Mire
- Implement `strings.repeat`, `strings.is_empty`, `strings.index_of` in Mire
- Add minimal get/set primitives to Runtime Lists if needed

### Phase C — Remove std, Kioto as sole library
- Add missing low-level extern fns to kioto (gpu, expanded time/cpu/mem/proc)
- Delete `src/modules/std/`
- Remove `__std_all__` magic from type checker + loader
- Backward compat: `import std` → kioto

### Phase D — Kioto Core + Extensions
- **Core** (frozen ABI): strings, fs, env, proc, time, cpu, mem, lists, dicts, gpu, term, math
- **Extensions** (open): iter, maybe, result, tuple, types, future modules

### Phase E — Owl test subcommand
- `mire test [paths...] [--all] [--no-run] [--verbose]`
- Auto-detect `tests/` directory in project root, scan for `*.mire`
- Compile each and report pass/fail
- Warning when tests exist but aren't run:
  ```
  warning: N test(s) in tests/ were not executed (use 'mire test --all' to run them)
  ```
- If no `tests/` dir:
  ```
  no extra tests found
  ```

### Phase F — Test strategy
- Rust `#[test]` for compiler-level regressions (existing)
- C unit tests for PAL (via `cc` crate)
- `mire test --all` for integration tests
- Manual smoke tests
