# PAL — Platform Abstraction Layer

## Architecture

```
Mire Program
     │
     ▼
┌─────────────────────┐
│  std/ (Kioto)       │  Higher-level logic (reverse, unique, filter, ...)
│  Core + Extensions  │  Implemented in Mire where possible
└─────────┬───────────┘
          │ extern fn __kioto_*
          ▼
┌─────────────────────┐
│  kioto_exports.c    │  TEMP shim: __kioto_* → rt_* / pal_*
│  (C, temporary)     │  Will be deleted once std/ is ported to rt_*/pal_*
└─────────┬───────────┘
          │ direct calls
          ▼
┌─────────────────────┐
│  Runtime Core (C)   │  Platform-independent: managed, strings, lists, dicts
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│  PAL (C)            │  Platform Abstraction Layer
│  linux/  wasm/      │  fs, env, proc, time, cpu, mem, gpu, term
└─────────┬───────────┘
          │ OS calls
          ▼
┌─────────────────────┐
│  Operating System   │  Linux, WASM, ...
└─────────────────────┘
```

The LLVM codegen emits `rt_*()` and `pal_*()` calls directly. The old
`@mire_*` symbols have been renamed to `@rt_*` / `@pal_*`. kioto_exports.c
is a temporary bridge for the `__kioto_*` externs that std/ still uses.
Once std/ modules are updated to call `rt_*` / `pal_*` directly (or are
replaced by inline Mire logic), the shim gets deleted.

## Directory Layout

```
src/
  runtime/                  # Platform-independent
    runtime.h               # Umbrella header for all runtime types
    managed.c               # Ref-counted strings
    strings.c               # String ops (concat, split, substr, contains, pad, ...)
    lists.c                 # List ops (create, push, pop, slice, concat)
    dicts.c                 # Dict ops (get, set, keys, values)
    kioto_exports.c         # TEMP: __kioto_* wrappers → rt_*() / pal_*()

  pal/
    pal.h                   # Umbrella header, platform selection
    linux/
      pal_fs.c
      pal_env.c
      pal_proc.c
      pal_time.c
      pal_cpu.c
      pal_mem.c
      pal_gpu.c
      pal_term.c
    wasm/                   # Stub implementations for WASM targets
```

## Runtime Core (platform-independent)

All declared in `runtime.h`:

| Module | Functions |
|--------|-----------|
| Managed | `rt_managed_alloc`, `rt_managed_ref`, `rt_managed_unref`, `rt_managed_from_cstr`, `rt_managed_from_slice`, `rt_managed_len`, `rt_managed_data`, `rt_managed_free` |
| Strings | `rt_string_copy`, `rt_string_concat`, `rt_string_append_owned`, `rt_i64_to_string`, `rt_bool_to_string`, `rt_f64_to_string`, `rt_string_to_upper`, `rt_string_to_lower`, `rt_strings_replace`, `rt_strings_replace_first`, `rt_strings_split_list`, `rt_strings_join`, `rt_strings_trim`, `rt_strings_starts_with`, `rt_strings_ends_with`, `rt_strings_contains`, `rt_strings_substr`, `rt_strings_pad_left`, `rt_strings_pad_right`, `rt_read_line`, `rt_get_args` |
| Lists | `rt_list_create`, `rt_list_push_i64`, `rt_list_push_scalar`, `rt_list_push_ptr`, `rt_list_pop_i64`, `rt_list_concat`, `rt_list_slice` |
| Dicts | `rt_dict_get_i64`, `rt_dict_get_ptr`, `rt_dict_set_i64`, `rt_dict_set_ptr`, `rt_dict_to_string`, `rt_dict_keys`, `rt_dict_values` |

## PAL (platform-dependent)

All declared in `pal.h`. Each backend implements the same signatures.

| Module | Functions |
|--------|-----------|
| FS | `pal_fs_write`, `pal_fs_append`, `pal_fs_read`, `pal_fs_copy`, `pal_fs_move`, `pal_fs_delete`, `pal_fs_mkdir`, `pal_fs_rmdir`, `pal_fs_exists`, `pal_fs_is_dir`, `pal_fs_size`, `pal_fs_list`, `pal_fs_join`, `pal_fs_dir`, `pal_fs_name`, `pal_fs_ext` |
| Env | `pal_env_get`, `pal_env_set`, `pal_env_all`, `pal_env_cwd`, `pal_env_chdir` |
| Proc | `pal_proc_run`, `pal_proc_exec`, `pal_proc_shell`, `pal_proc_wait`, `pal_proc_kill`, `pal_proc_exit`, `pal_proc_exists` |
| Time | `pal_time_unix_ms`, `pal_time_unix_ns`, `pal_time_since_ms`, `pal_time_since_ns`, `pal_time_sleep_ms`, `pal_time_sleep_ns`, `pal_time_mark`, `pal_time_elapsed_ms`, `pal_time_elapsed_ns` |
| CPU | `pal_cpu_time_ns`, `pal_cpu_time_ms`, `pal_cpu_mark`, `pal_cpu_elapsed_ms`, `pal_cpu_elapsed_ns`, `pal_cpu_count`, `pal_cpu_freq_mhz`, `pal_cpu_cycles_est`, `pal_cpu_loadavg`, `pal_cpu_snapshot` |
| Mem | `pal_mem_used`, `pal_mem_total`, `pal_mem_free`, `pal_mem_available`, `pal_mem_percent`, `pal_mem_process_bytes`, `pal_mem_format`, `pal_mem_snapshot` |
| GPU | `pal_gpu_snapshot` |
| Term | `pal_term_style`, `pal_term_hr`, `pal_term_clear` |
| I/O | `pal_io_print`, `pal_io_print_err`, `pal_io_readln` |

## ABI Map

`abi_map.toml` at the project root lists every `@mire_*` → `@rt_*` / `@pal_*`
mapping. It's the source of truth if you need to trace what happened during
the migration.

## What's Done

- Phase 0: Clippy warnings fixed
- Phase A: runtime_support.c split into Runtime Core + PAL + kioto_exports.c
- Phase B: All @mire_* symbols renamed to @rt_* / @pal_* in LLVM codegen.
  kioto_abi.c deleted. Build clean, 127 regressions pass.
- kioto_exports.c is the only remaining bridge (temporary).

## WASM Backend

`src/pal/wasm/` contains stub implementations for WASM targets.
Most functions return errors or empty results since WASM has no
real filesystem, process, or OS-level introspection. Time functions
use standard `clock_gettime` (available through WASI).

Select the WASM backend at build time:

```bash
MIRE_PAL=wasm cargo run -- run hello.mire
```

For WASM cross-compilation, pass `--target` to clang (not yet wired
automatically):

```bash
MIRE_PAL=wasm clang --target=wasm32-unknown-wasi ...
```

## What's Next

- Phase C: Update std/ modules to call rt_*/pal_* directly, delete kioto_exports.c
- Phase D: Move more C logic into Mire (kioto core modules)
- Phase E: Remove std/, promote kioto as sole library
