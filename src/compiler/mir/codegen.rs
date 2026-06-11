use super::*;
use std::collections::HashMap;

pub(crate) struct LlvmCtx {
    pub(crate) strings: Vec<String>,
    vars: HashMap<usize, String>,
    temp_types: HashMap<usize, String>,
    param_types: HashMap<String, String>,
    next_tmp: usize,
}

pub fn mir_to_llvm(program: &MirProgram) -> (String, Vec<(String, String)>) {
    let mut extern_decls = Vec::new();
    for ext in &program.extern_functions {
        let ret = llvm_type_str(&ext.return_type);
        let params: Vec<String> = ext.params.iter().map(|t| llvm_type_str(t)).collect();
        let sig = params.join(", ");
        extern_decls.push(format!("declare {} @{}({})", ret, ext.name, sig));
    }

    let mut function_irs = Vec::new();
    let mut ctx = LlvmCtx {
        strings: Vec::new(),
        vars: HashMap::new(),
        temp_types: HashMap::new(),
        param_types: HashMap::new(),
        next_tmp: 0,
    };
    for func in &program.functions {
        let func_ir = compile_function_to_llvm(func, &mut ctx);
        function_irs.push(func_ir);
    }
    let strings = ctx.strings;

    let mut out = Vec::new();
    out.push("target triple = \"x86_64-unknown-linux-gnu\"".to_string());
    out.push(String::new());
    out.extend(extern_decls);
    out.push(String::new());
    out.extend(strings);
    out.push(String::new());
    out.extend(function_irs);

    (out.join("\n"), Vec::new())
}

pub(crate) fn compile_function_to_llvm(func: &MirFunction, ctx: &mut LlvmCtx) -> String {
    let llvm_name = format!("@fn_{}", func.name);
    let ret_type = llvm_type_str(&func.ret_type);
    let saved_vars = std::mem::take(&mut ctx.vars);
    let saved_temp_types = std::mem::take(&mut ctx.temp_types);
    let saved_next_tmp = ctx.next_tmp;

    let mut param_strs = Vec::new();
    ctx.param_types.clear();
    ctx.next_tmp = 0;
    for p in &func.params {
        let ty = llvm_type_str(&p.data_type);
        let arg_n = format!("%arg_{}", p.name);
        ctx.param_types.insert(p.name.clone(), ty.clone());
        param_strs.push(format!("{} {}", ty, arg_n));
    }

    let mut parts = Vec::new();
    parts.push(format!("define {} {}({}) {{", ret_type, llvm_name, param_strs.join(", ")));

    for block in &func.blocks {
        if block.id > 0 {
            parts.push(String::new());
        }
        parts.push(format!("bb_{}:", block.id));

        for inst in &block.insts {
            if let Some(line) = compile_inst(inst, ctx) {
                parts.push(format!("  {}", line));
            }
        }

        let term = if matches!(block.terminator, MirTerminator::Unreachable)
            && block.id + 1 == func.blocks.len()
        {
            default_return_for_type(&ret_type)
        } else {
            compile_terminator(&block.terminator, ctx, &ret_type)
        };
        if !term.is_empty() {
            parts.push(format!("  {}", term));
        }
    }

    parts.push("}".to_string());
    ctx.vars = saved_vars;
    ctx.temp_types = saved_temp_types;
    ctx.next_tmp = saved_next_tmp;
    ctx.param_types.clear();
    parts.join("\n")
}

fn llvm_type_str(dt: &DataType) -> String {
    match dt {
        DataType::I64 | DataType::Char | DataType::U64 => "i64".to_string(),
        DataType::I32 | DataType::U32 => "i32".to_string(),
        DataType::I16 | DataType::U16 => "i16".to_string(),
        DataType::I8 | DataType::U8 => "i8".to_string(),
        DataType::F32 => "float".to_string(),
        DataType::F64 => "double".to_string(),
        DataType::Bool => "i1".to_string(),
        DataType::None => "i64".to_string(),
        _ => "ptr".to_string(),
    }
}

fn const_str(c: &MirConst) -> String {
    match c {
        MirConst::Int(v) => format!("{}", v),
        MirConst::Float(v) => {
            let bits = v.to_bits();
            format!("{:#x}", bits)
        }
        MirConst::Bool(v) => {
            if *v { "1" } else { "0" }.to_string()
        }
        MirConst::Char(c) => format!("{}", *c as u32),
        MirConst::Str(_s) => {
            "null".to_string()
        }
        MirConst::None => "0".to_string(),
    }
}

fn resolve_typed(val: &MirValue, ctx: &LlvmCtx) -> (String, String) {
    match val {
        MirValue::Const(c) => {
            let v = const_str(c);
            let t = match c {
                MirConst::Int(_) | MirConst::Char(_) | MirConst::None => "i64",
                MirConst::Float(_) => "double",
                MirConst::Bool(_) => "i1",
                MirConst::Str(_) => "ptr",
            };
            (v, t.to_string())
        }
        MirValue::Temp(id) => {
            let n = ctx.vars.get(id).cloned().unwrap_or_else(|| format!("%t{}", id));
            let t = ctx.temp_types.get(id).cloned().unwrap_or_else(|| "i64".to_string());
            (n, t)
        }
        MirValue::Param(name) => {
            let t = ctx.param_types.get(name).cloned().unwrap_or_else(|| "i64".to_string());
            (format!("%arg_{}", name), t)
        }
        MirValue::Global(name) => {
            (format!("@{}", name), "ptr".to_string())
        }
    }
}

fn tmp_with_type(ctx: &mut LlvmCtx, ty: &str) -> usize {
    let id = ctx.next_tmp;
    ctx.next_tmp += 1;
    let name = format!("%t{}", id);
    ctx.vars.insert(id, name);
    ctx.temp_types.insert(id, ty.to_string());
    id
}

fn compile_inst(inst: &MirInst, ctx: &mut LlvmCtx) -> Option<String> {
    match &inst.op {
        MirOp::Alloca(ty) => {
            let llty = llvm_type_str(&ty.data_type);
            let result = tmp_with_type(ctx, "ptr");
            Some(format!("%t{} = alloca {}", result, llty))
        }
        MirOp::Load(src, ty) => {
            let (src_s, _) = resolve_typed(src, ctx);
            let llty = llvm_type_str(&ty.data_type);
            let result = tmp_with_type(ctx, &llty);
            Some(format!("%t{} = load {}, ptr {}", result, llty, src_s))
        }
        MirOp::Store(dst, src) => {
            let (src_s, src_ty) = resolve_typed(src, ctx);
            let (dst_s, _) = resolve_typed(dst, ctx);
            Some(format!("store {} {}, ptr {}", src_ty, src_s, dst_s))
        }
        MirOp::Add(l, r) => {
            let (l, lt) = resolve_typed(l, ctx);
            let (r, _) = resolve_typed(r, ctx);
            let ty = if lt == "double" { "double" } else { "i64" };
            let result = tmp_with_type(ctx, ty);
            let op = if ty == "double" { "fadd" } else { "add" };
            Some(format!("%t{} = {} {} {}, {}", result, op, ty, l, r))
        }
        MirOp::Sub(l, r) => {
            let (l, lt) = resolve_typed(l, ctx);
            let (r, _) = resolve_typed(r, ctx);
            let ty = if lt == "double" { "double" } else { "i64" };
            let result = tmp_with_type(ctx, ty);
            let op = if ty == "double" { "fsub" } else { "sub" };
            Some(format!("%t{} = {} {} {}, {}", result, op, ty, l, r))
        }
        MirOp::Mul(l, r) => {
            let (l, lt) = resolve_typed(l, ctx);
            let (r, _) = resolve_typed(r, ctx);
            let ty = if lt == "double" { "double" } else { "i64" };
            let result = tmp_with_type(ctx, ty);
            let op = if ty == "double" { "fmul" } else { "mul" };
            Some(format!("%t{} = {} {} {}, {}", result, op, ty, l, r))
        }
        MirOp::SDiv(l, r) => {
            let (l, lt) = resolve_typed(l, ctx);
            let (r, _) = resolve_typed(r, ctx);
            let ty = if lt == "double" { "double" } else { "i64" };
            let result = tmp_with_type(ctx, ty);
            let op = if ty == "double" { "fdiv" } else { "sdiv" };
            Some(format!("%t{} = {} {} {}, {}", result, op, ty, l, r))
        }
        MirOp::Shl(l, r) => {
            let (l, _lt) = resolve_typed(l, ctx);
            let (r, _) = resolve_typed(r, ctx);
            let result = tmp_with_type(ctx, "i64");
            Some(format!("%t{} = shl i64 {}, {}", result, l, r))
        }
        MirOp::ICmp(cmp, l, r) => {
            let (l, lt) = resolve_typed(l, ctx);
            let (r, _) = resolve_typed(r, ctx);
            let result = tmp_with_type(ctx, "i1");
            let cond = match cmp {
                MirCmp::Eq => "eq",
                MirCmp::Ne => "ne",
                MirCmp::Lt => "slt",
                MirCmp::Le => "sle",
                MirCmp::Gt => "sgt",
                MirCmp::Ge => "sge",
            };
            Some(format!("%t{} = icmp {} {} {}, {}", result, cond, lt, l, r))
        }
        MirOp::FCmp(cmp, l, r) => {
            let (l, _lt) = resolve_typed(l, ctx);
            let (r, _) = resolve_typed(r, ctx);
            let result = tmp_with_type(ctx, "i1");
            let cond = match cmp {
                MirCmp::Eq => "oeq",
                MirCmp::Ne => "one",
                MirCmp::Lt => "olt",
                MirCmp::Le => "ole",
                MirCmp::Gt => "ogt",
                MirCmp::Ge => "oge",
            };
            Some(format!("%t{} = fcmp {} double {}, {}", result, cond, l, r))
        }
        MirOp::Call(name, args, ret_ty) => {
            let ll_ret = llvm_type_str(&ret_ty.data_type);
            let result = tmp_with_type(ctx, &ll_ret);
            let mut arg_strs = Vec::new();
            for a in args {
                let (v, t) = resolve_typed(a, ctx);
                arg_strs.push(format!("{} {}", t, v));
            }
            Some(format!(
                "%t{} = call {} @{}({})",
                result, ll_ret, name, arg_strs.join(", ")
            ))
        }
        MirOp::ZExt(val, ty) => {
            let (v, _) = resolve_typed(val, ctx);
            let llty = llvm_type_str(&ty.data_type);
            let result = tmp_with_type(ctx, &llty);
            Some(format!("%t{} = zext i1 {} to {}", result, v, llty))
        }
        MirOp::Copy(v) => {
            let (src, ty) = resolve_typed(v, ctx);
            let result = tmp_with_type(ctx, &ty);
            Some(format!("%t{} = select i1 true, {} {}, {} {}", result, ty, src, ty, src))
        }
        _ => None,
    }
}

fn default_return_for_type(ret_type: &str) -> String {
    match ret_type {
        "ptr" => "ret ptr null".to_string(),
        "double" => "ret double 0.0".to_string(),
        "float" => "ret float 0.0".to_string(),
        "i1" => "ret i1 0".to_string(),
        _ => format!("ret {} 0", ret_type),
    }
}

fn compile_terminator(term: &MirTerminator, ctx: &mut LlvmCtx, ret_type: &str) -> String {
    match term {
        MirTerminator::Br(target) => {
            format!("br label %bb_{}", target)
        }
        MirTerminator::BrCond(cond, t, f) => {
            let (c, _) = resolve_typed(cond, ctx);
            format!("br i1 {}, label %bb_{}, label %bb_{}", c, t, f)
        }
        MirTerminator::Ret(Some(val)) => {
            if matches!(val, &MirValue::Const(MirConst::None)) {
                return default_return_for_type(ret_type);
            }
            let (v, t) = resolve_typed(val, ctx);
            format!("ret {} {}", t, v)
        }
        MirTerminator::Ret(None) => default_return_for_type(ret_type),
        MirTerminator::Unreachable => "unreachable".to_string(),
    }
}
