use super::*;

impl LlvmIrGen {
    pub(super) fn compile_dict_get(&mut self, args: &[Expression]) -> Result<LlValue> {
        if args.len() != 2 && args.len() != 3 {
            return Err(MireError::new(ErrorKind::Runtime {
                message: "Avenys dict.get(...) expects 2 or 3 arguments".to_string(),
            }));
        }

        let (dict_key_type, dict_value_type) = match self.expression_data_type(&args[0]) {
            DataType::Map {
                key_type,
                value_type,
            } => (*key_type, *value_type),
            _ => (DataType::Unknown, DataType::I64),
        };
        let dict_val = self.compile_expr(&args[0])?;
        let dict = self.ensure_ptr(dict_val);
        let key = self.compile_expr(&args[1])?;
        let key_kind = self.runtime_kind_code(&dict_key_type);
        let key_i64 = if key.ty == LlType::Ptr {
            LlValue {
                ty: LlType::I64,
                repr: "0".to_string(),
                owned: false,
            }
        } else {
            self.cast_to_i64(key.clone())?
        };
        let key_ptr = if key.ty == LlType::Ptr {
            key
        } else {
            LlValue {
                ty: LlType::Ptr,
                repr: "null".to_string(),
                owned: false,
            }
        };

        if matches!(
            dict_value_type,
            DataType::Map { .. }
                | DataType::Vector { .. }
                | DataType::Array { .. }
                | DataType::Slice { .. }
                | DataType::Anything
                | DataType::Str
        ) {
            let default_value = if args.len() == 3 {
                let value = self.compile_expr(&args[2])?;
                self.cast_to_type(value, LlType::Ptr)?
            } else {
                LlValue {
                    ty: LlType::Ptr,
                    repr: "null".to_string(),
                    owned: false,
                }
            };
            let result = self.tmp();
            self.body.push(format!(
                "  {result} = call ptr @rt_dict_get_ptr(ptr {}, i64 {}, i64 {}, ptr {}, ptr {})",
                dict.repr, key_kind, key_i64.repr, key_ptr.repr, default_value.repr
            ));
            return Ok(LlValue {
                ty: LlType::Ptr,
                repr: result,
                owned: false,
            });
        }

        let default_value = if args.len() == 3 {
            let value = self.compile_expr(&args[2])?;
            self.cast_to_i64(value)?
        } else {
            LlValue {
                ty: LlType::I64,
                repr: "0".to_string(),
                owned: false,
            }
        };
        let result = self.tmp();
        self.body.push(format!(
            "  {result} = call i64 @rt_dict_get_i64(ptr {}, i64 {}, i64 {}, ptr {}, i64 {})",
            dict.repr, key_kind, key_i64.repr, key_ptr.repr, default_value.repr
        ));
        Ok(LlValue {
            ty: LlType::I64,
            repr: result,
            owned: false,
        })
    }

    pub(super) fn compile_dict_set(&mut self, args: &[Expression]) -> Result<LlValue> {
        if args.len() != 3 {
            return Err(MireError::new(ErrorKind::Runtime {
                message: "Avenys dict.set(...) expects 3 arguments".to_string(),
            }));
        }
        let dict_type = self.expression_data_type(&args[0]);
        let (key_data_type, value_data_type) = match dict_type {
            DataType::Map {
                key_type,
                value_type,
            } => (*key_type, *value_type),
            _ => (
                self.expression_data_type(&args[1]),
                self.expression_data_type(&args[2]),
            ),
        };
        let dict_val = self.compile_expr(&args[0])?;
        let dict = self.ensure_ptr(dict_val);
        let key = self.compile_expr(&args[1])?;
        let value_expr = self.compile_expr(&args[2])?;
        let key_kind = self.runtime_kind_code(&key_data_type);
        let value_kind = self.runtime_kind_code(&value_data_type);
        let key_i64 = if key.ty == LlType::Ptr {
            LlValue {
                ty: LlType::I64,
                repr: "0".to_string(),
                owned: false,
            }
        } else {
            self.cast_to_i64(key.clone())?
        };
        let key_ptr = if key.ty == LlType::Ptr {
            key
        } else {
            LlValue {
                ty: LlType::Ptr,
                repr: "null".to_string(),
                owned: false,
            }
        };
        let result = self.tmp();

        if value_expr.ty == LlType::Ptr {
            let value = self.cast_to_type(value_expr, LlType::Ptr)?;
            self.body.push(format!(
                "  {result} = call ptr @rt_dict_set_ptr(ptr {}, i64 {}, i64 {}, i64 {}, ptr {}, ptr {})",
                dict.repr, key_kind, value_kind, key_i64.repr, key_ptr.repr, value.repr
            ));
        } else {
            let value = self.cast_to_i64(value_expr)?;
            self.body.push(format!(
                "  {result} = call ptr @rt_dict_set_i64(ptr {}, i64 {}, i64 {}, i64 {}, ptr {}, i64 {})",
                dict.repr, key_kind, value_kind, key_i64.repr, key_ptr.repr, value.repr
            ));
        }
        Ok(LlValue {
            ty: LlType::Ptr,
            repr: result,
            owned: true,
        })
    }

    pub(super) fn compile_dict_keys(&mut self, args: &[Expression]) -> Result<LlValue> {
        if args.len() != 1 {
            return Err(MireError::new(ErrorKind::Runtime {
                message: "dicts.keys(...) expects 1 argument".to_string(),
            }));
        }
        let dict_val = self.compile_expr(&args[0])?;
        let dict = self.ensure_ptr(dict_val);
        let result = self.tmp();
        self.body.push(format!(
            "  {result} = call ptr @rt_dict_keys(ptr {})",
            dict.repr
        ));
        Ok(LlValue {
            ty: LlType::Ptr,
            repr: result,
            owned: true,
        })
    }

    pub(super) fn compile_dict_values(&mut self, args: &[Expression]) -> Result<LlValue> {
        if args.len() != 1 {
            return Err(MireError::new(ErrorKind::Runtime {
                message: "dicts.values(...) expects 1 argument".to_string(),
            }));
        }
        let dict_val = self.compile_expr(&args[0])?;
        let dict = self.ensure_ptr(dict_val);
        let result = self.tmp();
        self.body.push(format!(
            "  {result} = call ptr @rt_dict_values(ptr {})",
            dict.repr
        ));
        Ok(LlValue {
            ty: LlType::Ptr,
            repr: result,
            owned: true,
        })
    }

    pub(super) fn compile_dict_literal(
        &mut self,
        entries: &[(Expression, Expression)],
    ) -> Result<LlValue> {
        let mut current = LlValue {
            ty: LlType::Ptr,
            repr: "null".to_string(),
            owned: false,
        };

        for (key_expr, value_expr) in entries {
            let key_data_type = self.expression_data_type(key_expr);
            let value_data_type = self.expression_data_type(value_expr);
            let key = self.compile_expr(key_expr)?;
            let value = self.compile_expr(value_expr)?;
            let key_kind = self.runtime_kind_code(&key_data_type);
            let value_kind = self.runtime_kind_code(&value_data_type);
            let key_i64 = if key.ty == LlType::Ptr {
                LlValue {
                    ty: LlType::I64,
                    repr: "0".to_string(),
                    owned: false,
                }
            } else {
                self.cast_to_i64(key.clone())?
            };
            let key_ptr = if key.ty == LlType::Ptr {
                key
            } else {
                LlValue {
                    ty: LlType::Ptr,
                    repr: "null".to_string(),
                    owned: false,
                }
            };
            let result = self.tmp();

            if value.ty == LlType::Ptr {
                let casted = self.cast_to_type(value, LlType::Ptr)?;
                self.body.push(format!(
                    "  {result} = call ptr @rt_dict_set_ptr(ptr {}, i64 {}, i64 {}, i64 {}, ptr {}, ptr {})",
                    current.repr, key_kind, value_kind, key_i64.repr, key_ptr.repr, casted.repr
                ));
            } else {
                let casted = self.cast_to_i64(value)?;
                self.body.push(format!(
                    "  {result} = call ptr @rt_dict_set_i64(ptr {}, i64 {}, i64 {}, i64 {}, ptr {}, i64 {})",
                    current.repr, key_kind, value_kind, key_i64.repr, key_ptr.repr, casted.repr
                ));
            }

            current = LlValue {
                ty: LlType::Ptr,
                repr: result,
                owned: true,
            };
        }

        Ok(current)
    }
}
