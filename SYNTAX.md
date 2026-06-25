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

## 2. Variables

```mire
# Immutable (default)
set name = "Mire"
set count = 42

# Mutable
set counter = 0 :i64 mut
set buffer = "" :str mut

# With explicit type
set x = 10 :i64
set active = true :bool

# Reassignment
set counter = counter + 1
set buffer = buffer + "more"
```

Types: `str`, `i64`, `bool`, `f64`, `vec[str]`, `vec[i64]`

---

## 3. Functions

```mire
# Public function
pub fn greet: (name :&str) {
    use dasu("Hello, " + *name)
}

# Private function (default)
fn add: (a :i64, b :i64) :i64 {
    return a + b
}

# No return value
pub fn main: () {
    set result = add(5 3)
    use dasu(result)
}
```

**Key rules:**
- Return type follows params: `fn name: (params) :return_type { }`
- Call arguments are **space-separated**: `add(5 3)` not `add(5, 3)`
- Param declarations support comma OR space: `(a :i64, b :i64)` or `(a :i64 b :i64)`

---

## 4. Control flow

```mire
# If/else — braces REQUIRED
if x > 0 {
    use dasu("positive")
} else {
    use dasu("non-positive")
}

# Nested conditions
if x > 10 {
    use dasu("large")
} else {
    if x > 0 {
        use dasu("small positive")
    } else {
        use dasu("negative")
    }
}

# While loop
set i = 0 :i64 mut
while i < 5 {
    use dasu(i)
    set i = i + 1
}

# For loop
for item in items {
    use dasu(item)
}
```

**`} else {` must be on the SAME line** — Mire requires the closing brace and `else {` on one line.

---

## 5. Strings

```mire
set s = "hello" :str mut
set upper = strings::upper(s)
set lower = strings::lower(s)
set trimmed = strings::trim("  ok  ")
set parts = strings::split("a,b,c" ",")
set sub = strings::substr("hello" 1 3)
set has = strings::contains("hello world" "world")
set pos = strings::index_of("hello world" "world")
set replaced = strings::replace("old" "old" "new")
set n = strings::len(s)
set num = strings::from_i64(42)

# Concatenation
set full = s + " world"

# Borrows
fn process: (input :&str) :str {
    set s = *input          # dereference &str → str
    return s + " done"
}
```

---

## 6. Collections

```mire
# Vectors (dynamic)
set items = strings::split("a b c" " ")   # vec[str]
set n = rt_vec_len(items)
set first = rt_vec_get_str(items 0)

# Iteration
set count = iter::count(items)
set has = iter::contains(items "b")

# Lists (i64)
set nums = [] :vec[i64] mut
set nums = lists::push(nums 42)
set val = lists::get(nums 0)
```

---

## 7. Structs

```mire
# Definition
pub struct Point {
    x :i64
    y :i64
}

# Construction
set p = (Point x: 10, y: 20)

# Field access
use dasu(p.x)
use dasu(p.y)

# With methods
impl Point {
    fn sum: (self) :i64 {
        return self.x + self.y
    }

    fn translate: (self, dx :i64, dy :i64) {
        set self.x = self.x + dx
        set self.y = self.y + dy
    }
}

# Method call
set total = p.sum()
```

**Struct literal syntax:** `(TypeName field: value, ...)` — parentheses required, field names use colon.

---

## 8. Enums

```mire
# Definition
pub enum Status {
    Pending
    Active
    Done
    Failed
}

pub enum Result {
    Ok(value :i64)
    Err(msg :str)
}

# Construction
set s = Status.Active
set r = Result.Ok(42)
set e = Result.Err("not found")

# Match
match s {
    Status.Pending { use dasu("waiting") }
    Status.Active  { use dasu("running") }
    Status.Done    { use dasu("complete") }
    Status.Failed  { use dasu("error") }
}
```

---

## 9. Skills (Traits)

```mire
pub skill Printable {
    fn print: (self) :str
}

pub skill Sized {
    fn size: (self) :i64
}

impl Printable for Point {
    fn print: (self) :str {
        return "(" + strings::from_i64(self.x) + ", " + strings::from_i64(self.y) + ")"
    }
}
```

---

## 10. Module system

### 10.1 Module declaration
```mire
module mymod

pub fn hello: () :str {
    return "hi"
}
```

### 10.2 Loading modules
```mire
load kioto              # loads the standard library
load mylib::core        # loads a submodule
```

### 10.3 Namespace access
```mire
strings::split(data "\n")     # standard library
json::get(response "key")      # kioto module
net::http::get("https://...")  # nested module
```

### 10.4 owl.toml exports
```toml
[exports]
mymod    = "path/to/mod"
strings  = "core/strings"
net      = "core/net"
```

Each intermediate directory needs its own `owl.toml` to expose submodules:

```
mylib/
  owl.toml         ← exports "net" = "core/net"
  core/
    net/
      owl.toml     ← exports "http" = "http/mod.mire"
      http/
        mod.mire   ← contains functions
```

---

## 11. External libraries (FFI)

### 11.1 Extern functions
```mire
extern lib "SDL2"
extern lib "mylib" "/usr/lib/libmylib.so"

extern fn SDL_Init: (flags :i64) :i64 lib "SDL2"
extern fn SDL_Quit: () lib "SDL2"
extern fn puts: (msg :*mut i8) :i32 lib "c"
```

### 11.2 Wrapping in safe functions
```mire
module sdl2

extern lib "SDL2"
extern fn SDL_Init: (flags :i64) :i64 lib "SDL2"
extern fn SDL_GetError: () :str lib "SDL2"

pub fn init_video: () :bool {
    return SDL_Init(0x00000020) == 0
}

pub fn get_error: () :str {
    return SDL_GetError()
}
```

**Supported FFI types:** `i64`, `str`, `bool`, `*mut i8`, `*const i8`

**Linking:** `extern lib "name"` maps to `-lname` at link time.
For `.so` files, add the path: `extern lib "name" "/usr/lib/libname.so"`.

---

## 12. Built-in functions

| Function | Description |
|----------|-------------|
| `use dasu(msg)` | Print to stdout |
| `proc_run(cmd)` | Run shell command, return output |
| `proc::spawn_shell(cmd)` | Spawn background process |
| `proc::wait(pid)` | Wait for spawned process |
| `strings::from_i64(n)` | i64 → str |
| `strings::to_i64(s)` | str → i64 |
| `strings::len(s)` | String length |
| `fs::read(path)` | Read file |
| `fs::write(path, data)` | Write file |
| `fs::exists(path)` | Check file exists |
| `rt_vec_len(v)` | Vector length |
| `rt_vec_get_str(v, i)` | Vector element |

---

## 13. Ownership & borrowing

```mire
# Borrows (&str) — read-only reference
pub fn print: (msg :&str) {
    use dasu(*msg)      # dereference with *
}

# Owned (str) — function takes ownership
fn consume: (data :str) {
    use dasu(data)
    # data freed here
}

# Mutable variables
set count = 0 :i64 mut      # :type mut
set buf = "" :str mut

# Mutable struct fields
struct Counter {
    value :i64 mut
}
```

**Key rule:** helper functions should take `&str` borrows, not `str` owned,
to avoid consuming the caller's value.

---

## 14. Comments

```mire
# Line comment
#!cfg::test              # test annotation
```

---

## 15. Common patterns

```mire
# Reading a file
set content = fs::read("input.txt")

# Writing a file
use fs::write("output.txt" content)

# HTTP GET
set data = net::http::get("https://api.example.com/data")

# JSON parsing
set name = json::get(data "user.name")
set age = json::get(data "user.age")

# Maybe (Option)
set m = maybe::some("value")
if maybe::is_some(m) {
    use dasu(maybe::unwrap(m))
}

# Result
set r = result::ok("success")
if result::is_ok(r) {
    use dasu(result::unwrap(r))
}

# Logging
use log::info("server started")
use log::warn("low memory")
use log::error("connection lost")

# WebSocket client
set fd = ws::connect("ws://echo.example.com/")
use ws::send::text(fd "hello")
set msg = ws::recv::all(fd)

# HTTP server
set method = net::http::req_method(raw)
set path = net::http::req_path(raw)
set ct = net::http::server_mime("style.css")

# SDL2
if sdl2::init_video() {
    set win = sdl2::create_window("Demo" 800 600)
    sdl2::delay(2000)
    sdl2::quit()
}
```
