# Mire Language Reference

Version: **3.11.33** · 151 tests passing

---

## 1. First program

```mire
pub fn main: () {
    use dasu("Hello, Mire!")
}
```

Save as `hello.mire` and run: `mire run hello.mire`

---

## 2. Comments

```mire
# Line comment — runs to end of line

#!cfg::test           # marks the next function as a test
```

Mire uses `#` for single-line comments. There are no multi-line comment
delimiters; consecutive `#` lines serve the same purpose.

---

## 3. Variables

```mire
# Immutable (default)
set name = "Mire"
set count = 42

# Mutable — add `:type mut`
set counter = 0 :i64 mut
set buffer = "" :str mut

# With explicit type annotation
set x = 10 :i64
set active = true :bool

# Reassignment (variable must already exist)
set counter = counter + 1
set buffer = buffer + " more"
```

**`set` is the universal keyword** for both declaration and reassignment.
If the name doesn't exist, `set` declares it. If it exists, `set` reassigns.

**Compound assignment:** `set x += 1`, `set x -= 1`

**Types:** `str`, `i64`, `bool`, `f64`, `vec[str]`, `vec[i64]`

---

## 4. Ownership & borrowing

Ownership is the memory model that makes Mire safe without a garbage collector.
Every value has exactly one owner at a time. When the owner goes out of scope,
the value is freed.

### 4.1 Owned values (`str`)

```mire
fn consume: (data :str) {
    use dasu(data)
    # data is freed here — the function took ownership
}
```

When you pass an owned `str` to a function, the caller **loses access** to it:

```mire
pub fn main: () {
    set x = "hello" :str mut
    use consume(x)            # x is MOVED into consume
    use dasu(x)               # ❌ ERROR: use after move
}
```

The compiler catches this and reports `MSS Error: Use after move`.

### 4.2 Borrowed references (`&str`)

```mire
fn print: (msg :&str) {
    use dasu(*msg)            # dereference with * to get the str inside
}
```

A borrow lets a function read the value **without taking ownership**.
The caller keeps the value after the call:

```mire
pub fn main: () {
    set x = "hello" :str mut
    use print(x)              # x is borrowed, not moved
    use dasu(x)               # ✅ still works
}
```

### 4.3 Golden rule

> **If the function doesn't need to destroy the value, take `&str`.**

This is why all kioto helper functions use borrows:

```mire
# ✅ Good — reusable
fn helper: (s :&str) :str { return *s + "!" }

# ❌ Bad — consumes the caller's value
fn helper: (s :str) :str { return s + "!" }
```

### 4.4 Mutable variables and fields

```mire
set count = 0 :i64 mut       # :type mut on a variable

struct Counter {
    value :i64 mut            # mut on a struct field
}
```

---

## 5. Functions

### 5.1 Declaration

```mire
# Public function
pub fn greet: (name :&str) {
    use dasu("Hello, " + *name)
}

# Private function (default — omit `pub`)
fn add: (a :i64, b :i64) :i64 {
    return a + b
}

# No parameters, no return
pub fn main: () {
    set result = add(5 3)
    use dasu(result)
}
```

The return type goes after the closing paren: `fn name: (params) :return_type { }`

### 5.2 Function calls — space-separated arguments

In Mire, arguments at call sites are **separated by spaces**, not commas:

```mire
add(5 3)              # ✅ two arguments
strings::split(s "\n") # ✅ two arguments
```

**Commas also work** — the parser accepts both `foo(a b)` and `foo(a, b)`.
The Mire codebase convention is space-separated, and that's the style used
throughout kioto and owl. The comma style exists for familiarity but it's
not the canonical idiom.

| Context | Separator | Example |
|---------|-----------|---------|
| Parameter declarations | Comma or space | `(a :i64, b :i64)` or `(a :i64 b :i64)` |
| Function call arguments | Space (comma tolerated) | `add(5 3)` or `add(5, 3)` |

### 5.3 Return

```mire
return value
return                # void return (allowed but optional)
```

---

## 6. Control flow

```mire
# If/else
if x > 0 {
    use dasu("positive")
} else {
    use dasu("non-positive")
}

# While
set i = 0 :i64 mut
while i < 5 {
    use dasu(i)
    set i = i + 1
}

# For
for item in items {
    use dasu(item)
}

# Match
match status {
    Status.Ok(v)  { use dasu("ok: " + strings::from_i64(v)) }
    Status.Err(m) { use dasu("error: " + *m) }
}
```

**`} else {` must be on the SAME line.** Mire does not allow `else` on a new line:

```mire
# ✅ Correct
if cond {
    body
} else {
    other
}

# ❌ Wrong — `else` on new line
if cond {
    body
}
else {
    other
}
```

---

## 7. Strings

```mire
set s = "hello" :str mut

# Transformation
set upper = strings::upper(s)
set lower = strings::lower(s)
set trimmed = strings::trim("  ok  ")
set replaced = strings::replace(s "hello" "hi")

# Splitting and joining
set parts = strings::split("a,b,c" ",")
set joined = strings::join(parts "|")

# Substring and search
set sub = strings::substr("hello" 1 3)   # "ell"
set has = strings::contains(s "ll")       # true
set pos = strings::index_of(s "ll")       # 2

# Length and conversion
set n = strings::len(s)                   # 5
set num_str = strings::from_i64(42)       # "42"
set val = strings::to_i64("42")           # 42

# Concatenation
set full = s + " world"
```

---

## 8. Collections

```mire
# Vectors (dynamic) — from strings::split
set items = strings::split("a b c" " ")
set n = rt_vec_len(items)
set first = rt_vec_get_str(items 0)

# Iteration helpers
set count = iter::count(items)
set has = iter::contains(items "b")

# Lists of i64
set nums = [] :vec[i64] mut
set nums = lists::push(nums 42)
set val = lists::get(nums 0)
```

**Access patterns:**
- Vector elements: `rt_vec_get_str(vec index)` or `rt_vec_get_i64(vec index)`
- Vector length: `rt_vec_len(vec)`
- List operations: `lists::push`, `lists::get`, `lists::len`, `lists::pop`

---

## 9. Structs

### 9.1 Definition and construction

```mire
pub struct Point {
    x :i64
    y :i64
}

set p = (Point x: 10, y: 20)
set q = (Point x: 3, y: 4)
```

### 9.2 Why parentheses?

Struct construction syntax is `(TypeName field: value, ...)` with mandatory
parentheses. This is a deliberate design choice for parser consistency:

- Without parentheses, `Point x: 10` would be ambiguous — is it a variable
  declaration `set Point = ...`? A named argument? A type annotation?
- Mire uses `(...)` as the **universal grouping token**: function parameters,
  expression grouping, tuple-like values, AND struct construction all share
  the same delimiter.
- The parser sees `(TypeName ...)` and unambiguously knows it's a struct
  literal. No new syntax token needed.

```mire
# ✅ Struct construction inside parentheses
set p = (Point x: 1, y: 2)

# ❌ Ambiguous without parentheses
set p = Point x: 1, y: 2
```

### 9.3 Methods

```mire
impl Point {
    fn sum: (self) :i64 {
        return self.x + self.y
    }

    fn translate: (self, dx :i64, dy :i64) {
        set self.x = self.x + dx
        set self.y = self.y + dy
    }
}

set total = p.sum()
```

---

## 10. Enums

```mire
# Definition — simple variants
pub enum Status {
    Pending
    Active
    Done
    Failed
}

# Definition — variants with payloads
pub enum Result {
    Ok(value :i64)
    Err(msg :str)
}

# Construction
set s = Status.Active
set r = Result.Ok(42)
set e = Result.Err("not found")

# Pattern matching
match s {
    Status.Pending { use dasu("waiting") }
    Status.Active  { use dasu("running") }
    Status.Done    { use dasu("complete") }
    Status.Failed  { use dasu("error") }
}
```

---

## 11. Skills (Traits)

```mire
# Trait declaration
pub skill Printable {
    fn print: (self) :str
}

pub skill Sized {
    fn size: (self) :i64
}

# Trait implementation
impl Printable for Point {
    fn print: (self) :str {
        set x = strings::from_i64(self.x)
        set y = strings::from_i64(self.y)
        return "(" + x + ", " + y + ")"
    }
}
```

---

## 12. Module system

### 12.1 Declaring a module

```mire
module mylib

pub fn version: () :str {
    return "1.0"
}
```

### 12.2 Loading modules

```mire
load kioto               # the standard library
load mylib               # a user library
load kioto::json         # a specific submodule
```

### 12.3 Namespace access

```mire
strings::split(data "\n")       # standard library
json::get(response "key")       # kioto submodule
net::http::get("https://...")   # nested module (3 levels)
```

### 12.4 owl.toml exports

```toml
[project]
name = "mylib"
version = "0.1.0"
entry = "mod.mire"

[exports]
strings  = "core/strings"
net      = "core/net"
json     = "ext/json"
```

**Each intermediate directory needs its own `owl.toml`** to expose submodules
to deeper nesting:

```
mylib/
  owl.toml           ← exports "net" = "core/net"
  core/
    net/
      owl.toml       ← exports "http" = "http/mod.mire"
      http/
        mod.mire     ← contains pub fn get, pub fn post
```

Without `core/net/owl.toml`, the path `mylib::net::http` cannot resolve.

---

## 13. External libraries (FFI)

### 13.1 Extern declarations

```mire
extern lib "SDL2"                        # single-token: name = link flag
extern lib "mylib" "/usr/lib/libx.so"   # two-token: name + path

extern fn SDL_Init: (flags :i64) :i64 lib "SDL2"
extern fn SDL_Quit: () lib "SDL2"
extern fn puts: (msg :*mut i8) :i32 lib "c"
```

### 13.2 Wrapping in safe functions

```mire
module sdl2

extern lib "SDL2"
extern fn SDL_Init: (flags :i64) :i64 lib "SDL2"
extern fn SDL_GetError: () :str lib "SDL2"

pub fn init_video: () :bool {
    return SDL_Init(0x00000020) == 0
}

pub fn error_message: () :str {
    return SDL_GetError()
}
```

**Supported FFI types:** `i64`, `str`, `bool`, `*mut i8`, `*const i8`

**Linking:** `extern lib "name"` adds `-lname` to the linker. For `.so` files,
add the full path: `extern lib "name" "/usr/lib/libname.so"` (adds `-L/path`).

See [`docs/FFI.md`](docs/FFI.md) for the complete FFI reference.

---

## 14. Built-in functions

Mire's core I/O primitives use Japanese verb names as a deliberate design
choice — short, unambiguous, and visually distinct from English keywords.

| Function | Origin | Description |
|----------|--------|-------------|
| `use dasu(msg)` | 出す (*dasu*, "put out") | Print to stdout |
| `set line = ireru()` | 入れる (*ireru*, "put in") | Read line from stdin (legacy codegen only) |

`ireru` is parsed and type-checked but the MIR codegen path (default) does not
emit its runtime symbol yet. Use `MIRE_LEGACY_CODEGEN=1` to access it.

The rest of the standard library (kioto) uses English names: `log::info`,
`fs::read`, `net::http::get`, etc. Only the two I/O primitives use Japanese.

### Full reference

| Function | Description |
|----------|-------------|
| `proc_run(cmd)` | Run shell command, capture stdout |
| `proc::spawn_shell(cmd)` | Spawn background process (returns pid) |
| `proc::wait(pid)` | Wait for spawned process |
| `strings::from_i64(n)` | i64 → str |
| `strings::to_i64(s)` | str → i64 |
| `strings::len(s)` | String length |
| `fs::read(path)` | Read file contents |
| `fs::write(path, data)` | Write string to file |
| `fs::exists(path)` | Check if file exists |
| `rt_vec_len(v)` | Vector length |
| `rt_vec_get_str(v, i)` | Vector element (indexed) |

---

## 15. Common patterns — Basic

```mire
load kioto

pub fn main: () {
    # ── File I/O ──
    set content = fs::read("input.txt")
    use fs::write("output.txt" content)

    # ── JSON ──
    set data = "{\"user\":{\"name\":\"Alice\",\"age\":30}}"
    set name = json::get(data "user.name")       # "Alice"
    if json::is_valid(data) { ... }

    # ── String manipulation ──
    set parts = strings::split("a,b,c" ",")
    set trimmed = strings::trim("  ok  ")
    set has = strings::contains("hello world" "world")

    # ── Maybe (Option) ──
    set m = maybe::some("value")
    set v = maybe::unwrap_or(m "default")
    if maybe::is_some(m) { use dasu(maybe::unwrap(m)) }

    # ── Result ──
    set r = result::ok("success")
    if result::is_ok(r) { use dasu(result::unwrap(r)) }
    set v2 = result::unwrap_or(result::err("fail") "fallback")

    # ── Logging ──
    use log::info("server started")
    use log::warn("low memory")
    use log::error("connection lost")

    # ── Iteration ──
    set range = iter::range(0 5)          # "0\n1\n2\n3\n4"
    set rparts = strings::split(range "\n")
    set count = iter::count(rparts)       # 5
}
```

---

## 16. Common patterns — Advanced

```mire
load kioto

pub fn main: () {
    # ── HTTP client ──
    set body = net::http::get("https://api.example.com/data")
    set resp = net::http::post("https://api.example.com/create" body_json "application/json")

    # ── HTTP server ──
    set method = net::http::req_method(raw_request)
    set path = net::http::req_path(raw_request)
    set host = net::http::req_header(raw_request "Host")
    set qs = net::http::req_query(raw_request)
    set ct = net::http::server_mime("style.css")
    set response = net::http::resp_200(data ct)

    # ── WebSocket client ──
    set fd = ws::connect("ws://echo.example.com/")
    use ws::send::text(fd "hello")
    set msg = ws::recv::all(fd)
    use ws::close(fd)

    # ── SDL2 (graphics) ──
    if sdl2::init_video() {
        set win = sdl2::create_window("Demo" 800 600)
        sdl2::delay(2000)
        sdl2::destroy_window(win)
        sdl2::quit()
    }
}
```
