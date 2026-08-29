# Mire Runtime Symbol Reference

This document maps Mire language features to the runtime (`rt_*`) and PAL (`pal_*`)
symbols they require. Use this to understand what the `minimal` tier will compile,
and to write freestanding (`runtime = "none"`) programs.

## Symbol Categories

| Prefix | Category | Tier Required |
|--------|----------|---------------|
| `rt_managed_*` | Memory management (string/struct allocation) | minimal |
| `rt_panic_loc` | Panic with source location | minimal |
| `rt_div_i64`, `rt_rem_i64` | Integer division/modulo with div-by-zero check | minimal |
| `rt_check_bounds_i64` | Array/vector bounds checking | minimal |
| `rt_*_to_string` | Integer/float/bool → string conversion | minimal |
| `rt_string_*` | String operations (concat, substr, etc.) | minimal |
| `rt_list_*`, `rt_vecs_*` | Vector operations (push, get, len, etc.) | minimal |
| `rt_dict_*`, `rt_maps_*`, `rt_dicts_*` | Map/dict operations | minimal |
| `rt_math_*` | Math functions (sqrt, pow, trig, etc.) | minimal |
| `rt_math_random_*` | Random number generation | minimal |
| `rt_result_*` | Result[T,E] operations (ok, err, unwrap, etc.) | minimal |
| `rt_arr_*` | Fixed-size array operations | minimal |
| `rt_closure_env_*` | Closure environment handling | minimal |
| `rt_thread_*` | Thread spawning | minimal |
| `rt_get_args`, `rt_free_argv` | Command-line argument access | minimal |
| `rt_hex_to_file`, `rt_free_raw`, `rt_blend_*`, `rt_read_*`, `rt_write_*` | Helpers (I/O, crypto, etc.) | minimal |
| `pal_*` | All PAL capabilities (filesystem, process, crypto, etc.) | minimal + PAL |

## Feature → Symbol Mapping

### Core Language (always needed)
| Feature | Runtime Symbols | PAL Symbols |
|---------|----------------|-------------|
| Integer arithmetic (`+`, `-`, `*`, `/`, `%`) | `rt_div_i64`, `rt_rem_i64`, `rt_panic_loc` | — |
| Bounds checking (arrays, vectors) | `rt_check_bounds_i64`, `rt_panic_loc` | — |
| Panic / `?` on `Result`/`Maybe` | `rt_panic_loc` | — |
| Memory management (strings, structs) | `rt_managed_alloc`, `rt_managed_free`, `rt_managed_ensure_managed`, `rt_managed_contains`, `rt_managed_retain`, `rt_managed_from_cstr`, `rt_managed_from_slice`, `rt_managed_is_managed`, `rt_managed_len`, `rt_managed_printf_i64`, `rt_managed_printf_f64` | — |

### String Operations
| Feature | Runtime Symbols |
|---------|----------------|
| `str::concat` / `+` on strings | `rt_string_concat` |
| `str::len` | `rt_strings_len` (elided for literals) |
| `str::substr` | `rt_strings_substr` |
| `str::index` / `str::char_at` | `rt_strings_char_at` |
| `str::contains` / `starts_with` / `ends_with` | `rt_strings_contains`, `rt_strings_starts_with`, `rt_strings_ends_with` |
| `str::replace` / `replace_first` | `rt_strings_replace`, `rt_strings_replace_first` |
| `str::split` | `rt_strings_split`, `rt_strings_split_list` |
| `str::join` | `rt_strings_join`, `rt_strings_join_list` |
| `str::pad_left` / `pad_right` | `rt_strings_pad_left`, `rt_strings_pad_right` |
| `str::trim` / `strip` | `rt_strings_strip`, `rt_strings_trim` |
| `str::to_upper` / `to_lower` | `rt_strings_to_upper`, `rt_strings_to_lower` |
| `str::repeat` | `rt_strings_repeat` |
| `str::copy` | `rt_string_copy` |
| `str::from::i64` / `i128` / `f64` / `bool` | `rt_i64_to_string`, `rt_i128_to_string`, `rt_f64_to_string`, `rt_bool_to_string` |
| `str::to_i64` | `rt_string_to_i64` |

### Vector Operations (`vec`)
| Feature | Runtime Symbols |
|---------|----------------|
| `vec::len` | `rt_vecs_len` / `rt_list_len` |
| `vec::push` | `rt_list_push_i64`, `rt_list_push_ptr`, `rt_list_push_scalar` |
| `vec::get` / `index` | `rt_list_get_i64`, `rt_list_get_ptr`, `rt_vecs_get_str` |
| `vec::set` | `rt_vecs_set_i64` |
| `vec::first` / `last` | `rt_lists_first`, `rt_lists_last` |
| `vec::pop` | `rt_list_pop_i64` |
| `vec::remove` | `rt_list_remove` |
| `vec::clear` | `rt_list_clear` |
| `vec::slice` | `rt_list_slice` |
| `vec::concat` | `rt_list_concat` |
| `vec::reverse` | `rt_lists_reverse` |
| `vec::unique` | `rt_lists_unique` |
| `vec::sort` | `rt_lists_sort` (if implemented) |
| `vec::flatten` | `rt_lists_flatten` (if implemented) |

### Map/Dict Operations
| Feature | Runtime Symbols |
|---------|----------------|
| `map::len` | `rt_dicts_len` / `rt_dict_len` |
| `map::has` | `rt_dict_has` |
| `map::get` | `rt_dict_get_i64`, `rt_maps_get_str` |
| `map::set` | `rt_dicts_set_i64`, `rt_maps_set_str` |
| `map::remove` | `rt_dicts_remove` |
| `map::keys` | `rt_dict_keys` |
| `map::values` | `rt_dict_values` / `rt_dicts_values_i64` |
| `map::entries` | `rt_dicts_entries` |
| `map::merge` | `rt_maps_merge` |
| `map::is_empty` | `rt_dicts_is_empty` |

### Math Functions
| Feature | Runtime Symbols |
|---------|----------------|
| `math::sqrt` | `rt_math_sqrt` |
| `math::pow` | `rt_math_pow` |
| `math::round` / `floor` / `ceil` | `rt_math_round`, `rt_math_floor`, `rt_math_ceil` |
| `math::sin` / `cos` / `tan` / `asin` / `acos` / `atan` / `atan2` | `rt_math_sin`, `rt_math_cos`, etc. |
| `math::exp` / `log` / `log10` | `rt_math_exp`, `rt_math_log`, `rt_math_log10` |
| `math::abs` / `min` / `max` / `clamp` | `rt_math_abs`, `rt_math_min`, `rt_math_max`, `rt_math_clamp` |
| `math::random::*` | `rt_math_random_u64`, `rt_math_random_f64`, `rt_math_random_range_i64`, `rt_math_random_range_f64`, `rt_math_random_bool`, `rt_math_random_seed` |

### Result[T, E] Operations
| Feature | Runtime Symbols |
|---------|----------------|
| `Ok(x)` / `Err(e)` | `rt_result_ok_i64`, `rt_result_ok_str`, `rt_result_ok_ptr`, `rt_result_err_i64`, etc. |
| `result.is_ok()` / `is_err()` | `rt_result_is_ok`, `rt_result_is_err` |
| `result.unwrap()` / `?` | `rt_result_unwrap_i64`, `rt_result_unwrap_str`, etc. |
| `result.unwrap_or(default)` | `rt_result_unwrap_or_i64`, etc. |
| `result.err_payload()` | `rt_result_err_payload` |
| `result.free()` | `rt_result_free` |

### Array Operations (fixed-size)
| Feature | Runtime Symbols |
|---------|----------------|
| `len(arr)` | `rt_arr_len` (elided for literals → constant) |
| `arr[i]` indexing | `rt_check_bounds_i64` |
| Array literal → struct field | `rt_arr_*` constructors |

### Maybe[T] Operations (unboxed, **no runtime calls**)
| Feature | Implementation |
|---------|----------------|
| `Some(x)` | Inline `{i1 1, x}` struct construction |
| `None` | Inline `{i1 0, zero}` struct construction |
| `maybe.is_some()` | `ExtractValue(tag) == 1` |
| `maybe.is_none()` | `ExtractValue(tag) == 0` |
| `maybe.unwrap()` | `ExtractValue(value)` (assumes Some) |
| `maybe.unwrap_or(default)` | `Select(ExtractValue(tag)==1, value, default)` |
| `?` on `Maybe` | Inline tag check + branch |

### Closure / Function Values
| Feature | Runtime Symbols |
|---------|----------------|
| Closure creation | `rt_closure_env_create` |
| Closure call | `rt_closure_env_call` |
| Function pointer | (handled by codegen, no runtime) |

### Threading
| Feature | Runtime Symbols | PAL Symbols |
|---------|----------------|-------------|
| `thread::spawn` | `rt_thread_spawn_closure` | `pal_thread_create` |

### Command-line Arguments
| Feature | Runtime Symbols |
|---------|----------------|
| `env::args()` | `rt_get_args`, `rt_free_argv` |

### I/O Helpers
| Feature | Runtime Symbols |
|---------|----------------|
| `fs::read` (raw) | `rt_read_file` (PAL `pal_fs_read_file`) |
| `fs::write` (raw) | `rt_write_file` (PAL `pal_fs_write_file`) |
| Hex encoding | `rt_hex_to_file` |
| Blend/alpha | `rt_blend_*` |

### PAL Capabilities (require `pal_*` symbols)
| Capability | PAL Symbols | C File |
|------------|-------------|--------|
| Filesystem (read, write, list, remove, mkdir) | `pal_fs_*` | `pal/core/fs.c`, `pal/{linux,darwin,windows}/fs.c` |
| Process (spawn, exec, capture) | `pal_proc_*` | `pal/core/proc.c`, `pal/{platform}/proc.c` |
| Channels (create, send, recv) | `pal_channel_*` | `pal/core/channel.c`, `pal/{platform}/channel.c` |
| Sockets (connect, send, recv) | `pal_socket_*` | `pal/core/socket.c`, `pal/{platform}/socket.c` |
| Listeners (bind, accept) | `pal_listener_*` | `pal/core/listener.c`, `pal/{platform}/listener.c` |
| Crypto (hash, sign, verify, random) | `pal_crypto_*` | `pal/core/crypto.c`, `pal/{platform}/crypto.c` |
| Time (now, sleep, elapsed) | `pal_time_*` | `pal/core/time.c`, `pal/{platform}/time.c` |
| Environment (vars, cwd) | `pal_env_*` | `pal/core/env.c`, `pal/{platform}/env.c` |
| Secrets / keys | `pal_secret_*`, `pal_pubkey_*` | `pal/core/secret.c`, `pal/{platform}/secret.c` |

## Minimal Tier: What Gets Compiled

For a program using only **integer math + strings + vectors**:
```
Runtime .c files: managed.c, safety.c, strings.c, vecs.c, math.c, helpers.c
PAL .c files:     (none, if no PAL capabilities used)
Linker flags:     -lm -lc (no -lssl, -lcrypto, -lsodium)
```

For a program using **filesystem + process**:
```
Runtime .c files: managed.c, safety.c, strings.c, vecs.c, math.c, helpers.c, thread.c
PAL .c files:     pal/core/fs.c, pal/linux/fs.c, pal/core/proc.c, pal/linux/proc.c, ...
Linker flags:     -lm -lssl -lcrypto -lsodium -lpthread
```

## Freestanding (`runtime = "none"`)

**Allowed:**
- Pure integer/float math
- Control flow (if, while, for, match)
- Structs/enums (no methods that need runtime)
- `dasu` (print) — emits direct LLVM `call @printf`
- `str::from::i64` etc. — **NOT allowed** (needs `rt_i64_to_string`)

**Forbidden (compile error):**
- Any `rt_*` call (strings, vectors, maps, math, etc.)
- `?` on `Result`/`Maybe` (needs runtime unwrap)
- String operations
- Vector/map operations
- Array bounds checking (if not elided)

## Minimal Tier Decision Flow

```
Does the program use PAL capabilities?
  ├─ NO → Only runtime .c files needed
  │       ├─ Strings? → strings.c (+ vecs.c, managed.c, safety.c)
  │       ├─ Vectors? → vecs.c (+ managed.c, safety.c, strings.c for joins)
  │       ├─ Maps? → maps.c (+ vecs.c, managed.c, safety.c)
  │       ├─ Math? → math.c (+ vecs.c)
  │       └─ Random? → random.c
  │
  └─ YES → Add PAL .c files for each capability used
          → Link crypto libs (-lssl -lcrypto -lsodium)
```

## Derive — `@[derive(...)]` Runtime Symbols

When using `@[derive(...)]`, the generated `impl` blocks reference stdlib functions.
These are **not** built-in compiler symbols — they are ordinary Mire stdlib calls that
must be reachable-import-selected.

| Derive | Required stdlib symbols (must `load mire::str` or `load kioto`) |
|--------|---------------------------------------------------------------|
| `Default` | None (only uses literal zero-expressions) |
| `Clone` | `str::copy` for `str` fields |
| `PartialEq` | `==` on primitive fields (no runtime call) |
| `Debug` | `str::copy`, `str::from::i64`, `str::from::f64`, `str::from::bool`, `str::concat` (`+`) |

**Note**: The derive expansion runs before reachable-import selection, so these symbols
are automatically included in the dependency candidates when the corresponding derive
is used. You still need `load mire::str` in your source.

```
Derive used? → YES → Adds required stdlib symbols to dependency candidates
                      ↓
            Reachable-import selection includes them from mire::str/kioto
```

---

## Verifying Your Build

```bash
# See which .c files are compiled (minimal tier)
mire build code/main.mire --release --verbose --no-analysis-cache 2>&1 | grep "R3.2"

# Check linked libraries
ldd bin/release/main

# Check for PAL symbols in binary
nm bin/release/main | grep pal_
```

## Summary Table

| Program Type | .c Files | Libraries |
|--------------|----------|-----------|
| Pure math | 2 (managed, safety) | `-lm -lc` |
| Math + strings | 4–5 | `-lm -lc` |
| Math + strings + vectors | 5–6 | `-lm -lc` |
| Math + vectors + maps | 6–7 | `-lm -lc` |
| + filesystem | + PAL fs | `-lssl -lcrypto -lsodium` |
| + process | + PAL proc | `-lssl -lcrypto -lsodium` |
| + crypto | + PAL crypto | `-lssl -lcrypto -lsodium` |
| Full stdlib | 15 | all |

---

*Generated for Mire v3.24.31+ with zero-cost runtime tiers.*