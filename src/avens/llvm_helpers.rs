use super::*;

impl LlvmIrGen {
    pub(super) fn struct_name_from_expr(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Call {
                name, data_type, ..
            } if data_type.is_struct_like() => {
                data_type.struct_name().map(ToOwned::to_owned).or_else(|| {
                    if self
                        .user_structs
                        .contains_key(&normalize_nominal_name(name))
                    {
                        Some(normalize_nominal_name(name))
                    } else if let Some((owner, _method)) = name.split_once('.') {
                        self.vars
                            .get(owner)
                            .and_then(|info| info.struct_name.clone())
                            .or_else(|| {
                                self.user_structs
                                    .contains_key(&normalize_nominal_name(owner))
                                    .then(|| normalize_nominal_name(owner))
                            })
                    } else {
                        None
                    }
                })
            }
            Expression::Identifier(Identifier { name, .. }) => self
                .vars
                .get(name)
                .and_then(|info| info.struct_name.clone()),
            _ => None,
        }
    }

    pub(super) fn expression_data_type(&self, expr: &Expression) -> DataType {
        match expr {
            Expression::Literal(Literal::Str(_)) => DataType::Str,
            Expression::Literal(Literal::Char(_)) => DataType::Char,
            Expression::Literal(Literal::Bool(_)) => DataType::Bool,
            Expression::Literal(Literal::Int(_)) => DataType::I64,
            Expression::Literal(Literal::Float(_)) => DataType::F64,
            Expression::Literal(Literal::List(_)) => DataType::Vector {
                element_type: Box::new(DataType::Unknown),
                dynamic: false,
            },
            Expression::Literal(Literal::Dict(_)) => DataType::Map {
                key_type: Box::new(DataType::Unknown),
                value_type: Box::new(DataType::Unknown),
            },
            Expression::Literal(_) => DataType::Unknown,
            Expression::Identifier(identifier) => {
                if identifier.data_type != DataType::Unknown {
                    identifier.data_type.clone()
                } else {
                    self.vars
                        .get(&identifier.name)
                        .map(|var| var.data_type.clone())
                        .unwrap_or(DataType::Unknown)
                }
            }
            Expression::BinaryOp { data_type, .. }
            | Expression::UnaryOp { data_type, .. }
            | Expression::NamedArg { data_type, .. }
            | Expression::Call { data_type, .. }
            | Expression::List { data_type, .. }
            | Expression::Dict { data_type, .. }
            | Expression::Tuple { data_type, .. }
            | Expression::Reference { data_type, .. }
            | Expression::Dereference { data_type, .. }
            | Expression::Box { data_type, .. }
            | Expression::Pipeline { data_type, .. }
            | Expression::Match { data_type, .. }
            | Expression::Try { data_type, .. }
            | Expression::Ok { data_type, .. }
            | Expression::Err { data_type, .. }
            | Expression::EnumVariantPath { data_type, .. }
            | Expression::EnumVariant { data_type, .. } => data_type.clone(),
            Expression::MemberAccess {
                target,
                member,
                data_type,
            } => {
                if *data_type != DataType::Unknown {
                    return data_type.clone();
                }
                let target_type = self.expression_data_type(target);
                let Some(struct_name) = target_type
                    .struct_name()
                    .map(ToOwned::to_owned)
                    .or_else(|| self.struct_name_from_expr(target))
                else {
                    return DataType::Unknown;
                };
                self.user_structs
                    .get(&struct_name)
                    .and_then(|info| {
                        info.field_indices
                            .get(member)
                            .and_then(|index| info.field_data_types.get(*index))
                            .cloned()
                    })
                    .unwrap_or(DataType::Unknown)
            }
            Expression::Index {
                target, data_type, ..
            } => {
                if *data_type != DataType::Unknown {
                    return data_type.clone();
                }
                match self.expression_data_type(target) {
                    DataType::Array { element_type, .. }
                    | DataType::Slice { element_type }
                    | DataType::Vector { element_type, .. } => *element_type,
                    DataType::Map { value_type, .. } => *value_type,
                    _ => DataType::Unknown,
                }
            }
            Expression::Closure { return_type, .. } => return_type.clone(),
        }
    }

    pub(super) fn map_type(&self, data_type: &DataType) -> Result<LlType> {
        match data_type {
            DataType::I64 | DataType::Unknown | DataType::Anything | DataType::Generic(_) => {
                Ok(LlType::I64)
            }
            DataType::I32 => Ok(LlType::I64),
            DataType::I8 | DataType::I16 => Ok(LlType::I64),
            DataType::U8 | DataType::U16 | DataType::U32 | DataType::U64 | DataType::Char => {
                Ok(LlType::I64)
            }
            DataType::F32 | DataType::F64 => Ok(LlType::F64),
            DataType::Bool => Ok(LlType::I1),
            DataType::Str => Ok(LlType::Ptr),
            DataType::List
            | DataType::Vector { .. }
            | DataType::Dict
            | DataType::Map { .. }
            | DataType::Set
            | DataType::Tuple
            | DataType::Array { .. }
            | DataType::Slice { .. }
            | DataType::Struct
            | DataType::StructNamed(_)
            | DataType::Enum
            | DataType::EnumNamed(_)
            | DataType::Ref { .. }
            | DataType::RefMut { .. } => Ok(LlType::Ptr),
            DataType::Pointer(_)
            | DataType::Function
            | DataType::Db
            | DataType::Datetime
            | DataType::Box
            | DataType::DynTrait { .. } => Ok(LlType::Ptr),
            DataType::Result { .. } => Ok(LlType::Struct(vec![LlType::I8, LlType::Ptr])),
            DataType::None => Ok(LlType::I64),
        }
    }

    pub(super) fn element_size(&self, data_type: &DataType) -> i64 {
        match data_type {
            DataType::Bool | DataType::I8 | DataType::U8 => 1,
            DataType::I16 | DataType::U16 => 2,
            DataType::I32 | DataType::U32 => 4,
            DataType::Str
            | DataType::List
            | DataType::Vector { .. }
            | DataType::Dict
            | DataType::Map { .. }
            | DataType::Set
            | DataType::Tuple
            | DataType::Array { .. }
            | DataType::Slice { .. }
            | DataType::F32
            | DataType::F64 => 8,
            DataType::Result { .. } => 16,
            _ => 8,
        }
    }

    pub(super) fn scalar_storage_ir_type(&self, data_type: &DataType) -> &'static str {
        match data_type {
            DataType::Bool | DataType::I8 | DataType::U8 => "i8",
            DataType::I16 | DataType::U16 => "i16",
            DataType::I32 | DataType::U32 => "i32",
            _ => "i64",
        }
    }

    pub(super) fn default_value(&mut self, ty: LlType) -> LlValue {
        match ty {
            LlType::I8 => LlValue {
                ty,
                repr: "0".to_string(),
                owned: false,
            },
            LlType::I64 => LlValue {
                ty,
                repr: "0".to_string(),
                owned: false,
            },
            LlType::I1 => LlValue {
                ty,
                repr: "0".to_string(),
                owned: false,
            },
            LlType::F64 => LlValue {
                ty,
                repr: "0.0".to_string(),
                owned: false,
            },
            LlType::Ptr => self.string_value(""),
            LlType::Struct(fields) => LlValue {
                ty: LlType::Struct(fields),
                repr: "zeroinitializer".to_string(),
                owned: false,
            },
        }
    }

    pub(super) fn string_value(&mut self, value: &str) -> LlValue {
        let label = format!("@.str{}", self.strings.len());
        let escaped = escape_llvm_string(value);
        let len = string_byte_len(value) + 1;
        self.strings.push(format!(
            "{label} = private unnamed_addr constant [{len} x i8] c\"{escaped}\\00\""
        ));
        let tmp = self.tmp();
        self.body.push(format!(
            "  {tmp} = getelementptr inbounds [{len} x i8], ptr {label}, i64 0, i64 0"
        ));
        LlValue {
            ty: LlType::Ptr,
            repr: tmp,
            owned: false,
        }
    }

    pub(super) fn ty(&self, ty: LlType) -> String {
        match ty {
            LlType::I8 => "i8".to_string(),
            LlType::I64 => "i64".to_string(),
            LlType::I1 => "i1".to_string(),
            LlType::F64 => "double".to_string(),
            LlType::Ptr => "ptr".to_string(),
            LlType::Struct(fields) => self.render_struct_ty(&fields),
        }
    }

    pub(super) fn render_struct_ty(&self, fields: &[LlType]) -> String {
        let rendered = fields
            .iter()
            .map(|field| self.ty(field.clone()))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{{ {} }}", rendered)
    }

    pub(super) fn tmp(&mut self) -> String {
        let out = format!("%t{}", self.next_tmp);
        self.next_tmp += 1;
        out
    }

    pub(super) fn label(&mut self, prefix: &str) -> String {
        let out = format!("{prefix}_{}", self.next_label);
        self.next_label += 1;
        out
    }

    pub(super) fn null_value(&self) -> LlValue {
        LlValue {
            ty: LlType::Ptr,
            repr: "null".to_string(),
            owned: false,
        }
    }

    /// Cast value to Ptr if it is I64 (from DataType::Unknown), otherwise pass through.
    pub(super) fn ensure_ptr(&mut self, value: LlValue) -> LlValue {
        if value.ty == LlType::I64 {
            let result = self.tmp();
            self.body
                .push(format!("  {result} = inttoptr i64 {} to ptr", value.repr));
            LlValue {
                ty: LlType::Ptr,
                repr: result,
                owned: value.owned,
            }
        } else {
            value
        }
    }
}
