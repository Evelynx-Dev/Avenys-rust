# Mire Syntax Reference

Complete language syntax derived from test files and working examples.

---

## Table of Contents

1. [Minimal Program](#1-minimal-program)
2. [Bindings](#2-bindings)
3. [Functions](#3-functions)
4. [Structs](#4-structs)
5. [Impl and Methods](#5-impl-and-methods)
6. [Enums](#6-enums)
7. [Collections](#7-collections)
8. [Control Flow](#8-control-flow)
9. [Unsafe, Asm, Extern](#9-unsafe-asm-extern)
10. [Lifecycle Operations](#10-lifecycle-operations)
11. [Pipeline Operator](#11-pipeline-operator)
12. [I/O: dasu and ireru](#12-io-dasu-and-ireru)
13. [String Interpolation](#13-string-interpolation)
14. [Imports](#14-imports)
14. [Traits/Skills](#14-traitsskills)
15. [Operators](#15-operators)
16. [Ownership and References](#16-ownership-and-references)
17. [Types](#17-types)
18. [Stability](#18-stability)

---

## 1. Minimal Program

```mire
import std

pub fn main: () {
    use dasu("Hello Mire")
}
```

---

## 2. Bindings

```mire
set age = 25 :i64
set name = "mire" :str
set ready = true :bool
set total = 0 :i64 mut
set immutable = "constant" :str const
set counts = [] :vec[i64] mut
set counts = lists.push(counts 4)
```

Rules:
- `set` declares a binding
- Type annotations use `name :Type`
- `mut` enables reassignment of the binding
- `const` enforces immutability (compile-time constant)
- Commas are optional in most positions

### Compound Assignment

```mire
set x = 5 :i64 mut
set x += 3    # x = x + 3
set x -= 2    # x = x - 2
set x *= 4    # x = x * 4
set x /= 2    # x = x / 2
set x %= 3    # x = x % 3
```

---

## 3. Functions

```mire
fn add: (a :i64, b :i64) :i64 {
    return a + b
}

fn get_str: () :str {
    return "hello"
}

pub fn main: () {
    set result = add(5 3) :i64
}
```

Parameter and return types use `name :Type` syntax.
Visibility is private by default; use `pub` only for exported APIs.

### Closures

```mire
set double = (x :i64) => x * 2
set result = lists.map((x) => x * 2, [1 2 3])
```

### Function Generics

```mire
fn identity[T]: (x :T) :T {
    return x
}

set a = identity[i64](42)   # explicit generic argument
set b = identity("ok")      # inferred as T = str

skill Show {
    fn show: (self) :str
}

fn print_it[T: Show]: (x :T) {
    use dasu("ok")
}

impl[T] Box[T] {
    fn get: (self) :T {
        return self.value
    }
}
```

---

## 4. Structs

```mire
struct Point {
    x :i64
    y :i64
}

struct Box {
    width :i64
    height :i64
}

struct Stack {
    items :arr[i64 10]
    count :i64
}

struct Counter {
    value :i64 mut
    step :i64
}
```

### Construction

```mire
set p = (Point x: 1, y: 2)
set b = (Box width: 10, height: 20)
```

### Generic Nominal Types

```mire
type Box[T] {
    value :T
}

set b = Box[i64](42)
```

### Field Access and Mutation

```mire
use dasu(p.x)
set p.x = 5

impl Counter {
    fn increment: (self) {
        set self.value = self.value + self.step
    }
}
```

Struct fields declared with `mut` can be reassigned through `self` inside impl methods.

---

## 5. Impl and Methods

### Instance Methods (with explicit `self`)

```mire
impl Point {
    fn sum: (self) :i64 {
        return self.x + self.y
    }
}

use dasu(p.sum())
```

### Associated/Static Methods (with `::`)

```mire
impl Point {
    fn new: (x :i64, y :i64) :Point {
        return (Point x: x, y: y)
    }
}

set p = Point::new(1 2)
```

### Intentional Split

- `Enum.Variant(...)` for enum construction
- `Type::method(...)` for associated/static methods
- `value.method(...)` for instance methods

### Skill Implementation

```mire
impl Show for Box {
    fn show: (self) :str {
        return "Box"
    }
}
```

---

## 6. Enums

```mire
enum Color {
    Red
    Green
    Blue
}

enum Maybe {
    None
    Some(value :i64)
}

enum Status {
    Ok
    Error
    Loading(progress :i64, total :i64)
}
```

### Construction

```mire
set c = Color.Red
set m = Maybe.Some(value: 42)
set r = Result.Ok(42)
set s = Status.Loading(progress: 75, total: 100)
```

```mire
enum Option[T] {
    None
    Some(value :T)
}

set o = Option[i64].Some(7)
```

### Match Patterns

```mire
match c {
    Color.Red { use dasu("red") }
    Color.Green { use dasu("green") }
    Color.Blue { use dasu("blue") }
}

match m {
    Maybe.None { use dasu("nothing") }
    Maybe.Some(v) { use dasu(v) }
}
```

---

## 7. Collections

### Arrays (fixed-size)

```mire
set arr = [1 2 3] :arr[i64 3]
set first = arr at 0
set arr at 1 = 99
```

### Vectors (dynamic)

```mire
set counts = [] :vec[i64] mut
set counts = lists.push(counts 4)
set first = lists.get(counts 0)
```

### Dicts/Maps

```mire
set m = {a: 1, b: 2} :map[str i64]
```

Bare undeclared identifiers in dict keys are coerced to string keys inside dict literals:
`{a: 1}` means `{"a": 1}`.

### List HOF

```mire
set sum = lists.fold(0, (acc elem) => acc + elem, [1 2 3])
set doubled = lists.map((x) => x * 2, [1 2 3])
set filtered = lists.filter((x) => x > 1, [1 2 3])
```

Calling convention: `lists.fold(acc, closure, list)`, `lists.map(closure, list)`, `lists.filter(closure, list)`.

### Slices

```mire
set slice = lists.slice(list, 1, 3)
```

---

## 8. Control Flow

### If/Elif/Else

```mire
if age >= 18 {
    use dasu("adult")
} else {
    use dasu("minor")
}

if x > 10 {
    use dasu("greater")
} elif x == 10 {
    use dasu("equal")
} else {
    use dasu("lower")
}
```

### While

```mire
while count < 10 {
    set count += 1
}
```

### For

```mire
for i in range(10) {
    use dasu(i)
}

for value, index in range(10) {
    use dasu("{index}:{value}")
}
```

### Do-While

```mire
do {
    set count += 1
} while count != 10
```

### Match (Statement)

```mire
match code {
    200 { use dasu("ok") }
    _ { use dasu("error") }
}

match x < 5 :bool {
    true { 1 }
    false { 2 }
}
```

### Break / Continue

```mire
for i in range(10) {
    if i == 5 { break }
    if i % 2 == 0 { continue }
}
```

### Find

```mire
find item in collection {
    use dasu(item)
}
```

---

## 9. Unsafe, Asm, Extern

### Unsafe Blocks

Bypasses ownership/borrow checking for the enclosed block:

```mire
unsafe {
    set x = 2 :i64
    set raw_ptr = &x
}
```

The body is compiled normally but without borrow checker restrictions.

### Inline Assembly

```mire
asm {
    mov rax, rbx
    add rax, rcx
}
```

The parser accepts `asm` blocks. Backend emits LLVM `asm sideeffect` with operands.

### Extern Libraries

```mire
extern lib "c" "libc.so.6"
```

Registers a library alias and path for FFI linking.

### Extern Functions

```mire
extern fn puts: (msg :*const i8) :i32 lib "c"
```

- Registers a function signature for type checking without a Mire body.
- Pointer types (`*const T`, `*mut T`) are modeled as `i64` in the frontend.
- Backend emits LLVM `declare` for the function signature.

---

## 10. Lifecycle Operations

Lifecycle operations provide explicit ownership intent:

```mire
new::() :vec[i64]
new::([1 2 3]) :arr[i64 3]

own::(42) :i64

move::(source) to target

drop::(value)
drop::(a, b, c)
```

Notes:
- `new::()` requires a type annotation (`:T`).
- `own::()` allocates with explicit ownership intent.
- `move::(...) to ...` invalidates source ownership after transfer.
- `drop::(...)` can destroy one or many values explicitly.

---

## 11. Pipeline Operator

The `|>` operator pipes a value through a function call. `_` is the placeholder for the piped value:

```mire
set doubled = [1 2 3]
    |> lists.map((x) => x * 2)
    |> lists.filter((x) => x > 2)

# Same as:
set step1 = lists.map((x) => x * 2, [1 2 3])
set doubled = lists.filter((x) => x > 2, step1)
```

Safe pipeline `|?>` propagates errors:

```mire
set result = read_file("data.txt")
    |?> parse_json
```

---

## 12. I/O: dasu and ireru

### Output: `dasu`

Prints any value to stdout followed by a newline:

```mire
use dasu("hello")       // str
use dasu(42)            // i64
use dasu(3.14)          // f64
use dasu(true)          // bool
use dasu({key: "val"})  // dict
```

String interpolation works inside `dasu`:

```mire
use dasu("Hello {name}")
use dasu("Count: {count}")
use dasu("Result: {add(5 3)}")
```

### Input: `ireru`

Reads a line from stdin. Accepts 0 or 1 arguments (an optional prompt). Returns `str` by default, or the annotated type:

```mire
// Read a line with no prompt
set line = ireru()

// Read a line with a prompt
set name = ireru("Name: ")

// Read and parse as integer
set age = ireru("Age: ") :i64

// Read and parse as float
set height = ireru("Height: ") :f64

// Read and parse as bool ("true"/"1" → true, "false"/"0" → false)
set flag = ireru("Flag: ") :bool
```

The prompt (if provided) is printed to stdout without a newline before reading. The trailing newline from the user's input is stripped automatically.

---

## 14. Imports

Modules are imported with the `import` keyword:

```mire
import std
import math
import strings
import fs as fs
import ./utils
import strings: (split replace trim)
```

Specific imports select only named items from a module:

```mire
import strings: (split replace trim)
```

Module dependencies can also be declared in `owl.toml` under the `[imports]` section:

```toml
[imports]
kioto = { version = "0.2" }
my-lib = { path = "./lib/my-lib" }
```

Use the CLI to manage imports:
```
mire import kioto --version 0.2
mire import ./local-lib --path lib/local-lib
```

---

## 15. Traits/Skills

```mire
pub skill Show {
    fn show: (self) :str
}

pub skill Size {
    fn size: (self) :i64
}

impl Show for Box {
    fn show: (self) :str {
        return "Box"
    }
}
```

---

## 16. Operators

### Arithmetic

```mire
set sum = a + b
set diff = a - b
set prod = a * b
set quot = a / b
set rem = a % b
```

### Comparison

```mire
if x >= 18 { }
if x == 10 { }
if x != 5 { }
if x < 0 { }
if y > 0 { }
if z <= 100 { }
```

### Logical

```mire
if a && b { }
if a || b { }
if !flag { }
```

### Bitwise

```mire
set result = a & b    # AND
set result = a | b    # OR
set result = a ^ b    # XOR
set result = a << b   # Shift left
set result = a >> b   # Shift right
```

### Index Access

```mire
set first = arr at 0
set arr at 1 = 99
```

---

## 17. Ownership and References

### References

```mire
set x = 1 :i64
set shared = &x               # shared reference
set copied = *shared          # dereference

set m = 10 :i64 mut
set rm = &m                   # mutable reference (inferred from mut binding)

fn read_ref: (value :&i64) :i64 {
    return *value
}

set y = read_ref(shared)
```

### Type Rules

- `&T` can flow into plain `T` through auto-deref
- `&mut T` can satisfy `&T` (reborrow)
- `&T` does NOT satisfy `&mut T`
- `&x` derives mutability from the original binding (`mut` => mutable ref, otherwise shared)
- `&mut x` is rejected when `x` is immutable

### Ownership Checker Rules

- No use-after-move
- No mutation while a shared borrow exists
- No multiple mutable references
- No return of local references

### Box (Heap Allocation)

```mire
set owned = box[i64]
```

---

## 18. Types

### Primitive Types

| Type | Description | Literal Example |
|------|-------------|-----------------|
| `i8`, `i16`, `i32`, `i64` | Signed integers | `42 :i64` |
| `u8`, `u16`, `u32`, `u64` | Unsigned integers | `42 :u32` |
| `f32`, `f64` | Floating point | `3.14 :f64` |
| `char` | Unicode scalar (`u32`) | `'a' :char`, `'\n' :char` |
| `str` | String (heap-allocated) | `"hello" :str` |
| `bool` | Boolean | `true`, `false` |
| `mu` | Unit / void type | `set x = mu :mu` |

### Literal Forms

```mire
set i = 42              # inferred i64
set f = 3.14            # inferred f64
set s = "hello" :str    # explicit str
set c = 'a' :char       # char literal
set nl = '\n' :char     # escaped char
set bin = 0b1010 :i64   # binary literal
set oct = 0o12 :i64     # octal literal
set hex = 0xFF :i64     # hex literal
set raw1 = r"hello"                     # raw string (no escapes)
set raw2 = r#"hello "world""#           # raw with delimiter
set raw3 = r##"hello "world" with ##"## # raw with double delimiter
```

**Important:** String literals must always be annotated with `:str` when the type cannot be inferred. `:char` is only for single-character literals (`'a'`, `'\n'`). Assigning `"text" :char` is a type error.

### Reference Types

| Type | Description |
|------|-------------|
| `&T` | Shared reference |
| `&mut T` | Mutable reference |

### Collection Types

| Type | Syntax | Example |
|------|--------|---------|
| Array | `arr[T N]` | `arr[i64 10]` |
| Vector | `vec[T]` | `vec[i64]` |
| Map | `map[K V]` | `map[str i64]` |
| Slice | `slice[T]` | `slice[i64]` |

### Special Types

| Type | Description |
|------|-------------|
| `result[T]` | Operation that succeeds with `T` or fails with error string |
| `box[T]` | Heap-allocated value |
| `datetime` | Date/time value |
| `db` | Database connection handle |

### Custom Types

```mire
set p = (Point x: 1, y: 2) :Point
```

---

## 19. Stability

Compatibility note (v2.7.0): no syntax changes were introduced in this release.

**Stable:**
- `struct`, field access, construction
- `impl` with explicit `self`
- `Type::method(...)` with `::`
- `enum` with named variants and payloads
- Collections with type annotations
- Ownership/borrow checking
- `match` statement
- `if`/`elif`/`else`
- `for`, `while`, `do-while`
- Compound assignment (`+=`, `-=`, `*=`, `/=`, `%=`)
- Pipeline (`|>`, `|?>`)
- `unsafe` blocks
- `extern lib` / `extern fn`
- `move` / `drop` statements
- Inline assembly (`asm`)
- String interpolation
- Character literals (`char`)
- Prefixed integer literals (`0b`, `0o`, `0x`)
- Raw strings (`r"..."`, `r#"..."#`)

**Still improving:**
- Advanced skill conformance
- FFI ABI stability
- Field-level constructor validation

## Prototype Validation

La sintaxis de este documento se valida con un prototipo ejecutable:

- `tests/syntax/prototype.mire`
- Regresión: `syntax_reference_prototype_compiles_and_runs` en `tests/language_regressions.rs`
