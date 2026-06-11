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
}

pub fn lower_program(program: &Program) -> MirProgram {
    let mut functions = Vec::new();
    let mut entry_point = None;
    let mut extern_functions = Vec::new();
    let mut seen_functions = HashSet::new();

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
                    _ => {}
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
            _ => {}
        }
    }

    fn lower_expression(&mut self, expr: &Expression) -> MirValue {
        let loc = (0, 0);
        match expr {
            Expression::Literal(lit) => self.lower_literal(lit),
            Expression::Identifier(id) => {
                if let Some(&ptr) = self.vars.get(&id.name) {
                    let loaded = self.new_temp();
                    let last = self.func.blocks.len() - 1;
                    let ty = self.var_types.get(&id.name).cloned().unwrap_or(DataType::Unknown);
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
