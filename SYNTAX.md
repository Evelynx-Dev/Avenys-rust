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
14. [Module System](#14-module-system)
15. [Kioto Standard Library](#15-kioto-standard-library)
16. [Traits/Skills](#16-traitsskills)
17. [Operators](#17-operators)
18. [Ownership and References](#18-ownership-and-references)
19. [Types](#19-types)
20. [Manifest (owl.toml)](#20-manifest-owltoml)
21. [CLI Reference](#21-cli-reference)
22. [Stability](#22-stability)
23. [Tests](#23-tests)

---

## 1. Minimal Program

```mire
load kioto

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

### Comments

```mire
# Line comment with hash
// Line comment with slashes
//! Block comment !//
```

`#`, `//` (line), and `//! ... !//` (block) comments are stripped at the lexer level.

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

## 13. String Interpolation

String literals support interpolation with curly braces:

```mire
set name = "Mire"
use dasu("Hello {name}")              # variable interpolation
use dasu("Count: {add(5 3)}")         # expression interpolation
use dasu("Result: {if true {1}}")     # block interpolation
```

Interpolation calls `str()` on the inner expression and concatenates at compile time. Any expression or block is valid inside `{}`.

---

## 14. Module System

Mire's module system is package-based. You declare dependencies in `owl.toml`.
Two keywords control visibility:

| Keyword | Rol |
|---------|-----|
| `load`  | Declara que se usará un paquete. Debe estar a nivel top-level. |
| `use`   | Navega `[exports]` vía `::` e inyecta símbolos en scope. |

### `load` — registrar un paquete

```mire
load kioto                        # registra el paquete kioto
load kioto::math                  # registra kioto + navega a math
load kioto::math::basic           # registra kioto → math → basic
load kioto::strings as str        # alias: str.len() en vez de strings.len()
```

`load` resuelve el primer segmento contra `[dependencies]` en `owl.toml`,
registra el paquete en `PackageRegistry`, y carga los archivos `.mire`.

**Regla crítica**: `load` solo funciona a nivel top-level del archivo.
Dentro de una función, el parser lo rechaza con un error:

```mire
// ✅ Correcto
load kioto

pub fn main: () { ... }

// ❌ Error
pub fn main: () {
    load kioto::math    // "`load` must be at the top level"
}
```

### `use` — importar símbolos

`use` con rutas `::` navega la cadena de `[exports]` (TOML chain) para
encontrar el archivo destino e inyecta los símbolos en el scope actual.

Dos modos:

**Módulo** (con prefijo):
```mire
use kioto::math::basic
// → basic.hypot(), basic.pi(), basic.sqrt(), ...
// el último segmento (basic) se usa como prefijo.
```

**Ítem** (sin prefijo):
```mire
use kioto::math::basic::hypot
// → hypot() directamente.
// hypot no es un export TOML → el sistema retrocede,
// encuentra basic.mire, extrae hypot como ítem.
```

El parser disambigua automáticamente:

| Código | Interpretación |
|--------|----------------|
| `use kioto::math\n` | Import de módulo |
| `use kioto::math::basic::hypot\n` | Import de ítem |
| `use dasu("text")\n` | Output expression |
| `use kioto::math::sum(nums)\n` | Output expression (namespace call) |

La regla: si después de la cadena `::` viene `(`, es una expresión call.
Si viene newline/eof, es un import.

### `module` — identidad del archivo

Cada archivo `.mire` puede declarar su identidad:

```mire
module basic

pub fn greet: () :str {
    return "hola desde basic"
}
```

### Cómo se arma un paquete

```toml
[project]
name = "mylib"
version = "0.1.0"
entry = "mod.mire"

[exports]
utils   = "utils.mire"
parsing = "parsing/mod.mire"
```

Cada export apunta a un `.mire` o a un directorio con su propio `owl.toml`
y más `[exports]`. Las rutas jerárquicas navegan esta cadena:

- `load mylib::utils` → `utils.mire`
- `load mylib::parsing::tokens` → `parsing/mod.mire` → busca `owl.toml`
  en `parsing/` → resuelve `tokens` en sus exports

### Sin atajos, sin magia

- `load kioto::math` es válido. `load math` no (a menos que `math` esté en `[dependencies]`).
- No existe `load ./path` ni `load ../relative`. Usá `{ path = "./lib/algo" }` en `owl.toml`.
- `import` fue eliminado. Solo existen `load` y `use`.

### Ejemplo completo

```toml
# owl.toml
[project]
name = "mi-app"
version = "0.1.0"
entry = "main.mire"

[dependencies]
kioto = { path = "../kioto" }
midic = { path = "./lib/midic" }
```

```mire
# main.mire
load kioto

pub fn main: () {
    // Importación de módulo (con prefijo)
    use kioto::math::basic
    set x = basic.sqrt(16.0)

    // Importación de ítem (sin prefijo)
    use kioto::math::basic::hypot
    set y = hypot(3.0, 4.0)

    // Módulos cargados por load kioto (strings, lists, etc.)
    set s = strings.upper("hola")

    use dasu("x={x} y={y} s={s}")
}
```

### Ejemplo completo

```toml
# owl.toml
[project]
name = "mi-app"
version = "0.1.0"
entry = "main.mire"

[dependencies]
kioto = { path = "../kioto" }
utilidades = { path = "./lib/utils" }
```

```mire
# main.mire
load kioto
load utilidades::strings

pub fn main: () {
    use dasu(utilidades.strings.reverse("Hola"))
}
```

---

## 15. Kioto Standard Library

Kioto (antes conocido como `std`) es la librería estándar de Mire. Vive como un paquete separado en `../kioto/` y se declara como dependencia en `owl.toml`:

```toml
[dependencies]
kioto = { path = "../kioto" }
```

(El compilador lo inyecta automáticamente si no está en `[dependencies]`, así que no te preocupes si olvidás declararlo.)

### Árbol de módulos

```
kioto/
  mod.mire              # entry point — carga todo
  core/
    strings/            # upper, lower, strip, trim, ltrim, rtrim, split, join, replace, replace_first, contains, startswith, endswith, len, substr, pad_left, pad_right, repeat, is_empty, index_of, from_i64
    lists/              # push, pop, get, slice, map, filter, fold, concat, contains, index_of
    dicts/              # get, set, keys, values, has, remove, entries, merge
    time/               # now, sleep, format, elapsed, mark
    fs/                 # read, write, exists, copy, move, delete, mkdir, list
    env/                # get, set, all, args, cwd, chdir
    proc/               # run, exec, shell, spawn, wait, kill, exit
    async/              # task helpers, spawn/join
    mem/                # used, total, free, available, percent, snapshot
    cpu/                # count, freq, loadavg, time, snapshot
    gpu/                # available, snapshot
    term/               # style, hr, clear
    math/               # basic, stats, random, complex, decimal
      basic.mire        # add, sub, mul, div, abs, min, max
      stats.mire        # mean, median, stddev, variance
      random.mire       # int, float, shuffle
      complex.mire      # new, add, sub, mul, conj, mag
      decimal.mire      # new, add, sub, mul, round, to_str
  ext/
    types/              # type utilities, type_name
    maybe/              # Maybe[T] con map, unwrap, unwrap_or
    result/             # Result[T] con map, unwrap, unwrap_or, is_ok, is_err
    tuple/              # first, second, swap, to_list
    iter/               # Iterator utilities: map, filter, fold, collect
```

### Uso

Cargá todo Kioto de una:

```mire
load kioto

pub fn main: () {
    use dasu(strings.join(strings.split("a,b,c" ",") "|"))
}
```

O cargá solo lo que necesitás con rutas jerárquicas:

```mire
load kioto::math::basic

pub fn main: () {
    // Importación de módulo (prefijo basic.)
    use kioto::math::basic
    use dasu(str(basic.pi()))

    // Importación de ítem (sin prefijo)
    use kioto::math::basic::hypot
    set h = hypot(3.0, 4.0)
    use dasu("hypot={h}")
}
```

O con alias:

```mire
load kioto::strings as str

pub fn main: () {
    use dasu(str.upper("hola"))
}
```

### Implementation

Kioto modules are written in Mire and call `rt_*` / `pal_*` extern functions directly. The Platform Abstraction Layer (PAL, see [PAL.md](./PAL.md)) provides OS-level operations (filesystem, process, time, environment) while the runtime core (strings, lists, dicts) is implemented in C under `src/runtime/`.

---

## 16. Traits/Skills

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

## 17. Operators

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

## 18. Ownership and References

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

## 19. Types

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

## 20. Manifest (owl.toml)

El archivo `owl.toml` es el corazón de cada proyecto Mire. Declara quién sos, qué versión tenés, cuál es tu entry point, qué paquetes externos usás, y qué exportás.

### Estructura básica

```toml
[project]
name = "mi-app"
version = "0.1.0"
entry = "main.mire"

[dependencies]
kioto = { path = "../kioto" }
utilidades = { path = "./lib/utilidades" }

[exports]
utils = "src/utils.mire"
parsing = "src/parsing/mod.mire"
```

### `[project]` — identidad

- `name` (opcional, default vacío): nombre del paquete
- `version` (opcional, default vacío): versión semver
- `entry` (opcional, default `"mod.mire"`): archivo de entrada

Backward compat: también acepta `[owl]` como alias de `[project]`.

### `[dependencies]` — ¿de quién dependés?

Cada entrada es un nombre y una forma de resolverlo:

```toml
[dependencies]
# Por path (recomendado para desarrollo local):
kioto = { path = "../kioto" }
milib = { path = "./lib/milib" }

# Por versión (busca en OWL_HOME o cache):
kioto = { version = "3.11.27" }

# Ambos (path + version metadata):
milib = { version = "0.2.0", path = "./lib/milib" }
```

Backward compat: también acepta `[imports]` como alias de `[dependencies]`.

### `[exports]` — qué ofrecés al mundo

```toml
[exports]
utils   = "src/utils.mire"
parsing = "src/parsing/mod.mire"
math    = "src/math"
```

Cada export mapea un nombre a un archivo `.mire` o a un directorio con su propio `owl.toml`. Los directorios permiten jerarquías:

```toml
# kioto/owl.toml
[exports]
math = "core/math/mod.mire"

# core/math/owl.toml
[exports]
basic   = "basic.mire"
stats   = "stats.mire"
complex = "complex.mire"
random  = "random.mire"
decimal = "decimal.mire"
```

Así `load kioto::math::basic` resuelve: kioto → `../kioto/` → math → `core/math/mod.mire` → basic → `core/math/basic.mire`.

### Comandos útiles

```bash
# Validar que el owl.toml esté bien
mire validate

# Agregar una dependencia
mire owl add mi-lib --path ./lib/mi-lib

# Agregar con versión
mire owl add kioto --version 3.11.10

# Agregar path + version
mire owl add mi-lib --path ./lib/mi-lib --version 0.1.0

# Sacar una dependencia
mire owl remove mi-lib
```

---

## 21. CLI Reference

```bash
mire run [file] [options] [-- args]      # Compilar y ejecutar
mire build [file] [options]               # Compilar a binario
mire check [file] [options]               # Type-check sin codegen
mire debug [file] [options]               # Compilación debug con IR
mire test [paths...] [options]            # Compilar/ejecutar tests .mire

# Comandos de manifiesto
mire validate                             # Validar owl.toml
mire owl add <name> [--path] [--version]  # Agregar dependencia
mire owl remove <name>                    # Eliminar dependencia
```

Default profile es `debug` (`-O0`). Usá `--release` o `-O2` para builds optimizados.

### Opciones

| Flag | Descripción |
|------|-------------|
| `--debug` | Build debug (default) |
| `--release` | Build release |
| `-O, --opt-level <n>` | 0\|1\|2\|3\|s\|z |
| `--owl-home <path>` | Override OWL_HOME |
| `-W <code>` | Habilitar warning específico |
| `--deny <code>` | Elevar warning a error |
| `--warn-all` | Todos los warnings |
| `--no-run` (test) | Solo compilar, no ejecutar |
| `--verbose, -v` (test) | Mostrar resultado por test |

---

## 22. Stability

Compiler version: `3.11.27`.

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
- Module loading (`load`) — package-based via `owl.toml [dependencies]`
- Hierarchical exports (`load kioto::math::basic`)
- Generic functions, structs, enums
- Incremental compilation
- Pipeline `|>` and safe pipeline `|?>`

**Still improving:**
- Advanced skill conformance
- FFI ABI stability
- Field-level constructor validation
- WASM backend (PAL stubs)

---

## 23. Tests

The test suite lives in `tests/` and covers the full compiler pipeline.

### Running Tests

```bash
# Run all Rust integration tests
cargo test

# Run a specific regression test
cargo test syntax_reference_prototype_compiles_and_runs

# Run Mire source tests through OWL
owl test

# Run a single .mire source file
mire run tests/behavior/hello.mire
```

The `owl test` command discovers test functions in `.mire` files under `tests/`.
Functions marked with a test directive are compiled and run; non-test functions
are ignored. Output uses `cargo-test`-style formatting.

### Test Directives

Place a directive comment on the line immediately before a `pub fn` to mark it:

```mire
#!cfg::test
pub fn test_addition: () {
    # test body
}

![test] basic arithmetic
pub fn test_arithmetic: () {
    assert_eq(add(2 3) 5)
}
```

Supported directives:

| Directive | Description |
|-----------|-------------|
| `#!cfg::test` | Marks the following function as a test |
| `#!cfg::bench` | Marks the following function as a benchmark |
| `#!cfg::example` | Marks the following function as an example |
| `#!cfg::ignore` | Skips the following function during test runs |
| `![test]` | Alternative `![]` syntax for marking a test |
| `![bench]` | Alternative `![]` syntax for marking a benchmark |
| `![example]` | Alternative `![]` syntax for marking an example |
| `![ignore]` | Alternative `![]` syntax for skipping a test |

An optional description string can follow the bracket directive (e.g., `![test] basic arithmetic`).

### Test Assertions

Available in test harness:

```mire
assert_eq(actual expected)
assert_ne(actual expected)
```

### Test Categories

| Directory | Description |
|-----------|-------------|
| `tests/language_regressions.rs` | ~60+ Rust integration tests — the main test suite covering parsing, type checking, ownership, codegen, and runtime |
| `tests/syntax/prototype.mire` | Validates all documented syntax compiles and runs (checked by `syntax_reference_prototype_compiles_and_runs`) |
| `tests/error/` | Error-case tests: lexer, parser, type, runtime, and ownership errors |
| `tests/loads/` | Module loading tests with inline `owl.toml` projects |
| `tests/behavior/` | Behavioral correctness tests |
| `tests/level/` | Beginner-to-advanced progression tests |
| `tests/complex/` | Complex multi-file scenarios |
| `tests/type/` | Type system edge cases |
| `tests/warnings/` | Compiler warning tests |
| `tests/security/` | Security-related tests |
| `tests/stress/` | Stress and performance tests |
| `tests/performance/` | Performance benchmarks |

### Smoke Test

```bash
cargo run -- run tests/smoke.mire
```
