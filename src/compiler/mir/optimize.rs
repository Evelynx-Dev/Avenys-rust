use super::inline::inlining;
use super::*;
use std::collections::{HashMap, HashSet};

pub fn optimize(program: &mut MirProgram) -> usize {
    let mut total = 0;
    total += inlining(program);
    for func in &mut program.functions {
        total += optimize_function(func);
    }
    total
}

pub fn optimize_function(func: &mut MirFunction) -> usize {
    let mut total = 0;
    loop {
        let c = constant_fold_function(func)
            + algebraic_simplify(func)
            + strength_reduce(func)
            + copy_propagate(func)
            + fold_constant_branches(func)
            + dce_function(func)
            + dead_block_elim(func)
            + merge_blocks(func);
        if c == 0 {
            break;
        }
        total += c;
    }
    total
}

fn constant_fold_function(func: &mut MirFunction) -> usize {
    let mut count = 0;
    for block in &mut func.blocks {
        let mut i = 0;
        while i < block.insts.len() {
            let result = try_fold(&block.insts[i]);
            if let Some(folded) = result {
                block.insts[i].op = MirOp::Copy(MirValue::Const(folded));
                count += 1;
            }
            i += 1;
        }
    }
    count
}

fn try_fold(inst: &MirInst) -> Option<MirConst> {
    use MirOp::*;
    match &inst.op {
        Add(MirValue::Const(a), MirValue::Const(b)) => binop_const(a, b, |x, y| x + y, |x, y| x + y),
        Sub(MirValue::Const(a), MirValue::Const(b)) => binop_const(a, b, |x, y| x - y, |x, y| x - y),
        Mul(MirValue::Const(a), MirValue::Const(b)) => binop_const(a, b, |x, y| x * y, |x, y| x * y),
        SDiv(MirValue::Const(a), MirValue::Const(b)) => {
            match (a, b) {
                (MirConst::Int(0), _) => None,
                (_, MirConst::Int(0)) => None,
                _ => binop_const(a, b, |x, y| x / y, |_, _| 0.0),
            }
        }
        Shl(MirValue::Const(a), MirValue::Const(b)) => {
            match (a, b) {
                (MirConst::Int(x), MirConst::Int(y)) => Some(MirConst::Int(x << y)),
                _ => None,
            }
        }
        ICmp(cmp, MirValue::Const(a), MirValue::Const(b)) => cmp_const(cmp, a, b),
        FCmp(cmp, MirValue::Const(a), MirValue::Const(b)) => fcmp_const(cmp, a, b),
        ZExt(MirValue::Const(c), _) => match c {
            MirConst::Bool(v) => Some(MirConst::Int(if *v { 1 } else { 0 })),
            _ => None,
        },
        _ => None,
    }
}

fn binop_const<F: Fn(i64, i64) -> i64, G: Fn(f64, f64) -> f64>(
    a: &MirConst,
    b: &MirConst,
    int_op: F,
    float_op: G,
) -> Option<MirConst> {
    match (a, b) {
        (MirConst::Int(x), MirConst::Int(y)) => Some(MirConst::Int(int_op(*x, *y))),
        (MirConst::Float(x), MirConst::Float(y)) => Some(MirConst::Float(float_op(*x, *y))),
        _ => None,
    }
}

fn cmp_const(cmp: &MirCmp, a: &MirConst, b: &MirConst) -> Option<MirConst> {
    let result = match (a, b) {
        (MirConst::Int(x), MirConst::Int(y)) => Some(int_cmp(cmp, *x, *y)),
        (MirConst::Float(x), MirConst::Float(y)) => Some(float_cmp(cmp, *x, *y)),
        (MirConst::Bool(x), MirConst::Bool(y)) => Some(int_cmp(cmp, *x as i64, *y as i64)),
        (MirConst::Char(x), MirConst::Char(y)) => Some(int_cmp(cmp, *x as u32 as i64, *y as u32 as i64)),
        _ => None,
    };
    result.map(MirConst::Bool)
}

fn fcmp_const(cmp: &MirCmp, a: &MirConst, b: &MirConst) -> Option<MirConst> {
    match (a, b) {
        (MirConst::Float(x), MirConst::Float(y)) => Some(MirConst::Bool(float_cmp(cmp, *x, *y))),
        _ => None,
    }
}

fn int_cmp(cmp: &MirCmp, x: i64, y: i64) -> bool {
    match cmp {
        MirCmp::Eq => x == y,
        MirCmp::Ne => x != y,
        MirCmp::Lt => x < y,
        MirCmp::Le => x <= y,
        MirCmp::Gt => x > y,
        MirCmp::Ge => x >= y,
    }
}

fn float_cmp(cmp: &MirCmp, x: f64, y: f64) -> bool {
    match cmp {
        MirCmp::Eq => (x - y).abs() < 1e-12,
        MirCmp::Ne => (x - y).abs() >= 1e-12,
        MirCmp::Lt => x < y,
        MirCmp::Le => x <= y,
        MirCmp::Gt => x > y,
        MirCmp::Ge => x >= y,
    }
}

fn algebraic_simplify(func: &mut MirFunction) -> usize {
    let mut count = 0;
    for block in &mut func.blocks {
        let mut i = 0;
        while i < block.insts.len() {
            let simplified = try_simplify(&block.insts[i]);
            if let Some(simple_op) = simplified {
                block.insts[i].op = simple_op;
                count += 1;
            }
            i += 1;
        }
    }
    count
}

fn try_simplify(inst: &MirInst) -> Option<MirOp> {
    use MirOp::*;
    match &inst.op {
        Add(l, r) => match (l, r) {
            (_, MirValue::Const(MirConst::Int(0))) => Some(Copy(l.clone())),
            (MirValue::Const(MirConst::Int(0)), _) => Some(Copy(r.clone())),
            _ => None,
        },
        Sub(l, r) => match (l, r) {
            (_, MirValue::Const(MirConst::Int(0))) => Some(Copy(l.clone())),
            (MirValue::Temp(a), MirValue::Temp(b)) if a == b => {
                Some(Copy(MirValue::Const(MirConst::Int(0))))
            }
            _ => None,
        },
        Mul(l, r) => match (l, r) {
            (_, MirValue::Const(MirConst::Int(1))) => Some(Copy(l.clone())),
            (MirValue::Const(MirConst::Int(1)), _) => Some(Copy(r.clone())),
            (_, MirValue::Const(MirConst::Int(0))) => Some(Copy(MirValue::Const(MirConst::Int(0)))),
            (MirValue::Const(MirConst::Int(0)), _) => Some(Copy(MirValue::Const(MirConst::Int(0)))),
            _ => None,
        },
        SDiv(l, r) => match (l, r) {
            (_, MirValue::Const(MirConst::Int(1))) => Some(Copy(l.clone())),
            _ => None,
        },
        ICmp(cmp, l, r) => match (cmp, l, r) {
            (MirCmp::Eq, MirValue::Temp(a), MirValue::Temp(b)) if a == b => {
                Some(Copy(MirValue::Const(MirConst::Bool(true))))
            }
            (MirCmp::Ne, MirValue::Temp(a), MirValue::Temp(b)) if a == b => {
                Some(Copy(MirValue::Const(MirConst::Bool(false))))
            }
            _ => None,
        },
        _ => None,
    }
}

fn strength_reduce(func: &mut MirFunction) -> usize {
    let mut count = 0;
    for block in &mut func.blocks {
        let mut i = 0;
        while i < block.insts.len() {
            let reduced = try_strength_reduce(&block.insts[i]);
            if let Some(new_op) = reduced {
                block.insts[i].op = new_op;
                count += 1;
            }
            i += 1;
        }
    }
    count
}

fn try_strength_reduce(inst: &MirInst) -> Option<MirOp> {
    use MirOp::*;
    match &inst.op {
        Mul(MirValue::Temp(t), MirValue::Const(MirConst::Int(c)))
        | Mul(MirValue::Const(MirConst::Int(c)), MirValue::Temp(t)) => {
            let uc = *c as u64;
            if *c > 0 && uc.is_power_of_two() {
                let k = uc.trailing_zeros() as i64;
                Some(Shl(MirValue::Temp(*t), MirValue::Const(MirConst::Int(k))))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn fold_constant_branches(func: &mut MirFunction) -> usize {
    let mut count = 0;
    for block in &mut func.blocks {
        let folded = match &block.terminator {
            MirTerminator::BrCond(MirValue::Const(MirConst::Bool(true)), t, _) => {
                Some(MirTerminator::Br(*t))
            }
            MirTerminator::BrCond(MirValue::Const(MirConst::Bool(false)), _, f) => {
                Some(MirTerminator::Br(*f))
            }
            _ => None,
        };
        if let Some(new_term) = folded {
            block.terminator = new_term;
            count += 1;
        }
    }
    count
}

fn dead_block_elim(func: &mut MirFunction) -> usize {
    let pred_counts = compute_predecessor_counts(func);
    let mut removed = 0;
    func.blocks.retain(|b| {
        let is_entry = b.id == 0 || b.label == "entry";
        let has_preds = pred_counts.get(&b.id).copied().unwrap_or(0) > 0;
        let keep = is_entry || has_preds;
        if !keep {
            removed += 1;
        }
        keep
    });
    if removed > 0 {
        fix_block_ids(func);
    }
    removed
}

fn copy_propagate(func: &mut MirFunction) -> usize {
    let mut copies: HashMap<usize, MirValue> = HashMap::new();

    for block in &func.blocks {
        for inst in &block.insts {
            if let MirOp::Copy(ref v) = inst.op {
                if let Some(id) = inst.result {
                    let resolved = resolve_copy(v, &copies);
                    copies.insert(id, resolved);
                }
            }
        }
    }

    if copies.is_empty() {
        return 0;
    }

    let mut count = 0;
    for block in &mut func.blocks {
        for inst in &mut block.insts {
            count += replace_value_in_op(&mut inst.op, &copies);
        }
        count += replace_value_in_terminator(&mut block.terminator, &copies);
    }
    count
}

fn resolve_copy(v: &MirValue, copies: &HashMap<usize, MirValue>) -> MirValue {
    match v {
        MirValue::Temp(id) => copies.get(id).cloned().unwrap_or_else(|| v.clone()),
        _ => v.clone(),
    }
}

fn replace_value_in_op(op: &mut MirOp, copies: &HashMap<usize, MirValue>) -> usize {
    let mut count = 0;
    fn replace(v: &mut MirValue, copies: &HashMap<usize, MirValue>, count: &mut usize) {
        if let MirValue::Temp(id) = v {
            if let Some(replacement) = copies.get(id) {
                *v = replacement.clone();
                *count += 1;
            }
        }
    }
    match op {
        MirOp::Load(v, _)
        | MirOp::PtrToInt(v, _)
        | MirOp::IntToPtr(v, _)
        | MirOp::BitCast(v, _)
        | MirOp::ZExt(v, _)
        | MirOp::Trunc(v, _)
        | MirOp::Sitofp(v, _)
        | MirOp::Fptosi(v, _)
        | MirOp::Copy(v) => replace(v, copies, &mut count),
        MirOp::Store(dst, src) => {
            replace(dst, copies, &mut count);
            replace(src, copies, &mut count);
        }
        MirOp::Add(l, r)
        | MirOp::Sub(l, r)
        | MirOp::Mul(l, r)
        | MirOp::SDiv(l, r)
        | MirOp::Shl(l, r)
        | MirOp::And(l, r)
        | MirOp::Or(l, r)
        | MirOp::ICmp(_, l, r)
        | MirOp::FCmp(_, l, r) => {
            replace(l, copies, &mut count);
            replace(r, copies, &mut count);
        }
        MirOp::Call(_, args, _) => {
            for a in args {
                replace(a, copies, &mut count);
            }
        }
        MirOp::Gep(base, indices, _name) => {
            replace(base, copies, &mut count);
            for i in indices {
                replace(i, copies, &mut count);
            }
        }
        MirOp::Phi(vals, _) => {
            for (v, _) in vals {
                replace(v, copies, &mut count);
            }
        }
        MirOp::Select(c, t, f) => {
            replace(c, copies, &mut count);
            replace(t, copies, &mut count);
            replace(f, copies, &mut count);
        }
        MirOp::Alloca(_) => {}
    }
    count
}

fn replace_value_in_terminator(term: &mut MirTerminator, copies: &HashMap<usize, MirValue>) -> usize {
    let mut count = 0;
    fn replace(v: &mut MirValue, copies: &HashMap<usize, MirValue>, count: &mut usize) {
        if let MirValue::Temp(id) = v {
            if let Some(replacement) = copies.get(id) {
                *v = replacement.clone();
                *count += 1;
            }
        }
    }
    match term {
        MirTerminator::BrCond(v, _, _) | MirTerminator::Ret(Some(v)) => {
            replace(v, copies, &mut count);
        }
        _ => {}
    }
    count
}

fn dce_function(func: &mut MirFunction) -> usize {
    let mut count = 0;
    let mut used = HashSet::new();

    for block in &func.blocks {
        for inst in &block.insts {
            collect_uses(&inst.op, &mut used);
        }
        collect_terminator_uses(&block.terminator, &mut used);
    }

    for block in &mut func.blocks {
        block.insts.retain(|inst| {
            let has_side_effect = has_side_effect(&inst.op);
            let is_used = inst.result.map_or(true, |r| used.contains(&r));
            if !has_side_effect && !is_used && inst.result.is_some() {
                count += 1;
                return false;
            }
            true
        });
    }

    count
}

fn collect_uses(op: &MirOp, used: &mut HashSet<usize>) {
    fn collect_val(v: &MirValue, used: &mut HashSet<usize>) {
        if let MirValue::Temp(id) = v {
            used.insert(*id);
        }
    }
    match op {
        MirOp::Load(v, _)
        | MirOp::PtrToInt(v, _)
        | MirOp::IntToPtr(v, _)
        | MirOp::BitCast(v, _)
        | MirOp::ZExt(v, _)
        | MirOp::Trunc(v, _)
        | MirOp::Sitofp(v, _)
        | MirOp::Fptosi(v, _) => collect_val(v, used),
        MirOp::Store(dst, src) => {
            collect_val(dst, used);
            collect_val(src, used);
        }
        MirOp::Add(l, r)
        | MirOp::Sub(l, r)
        | MirOp::Mul(l, r)
        | MirOp::SDiv(l, r)
        | MirOp::Shl(l, r)
        | MirOp::And(l, r)
        | MirOp::Or(l, r)
        | MirOp::ICmp(_, l, r)
        | MirOp::FCmp(_, l, r) => {
            collect_val(l, used);
            collect_val(r, used);
        }
        MirOp::Call(_, args, _) => {
            for a in args {
                collect_val(a, used);
            }
        }
        MirOp::Gep(base, indices, _name) => {
            collect_val(base, used);
            for i in indices {
                collect_val(i, used);
            }
        }
        MirOp::Phi(vals, _) => {
            for (v, _) in vals {
                collect_val(v, used);
            }
        }
        MirOp::Select(c, t, f) => {
            collect_val(c, used);
            collect_val(t, used);
            collect_val(f, used);
        }
        MirOp::Copy(v) => collect_val(v, used),
        MirOp::Alloca(_) => {}
    }
}

fn collect_terminator_uses(term: &MirTerminator, used: &mut HashSet<usize>) {
    match term {
        MirTerminator::BrCond(v, _, _) | MirTerminator::Ret(Some(v)) => {
            if let MirValue::Temp(id) = v {
                used.insert(*id);
            }
        }
        _ => {}
    }
}

fn has_side_effect(op: &MirOp) -> bool {
    matches!(op, MirOp::Store(_, _) | MirOp::Call(_, _, _))
}

fn merge_blocks(func: &mut MirFunction) -> usize {
    if func.blocks.len() < 2 {
        return 0;
    }

    let pred_counts = compute_predecessor_counts(func);
    let mut count = 0;
    let mut i = 0;
    while i + 1 < func.blocks.len() {
        let can_merge = {
            let term_is_br = matches!(
                func.blocks[i].terminator,
                MirTerminator::Br(target) if target == i + 1
            );
            let next_has_one_pred = pred_counts.get(&(i + 1)).copied().unwrap_or(0) == 1;
            term_is_br && next_has_one_pred
        };

        if can_merge {
            let next = func.blocks.remove(i + 1);
            let prev = &mut func.blocks[i];
            prev.insts.extend(next.insts);
            prev.terminator = next.terminator;
            count += 1;
        } else {
            i += 1;
        }
    }

    fix_block_ids(func);
    count
}

fn compute_predecessor_counts(func: &MirFunction) -> HashMap<usize, usize> {
    let mut counts = HashMap::new();
    for block in &func.blocks {
        match &block.terminator {
            MirTerminator::Br(t) => *counts.entry(*t).or_insert(0) += 1,
            MirTerminator::BrCond(_, t, f) => {
                *counts.entry(*t).or_insert(0) += 1;
                *counts.entry(*f).or_insert(0) += 1;
            }
            _ => {}
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::DataType;

    fn make_func(name: &str, insts: Vec<MirInst>, terminator: MirTerminator) -> MirFunction {
        let mut f = MirFunction {
            name: name.to_string(),
            params: vec![],
            ret_type: DataType::I64,
            blocks: vec![MirBlock {
                id: 0,
                label: "entry".to_string(),
                insts,
                terminator,
            }],
            body_hash: 0,
        };
        f.body_hash = f.compute_hash();
        f
    }

    fn inst(result: usize, op: MirOp) -> MirInst {
        MirInst {
            result: Some(result),
            op,
            loc: (0, 0),
        }
    }

    fn void_inst(op: MirOp) -> MirInst {
        MirInst {
            result: None,
            op,
            loc: (0, 0),
        }
    }

    fn t(id: usize) -> MirValue {
        MirValue::Temp(id)
    }

    fn i64(v: i64) -> MirValue {
        MirValue::Const(MirConst::Int(v))
    }

    // ── Algebraic simplification ──

    #[test]
    fn alg_x_plus_0() {
        let mut f = make_func("f", vec![inst(1, MirOp::Add(t(0), i64(0)))], MirTerminator::Ret(None));
        assert_eq!(algebraic_simplify(&mut f), 1);
        assert!(matches!(f.blocks[0].insts[0].op, MirOp::Copy(MirValue::Temp(0))));
    }

    #[test]
    fn alg_0_plus_x() {
        let mut f = make_func("f", vec![inst(1, MirOp::Add(i64(0), t(0)))], MirTerminator::Ret(None));
        assert_eq!(algebraic_simplify(&mut f), 1);
        assert!(matches!(f.blocks[0].insts[0].op, MirOp::Copy(MirValue::Temp(0))));
    }

    #[test]
    fn alg_x_mul_1() {
        let mut f = make_func("f", vec![inst(1, MirOp::Mul(t(0), i64(1)))], MirTerminator::Ret(None));
        assert_eq!(algebraic_simplify(&mut f), 1);
        assert!(matches!(f.blocks[0].insts[0].op, MirOp::Copy(MirValue::Temp(0))));
    }

    #[test]
    fn alg_1_mul_x() {
        let mut f = make_func("f", vec![inst(1, MirOp::Mul(i64(1), t(0)))], MirTerminator::Ret(None));
        assert_eq!(algebraic_simplify(&mut f), 1);
        assert!(matches!(f.blocks[0].insts[0].op, MirOp::Copy(MirValue::Temp(0))));
    }

    #[test]
    fn alg_x_mul_0() {
        let mut f = make_func("f", vec![inst(1, MirOp::Mul(t(0), i64(0)))], MirTerminator::Ret(None));
        assert_eq!(algebraic_simplify(&mut f), 1);
        assert!(matches!(f.blocks[0].insts[0].op, MirOp::Copy(MirValue::Const(MirConst::Int(0)))));
    }

    #[test]
    fn alg_0_mul_x() {
        let mut f = make_func("f", vec![inst(1, MirOp::Mul(i64(0), t(0)))], MirTerminator::Ret(None));
        assert_eq!(algebraic_simplify(&mut f), 1);
        assert!(matches!(f.blocks[0].insts[0].op, MirOp::Copy(MirValue::Const(MirConst::Int(0)))));
    }

    #[test]
    fn alg_x_minus_0() {
        let mut f = make_func("f", vec![inst(1, MirOp::Sub(t(0), i64(0)))], MirTerminator::Ret(None));
        assert_eq!(algebraic_simplify(&mut f), 1);
        assert!(matches!(f.blocks[0].insts[0].op, MirOp::Copy(MirValue::Temp(0))));
    }

    #[test]
    fn alg_x_minus_x() {
        let mut f = make_func("f", vec![inst(1, MirOp::Sub(t(0), t(0)))], MirTerminator::Ret(None));
        assert_eq!(algebraic_simplify(&mut f), 1);
        assert!(matches!(f.blocks[0].insts[0].op, MirOp::Copy(MirValue::Const(MirConst::Int(0)))));
    }

    #[test]
    fn alg_x_div_1() {
        let mut f = make_func("f", vec![inst(1, MirOp::SDiv(t(0), i64(1)))], MirTerminator::Ret(None));
        assert_eq!(algebraic_simplify(&mut f), 1);
        assert!(matches!(f.blocks[0].insts[0].op, MirOp::Copy(MirValue::Temp(0))));
    }

    // ── Copy propagation ──

    #[test]
    fn copy_prop_simple() {
        // t1 = Copy(x); t2 = Add(t1, 1)  →  t2 = Add(x, 1)
        let mut f = make_func(
            "f",
            vec![
                inst(1, MirOp::Copy(t(0))),
                inst(2, MirOp::Add(t(1), i64(1))),
            ],
            MirTerminator::Ret(Some(t(2))),
        );
        assert_eq!(copy_propagate(&mut f), 1);
        assert!(matches!(f.blocks[0].insts[1].op, MirOp::Add(MirValue::Temp(0), _)));
    }

    #[test]
    fn copy_prop_transitive() {
        // t1 = Copy(x); t2 = Copy(t1); t3 = Add(t2, 1)
        //   →  t2 = Copy(x); t3 = Add(x, 1)  (t1 replacement in t2)
        let mut f = make_func(
            "f",
            vec![
                inst(1, MirOp::Copy(t(0))),
                inst(2, MirOp::Copy(t(1))),
                inst(3, MirOp::Add(t(2), i64(1))),
            ],
            MirTerminator::Ret(Some(t(3))),
        );
        let count = copy_propagate(&mut f);
        assert!(count >= 2, "expected at least 2 replacements, got {count}");
        // t2 = Copy(x) — already handled by first pass
        assert!(matches!(f.blocks[0].insts[2].op, MirOp::Add(MirValue::Temp(0), _)));
    }

    // ── Dead code elimination ──

    #[test]
    fn dce_removes_unused_add() {
        let mut f = make_func("f", vec![inst(1, MirOp::Add(i64(1), i64(2)))], MirTerminator::Ret(None));
        assert_eq!(dce_function(&mut f), 1);
        assert!(f.blocks[0].insts.is_empty());
    }

    #[test]
    fn dce_preserves_call() {
        let mut f = make_func(
            "f",
            vec![void_inst(MirOp::Call("print".to_string(), vec![i64(42)], MirType { data_type: DataType::None }))],
            MirTerminator::Ret(None),
        );
        assert_eq!(dce_function(&mut f), 0);
        assert_eq!(f.blocks[0].insts.len(), 1);
    }

    #[test]
    fn dce_preserves_store() {
        // Store to a temp (dst, src)
        let mut f = make_func(
            "f",
            vec![void_inst(MirOp::Store(t(0), t(1)))],
            MirTerminator::Ret(None),
        );
        assert_eq!(dce_function(&mut f), 0);
        assert_eq!(f.blocks[0].insts.len(), 1);
    }

    #[test]
    fn dce_removes_unused_but_keeps_used() {
        // t1 = Add(1,2) — unused, removed
        // t2 = Add(t0, 1) — used by Ret, kept
        let mut f = make_func(
            "f",
            vec![
                inst(1, MirOp::Add(i64(1), i64(2))),
                inst(2, MirOp::Add(t(0), i64(1))),
            ],
            MirTerminator::Ret(Some(t(2))),
        );
        assert_eq!(dce_function(&mut f), 1);
        assert_eq!(f.blocks[0].insts.len(), 1);
        assert_eq!(f.blocks[0].insts[0].result, Some(2));
    }

    #[test]
    fn dce_preserves_void_call_unused_result() {
        // result = Call(...) — has side effect, kept even if result unused
        let mut f = make_func(
            "f",
            vec![inst(1, MirOp::Call("print".to_string(), vec![i64(42)], MirType { data_type: DataType::None }))],
            MirTerminator::Ret(None),
        );
        assert_eq!(dce_function(&mut f), 0);
        assert_eq!(f.blocks[0].insts.len(), 1);
    }

    // ── Constant branch folding ──

    #[test]
    fn fold_brcond_true() {
        let mut f = make_func("f", vec![], MirTerminator::BrCond(MirValue::Const(MirConst::Bool(true)), 1, 2));
        assert_eq!(fold_constant_branches(&mut f), 1);
        assert!(matches!(f.blocks[0].terminator, MirTerminator::Br(1)));
    }

    #[test]
    fn fold_brcond_false() {
        let mut f = make_func("f", vec![], MirTerminator::BrCond(MirValue::Const(MirConst::Bool(false)), 1, 2));
        assert_eq!(fold_constant_branches(&mut f), 1);
        assert!(matches!(f.blocks[0].terminator, MirTerminator::Br(2)));
    }

    #[test]
    fn fold_brcond_skip_nonconst() {
        let mut f = make_func("f", vec![], MirTerminator::BrCond(MirValue::Temp(0), 1, 2));
        assert_eq!(fold_constant_branches(&mut f), 0);
        assert!(matches!(f.blocks[0].terminator, MirTerminator::BrCond(..)));
    }

    // ── Dead block elimination ──

    fn make_multi_block(blocks: Vec<(Vec<MirInst>, MirTerminator)>) -> MirFunction {
        let mut f = MirFunction {
            name: "f".to_string(),
            params: vec![],
            ret_type: DataType::I64,
            blocks: blocks
                .into_iter()
                .enumerate()
                .map(|(id, (insts, terminator))| MirBlock {
                    id,
                    label: format!("b{id}"),
                    insts,
                    terminator,
                })
                .collect(),
            body_hash: 0,
        };
        f.body_hash = f.compute_hash();
        f
    }

    #[test]
    fn dead_elim_simple() {
        // block 0: entry, Br(1)
        // block 1: Ret, but block 2 is unreachable
        // block 2: dead (no predecessors)
        let mut f = make_multi_block(vec![
            (vec![], MirTerminator::Br(1)),
            (vec![], MirTerminator::Ret(None)),
            (vec![inst(0, MirOp::Add(i64(1), i64(2)))], MirTerminator::Ret(None)),
        ]);
        assert_eq!(dead_block_elim(&mut f), 1);
        assert_eq!(f.blocks.len(), 2);
    }

    #[test]
    fn dead_elim_keeps_entry() {
        // block 0 alone, no predecessors — entry must be kept
        let mut f = make_multi_block(vec![
            (vec![], MirTerminator::Ret(None)),
        ]);
        assert_eq!(dead_block_elim(&mut f), 0);
        assert_eq!(f.blocks.len(), 1);
    }

    #[test]
    fn dead_elim_after_branch_folding() {
        // Full pipeline: copy_prop + fold_brcond + dead_block_elim
        // t0 = Copy(Const(Bool(true)))
        // BrCond(t0, 1, 2)
        // block 1: Ret
        // block 2: dead (unreachable after folding)
        let mut f = make_multi_block(vec![
            (
                vec![inst(0, MirOp::Copy(MirValue::Const(MirConst::Bool(true))))],
                MirTerminator::BrCond(MirValue::Temp(0), 1, 2),
            ),
            (vec![], MirTerminator::Ret(None)),
            (vec![inst(1, MirOp::Add(i64(1), i64(2)))], MirTerminator::Ret(None)),
        ]);
        // Apply passes in order
        copy_propagate(&mut f);
        fold_constant_branches(&mut f);
        let removed = dead_block_elim(&mut f);
        assert_eq!(removed, 1, "dead block 2 should be removed");
        assert_eq!(f.blocks.len(), 2);
        // block 0 should now be Br(1) after folding
        assert!(matches!(f.blocks[0].terminator, MirTerminator::Br(1)));
    }

    // ── Strength reduction ──

    #[test]
    fn sr_mul_by_2_to_shl() {
        // x * 2 → x << 1
        let mut f = make_func("f", vec![inst(1, MirOp::Mul(t(0), i64(2)))], MirTerminator::Ret(None));
        assert_eq!(strength_reduce(&mut f), 1);
        let inst = &f.blocks[0].insts[0];
        assert!(matches!(inst.op, MirOp::Shl(MirValue::Temp(0), MirValue::Const(MirConst::Int(1)))));
    }

    #[test]
    fn sr_mul_by_8_to_shl_3() {
        // x * 8 → x << 3
        let mut f = make_func("f", vec![inst(1, MirOp::Mul(t(0), i64(8)))], MirTerminator::Ret(None));
        assert_eq!(strength_reduce(&mut f), 1);
        let inst = &f.blocks[0].insts[0];
        assert!(matches!(inst.op, MirOp::Shl(MirValue::Temp(0), MirValue::Const(MirConst::Int(3)))));
    }

    #[test]
    fn sr_commutative_const_first() {
        // 4 * x → x << 2
        let mut f = make_func("f", vec![inst(1, MirOp::Mul(i64(4), t(0)))], MirTerminator::Ret(None));
        assert_eq!(strength_reduce(&mut f), 1);
        let inst = &f.blocks[0].insts[0];
        assert!(matches!(inst.op, MirOp::Shl(MirValue::Temp(0), MirValue::Const(MirConst::Int(2)))));
    }

    #[test]
    fn sr_non_power_of_two_unchanged() {
        // x * 3 → unchanged (not a power of 2)
        let mut f = make_func("f", vec![inst(1, MirOp::Mul(t(0), i64(3)))], MirTerminator::Ret(None));
        assert_eq!(strength_reduce(&mut f), 0);
        assert!(matches!(f.blocks[0].insts[0].op, MirOp::Mul(_, _)));
    }

    #[test]
    fn sr_zero_unchanged() {
        // x * 0 → unchanged (handled by algebraic_simplify)
        let mut f = make_func("f", vec![inst(1, MirOp::Mul(t(0), i64(0)))], MirTerminator::Ret(None));
        assert_eq!(strength_reduce(&mut f), 0);
        assert!(matches!(f.blocks[0].insts[0].op, MirOp::Mul(_, _)));
    }

    #[test]
    fn sr_negative_unchanged() {
        // x * -2 → unchanged (negative, not a power of 2 in i64 sense)
        let mut f = make_func("f", vec![inst(1, MirOp::Mul(t(0), i64(-2)))], MirTerminator::Ret(None));
        assert_eq!(strength_reduce(&mut f), 0);
        assert!(matches!(f.blocks[0].insts[0].op, MirOp::Mul(_, _)));
    }

    #[test]
    fn sr_in_pipeline() {
        // End-to-end: Mul by power of 2 should be strength-reduced in optimize()
        // Use Ret(Temp(1)) so DCE doesn't remove the instruction
        let f = make_func("f", vec![inst(1, MirOp::Mul(t(0), i64(16)))], MirTerminator::Ret(Some(MirValue::Temp(1))));
        let mut prog = MirProgram {
            functions: vec![f],
            entry_point: None,
            extern_functions: vec![],
        };
        let total = optimize(&mut prog);
        assert!(total >= 1, "expected at least strength_reduction, got {total}");
        let inst = &prog.functions[0].blocks[0].insts[0];
        assert!(matches!(inst.op, MirOp::Shl(MirValue::Temp(0), MirValue::Const(MirConst::Int(4)))));
    }

    #[test]
    fn full_pipeline_dead_block() {
        // End-to-end: optimize() should fold const branch + remove dead block
        let f = make_multi_block(vec![
            (
                vec![inst(0, MirOp::Copy(MirValue::Const(MirConst::Bool(false))))],
                MirTerminator::BrCond(MirValue::Temp(0), 1, 2),
            ),
            (vec![], MirTerminator::Ret(None)),
            (vec![], MirTerminator::Ret(None)),
        ]);
        // Wrap in a program
        let mut prog = MirProgram {
            functions: vec![f],
            entry_point: None,
            extern_functions: vec![],
        };
        let total = optimize(&mut prog);
        assert!(total >= 3, "expected copy_prop + fold + dead_elim + merge, got {total}");
        // After all passes, the two remaining blocks should be merged into one
        assert_eq!(prog.functions[0].blocks.len(), 1);
        assert!(matches!(prog.functions[0].blocks[0].terminator, MirTerminator::Ret(None)));
    }
}

fn fix_block_ids(func: &mut MirFunction) {
    let old_to_new: HashMap<usize, usize> = func
        .blocks
        .iter()
        .enumerate()
        .map(|(new_id, block)| (block.id, new_id))
        .collect();

    for (new_id, block) in func.blocks.iter_mut().enumerate() {
        block.id = new_id;
        block.terminator = match &block.terminator {
            MirTerminator::Br(t) => MirTerminator::Br(*old_to_new.get(t).unwrap_or(t)),
            MirTerminator::BrCond(c, t, f) => MirTerminator::BrCond(
                c.clone(),
                *old_to_new.get(t).unwrap_or(t),
                *old_to_new.get(f).unwrap_or(f),
            ),
            other => other.clone(),
        };
    }
}
