use super::*;

impl LlvmIrGen {

    pub(super) fn compile_list_len(&mut self, args: &[Expression]) -> Result<LlValue> {
        if args.len() != 1 {
            return Err(MireError::new(ErrorKind::Runtime {
                message: "Avenys lists.len expects 1 argument".to_string(),
            }));
        }
        let list = self.compile_expr(&args[0])?;
        self.compile_list_len_value(list)
    }

    pub(super) fn compile_list_len_value(&mut self, list: LlValue) -> Result<LlValue> {
        let list = self.ensure_ptr(list);
        let is_null = self.tmp();
        let loaded_len = self.tmp();
        let len = self.tmp();
        let null_label = self.label("list_len_null");
        let load_label = self.label("list_len_load");
        let end_label = self.label("list_len_end");
        let result_ptr = self.tmp();
        self.entry_allocas
            .push(format!("  {result_ptr} = alloca i64"));

        self.body
            .push(format!("  {is_null} = icmp eq ptr {}, null", list.repr));
        self.body.push(format!(
            "  br i1 {is_null}, label %{null_label}, label %{load_label}"
        ));

        self.body.push(format!("{null_label}:"));
        self.body.push(format!("  store i64 0, ptr {result_ptr}"));
        self.body.push(format!("  br label %{end_label}"));

        self.body.push(format!("{load_label}:"));
        self.body
            .push(format!("  {loaded_len} = load i64, ptr {}", list.repr));
        self.body
            .push(format!("  store i64 {loaded_len}, ptr {result_ptr}"));
        self.body.push(format!("  br label %{end_label}"));

        self.body.push(format!("{end_label}:"));
        self.body
            .push(format!("  {len} = load i64, ptr {result_ptr}"));
        Ok(LlValue {
            ty: LlType::I64,
            repr: len,
            owned: false,
        })
    }

    pub(super) fn compile_list_get(&mut self, args: &[Expression]) -> Result<LlValue> {
        if args.len() != 2 {
            return Err(MireError::new(ErrorKind::Runtime {
                message: "Avenys lists.get expects 2 arguments".to_string(),
            }));
        }
        let list = self.compile_expr(&args[0])?;
        let index = self.compile_expr(&args[1])?;
        let list_type = self.expression_data_type(&args[0]);
        let elem_type = match &list_type {
            DataType::Vector { element_type, .. } => *element_type.clone(),
            DataType::Array { element_type, .. } => *element_type.clone(),
            DataType::Slice { element_type } => *element_type.clone(),
            _ => DataType::I64,
        };
        self.compile_index(list, index, &list_type, &elem_type)
    }

    pub(super) fn compile_list_pop(&mut self, args: &[Expression]) -> Result<LlValue> {
        if args.len() != 1 {
            return Err(MireError::new(ErrorKind::Runtime {
                message: "Avenys list.pop(...) expects 1 argument".to_string(),
            }));
        }
        let list_val = self.compile_expr(&args[0])?;
        let list = self.ensure_ptr(list_val);
        let result = self.tmp();
        self.body.push(format!(
            "  {result} = call i64 @rt_list_pop_i64(ptr {})",
            list.repr
        ));
        Ok(LlValue {
            ty: LlType::I64,
            repr: result,
            owned: false,
        })
    }

    pub(super) fn compile_lists_push(&mut self, args: &[Expression]) -> Result<LlValue> {
        if args.len() != 2 {
            return Err(MireError::new(ErrorKind::Runtime {
                message: "Avenys lists.push expects 2 arguments".to_string(),
            }));
        }

        let list_val = self.compile_expr(&args[0])?;
        let list = self.ensure_ptr(list_val);
        let value = self.compile_expr(&args[1])?;
        let list_type = self.expression_data_type(&args[0]);
        let elem_type = match &list_type {
            DataType::Vector { element_type, .. } => *element_type.clone(),
            DataType::Array { element_type, .. } => *element_type.clone(),
            DataType::Slice { element_type } => *element_type.clone(),
            _ => DataType::I64,
        };
        let result = self.tmp();
        if value.ty == LlType::Ptr {
            self.body.push(format!(
                "  {result} = call ptr @rt_list_push_ptr(ptr {}, ptr {})",
                list.repr, value.repr
            ));
        } else {
            let value = self.cast_to_i64(value)?;
            let elem_size = self.element_size(&elem_type);
            if elem_size == 8 {
                self.body.push(format!(
                    "  {result} = call ptr @rt_list_push_i64(ptr {}, i64 {})",
                    list.repr, value.repr
                ));
            } else {
                self.body.push(format!(
                    "  {result} = call ptr @rt_list_push_scalar(ptr {}, i64 {}, i64 {})",
                    list.repr, value.repr, elem_size
                ));
            }
        }

        Ok(LlValue {
            ty: LlType::Ptr,
            repr: result,
            owned: false,
        })
    }

    pub(super) fn compile_lists_fold(&mut self, args: &[Expression]) -> Result<LlValue> {
        if args.len() != 3 {
            return Err(MireError::new(ErrorKind::Runtime {
                message: "Avenys lists.fold expects 3 arguments (initial, fn, list)".to_string(),
            }));
        }

        let initial = self.compile_expr(&args[0])?;
        let mut list = self.compile_expr(&args[2])?;
        let acc_type = self.expression_data_type(&args[0]);
        let list_type = self.expression_data_type(&args[2]);

        let elem_type = match &list_type {
            DataType::Vector { element_type, .. } => *element_type.clone(),
            DataType::Array { element_type, .. } => *element_type.clone(),
            DataType::Slice { element_type } => *element_type.clone(),
            other => {
                return Err(MireError::new(ErrorKind::Runtime {
                    message: format!(
                        "Avenys lists.fold expects vec/arr/slice input, got {:?}",
                        other
                    ),
                }));
            }
        };
        let Expression::Closure {
            params,
            body,
            return_type,
            ..
        } = &args[1]
        else {
            return Err(MireError::new(ErrorKind::Runtime {
                message: "Avenys lists.fold expects a closure as second argument".to_string(),
            }));
        };
        if params.len() != 2 {
            return Err(MireError::new(ErrorKind::Runtime {
                message: "Avenys lists.fold closure must have exactly 2 parameters".to_string(),
            }));
        }

        let acc_ty = self.map_type(&acc_type)?;
        let result_ptr = self.tmp();
        let index_ptr = self.tmp();
        self.entry_allocas.push(format!(
            "  {result_ptr} = alloca {}",
            self.ty(acc_ty.clone())
        ));
        self.entry_allocas
            .push(format!("  {index_ptr} = alloca i64"));
        self.store_casted(&result_ptr, acc_ty, initial)?;
        self.body.push(format!("  store i64 0, ptr {index_ptr}"));

        list = self.ensure_ptr(list);
        let is_null = self.tmp();
        let null_label = self.label("fold_null");
        let loop_cond_label = self.label("fold_cond");
        let loop_body_label = self.label("fold_body");
        let end_label = self.label("fold_end");
        self.body
            .push(format!("  {is_null} = icmp eq ptr {}, null", list.repr));
        self.body.push(format!(
            "  br i1 {is_null}, label %{null_label}, label %{loop_cond_label}"
        ));

        self.body.push(format!("{null_label}:"));
        self.body.push(format!("  br label %{end_label}"));

        let len = self.tmp();
        let index = self.tmp();
        let has_more = self.tmp();
        self.body.push(format!("{loop_cond_label}:"));
        self.body
            .push(format!("  {len} = load i64, ptr {}", list.repr));
        self.body
            .push(format!("  {index} = load i64, ptr {index_ptr}"));
        self.body
            .push(format!("  {has_more} = icmp slt i64 {index}, {len}"));
        self.body.push(format!(
            "  br i1 {has_more}, label %{loop_body_label}, label %{end_label}"
        ));

        self.body.push(format!("{loop_body_label}:"));
        let next_index = self.tmp();
        let elem = self.load_list_element_unchecked(&list.repr, &index, &elem_type)?;
        let current_acc = self.load_slot_value(&result_ptr, &acc_type)?;
        let next_acc =
            self.compile_bound_closure(params, &[current_acc, elem], body, return_type)?;
        self.store_casted(&result_ptr, self.map_type(&acc_type)?, next_acc)?;
        self.body
            .push(format!("  {next_index} = add i64 {index}, 1"));
        self.body
            .push(format!("  store i64 {next_index}, ptr {index_ptr}"));
        self.body.push(format!("  br label %{loop_cond_label}"));

        self.body.push(format!("{end_label}:"));
        self.load_slot_value(&result_ptr, &acc_type)
    }

    pub(super) fn compile_lists_map(&mut self, args: &[Expression]) -> Result<LlValue> {
        if args.len() != 2 {
            return Err(MireError::new(ErrorKind::Runtime {
                message: "Avenys lists.map expects 2 arguments (fn, list)".to_string(),
            }));
        }

        let mut list = self.compile_expr(&args[1])?;
        let list_type = self.expression_data_type(&args[1]);

        let elem_type = match &list_type {
            DataType::Vector { element_type, .. } => *element_type.clone(),
            DataType::Array { element_type, .. } => *element_type.clone(),
            DataType::Slice { element_type } => *element_type.clone(),
            other => {
                return Err(MireError::new(ErrorKind::Runtime {
                    message: format!(
                        "Avenys lists.map expects vec/arr/slice input, got {:?}",
                        other
                    ),
                }));
            }
        };
        let Expression::Closure {
            params,
            body,
            return_type,
            ..
        } = &args[0]
        else {
            return Err(MireError::new(ErrorKind::Runtime {
                message: "Avenys lists.map expects a closure as first argument".to_string(),
            }));
        };
        if params.len() != 1 {
            return Err(MireError::new(ErrorKind::Runtime {
                message: "Avenys lists.map closure must have exactly 1 parameter".to_string(),
            }));
        }
        let mapped_type = if *return_type == DataType::Unknown {
            params[0].1.clone()
        } else {
            return_type.clone()
        };

        let result_ptr = self.tmp();
        self.entry_allocas
            .push(format!("  {result_ptr} = alloca ptr"));
        let initial_result = self.tmp();
        self.body.push(format!(
            "  {initial_result} = call ptr @rt_list_create(i64 4, i64 {})",
            self.element_size(&mapped_type)
        ));
        self.body
            .push(format!("  store ptr {initial_result}, ptr {result_ptr}"));

        let len = self.tmp();
        let index_ptr = self.tmp();
        let is_null = self.tmp();
        self.entry_allocas
            .push(format!("  {index_ptr} = alloca i64"));
        self.body.push(format!("  store i64 0, ptr {index_ptr}"));

        list = self.ensure_ptr(list);
        self.body
            .push(format!("  {is_null} = icmp eq ptr {}, null", list.repr));
        let null_label = self.label("map_null");
        let loop_cond_label = self.label("map_cond");
        let loop_body_label = self.label("map_body");
        let end_label = self.label("map_end");
        self.body.push(format!(
            "  br i1 {is_null}, label %{null_label}, label %{loop_cond_label}"
        ));

        self.body.push(format!("{null_label}:"));
        self.body.push(format!("  br label %{end_label}"));

        let index = self.tmp();
        let has_more = self.tmp();
        self.body.push(format!("{loop_cond_label}:"));
        self.body
            .push(format!("  {len} = load i64, ptr {}", list.repr));
        self.body
            .push(format!("  {index} = load i64, ptr {index_ptr}"));
        self.body
            .push(format!("  {has_more} = icmp slt i64 {index}, {len}"));
        self.body.push(format!(
            "  br i1 {has_more}, label %{loop_body_label}, label %{end_label}"
        ));

        self.body.push(format!("{loop_body_label}:"));
        let next_index = self.tmp();
        let elem = self.load_list_element_unchecked(&list.repr, &index, &elem_type)?;
        let mapped = self.compile_bound_closure(params, &[elem], body, return_type)?;
        let current_result = self.tmp();
        self.body
            .push(format!("  {current_result} = load ptr, ptr {result_ptr}"));
        let updated = self.push_list_value(
            LlValue {
                ty: LlType::Ptr,
                repr: current_result,
                owned: true,
            },
            mapped,
            &mapped_type,
        )?;
        self.body
            .push(format!("  store ptr {}, ptr {result_ptr}", updated.repr));

        self.body
            .push(format!("  {next_index} = add i64 {index}, 1"));
        self.body
            .push(format!("  store i64 {next_index}, ptr {index_ptr}"));
        self.body.push(format!("  br label %{loop_cond_label}"));

        self.body.push(format!("{end_label}:"));
        let final_result = self.tmp();
        self.body
            .push(format!("  {final_result} = load ptr, ptr {result_ptr}"));

        Ok(LlValue {
            ty: LlType::Ptr,
            repr: final_result,
            owned: true,
        })
    }

    pub(super) fn compile_lists_filter(&mut self, args: &[Expression]) -> Result<LlValue> {
        if args.len() != 2 {
            return Err(MireError::new(ErrorKind::Runtime {
                message: "Avenys lists.filter expects 2 arguments (fn, list)".to_string(),
            }));
        }

        let mut list = self.compile_expr(&args[1])?;
        let list_type = self.expression_data_type(&args[1]);

        let elem_type = match &list_type {
            DataType::Vector { element_type, .. } => *element_type.clone(),
            DataType::Array { element_type, .. } => *element_type.clone(),
            DataType::Slice { element_type } => *element_type.clone(),
            other => {
                return Err(MireError::new(ErrorKind::Runtime {
                    message: format!(
                        "Avenys lists.filter expects vec/arr/slice input, got {:?}",
                        other
                    ),
                }));
            }
        };
        let Expression::Closure {
            params,
            body,
            return_type,
            ..
        } = &args[0]
        else {
            return Err(MireError::new(ErrorKind::Runtime {
                message: "Avenys lists.filter expects a closure as first argument".to_string(),
            }));
        };
        if params.len() != 1 {
            return Err(MireError::new(ErrorKind::Runtime {
                message: "Avenys lists.filter closure must have exactly 1 parameter".to_string(),
            }));
        }

        let result_ptr = self.tmp();
        self.entry_allocas
            .push(format!("  {result_ptr} = alloca ptr"));
        let initial_result = self.tmp();
        self.body.push(format!(
            "  {initial_result} = call ptr @rt_list_create(i64 4, i64 {})",
            self.element_size(&elem_type)
        ));
        self.body
            .push(format!("  store ptr {initial_result}, ptr {result_ptr}"));

        let len = self.tmp();
        let index_ptr = self.tmp();
        let is_null = self.tmp();
        self.entry_allocas
            .push(format!("  {index_ptr} = alloca i64"));
        self.body.push(format!("  store i64 0, ptr {index_ptr}"));

        list = self.ensure_ptr(list);
        self.body
            .push(format!("  {is_null} = icmp eq ptr {}, null", list.repr));
        let null_label = self.label("filter_null");
        let loop_cond_label = self.label("filter_cond");
        let loop_body_label = self.label("filter_body");
        let end_label = self.label("filter_end");
        self.body.push(format!(
            "  br i1 {is_null}, label %{null_label}, label %{loop_cond_label}"
        ));

        self.body.push(format!("{null_label}:"));
        self.body.push(format!("  br label %{end_label}"));

        let index = self.tmp();
        let has_more = self.tmp();
        self.body.push(format!("{loop_cond_label}:"));
        self.body
            .push(format!("  {len} = load i64, ptr {}", list.repr));
        self.body
            .push(format!("  {index} = load i64, ptr {index_ptr}"));
        self.body
            .push(format!("  {has_more} = icmp slt i64 {index}, {len}"));
        self.body.push(format!(
            "  br i1 {has_more}, label %{loop_body_label}, label %{end_label}"
        ));

        self.body.push(format!("{loop_body_label}:"));
        let next_index = self.tmp();
        let elem = self.load_list_element_unchecked(&list.repr, &index, &elem_type)?;
        let keep =
            self.compile_bound_closure(params, std::slice::from_ref(&elem), body, return_type)?;
        let keep = self.cast_to_i1(keep)?;

        let new_list = self.tmp();
        let cond_filter = self.label("filter_conditional");
        let after_filter = self.label("filter_after");
        self.body.push(format!(
            "  br i1 {}, label %{cond_filter}, label %{after_filter}",
            keep.repr
        ));

        self.body.push(format!("{cond_filter}:"));
        let current_result = self.tmp();
        self.body
            .push(format!("  {current_result} = load ptr, ptr {result_ptr}"));
        let updated = self.push_list_value(
            LlValue {
                ty: LlType::Ptr,
                repr: current_result,
                owned: true,
            },
            elem,
            &elem_type,
        )?;
        self.body
            .push(format!("  store ptr {}, ptr {result_ptr}", updated.repr));
        self.body.push(format!(
            "  {new_list} = ptrtoint ptr {} to i64",
            updated.repr
        ));
        self.body.push(format!("  br label %{after_filter}"));

        self.body.push(format!("{after_filter}:"));
        self.body
            .push(format!("  {next_index} = add i64 {index}, 1"));
        self.body
            .push(format!("  store i64 {next_index}, ptr {index_ptr}"));
        self.body.push(format!("  br label %{loop_cond_label}"));

        self.body.push(format!("{end_label}:"));
        let final_result = self.tmp();
        self.body
            .push(format!("  {final_result} = load ptr, ptr {result_ptr}"));

        Ok(LlValue {
            ty: LlType::Ptr,
            repr: final_result,
            owned: true,
        })
    }

    pub(super) fn compile_lists_slice(&mut self, args: &[Expression]) -> Result<LlValue> {
        if args.len() != 3 {
            return Err(MireError::new(ErrorKind::Runtime {
                message: "Avenys lists.slice expects 3 arguments".to_string(),
            }));
        }

        let list_val = self.compile_expr(&args[0])?;
        let list = self.ensure_ptr(list_val);
        let start = self.compile_expr(&args[1])?;
        let end = self.compile_expr(&args[2])?;

        let start_i64 = self.cast_to_i64(start)?;
        let end_i64 = self.cast_to_i64(end)?;

        let result = self.tmp();
        self.body.push(format!(
            "  {result} = call ptr @rt_list_slice(ptr {}, i64 {}, i64 {})",
            list.repr, start_i64.repr, end_i64.repr
        ));

        Ok(LlValue {
            ty: LlType::Ptr,
            repr: result,
            owned: true,
        })
    }

    pub(super) fn compile_list_literal(
        &mut self,
        elements: &[Expression],
        element_type: &DataType,
    ) -> Result<LlValue> {
        let size = elements.len() as i64;
        if size == 0 {
            let ptr = self.tmp();
            self.body.push(format!("  {ptr} = inttoptr i64 0 to ptr"));
            return Ok(LlValue {
                ty: LlType::Ptr,
                repr: ptr,
                owned: false,
            });
        }
        let malloc = self.tmp();
        let list_ptr = self.tmp();
        let elem_size = self.element_size(element_type);
        self.body.push(format!(
            "  {malloc} = call ptr @malloc(i64 {})",
            16 + size * elem_size
        ));
        self.body
            .push(format!("  store i64 {}, ptr {malloc}", size));
        self.body.push(format!(
            "  {list_ptr} = getelementptr i8, ptr {malloc}, i64 8"
        ));
        self.body
            .push(format!("  store i64 {}, ptr {list_ptr}", size));
        let elem_ll_ty = self.map_type(element_type).unwrap_or(LlType::I64);
        for (i, elem) in elements.iter().enumerate() {
            let val = self.compile_expr(elem)?;
            let elem_ptr = self.tmp();
            self.body.push(format!(
                "  {elem_ptr} = getelementptr i8, ptr {}, i64 {}",
                list_ptr,
                8 + i as i64 * elem_size
            ));
            if elem_ll_ty == LlType::Ptr {
                let stored = self.cast_to_type(val, LlType::Ptr)?;
                self.body
                    .push(format!("  store ptr {}, ptr {}", stored.repr, elem_ptr));
            } else {
                let (store_ty, store_repr) = self.cast_scalar_for_store(val, element_type)?;
                self.body.push(format!(
                    "  store {} {}, ptr {}",
                    store_ty, store_repr, elem_ptr
                ));
            }
        }
        Ok(LlValue {
            ty: LlType::Ptr,
            repr: list_ptr,
            owned: false,
        })
    }

}
