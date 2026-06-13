# MIR Pipeline (Mid-Level IR)

## Overview

The MIR pipeline is the **default codegen** for Avenys. It replaces the original `LlvmIrGen` AST→LLVM walker with a 4-phase mid-level intermediate representation. The legacy walker can be used via `MIRE_LEGACY_CODEGEN=1`.

Source: `src/compiler/mir/`

## Phases

### Phase 1: Types (`src/compiler/mir/mod.rs`)

Core data structures — all derive `Debug, Clone`:

| Type | Description |
|---|---|
| `MirProgram` | Top-level: functions, entry point, extern functions |
| `MirFunction` | Name, params, return type, basic blocks |
| `MirBlock` | ID, label, instruction list, terminator |
| `MirInst` | `result: Option<usize>`, `op: MirOp`, `loc` |
| `MirValue` | `Const(MirConst)`, `Temp(usize)`, `Param(String)`, `Global(String)` |
| `MirConst` | `Int(i64)`, `Float(f64)`, `Bool(bool)`, `Char(char)`, `Str(String)`, `None` |
| `MirOp` | Alloca, Load, Store, Add, Sub, Mul, SDiv, Shl, And, Or, ICmp, FCmp, Call, Gep, PtrToInt, IntToPtr, BitCast, ZExt, Trunc, Phi, Select, Copy |
| `MirCmp` | Eq, Ne, Lt, Le, Gt, Ge |
| `MirTerminator` | `Br(usize)`, `BrCond(MirValue, usize, usize)`, `Ret(Option<MirValue>)`, `Unreachable` |
| `MirExternFunction` | External function signatures (name, params, return type) |

### Phase 2: Lowering (`src/compiler/mir/lower.rs`)

`lower_program(program: &Program) -> MirProgram`

| AST Construct | Status |
|---|---|
| Function definitions | ✅ |
| Impl blocks (methods) | ✅ |
| Let/set declarations | ✅ |
| Assignment | ✅ |
| Return | ✅ |
| If/else | ✅ |
| While loops | ✅ |
| For loops (with optional index variable) | ✅ (lowered to while-loop with counter) |
| Binary ops (+, -, *, /, ==, !=, <, <=, >, >=, &&, \|\|) | ✅ |
| Unary ops (-, !) | ✅ |
| Function calls (user + builtins) | ✅ (builtins emit as regular calls) |
| Match expressions | ✅ (alloca + store per case + load) |
| If-expressions (`__if_expr`) | ✅ (BrCond + phi-like store/load) |
| Literals (int, float, bool, char, str) | ✅ |
| Variable references | ✅ (type-aware Load via `var_types`) |
| Extern functions | ✅ (collected as `MirExternFunction`) |
| Index expressions (array/map read) | ✅ (GEP + Load) |
| Index assignment (array/map write) | ✅ (GEP + Store) |
| Member access (struct field read) | ✅ (GEP + Load via struct metadata) |
| Member assignment (struct field write) | ✅ (load heap ptr + GEP + Store) |
| Struct construction via Tuple expr | ✅ (emitted as `Call(struct_name, args)`) |
| Reference (`&expr`) | ✅ (returns alloca ptr, skips Load) |
| Dereference (`*expr`) | ✅ (Load from pointer) |
| Unsafe blocks | ✅ (forwards body) |
| Closures | ❌ |
| Pipeline (`|>`) | ❌ |
| Enum variants | ⚠️ Returns dummy `Const(Int(0))` |
| Try/Ok/Err | ❌ |
| Dict/Map literals | ❌ (returns `Const(None)`) |

Var types tracked via `var_types: HashMap<String, DataType>` during lowering.
Struct metadata collected by `extract_struct_types()` and passed as
`struct_types: HashMap<String, Vec<(String, DataType)>>` through `MirProgram` /
`MirLower` / `LlvmCtx`.

### Phase 3: Codegen (`src/compiler/mir/codegen.rs`)

`mir_to_llvm(mir: &MirProgram) -> (String, Vec<(String, String)>)`

Returns LLVM IR text + extern libs (currently always empty).

| Feature | Status |
|---|---|
| Function defs with typed params | ✅ |
| Alloca for locals | ✅ |
| Load/Store with correct types | ✅ |
| Integer arithmetic (add, sub, mul, sdiv, shl) | ✅ |
| Float arithmetic (fadd, fsub, fmul, fdiv) | ✅ |
| Mixed-type arithmetic (coerce i64→double via sitofp) | ✅ |
| Integer comparison (icmp) | ✅ |
| Float comparison (fcmp) | ✅ |
| Boolean And/Or | ✅ (emits `and i1` / `or i1`) |
| Branch / conditional branch | ✅ |
| Return | ✅ |
| Function calls (typed args) | ✅ |
| ZExt bool→i64 | ✅ |
| Trunc i64→i32 | ✅ |
| Extern declarations from `ExternFunction` AST | ✅ |
| Extern function name resolution (strip root namespace) | ✅ (via `split_once('.')`) |
| SSA temp type tracking | ✅ (`temp_types`, `param_types`, `resolve_typed`) |
| Float hex encoding | ✅ (`to_bits()` for exact bit representation) |
| String constants | ⚠️ Returns `ptr null` |
| GEP (struct fields, array elements) | ✅ (2-index for structs, 1-index for arrays/pointers) |
| Phi, Select, PtrToInt, IntToPtr, BitCast | ✅ |
| Temporary ID separation: `%e{n}` for extras, `%t{mir_id}` for results | ✅ |
| Struct constructor calls via `Call(struct_name, ...)` | ✅ |
| Builtins (dasu, ireru, etc.) | ❌ Not expanded inline |

### Phase 4: Optimizations (`src/compiler/mir/optimize.rs`)

`optimize(program: &mut MirProgram) -> usize`

Fixed-point loop per function — iterates until no changes:

```
loop {
    constant_fold_function    // const-const binops (incl. Shl)
    + algebraic_simplify      // x+0→x, 0+x→x, x*1→x, 1*x→x, x*0→0, 0*x→x, x-0→x, x-x→0, x/1→x, ICmp(Eq, x,x)→true
    + strength_reduce         // x*2^k → x<<k
    + copy_propagate          // t1=Copy(v) → replace all Temp(t1) uses with v (transitive)
    + fold_constant_branches  // BrCond(Const(true), L1, L2) → Br(L1); BrCond(Const(false), L1, L2) → Br(L2)
    + dce_function            // remove unused instructions without side effects (stores/calls preserved)
    + dead_block_elim         // remove blocks with 0 predecessors (entry block always kept)
    + merge_blocks            // merge Br→single-predecessor chains
}
```

Returns total number of applied transformations.

#### Optimizations detail

| Optimization | Description |
|---|---|
| **Constant folding** | `Add(Const(1), Const(2))` → `Copy(Int(3))` for Add/Sub/Mul/SDiv/Shl/ICmp/FCmp/ZExt |
| **Algebraic simplification** | Identity/sink eliminations: `x+0`, `0+x`, `x*1`, `1*x`, `x*0`, `0*x`, `x-0`, `x-x`, `x/1`, `ICmp(Eq, x, x)`→true |
| **Strength reduction** | `Mul(x, Const(2^k))` → `Shl(x, Const(k))` (power-of-2 constants only) |
| **Copy propagation** | `t1 = Copy(v); t2 = Add(t1, ...)` → `t2 = Add(v, ...)`. Transitive: `t2 = Copy(t1); t3 = Add(t2, ...)` → `Add(v, ...)` |
| **BrCond folding** | `BrCond(Const(true), L1, L2)` → `Br(L1)`. Part of dead block elimination pipeline. |
| **Dead code elimination** | Removes unused instruction results; preserves `Store`, `Call` (side effects) |
| **Dead block elimination** | Removes basic blocks with 0 incoming edges after branch folding |
| **Block merging** | Merges `Br(id)` → single-predecessor successor blocks |

#### Confirmed test results (30 tests)

| Category | Tests | Status |
|---|---|---|
| Algebraic: `x+0`, `0+x`, `x*1`, `1*x`, `x*0`, `0*x`, `x-0`, `x-x`, `x/1` | 9 | ✅ |
| Copy prop: simple, transitive | 2 | ✅ |
| DCE: removes unused, preserves Call, preserves Store, preserves void Call, mixed | 5 | ✅ |
| BrCond fold: true, false, skip-nonconst | 3 | ✅ |
| Dead block: simple, keeps-entry, after-branch-folding, full-pipeline | 4 | ✅ |
| Strength reduction: `x*2→x<<1`, `x*8→x<<3`, `4*x→x<<2`, non-pow2, zero, negative, pipeline | 7 | ✅ |

## Integration

- **Default**: MIR pipeline is the default codegen path
- **Legacy**: `MIRE_LEGACY_CODEGEN=1` falls back to `LlvmIrGen` AST walker
- **Toggle point**: `compile_file_with_avenys()` in `build_pipeline.rs:225`
- **Runtime helpers**: C runtime files in `src/runtime/` and `src/pal/<pal>/*.c`

## Known Limitations

1. **Builtins not expanded**: `dasu`, `ireru`, `len`, etc. emitted as regular `call` — clang can't resolve. Existing codegen in `llvm_functions.rs` handles these with special-case C wrappers.

2. **Extern libs empty**: Second tuple element always `Vec::new()`. The existing codegen properly collects from `ExternLib` statements.

3. **String constants**: `MirConst::Str` emits `ptr null` instead of `@.str = constant [N x i8] c"..."`.

4. **Closures, dicts, enums with payloads**: Not fully lowered. Dict/map literals return `Const(None)`. Enum variants return `Const(Int(0))`.

5. **Memory pressure**: Full `build` (compile + link via clang) can use ~4GB RAM — a pre-existing issue from clang runtime compilation, not specific to MIR.

## Architecture Notes

- SSA-like IR with basic blocks and explicit terminators
- Type tracking per-temp (`temp_types` HashMap in `LlvmCtx`)
- Param types per-function (`param_types`)
- Lowering is single-pass (AST has structured control flow — no CFG construction needed)
- Codegen produces standard LLVM textual IR (compatible with clang/llc)
- Optimizations are module-level transforms on `MirProgram` before codegen (no LLVM opt passes)
