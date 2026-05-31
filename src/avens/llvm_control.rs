use super::*;

impl LlvmIrGen {
    pub(super) fn compile_if_expr(&mut self, args: &[Expression], data_type: &DataType) -> Result<LlValue> {
        if args.len() != 3 {
            return Err(MireError::new(ErrorKind::Runtime {
                message: "Avenys __if_expr expects 3 arguments".to_string(),
            }));
        }
        let then_expr = self.closure_return_expr(&args[1], "__if_expr then")?;
        let else_expr = self.closure_return_expr(&args[2], "__if_expr else")?;
        let result_ty = self.map_type(data_type)?;
        let result_ptr = self.tmp();
        let result_ty_clone = result_ty.clone();
        self.entry_allocas.push(format!(
            "  {result_ptr} = alloca {}",
            self.ty(result_ty_clone)
        ));

        let then_label = self.label("ifexpr_then");
        let else_label = self.label("ifexpr_else");
        let end_label = self.label("ifexpr_end");
        let cond_val = self.compile_expr(&args[0])?;
        let cond = self.cast_to_i1(cond_val)?;
        self.body.push(format!(
            "  br i1 {}, label %{then_label}, label %{else_label}",
            cond.repr
        ));

        self.body.push(format!("{then_label}:"));
        let then_value = self.compile_expr(then_expr)?;
        self.store_casted(&result_ptr, result_ty.clone(), then_value)?;
        self.body.push(format!("  br label %{end_label}"));

        self.body.push(format!("{else_label}:"));
        let else_value = self.compile_expr(else_expr)?;
        self.store_casted(&result_ptr, result_ty.clone(), else_value)?;
        self.body.push(format!("  br label %{end_label}"));

        self.body.push(format!("{end_label}:"));
        let loaded = self.tmp();
        self.body.push(format!(
            "  {loaded} = load {}, ptr {}",
            self.ty(result_ty.clone()),
            result_ptr
        ));
        Ok(LlValue {
            ty: result_ty,
            repr: loaded,
            owned: false,
        })
    }
    pub(super) fn compile_match_statement(
        &mut self,
        value: &Expression,
        cases: &[(Expression, Vec<Statement>)],
        default: &[Statement],
    ) -> Result<()> {
        let match_value = self.compile_expr(value)?;
        let match_data_type = self.expression_data_type(value);
        let end_label = self.label("match_end");
        let default_label = self.label("match_default");
        let mut next_label = None;

        for (index, (pattern, body)) in cases.iter().enumerate() {
            let check_label = next_label
                .take()
                .unwrap_or_else(|| self.label("match_check"));
            let body_label = self.label(&format!("match_body_{index}"));
            let fallthrough_label = if index + 1 == cases.len() {
                default_label.clone()
            } else {
                self.label(&format!("match_next_{index}"))
            };

            if index > 0 {
                self.body.push(format!("{check_label}:"));
            }

            let cond = self.compile_match_case_condition(&match_value, &match_data_type, pattern)?;
            self.body.push(format!(
                "  br i1 {}, label %{body_label}, label %{fallthrough_label}",
                cond.repr
            ));

            self.body.push(format!("{body_label}:"));
            let previous_binding = self.bind_match_pattern_payloads(&match_value, pattern)?;
            for stmt in body {
                self.compile_statement(stmt)?;
            }
            self.restore_match_pattern_payloads(previous_binding);
            self.body.push(format!("  br label %{end_label}"));
            next_label = Some(fallthrough_label);
        }

        let default_entry = next_label.unwrap_or(default_label.clone());
        self.body.push(format!("{default_entry}:"));
        for stmt in default {
            self.compile_statement(stmt)?;
        }
        self.body.push(format!("  br label %{end_label}"));
        self.body.push(format!("{end_label}:"));
        Ok(())
    }
    pub(super) fn compile_match_expr(
        &mut self,
        value: &Expression,
        cases: &[(Expression, Expression)],
        default: &Expression,
        data_type: &DataType,
    ) -> Result<LlValue> {
        let match_value = self.compile_expr(value)?;
        let match_data_type = self.expression_data_type(value);
        let result_ty = self.map_type(data_type)?;
        let result_ptr = self.tmp();
        self.entry_allocas.push(format!(
            "  {result_ptr} = alloca {}",
            self.ty(result_ty.clone())
        ));

        let end_label = self.label("match_expr_end");
        let default_label = self.label("match_expr_default");
        let mut next_label = None;

        for (index, (pattern, result_expr)) in cases.iter().enumerate() {
            let check_label = next_label
                .take()
                .unwrap_or_else(|| self.label("match_expr_check"));
            let body_label = self.label(&format!("match_expr_body_{index}"));
            let fallthrough_label = if index + 1 == cases.len() {
                default_label.clone()
            } else {
                self.label(&format!("match_expr_next_{index}"))
            };

            if index > 0 {
                self.body.push(format!("{check_label}:"));
            }

            let cond = self.compile_match_case_condition(&match_value, &match_data_type, pattern)?;
            self.body.push(format!(
                "  br i1 {}, label %{body_label}, label %{fallthrough_label}",
                cond.repr
            ));

            self.body.push(format!("{body_label}:"));
            let previous_binding = self.bind_match_pattern_payloads(&match_value, pattern)?;
            let body_value = self.compile_expr(result_expr)?;
            self.restore_match_pattern_payloads(previous_binding);
            self.store_casted(&result_ptr, result_ty.clone(), body_value)?;
            self.body.push(format!("  br label %{end_label}"));
            next_label = Some(fallthrough_label);
        }

        let default_entry = next_label.unwrap_or(default_label.clone());
        self.body.push(format!("{default_entry}:"));

        // Handle default case - if it's a wildcard _, use a default value
        // Also handle implicit None placeholder (when no default arm is provided)
        let is_implicit_none = matches!(default, Expression::Literal(Literal::None));
        if is_implicit_none {
            // No explicit default - use type's default value
            let default_val = self.default_value(result_ty.clone());
            self.store_casted(&result_ptr, result_ty.clone(), default_val)?;
        } else if let Expression::Identifier(ident) = default {
            if ident.name == "_" {
                // Default case - just set result to 0 or default for the type
                let default_val = self.default_value(result_ty.clone());
                self.store_casted(&result_ptr, result_ty.clone(), default_val)?;
            } else {
                let default_value = self.compile_expr(default)?;
                self.store_casted(&result_ptr, result_ty.clone(), default_value)?;
            }
        } else {
            let default_value = self.compile_expr(default)?;
            self.store_casted(&result_ptr, result_ty.clone(), default_value)?;
        }
        self.body.push(format!("  br label %{end_label}"));

        self.body.push(format!("{end_label}:"));
        let loaded = self.tmp();
        self.body.push(format!(
            "  {loaded} = load {}, ptr {}",
            self.ty(result_ty.clone()),
            result_ptr
        ));
        Ok(LlValue {
            ty: result_ty,
            repr: loaded,
            owned: false,
        })
    }
    pub(super) fn compile_match_case_condition(
        &mut self,
        value: &LlValue,
        value_data_type: &DataType,
        pattern: &Expression,
    ) -> Result<LlValue> {
        // Handle wildcard pattern - always matches (true)
        if let Expression::Identifier(ident) = pattern
            && ident.name == "_"
        {
            let result = self.tmp();
            self.body.push(format!("  {result} = add i1 0, 1"));
            return Ok(LlValue {
                ty: LlType::I1,
                repr: result,
                owned: false,
            });
        }

        if let Expression::Call { name, args, .. } = pattern {
            if name == "__match_guard" {
                if args.len() != 2 {
                    return Err(MireError::new(ErrorKind::Backend {
                        message: "Avenys __match_guard expects pattern and guard".to_string(),
                    }));
                }
                let pattern_cond =
                    self.compile_match_case_condition(value, value_data_type, &args[0])?;
                let guard_value = self.compile_expr(&args[1])?;
                let guard_cond = self.cast_to_i1(guard_value)?;
                let merged = self.tmp();
                self.body.push(format!(
                    "  {merged} = and i1 {}, {}",
                    pattern_cond.repr, guard_cond.repr
                ));
                return Ok(LlValue {
                    ty: LlType::I1,
                    repr: merged,
                    owned: false,
                });
            }
            if name == "__match_or" {
                if args.len() != 2 {
                    return Err(MireError::new(ErrorKind::Backend {
                        message: "Avenys __match_or expects two patterns".to_string(),
                    }));
                }
                let left = self.compile_match_case_condition(value, value_data_type, &args[0])?;
                let right = self.compile_match_case_condition(value, value_data_type, &args[1])?;
                let merged = self.tmp();
                self.body
                    .push(format!("  {merged} = or i1 {}, {}", left.repr, right.repr));
                return Ok(LlValue {
                    ty: LlType::I1,
                    repr: merged,
                    owned: false,
                });
            }
            if name == "__match_range" {
                if args.len() != 2 {
                    return Err(MireError::new(ErrorKind::Backend {
                        message: "Avenys __match_range expects start and end".to_string(),
                    }));
                }
                let start_raw = self.compile_expr(&args[0])?;
                let start = self.cast_to_i64(start_raw)?;
                let end_raw = self.compile_expr(&args[1])?;
                let end = self.cast_to_i64(end_raw)?;
                let value_i64 = self.cast_to_i64(value.clone())?;
                let ge = self.tmp();
                self.body.push(format!(
                    "  {ge} = icmp sge i64 {}, {}",
                    value_i64.repr, start.repr
                ));
                let le = self.tmp();
                self.body.push(format!(
                    "  {le} = icmp sle i64 {}, {}",
                    value_i64.repr, end.repr
                ));
                let merged = self.tmp();
                self.body.push(format!("  {merged} = and i1 {ge}, {le}"));
                return Ok(LlValue {
                    ty: LlType::I1,
                    repr: merged,
                    owned: false,
                });
            }
        }

        // Handle enum variant patterns (Status.Ok or Result.Ok(value))
        if let Expression::EnumVariantPath {
            enum_name,
            variant_name,
            ..
        } = pattern
            && value.ty == LlType::Ptr
        {
            let (enum_ty, tag) = {
                let (enum_info, variant) = self.lookup_enum_variant(enum_name, variant_name)?;
                (enum_info.llvm_type.clone(), variant.tag)
            };
            let tag_ptr = self.tmp();
            self.body.push(format!(
                "  {tag_ptr} = getelementptr inbounds {}, ptr {}, i32 0, i32 0",
                enum_ty, value.repr
            ));
            let loaded_tag = self.tmp();
            self.body
                .push(format!("  {loaded_tag} = load i32, ptr {tag_ptr}"));
            let result = self.tmp();
            self.body
                .push(format!("  {result} = icmp eq i32 {loaded_tag}, {}", tag));
            return Ok(LlValue {
                ty: LlType::I1,
                repr: result,
                owned: false,
            });
        }

        // Handle enum variant with payloads: Ok(value) / Pair(a b) in match pattern
        if let Expression::EnumVariant {
            enum_name,
            variant_name,
            payloads: _,
            ..
        } = pattern
            && value.ty == LlType::Ptr
        {
            let (enum_ty, tag) = {
                let (enum_info, variant) = self.lookup_enum_variant(enum_name, variant_name)?;
                (enum_info.llvm_type.clone(), variant.tag)
            };
            let tag_ptr = self.tmp();
            self.body.push(format!(
                "  {tag_ptr} = getelementptr inbounds {}, ptr {}, i32 0, i32 0",
                enum_ty, value.repr
            ));
            let loaded_tag = self.tmp();
            self.body
                .push(format!("  {loaded_tag} = load i32, ptr {tag_ptr}"));
            let result = self.tmp();
            self.body
                .push(format!("  {result} = icmp eq i32 {loaded_tag}, {}", tag));

            return Ok(LlValue {
                ty: LlType::I1,
                repr: result,
                owned: false,
            });
        }

        let pattern_value = self.compile_expr(pattern)?;
        let result = self.tmp();

        match (&value.ty, &pattern_value.ty) {
            (LlType::Ptr, LlType::Ptr) => {
                if matches!(value_data_type, DataType::Str) {
                    let cmp_value = self.tmp();
                    self.body.push(format!(
                        "  {cmp_value} = call i32 @strcmp(ptr {}, ptr {})",
                        value.repr, pattern_value.repr
                    ));
                    self.body
                        .push(format!("  {result} = icmp eq i32 {cmp_value}, 0"));
                } else {
                    self.body.push(format!(
                        "  {result} = icmp eq ptr {}, {}",
                        value.repr, pattern_value.repr
                    ));
                }
            }
            (LlType::I1, LlType::I1) => {
                self.body.push(format!(
                    "  {result} = icmp eq i1 {}, {}",
                    value.repr, pattern_value.repr
                ));
            }
            (LlType::I64, LlType::I64) => {
                self.body.push(format!(
                    "  {result} = icmp eq i64 {}, {}",
                    value.repr, pattern_value.repr
                ));
            }
            (LlType::I1, LlType::I64)
            | (LlType::I64, LlType::I1)
            | (LlType::I64, LlType::Ptr)
            | (LlType::Ptr, LlType::I64) => {
                let lhs = self.cast_to_i64(value.clone())?;
                let rhs = self.cast_to_i64(pattern_value)?;
                self.body.push(format!(
                    "  {result} = icmp eq i64 {}, {}",
                    lhs.repr, rhs.repr
                ));
            }
            _ => {
                return Err(MireError::new(ErrorKind::Runtime {
                    message: format!(
                        "Avenys does not yet compare match values of type {:?} against {:?}",
                        value.ty, pattern_value.ty
                    ),
                }));
            }
        }

        Ok(LlValue {
            ty: LlType::I1,
            repr: result,
            owned: false,
        })
    }

    pub(super) fn heap_box_value(&mut self, value: LlValue) -> Result<LlValue> {
        let (size, store_ty, store_repr) = match value.ty {
            LlType::I1 => (1, "i1", self.cast_to_i1(value)?.repr),
            LlType::I8 => (1, "i8", value.repr),
            LlType::I64 => (8, "i64", self.cast_to_i64(value)?.repr),
            LlType::F64 => (8, "double", value.repr),
            LlType::Ptr => (8, "ptr", self.cast_to_type(value, LlType::Ptr)?.repr),
            LlType::Struct(_) => {
                return Err(MireError::new(ErrorKind::Backend {
                    message: "Cannot heap-box a struct value".to_string(),
                }));
            }
        };

        let boxed = self.tmp();
        self.body
            .push(format!("  {boxed} = call ptr @malloc(i64 {size})"));
        self.body
            .push(format!("  store {store_ty} {store_repr}, ptr {boxed}"));

        Ok(LlValue {
            ty: LlType::Ptr,
            repr: boxed,
            owned: true,
        })
    }

    pub(super) fn bind_match_pattern_payloads(
        &mut self,
        value: &LlValue,
        pattern: &Expression,
    ) -> Result<Vec<(String, Option<VarInfo>)>> {
        let Expression::EnumVariant {
            enum_name,
            variant_name,
            payloads,
            ..
        } = pattern
        else {
            return Ok(Vec::new());
        };

        if value.ty != LlType::Ptr {
            return Ok(Vec::new());
        }

        let (enum_ty, variant_payload_types) = {
            let (enum_info, variant) = self.lookup_enum_variant(enum_name, variant_name)?;
            (enum_info.llvm_type.clone(), variant.payload_types.clone())
        };

        if variant_payload_types.is_empty() {
            return Ok(Vec::new());
        }

        let mut previous = Vec::new();
        for (index, payload) in payloads.iter().enumerate() {
            let Expression::Identifier(id) = payload else {
                continue;
            };

            let payload_ty = variant_payload_types
                .get(index)
                .cloned()
                .unwrap_or(LlType::I64);
            let payload_data_type = self.expression_data_type(payload);
            let payload_ptr = self.tmp();
            self.entry_allocas.push(format!(
                "  {payload_ptr} = alloca {}",
                self.ty(payload_ty.clone())
            ));
            let payload_gep = self.tmp();
            self.body.push(format!(
                "  {payload_gep} = getelementptr inbounds {}, ptr {}, i32 0, i32 1, i32 {}",
                enum_ty, value.repr, index
            ));
            let payload_raw = self.tmp();
            self.body
                .push(format!("  {payload_raw} = load i64, ptr {payload_gep}"));
            let payload_value = self.cast_enum_payload_value(payload_raw, payload_ty.clone())?;
            self.store_casted(&payload_ptr, payload_ty.clone(), payload_value)?;

            previous.push((
                id.name.clone(),
                self.vars.insert(
                    id.name.clone(),
                    VarInfo {
                        ptr: payload_ptr,
                        ty: payload_ty,
                        data_type: payload_data_type,
                        owns_heap_string: false,
                        struct_name: None,
                    },
                ),
            ));
        }

        Ok(previous)
    }

    pub(super) fn restore_match_pattern_payloads(&mut self, previous: Vec<(String, Option<VarInfo>)>) {
        for (name, prior) in previous {
            if let Some(prior) = prior {
                self.vars.insert(name, prior);
            } else {
                self.vars.remove(&name);
            }
        }
    }

    pub(super) fn lookup_enum_variant<'a>(
        &'a self,
        enum_name: &str,
        variant_name: &str,
    ) -> Result<(&'a EnumInfo, &'a VariantInfo)> {
        let enum_info = self.user_enums.get(enum_name).ok_or_else(|| {
            MireError::new(ErrorKind::Runtime {
                message: format!("Unknown enum '{}'", enum_name),
            })
        })?;
        let variant = enum_info.variants.get(variant_name).ok_or_else(|| {
            MireError::new(ErrorKind::Runtime {
                message: format!("Enum '{}' has no variant '{}'", enum_name, variant_name),
            })
        })?;
        Ok((enum_info, variant))
    }

    pub(super) fn cast_enum_payload_value(&mut self, raw_value: String, target_ty: LlType) -> Result<LlValue> {
        match target_ty {
            LlType::I64 => Ok(LlValue {
                ty: LlType::I64,
                repr: raw_value,
                owned: false,
            }),
            LlType::I1 => {
                let bool_value = self.tmp();
                self.body
                    .push(format!("  {bool_value} = icmp ne i64 {raw_value}, 0"));
                Ok(LlValue {
                    ty: LlType::I1,
                    repr: bool_value,
                    owned: false,
                })
            }
            LlType::F64 => Ok(LlValue {
                ty: LlType::F64,
                repr: raw_value,
                owned: false,
            }),
            LlType::I8 => Ok(LlValue {
                ty: LlType::I8,
                repr: raw_value,
                owned: false,
            }),
            LlType::Struct(_) => Err(MireError::new(ErrorKind::Backend {
                message: "Struct type not supported here".to_string(),
            })),
            LlType::Ptr => {
                let ptr_value = self.tmp();
                self.body
                    .push(format!("  {ptr_value} = inttoptr i64 {raw_value} to ptr"));
                Ok(LlValue {
                    ty: LlType::Ptr,
                    repr: ptr_value,
                    owned: false,
                })
            }
        }
    }

    pub(super) fn emit_nonzero_check(&mut self, value_repr: &str, message: &str) {
        let cond = self.tmp();
        self.body
            .push(format!("  {cond} = icmp ne i64 {value_repr}, 0"));
        self.emit_runtime_guard(cond, message);
    }

    pub(super) fn emit_nonzero_check_f64(&mut self, value_repr: &str, message: &str) {
        let cond = self.tmp();
        self.body
            .push(format!("  {cond} = fcmp une double {value_repr}, 0.0"));
        self.emit_runtime_guard(cond, message);
    }

    pub(super) fn emit_bounds_check(&mut self, index: LlValue, len: LlValue, message: &str) {
        let non_negative = self.tmp();
        self.body
            .push(format!("  {non_negative} = icmp sge i64 {}, 0", index.repr));
        let within_len = self.tmp();
        self.body.push(format!(
            "  {within_len} = icmp slt i64 {}, {}",
            index.repr, len.repr
        ));
        let in_bounds = self.tmp();
        self.body.push(format!(
            "  {in_bounds} = and i1 {non_negative}, {within_len}"
        ));
        self.emit_runtime_guard(in_bounds, message);
    }

    pub(super) fn emit_runtime_guard(&mut self, condition_repr: String, message: &str) {
        let ok_label = self.label("rt_ok");
        let fail_label = self.label("rt_fail");
        self.body.push(format!(
            "  br i1 {condition_repr}, label %{ok_label}, label %{fail_label}"
        ));
        self.body.push(format!("{fail_label}:"));
        let message_value = self.string_value(message);
        self.body.push(format!(
            "  call void @rt_panic(ptr {})",
            message_value.repr
        ));
        self.body.push("  unreachable".to_string());
        self.body.push(format!("{ok_label}:"));
    }
    pub(super) fn compile_do_while(&mut self, args: &[Expression]) -> Result<()> {
        if args.len() != 2 {
            return Err(MireError::new(ErrorKind::Runtime {
                message: "Avenys __do_while expects 2 closures".to_string(),
            }));
        }
        let body = self.closure_statements(&args[0], "__do_while body")?;
        let condition = self.closure_return_expr(&args[1], "__do_while condition")?;

        let body_label = self.label("dowhile_body");
        let cond_label = self.label("dowhile_cond");
        let end_label = self.label("dowhile_end");

        self.body.push(format!("  br label %{body_label}"));
        self.body.push(format!("{body_label}:"));
        self.loop_stack.push(LoopLabels {
            break_label: end_label.clone(),
            continue_label: cond_label.clone(),
        });
        for stmt in body {
            self.compile_statement(stmt)?;
        }
        self.loop_stack.pop();
        self.body.push(format!("  br label %{cond_label}"));

        self.body.push(format!("{cond_label}:"));
        let cond_val = self.compile_expr(condition)?;
        let cond = self.cast_to_i1(cond_val)?;
        self.body.push(format!(
            "  br i1 {}, label %{body_label}, label %{end_label}",
            cond.repr
        ));
        self.body.push(format!("{end_label}:"));
        Ok(())
    }
    pub(super) fn compile_for_range(
        &mut self,
        variable: &str,
        index: Option<&str>,
        iterable: &Expression,
        body: &[Statement],
    ) -> Result<()> {
        if !matches!(iterable, Expression::Call { name, .. } if name == "range") {
            return self.compile_for_collection(variable, index, iterable, body);
        }

        let (start_expr, end_expr, step_expr) = match iterable {
            Expression::Call { name, args, .. } if name == "range" => match args.len() {
                1 => (
                    Expression::Literal(Literal::Int(0)),
                    args[0].clone(),
                    Expression::Literal(Literal::Int(1)),
                ),
                2 => (
                    args[0].clone(),
                    args[1].clone(),
                    Expression::Literal(Literal::Int(1)),
                ),
                3 => (args[0].clone(), args[1].clone(), args[2].clone()),
                _ => {
                    return Err(MireError::new(ErrorKind::Runtime {
                        message: "Avenys range(...) supports 1 to 3 arguments".to_string(),
                    }));
                }
            },
            other => {
                return Err(MireError::new(ErrorKind::Runtime {
                    message: format!(
                        "Avenys for-loop currently supports range(...) only, found {:?}",
                        other
                    ),
                }));
            }
        };

        let start_value = self.compile_expr(&start_expr)?;
        let start = self.cast_to_i64(start_value)?;
        let end_value = self.compile_expr(&end_expr)?;
        let end = self.cast_to_i64(end_value)?;
        let step_value = self.compile_expr(&step_expr)?;
        let step = self.cast_to_i64(step_value)?;
        let iter_ptr = self.tmp();
        self.entry_allocas
            .push(format!("  {iter_ptr} = alloca i64"));
        self.body
            .push(format!("  store i64 {}, ptr {}", start.repr, iter_ptr));

        let saved = self.vars.insert(
            variable.to_string(),
            VarInfo {
                ptr: iter_ptr.clone(),
                ty: LlType::I64,
                data_type: DataType::I64,
                owns_heap_string: false,
                struct_name: None,
            },
        );
        let (index_ptr, saved_index) = if let Some(index_name) = index {
            let index_ptr = self.tmp();
            self.entry_allocas.push(format!("  {index_ptr} = alloca i64"));
            self.body.push(format!("  store i64 0, ptr {index_ptr}"));
            let saved = self.vars.insert(
                index_name.to_string(),
                VarInfo {
                    ptr: index_ptr.clone(),
                    ty: LlType::I64,
                    data_type: DataType::I64,
                    owns_heap_string: false,
                    struct_name: None,
                },
            );
            (Some(index_ptr), saved)
        } else {
            (None, None)
        };

        let cond_label = self.label("for_cond");
        let body_label = self.label("for_body");
        let continue_label = self.label("for_continue");
        let positive_label = self.label("for_positive");
        let negative_label = self.label("for_negative");
        let cond_merge_label = self.label("for_cond_merge");
        let end_label = self.label("for_end");
        let step_positive = self.tmp();
        let current_val = self.tmp();
        let pos_cmp = self.tmp();
        let neg_cmp = self.tmp();
        let cmp_ptr = self.tmp();
        self.entry_allocas.push(format!("  {cmp_ptr} = alloca i1"));

        self.body.push(format!("  br label %{cond_label}"));
        self.body.push(format!("{cond_label}:"));
        self.body
            .push(format!("  {step_positive} = icmp sgt i64 {}, 0", step.repr));
        self.body
            .push(format!("  {current_val} = load i64, ptr {}", iter_ptr));
        self.body.push(format!(
            "  br i1 {}, label %{positive_label}, label %{negative_label}",
            step_positive
        ));
        self.body.push(format!("{positive_label}:"));
        self.body.push(format!(
            "  {pos_cmp} = icmp slt i64 {}, {}",
            current_val, end.repr
        ));
        self.body
            .push(format!("  store i1 {}, ptr {}", pos_cmp, cmp_ptr));
        self.body.push(format!("  br label %{cond_merge_label}"));
        self.body.push(format!("{negative_label}:"));
        self.body.push(format!(
            "  {neg_cmp} = icmp sgt i64 {}, {}",
            current_val, end.repr
        ));
        self.body
            .push(format!("  store i1 {}, ptr {}", neg_cmp, cmp_ptr));
        self.body.push(format!("  br label %{cond_merge_label}"));
        self.body.push(format!("{cond_merge_label}:"));
        let cmp_tmp = self.tmp();
        self.body
            .push(format!("  {cmp_tmp} = load i1, ptr {}", cmp_ptr));
        self.body.push(format!(
            "  br i1 {}, label %{body_label}, label %{end_label}",
            cmp_tmp
        ));

        self.body.push(format!("{body_label}:"));
        self.loop_stack.push(LoopLabels {
            break_label: end_label.clone(),
            continue_label: continue_label.clone(),
        });
        for stmt in body {
            self.compile_statement(stmt)?;
        }
        self.loop_stack.pop();
        self.body.push(format!("  br label %{continue_label}"));

        self.body.push(format!("{continue_label}:"));
        let iter_value = self.tmp();
        let next_value = self.tmp();
        self.body
            .push(format!("  {iter_value} = load i64, ptr {}", iter_ptr));
        self.body.push(format!(
            "  {next_value} = add i64 {}, {}",
            iter_value, step.repr
        ));
        self.body
            .push(format!("  store i64 {}, ptr {}", next_value, iter_ptr));
        if let Some(index_ptr) = &index_ptr {
            let index_value = self.tmp();
            let next_index = self.tmp();
            self.body
                .push(format!("  {index_value} = load i64, ptr {index_ptr}"));
            self.body
                .push(format!("  {next_index} = add i64 {index_value}, 1"));
            self.body
                .push(format!("  store i64 {next_index}, ptr {index_ptr}"));
        }
        self.body.push(format!("  br label %{cond_label}"));
        self.body.push(format!("{end_label}:"));

        if let Some(saved) = saved {
            self.vars.insert(variable.to_string(), saved);
        } else {
            self.vars.remove(variable);
        }
        if let Some(index_name) = index {
            if let Some(saved_index) = saved_index {
                self.vars.insert(index_name.to_string(), saved_index);
            } else {
                self.vars.remove(index_name);
            }
        }

        Ok(())
    }
    pub(super) fn compile_for_collection(
        &mut self,
        variable: &str,
        index: Option<&str>,
        iterable: &Expression,
        body: &[Statement],
    ) -> Result<()> {
        let iterable_type = self.expression_data_type(iterable);
        let element_type = match &iterable_type {
            DataType::List | DataType::Tuple => DataType::I64,
            DataType::Vector { element_type, .. } | DataType::Slice { element_type } => {
                *element_type.clone()
            }
            _ => {
                return Err(MireError::new(ErrorKind::Runtime {
                    message: format!(
                        "Avenys for-loop supports range(...) and list/vector/slice, found {:?}",
                        iterable
                    ),
                }));
            }
        };

        let iterable_val = self.compile_expr(iterable)?;
        let len_val = self.compile_list_len_value(iterable_val.clone())?;
        let idx_ptr = self.tmp();
        self.entry_allocas.push(format!("  {idx_ptr} = alloca i64"));
        self.body.push(format!("  store i64 0, ptr {idx_ptr}"));

        let var_ll_ty = self.map_type(&element_type)?;
        let var_ptr = self.tmp();
        self.entry_allocas
            .push(format!("  {var_ptr} = alloca {}", self.ty(var_ll_ty.clone())));
        let saved = self.vars.insert(
            variable.to_string(),
            VarInfo {
                ptr: var_ptr.clone(),
                ty: var_ll_ty.clone(),
                data_type: element_type.clone(),
                owns_heap_string: false,
                struct_name: None,
            },
        );

        let saved_index = if let Some(index_name) = index {
            self.vars.insert(
                index_name.to_string(),
                VarInfo {
                    ptr: idx_ptr.clone(),
                    ty: LlType::I64,
                    data_type: DataType::I64,
                    owns_heap_string: false,
                    struct_name: None,
                },
            )
        } else {
            None
        };

        let cond_label = self.label("for_coll_cond");
        let body_label = self.label("for_coll_body");
        let continue_label = self.label("for_coll_continue");
        let end_label = self.label("for_coll_end");
        self.body.push(format!("  br label %{cond_label}"));
        self.body.push(format!("{cond_label}:"));
        let idx_val = self.tmp();
        let cond = self.tmp();
        self.body.push(format!("  {idx_val} = load i64, ptr {idx_ptr}"));
        self.body.push(format!(
            "  {cond} = icmp slt i64 {idx_val}, {}",
            len_val.repr
        ));
        self.body.push(format!(
            "  br i1 {cond}, label %{body_label}, label %{end_label}"
        ));

        self.body.push(format!("{body_label}:"));
        let elem_val = self.compile_index(
            iterable_val.clone(),
            LlValue {
                ty: LlType::I64,
                repr: idx_val.clone(),
                owned: false,
            },
            &iterable_type,
            &element_type,
        )?;
        self.store_casted(&var_ptr, var_ll_ty.clone(), elem_val)?;
        self.loop_stack.push(LoopLabels {
            break_label: end_label.clone(),
            continue_label: continue_label.clone(),
        });
        for stmt in body {
            self.compile_statement(stmt)?;
        }
        self.loop_stack.pop();
        self.body.push(format!("  br label %{continue_label}"));

        self.body.push(format!("{continue_label}:"));
        let next_idx = self.tmp();
        self.body
            .push(format!("  {next_idx} = add i64 {idx_val}, 1"));
        self.body.push(format!("  store i64 {next_idx}, ptr {idx_ptr}"));
        self.body.push(format!("  br label %{cond_label}"));
        self.body.push(format!("{end_label}:"));

        if let Some(saved) = saved {
            self.vars.insert(variable.to_string(), saved);
        } else {
            self.vars.remove(variable);
        }
        if let Some(index_name) = index {
            if let Some(saved_index) = saved_index {
                self.vars.insert(index_name.to_string(), saved_index);
            } else {
                self.vars.remove(index_name);
            }
        }
        Ok(())
    }

}
