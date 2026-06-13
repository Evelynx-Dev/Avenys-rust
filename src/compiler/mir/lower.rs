use super::*;
use crate::parser::ast::{
    AssignmentTarget, DataType, Expression, Literal, Program, Statement,
};
use std::collections::HashMap;
use std::collections::HashSet;

struct MirLower {
    func: MirFunction,
    next_temp: usize,
    vars: HashMap<String, usize>,
    var_types: HashMap<String, DataType>,
    struct_types: HashMap<String, Vec<(String, DataType)>>,
}

fn extract_struct_types(program: &Program) -> HashMap<String, Vec<(String, DataType)>> {
    let mut struct_types = HashMap::new();
    for stmt in &program.statements {
        if let Statement::Type { name, fields, .. } = stmt {
            let mut field_list = Vec::new();
            for f in fields {
                if let Statement::Let { name, data_type, .. } = f {
                    field_list.push((name.clone(), data_type.clone()));
                }
            }
            struct_types.insert(name.clone(), field_list);
        }
    }
    struct_types
}

pub fn lower_program(program: &Program) -> MirProgram {
    let mut functions = Vec::new();
    let mut entry_point = None;
    let mut extern_functions = Vec::new();
    let mut seen_functions = HashSet::new();
    let struct_types = extract_struct_types(program);

    for stmt in &program.statements {
        if let Statement::ExternFunction { name, params, return_type, .. } = stmt {
            extern_functions.push(MirExternFunction {
                name: name.clone(),
                params: params.iter().map(|(_, t)| t.clone()).collect(),
                return_type: return_type.clone(),
            });
        }
    }

    for stmt in &program.statements {
        match stmt {
            Statement::Function {
                name,
                params,
                return_type,
                body,
                ..
            } => {
                if !seen_functions.insert(name.clone()) {
                    continue;
                }
                let mir_params = params
                    .iter()
                    .map(|(pname, ptype)| MirParam {
                        name: pname.clone(),
                        data_type: ptype.clone(),
                    })
                    .collect();

                let mut lower = MirLower {
                    func: MirFunction::new(name.clone(), mir_params, return_type.clone()),
                    next_temp: 0,
                    vars: HashMap::new(),
                    var_types: HashMap::new(),
                    struct_types: struct_types.clone(),
                };

                lower.lower_function_body(body);
                if !lower.func.blocks.is_empty() {
                    lower.func.blocks[0].label = "entry".to_string();
                }
                lower.func.body_hash = lower.func.compute_hash();
                functions.push(lower.func);

                if name == "main" {
                    entry_point = Some(name.clone());
                }
            }
            Statement::Impl {
                type_name,
                methods,
                ..
            } => {
                for method in methods {
                    if let Statement::Function {
                        name,
                        params,
                        return_type,
                        body,
                        ..
                    } = method
                    {
                        let full_name = format!("{}.{}", type_name, name);
                        if !seen_functions.insert(full_name.clone()) {
                            continue;
                        }
                        let mir_params = params
                            .iter()
                            .map(|(pname, ptype)| MirParam {
                                name: pname.clone(),
                                data_type: ptype.clone(),
                            })
                            .collect();

                        let mut lower = MirLower {
                            func: MirFunction::new(full_name, mir_params, return_type.clone()),
                            next_temp: 0,
                            vars: HashMap::new(),
                            var_types: HashMap::new(),
                            struct_types: struct_types.clone(),
                        };

                        lower.lower_function_body(body);
                        if !lower.func.blocks.is_empty() {
                            lower.func.blocks[0].label = "entry".to_string();
                        }
                        lower.func.body_hash = lower.func.compute_hash();
                        functions.push(lower.func);
                    }
                }
            }
            _ => {}
        }
    }

    let mut mp = MirProgram::new(functions, entry_point);
    mp.extern_functions = extern_functions;
    mp.struct_types = struct_types;
    mp
}

impl MirLower {
    fn new_temp(&mut self) -> usize {
        let id = self.next_temp;
        self.next_temp += 1;
        id
    }

    fn new_block(&mut self, label: &str) -> usize {
        let id = self.func.blocks.len();
        self.func.push_block(format!("{}_{}", label, id));
        id
    }

    fn entry_block_id(&mut self) -> usize {
        if self.func.blocks.is_empty() {
            self.func.push_block("entry".to_string());
        }
        0
    }

    fn lower_function_body(&mut self, body: &[Statement]) {
        let entry_id = self.entry_block_id();

        let nparams = self.func.params.len();
        let mut param_list: Vec<(String, DataType, usize)> = Vec::with_capacity(nparams);
        for i in 0..nparams {
            let ptr = self.new_temp();
            param_list.push((
                self.func.params[i].name.clone(),
                self.func.params[i].data_type.clone(),
                ptr,
            ));
        }

        {
            let entry = &mut self.func.blocks[entry_id];
            for (pname, ptype, ptr) in &param_list {
                entry.push(
                    Some(*ptr),
                    MirOp::Alloca(MirType {
                        data_type: ptype.clone(),
                    }),
                    (0, 0),
                );
                self.var_types.insert(pname.clone(), ptype.clone());
            }
            for (pname, _ptype, ptr) in &param_list {
                entry.push(
                    None,
                    MirOp::Store(MirValue::temp(*ptr), MirValue::Param(pname.clone())),
                    (0, 0),
                );
            }
        }
        for (pname, _ptype, ptr) in &param_list {
            self.vars.insert(pname.clone(), *ptr);
        }

        for stmt in body {
            self.lower_statement(stmt);
        }
    }

    fn lower_statement(&mut self, stmt: &Statement) {
        let loc = (0, 0);
        match stmt {
            Statement::Let {
                name,
                data_type,
                value,
                ..
            } => {
                if self.func.blocks.is_empty() {
                    self.func.push_block("entry".to_string());
                }
                let last = self.func.blocks.len() - 1;
                let ptr = self.new_temp();
                self.func.blocks[last].push(
                    Some(ptr),
                    MirOp::Alloca(MirType {
                        data_type: data_type.clone(),
                    }),
                    loc,
                );
                self.vars.insert(name.clone(), ptr);
                self.var_types.insert(name.clone(), data_type.clone());

                if let Some(val) = value {
                    // Special-case: array literal initialization
                    if let DataType::Array { element_type, .. } = data_type {
                        if let Expression::List { elements, .. } = val {
                            let elem_llvm = llvm_elem_type_str(element_type);
                            for (i, elem) in elements.iter().enumerate() {
                                let elem_val = self.lower_expression(elem);
                                let last = self.func.blocks.len() - 1;
                                let gep = self.new_temp();
                                self.func.blocks[last].push(
                                    Some(gep),
                                    MirOp::Gep(
                                        MirValue::temp(ptr),
                                        vec![MirValue::Const(MirConst::Int(i as i64))],
                                        elem_llvm.clone(),
                                    ),
                                    loc,
                                );
                                self.func.blocks[last].push(
                                    None,
                                    MirOp::Store(MirValue::temp(gep), elem_val),
                                    loc,
                                );
                            }
                            return;
                        }
                    }
                    let v = self.lower_expression(val);
                    let last = self.func.blocks.len() - 1;
                    self.func.blocks[last].push(None, MirOp::Store(MirValue::temp(ptr), v), loc);
                }
            }
            Statement::Assignment { target, value, .. } => {
                let v = self.lower_expression(value);
                let last = self.func.blocks.len() - 1;
                match target {
                    AssignmentTarget::Variable(name) => {
                        if let Some(&ptr) = self.vars.get(name) {
                            self.func.blocks[last]
                                .push(None, MirOp::Store(MirValue::temp(ptr), v), loc);
                        }
                    }
                    AssignmentTarget::Index { target, index } => {
                        let target_val = self.lower_expression(target);
                        let index_val = self.lower_expression(index);
                        let last = self.func.blocks.len() - 1;
                        let gep = self.new_temp();
                        let elem_ty = self.get_target_elem_type(target);
                        self.func.blocks[last].push(
                            Some(gep),
                            MirOp::Gep(target_val, vec![index_val], elem_ty),
                            loc,
                        );
                        self.func.blocks[last]
                            .push(None, MirOp::Store(MirValue::temp(gep), v), loc);
                    }
                    AssignmentTarget::Field(path) => {
                        let parts: Vec<&str> = path.splitn(2, '.').collect();
                        if parts.len() == 2 {
                            let var_name = parts[0].to_string();
                            let field_name = parts[1].to_string();
                            let ptr = self.vars.get(&var_name).copied();
                            let var_type = self.var_types.get(&var_name).cloned();
                            if let (Some(ptr), Some(var_type)) = (ptr, var_type) {
                                if let DataType::StructNamed(ref struct_name) = var_type {
                                    let norm_name = struct_name.split_once('[')
                                        .map(|(base, _)| base.to_string())
                                        .unwrap_or_else(|| struct_name.clone());
                                    let field_idx = self.struct_types.get(&norm_name)
                                        .and_then(|fields| fields.iter().position(|(n, _)| n == &field_name));
                                    if let Some(field_idx) = field_idx {
                                        let data_ptr = self.new_temp();
                                        self.func.blocks[last].push(
                                            Some(data_ptr),
                                            MirOp::Load(
                                                MirValue::temp(ptr),
                                                MirType { data_type: var_type },
                                            ),
                                            loc,
                                        );
                                        let gep = self.new_temp();
                                        self.func.blocks[last].push(
                                            Some(gep),
                                            MirOp::Gep(
                                                MirValue::temp(data_ptr),
                                                vec![
                                                    MirValue::Const(MirConst::Int(0)),
                                                    MirValue::Const(MirConst::Int(field_idx as i64)),
                                                ],
                                                norm_name,
                                            ),
                                            loc,
                                        );
                                        self.func.blocks[last]
                                            .push(None, MirOp::Store(MirValue::temp(gep), v), loc);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Statement::Expression(expr) => {
                let _ = self.lower_expression(expr);
            }
            Statement::Return(val) => {
                let last = self.func.blocks.len() - 1;
                let v = val.as_ref().map(|e| self.lower_expression(e));
                self.func.blocks[last].terminator = MirTerminator::Ret(v);
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond = self.lower_expression(condition);
                let then_block = self.new_block("if_then");
                let else_id = self.new_block("if_else");
                let end_block = self.new_block("if_end");

                {
                    let last = self.func.blocks.len() - 4;
                    self.func.blocks[last].terminator =
                        MirTerminator::BrCond(cond, then_block, else_id);
                }

                for stmt in then_branch {
                    self.lower_statement(stmt);
                }
                {
                    let last = self.func.blocks.len() - 1;
                    self.func.blocks[last].terminator = MirTerminator::Br(end_block);
                }

                if let Some(else_body) = else_branch {
                    for stmt in else_body {
                        self.lower_statement(stmt);
                    }
                }
                {
                    let last = self.func.blocks.len() - 1;
                    self.func.blocks[last].terminator = MirTerminator::Br(end_block);
                }
            }
            Statement::While { condition, body, .. } => {
                let cond_block = self.new_block("while_cond");
                let body_block = self.new_block("while_body");
                let end_block = self.new_block("while_end");

                {
                    let last = self.func.blocks.len() - 3;
                    self.func.blocks[last].terminator = MirTerminator::Br(cond_block);
                }

                let cond = {
                    self.lower_expression(condition)
                };
                {
                    let last = self.func.blocks.len() - 2;
                    self.func.blocks[last].terminator =
                        MirTerminator::BrCond(cond, body_block, end_block);
                }

                for stmt in body {
                    self.lower_statement(stmt);
                }
                {
                    let last = self.func.blocks.len() - 1;
                    self.func.blocks[last].terminator = MirTerminator::Br(cond_block);
                }
            }
            Statement::For { variable, index, iterable, body } => {
                let iter_ptr = self.new_temp();
                let idx_ptr = self.new_temp();

                let last = self.func.blocks.len() - 1;

                // alloca for iterator and index
                self.func.blocks[last].push(
                    Some(iter_ptr),
                    MirOp::Alloca(MirType { data_type: DataType::Unknown }),
                    loc,
                );
                self.func.blocks[last].push(
                    Some(idx_ptr),
                    MirOp::Alloca(MirType { data_type: DataType::I64 }),
                    loc,
                );

                // store iterator value
                let iter_val = self.lower_expression(iterable);
                let last = self.func.blocks.len() - 1;
                self.func.blocks[last].push(
                    None,
                    MirOp::Store(MirValue::temp(iter_ptr), iter_val),
                    loc,
                );

                // store initial index 0
                self.func.blocks[last].push(
                    None,
                    MirOp::Store(
                        MirValue::temp(idx_ptr),
                        MirValue::Const(MirConst::Int(0)),
                    ),
                    loc,
                );

                let cond_block = self.new_block("for_cond");
                let body_block = self.new_block("for_body");
                let end_block = self.new_block("for_end");

                let last = self.func.blocks.len() - 4;
                self.func.blocks[last].terminator = MirTerminator::Br(cond_block);

                // Load index and compute len
                let idx_loaded = self.new_temp();
                self.func.blocks[cond_block].push(
                    Some(idx_loaded),
                    MirOp::Load(MirValue::temp(idx_ptr), MirType { data_type: DataType::I64 }),
                    loc,
                );
                // Use rt_list_len to get length
                let len_val = self.new_temp();
                self.func.blocks[cond_block].push(
                    Some(len_val),
                    MirOp::Call(
                        "rt_list_len".to_string(),
                        vec![MirValue::temp(iter_ptr)],
                        MirType { data_type: DataType::I64 },
                    ),
                    loc,
                );
                // Compare: idx < len
                let cond = self.new_temp();
                self.func.blocks[cond_block].push(
                    Some(cond),
                    MirOp::ICmp(MirCmp::Lt, MirValue::temp(idx_loaded), MirValue::temp(len_val)),
                    loc,
                );
                self.func.blocks[cond_block].terminator =
                    MirTerminator::BrCond(MirValue::temp(cond), body_block, end_block);

                // In body: variable = iter[idx]
                let elem_ptr = self.new_temp();
                let elem_idx_i64 = self.new_temp();
                let elem_idx = self.new_temp();
                self.func.blocks[body_block].push(
                    Some(elem_idx_i64),
                    MirOp::Load(MirValue::temp(idx_ptr), MirType { data_type: DataType::I64 }),
                    loc,
                );
                self.func.blocks[body_block].push(
                    Some(elem_idx),
                    MirOp::Trunc(
                        MirValue::temp(elem_idx_i64),
                        MirType { data_type: DataType::I32 },
                    ),
                    loc,
                );
                self.func.blocks[body_block].push(
                    Some(elem_ptr),
                    MirOp::Gep(MirValue::temp(iter_ptr), vec![MirValue::temp(elem_idx)], "ptr".to_string()),
                    loc,
                );
                let elem_val = self.new_temp();
                self.func.blocks[body_block].push(
                    Some(elem_val),
                    MirOp::Load(MirValue::temp(elem_ptr), MirType { data_type: DataType::Unknown }),
                    loc,
                );

                // Alloca and store the loop variable
                let var_ptr = self.new_temp();
                self.func.blocks[body_block].push(
                    Some(var_ptr),
                    MirOp::Alloca(MirType { data_type: DataType::Unknown }),
                    loc,
                );
                self.vars.insert(variable.clone(), var_ptr);
                self.var_types.insert(variable.clone(), DataType::Unknown);
                self.func.blocks[body_block].push(
                    None,
                    MirOp::Store(MirValue::temp(var_ptr), MirValue::temp(elem_val)),
                    loc,
                );

                // Register index variable if provided
                if let Some(idx_name) = index {
                    self.vars.insert(idx_name.clone(), idx_ptr);
                    self.var_types.insert(idx_name.clone(), DataType::I64);
                }

                for stmt in body {
                    self.lower_statement(stmt);
                }

                // Increment index
                let inc_body_last = self.func.blocks.len() - 1;
                let old_idx = self.new_temp();
                self.func.blocks[inc_body_last].push(
                    Some(old_idx),
                    MirOp::Load(MirValue::temp(idx_ptr), MirType { data_type: DataType::I64 }),
                    loc,
                );
                let new_idx = self.new_temp();
                self.func.blocks[inc_body_last].push(
                    Some(new_idx),
                    MirOp::Add(MirValue::temp(old_idx), MirValue::Const(MirConst::Int(1))),
                    loc,
                );
                self.func.blocks[inc_body_last].push(
                    None,
                    MirOp::Store(MirValue::temp(idx_ptr), MirValue::temp(new_idx)),
                    loc,
                );
                self.func.blocks[inc_body_last].terminator = MirTerminator::Br(cond_block);
            }
            Statement::Unsafe { body } => {
                for stmt in body {
                    self.lower_statement(stmt);
                }
            }
            _ => {}
        }
    }

    fn lower_expression(&mut self, expr: &Expression) -> MirValue {
        let loc = (0, 0);
        match expr {
            Expression::Literal(lit) => self.lower_literal(lit),
            Expression::Identifier(id) => {
                if let Some(&ptr) = self.vars.get(&id.name) {
                    let ty = self.var_types.get(&id.name).cloned().unwrap_or(DataType::Unknown);
                    // Don't load aggregate types — keep pointer for GEP
                    if matches!(&ty, DataType::Array { .. }) {
                        return MirValue::temp(ptr);
                    }
                    let loaded = self.new_temp();
                    let last = self.func.blocks.len() - 1;
                    self.func.blocks[last].push(
                        Some(loaded),
                        MirOp::Load(
                            MirValue::temp(ptr),
                            MirType { data_type: ty },
                        ),
                        loc,
                    );
                    MirValue::temp(loaded)
                } else {
                    MirValue::Global(id.name.clone())
                }
            }
            Expression::BinaryOp {
                operator,
                left,
                right,
                ..
            } => {
                let l = self.lower_expression(left);
                let r = self.lower_expression(right);
                let result = self.new_temp();
                let mir_op = match operator.as_str() {
                    "+" => MirOp::Add(l, r),
                    "-" => MirOp::Sub(l, r),
                    "*" => MirOp::Mul(l, r),
                    "/" => MirOp::SDiv(l, r),
                    "==" => MirOp::ICmp(MirCmp::Eq, l, r),
                    "!=" => MirOp::ICmp(MirCmp::Ne, l, r),
                    "<" => MirOp::ICmp(MirCmp::Lt, l, r),
                    "<=" => MirOp::ICmp(MirCmp::Le, l, r),
                    ">" => MirOp::ICmp(MirCmp::Gt, l, r),
                    ">=" => MirOp::ICmp(MirCmp::Ge, l, r),
                    "&&" => MirOp::And(l, r),
                    "||" => MirOp::Or(l, r),
                    _ => MirOp::Add(l, r),
                };
                let last = self.func.blocks.len() - 1;
                self.func.blocks[last].push(Some(result), mir_op, loc);
                MirValue::temp(result)
            }
            Expression::Call { name, args, data_type, .. } if name == "__if_expr" && args.len() == 3 => {
                let cond = self.lower_expression(&args[0]);
                let then_expr = Self::extract_closure_expr(&args[1]);
                let else_expr = Self::extract_closure_expr(&args[2]);
                let then_val = self.lower_expression(then_expr);
                let else_val = self.lower_expression(else_expr);
                let result = self.new_temp();
                let ret_ty = MirType { data_type: data_type.clone() };

                let current_block = self.func.blocks.len() - 1;
                self.func.blocks[current_block].push(
                    Some(result),
                    MirOp::Alloca(ret_ty.clone()),
                    loc,
                );

                let then_block = self.new_block("ifexpr_then");
                let else_block = self.new_block("ifexpr_else");
                let end_block = self.new_block("ifexpr_end");

                self.func.blocks[current_block].terminator =
                    MirTerminator::BrCond(cond, then_block, else_block);

                self.func.blocks[then_block].push(
                    None,
                    MirOp::Store(MirValue::temp(result), then_val),
                    loc,
                );
                self.func.blocks[then_block].terminator =
                    MirTerminator::Br(end_block);

                self.func.blocks[else_block].push(
                    None,
                    MirOp::Store(MirValue::temp(result), else_val),
                    loc,
                );
                self.func.blocks[else_block].terminator =
                    MirTerminator::Br(end_block);

                let loaded = self.new_temp();
                self.func.blocks[end_block].push(
                    Some(loaded),
                    MirOp::Load(MirValue::temp(result), ret_ty),
                    loc,
                );
                MirValue::temp(loaded)
            }
            Expression::Call {
                name,
                args,
                ..
            } if name == "__type_matches" => {
                MirValue::Const(MirConst::Bool(true))
            }
            Expression::Call {
                name,
                args,
                data_type,
                ..
            } => {
                let mir_args: Vec<MirValue> =
                    args.iter().map(|a| self.lower_expression(a)).collect();
                let result = self.new_temp();
                let last = self.func.blocks.len() - 1;
                self.func.blocks[last].push(
                    Some(result),
                    MirOp::Call(
                        name.clone(),
                        mir_args,
                        MirType {
                            data_type: data_type.clone(),
                        },
                    ),
                    loc,
                );
                MirValue::temp(result)
            }
            Expression::Match {
                value,
                cases,
                default,
                ..
            } => {
                let _val = self.lower_expression(value);
                let result_ptr = self.new_temp();
                let result_type = MirType {
                    data_type: DataType::Unknown,
                };
                let end_block = self.new_block("match_end");
                for i in 0..cases.len() {
                    let _ = self.new_block(&format!("match_case_{}", i));
                }
                let _default = self.new_block("match_default");

                {
                    let last = self.func.blocks.len() - 2 - cases.len();
                    self.func.blocks[last].push(
                        Some(result_ptr),
                        MirOp::Alloca(result_type.clone()),
                        loc,
                    );
                }

                for (pattern, body) in cases.iter() {
                    let _ = self.lower_expression(pattern);
                    let body_val = self.lower_expression(body);
                    let last = self.func.blocks.len() - 1;
                    self.func.blocks[last]
                        .push(None, MirOp::Store(MirValue::temp(result_ptr), body_val), loc);
                    self.func.blocks[last].terminator = MirTerminator::Br(end_block);
                }

                {
                    let default_val = self.lower_expression(default);
                    let last = self.func.blocks.len() - 1;
                    self.func.blocks[last]
                        .push(None, MirOp::Store(MirValue::temp(result_ptr), default_val), loc);
                    self.func.blocks[last].terminator = MirTerminator::Br(end_block);
                }

                let loaded = self.new_temp();
                let last = self.func.blocks.len() - 1;
                self.func.blocks[last].push(
                    Some(loaded),
                    MirOp::Load(MirValue::temp(result_ptr), result_type),
                    loc,
                );
                MirValue::temp(loaded)
            }
            Expression::NamedArg { value, .. } => {
                self.lower_expression(value)
            }
            Expression::MemberAccess { target, member, data_type } => {
                let struct_name = self.get_struct_name(target);
                if let Some(struct_name) = struct_name {
                    let norm_name = struct_name
                        .split_once('[')
                        .map(|(base, _)| base.to_string())
                        .unwrap_or_else(|| struct_name.clone());
                    if let Some(fields) = self.struct_types.get(&norm_name) {
                        if let Some(field_index) =
                            fields.iter().position(|(name, _)| name == member)
                        {
                            let target_val = self.lower_expression(target);
                            let last = self.func.blocks.len() - 1;
                            let gep_result = self.new_temp();
                            self.func.blocks[last].push(
                                Some(gep_result),
                                MirOp::Gep(
                                    target_val,
                                    vec![
                                        MirValue::Const(MirConst::Int(0)),
                                        MirValue::Const(MirConst::Int(field_index as i64)),
                                    ],
                                    norm_name.clone(),
                                ),
                                loc,
                            );
                            let load_result = self.new_temp();
                            self.func.blocks[last].push(
                                Some(load_result),
                                MirOp::Load(
                                    MirValue::temp(gep_result),
                                    MirType {
                                        data_type: data_type.clone(),
                                    },
                                ),
                                loc,
                            );
                            return MirValue::temp(load_result);
                        }
                    }
                }
                MirValue::Const(MirConst::None)
            }
            Expression::Tuple { elements, data_type } => {
                match data_type {
                    DataType::StructNamed(name) => {
                        let mir_args: Vec<MirValue> =
                            elements.iter().map(|e| self.lower_expression(e)).collect();
                        let result = self.new_temp();
                        let last = self.func.blocks.len() - 1;
                        self.func.blocks[last].push(
                            Some(result),
                            MirOp::Call(
                                name.clone(),
                                mir_args,
                                MirType {
                                    data_type: data_type.clone(),
                                },
                            ),
                            loc,
                        );
                        MirValue::temp(result)
                    }
                    _ => MirValue::Const(MirConst::None),
                }
            }
            Expression::Index { target, index, data_type } => {
                let target_val = self.lower_expression(target);
                let index_val = self.lower_expression(index);
                let last = self.func.blocks.len() - 1;
                let gep = self.new_temp();
                let elem_llvm = llvm_elem_type_str(data_type);
                self.func.blocks[last].push(
                    Some(gep),
                    MirOp::Gep(target_val, vec![index_val], elem_llvm),
                    loc,
                );
                let loaded = self.new_temp();
                self.func.blocks[last].push(
                    Some(loaded),
                    MirOp::Load(
                        MirValue::temp(gep),
                        MirType { data_type: data_type.clone() },
                    ),
                    loc,
                );
                MirValue::temp(loaded)
            }
            Expression::Reference { expr, .. } => {
                if let Expression::Identifier(id) = expr.as_ref() {
                    if let Some(&ptr) = self.vars.get(&id.name) {
                        MirValue::temp(ptr)
                    } else {
                        MirValue::Const(MirConst::None)
                    }
                } else {
                    // For non-trivial reference targets, lower the expr and return its ptr
                    // Currently unsupported, fallback
                    MirValue::Const(MirConst::None)
                }
            }
            Expression::Dereference { expr, data_type } => {
                let ptr_val = self.lower_expression(expr);
                let loaded = self.new_temp();
                let last = self.func.blocks.len() - 1;
                self.func.blocks[last].push(
                    Some(loaded),
                    MirOp::Load(
                        ptr_val,
                        MirType { data_type: data_type.clone() },
                    ),
                    loc,
                );
                MirValue::temp(loaded)
            }
            Expression::UnaryOp { operator, operand, .. } => {
                let op_val = self.lower_expression(operand);
                let result = self.new_temp();
                let last = self.func.blocks.len() - 1;
                match operator.as_str() {
                    "-" => {
                        let zero = MirValue::Const(MirConst::Int(0));
                        self.func.blocks[last].push(
                            Some(result),
                            MirOp::Sub(zero, op_val),
                            loc,
                        );
                    }
                    "!" => {
                        let zero = MirValue::Const(MirConst::Bool(false));
                        self.func.blocks[last].push(
                            Some(result),
                            MirOp::ICmp(MirCmp::Eq, op_val, zero),
                            loc,
                        );
                    }
                    _ => {}
                }
                MirValue::temp(result)
            }
            Expression::List { elements, element_type: _, data_type } => {
                match data_type {
                    DataType::Array { element_type, .. } => {
                        let last = self.func.blocks.len() - 1;
                        let arr_ptr = self.new_temp();
                        self.func.blocks[last].push(
                            Some(arr_ptr),
                            MirOp::Alloca(MirType { data_type: data_type.clone() }),
                            loc,
                        );
                        let elem_llvm = llvm_elem_type_str(element_type);
                        for (i, elem) in elements.iter().enumerate() {
                            let elem_val = self.lower_expression(elem);
                            let last = self.func.blocks.len() - 1;
                            let gep = self.new_temp();
                            self.func.blocks[last].push(
                                Some(gep),
                                MirOp::Gep(
                                    MirValue::temp(arr_ptr),
                                    vec![MirValue::Const(MirConst::Int(i as i64))],
                                    elem_llvm.clone(),
                                ),
                                loc,
                            );
                            self.func.blocks[last].push(
                                None,
                                MirOp::Store(MirValue::temp(gep), elem_val),
                                loc,
                            );
                        }
                        MirValue::temp(arr_ptr)
                    }
                    _ => MirValue::Const(MirConst::None),
                }
            }
            Expression::EnumVariantPath { .. } | Expression::EnumVariant { .. } => {
                MirValue::Const(MirConst::Int(0))
            }
            _ => MirValue::Const(MirConst::None),
        }
    }

    fn extract_closure_expr(expr: &Expression) -> &Expression {
        if let Expression::Closure { body, .. } = expr {
            if let Some(Statement::Return(Some(inner))) = body.first() {
                return inner;
            }
        }
        expr
    }

    fn get_target_elem_type(&self, target: &Expression) -> String {
        if let Expression::Identifier(id) = target {
            if let Some(ty) = self.var_types.get(&id.name) {
                match ty {
                    DataType::Array { element_type, .. }
                    | DataType::Vector { element_type, .. }
                    | DataType::Slice { element_type, .. } => {
                        return llvm_elem_type_str(element_type);
                    }
                    _ => {}
                }
            }
        }
        "i64".to_string()
    }

    fn get_struct_name(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Identifier(id) => {
                self.var_types.get(&id.name).and_then(|t| match t {
                    DataType::StructNamed(name) => Some(name.clone()),
                    _ => None,
                })
            }
            _ => None,
        }
    }

    fn lower_literal(&self, lit: &Literal) -> MirValue {
        match lit {
            Literal::Int(v) => MirValue::Const(MirConst::Int(*v)),
            Literal::Float(v) => MirValue::Const(MirConst::Float(*v)),
            Literal::Bool(v) => MirValue::Const(MirConst::Bool(*v)),
            Literal::Char(v) => {
                MirValue::Const(MirConst::Char(char::from_u32(*v).unwrap_or('\0')))
            }
            Literal::Str(v) => MirValue::Const(MirConst::Str(v.clone())),
            Literal::None => MirValue::Const(MirConst::None),
            _ => MirValue::Const(MirConst::None),
        }
    }
}

fn llvm_elem_type_str(dt: &DataType) -> String {
    match dt {
        DataType::I64 | DataType::Char | DataType::U64 => "i64".to_string(),
        DataType::I32 | DataType::U32 => "i32".to_string(),
        DataType::I16 | DataType::U16 => "i16".to_string(),
        DataType::I8 | DataType::U8 => "i8".to_string(),
        DataType::F32 => "float".to_string(),
        DataType::F64 => "double".to_string(),
        DataType::Bool => "i1".to_string(),
        DataType::None => "i64".to_string(),
        DataType::StructNamed(name) => format!("struct:{}", name),
        _ => "i64".to_string(),
    }
}
