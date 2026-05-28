use super::*;

impl TypeChecker {
    pub(super) fn resolve_binary_type(
        &self,
        operator: &str,
        left: &DataType,
        right: &DataType,
    ) -> Result<DataType> {
        match operator {
            "+" | "-" | "*" | "/" | "%" => {
                if operator == "+" && left == &DataType::Str && right == &DataType::Str {
                    return Ok(DataType::Str);
                }

                if operator == "+" {
                    match (left, right) {
                        (
                            DataType::Vector {
                                element_type: l_elem,
                                dynamic: l_dyn,
                            },
                            DataType::Vector {
                                element_type: r_elem,
                                dynamic: r_dyn,
                            },
                        ) => {
                            let unified_elem = Self::unify_types(l_elem, r_elem)?;
                            return Ok(DataType::Vector {
                                element_type: Box::new(unified_elem),
                                dynamic: *l_dyn || *r_dyn,
                            });
                        }
                        (DataType::Vector { .. }, DataType::List)
                        | (DataType::List, DataType::Vector { .. })
                        | (DataType::List, DataType::List) => {
                            return Ok(DataType::Vector {
                                element_type: Box::new(DataType::Unknown),
                                dynamic: true,
                            });
                        }
                        _ => {}
                    }
                }

                if Self::is_numeric(left) && Self::is_numeric(right) {
                    return Ok(Self::promote_numeric(left, right));
                }

                Err(type_error(format!(
                    "Operator '{}' not supported for {:?} and {:?}",
                    operator, left, right
                )))
            }
            "==" | "!=" | "<" | "<=" | ">" | ">=" => Ok(DataType::Bool),
            "&&" | "||" => {
                if left == &DataType::Unknown || right == &DataType::Unknown {
                    return Ok(DataType::Bool);
                }
                if Self::is_bool_like(left) && Self::is_bool_like(right) {
                    Ok(DataType::Bool)
                } else {
                    Err(type_error(format!(
                        "Logical operator '{}' requires bool operands, got {:?} and {:?}",
                        operator, left, right
                    )))
                }
            }
            "^" => {
                if left == &DataType::Unknown || right == &DataType::Unknown {
                    return Ok(DataType::Unknown);
                }
                if Self::is_bool_like(left) && Self::is_bool_like(right) {
                    Ok(DataType::Bool)
                } else if Self::is_integer_type(left) && Self::is_integer_type(right) {
                    Ok(left.clone())
                } else {
                    Err(type_error(format!(
                        "XOR operator '^' requires either bool or integer operands, got {:?} and {:?}",
                        left, right
                    )))
                }
            }
            "&" | "|" | "<<" | ">>" => {
                if left == &DataType::Unknown || right == &DataType::Unknown {
                    return Ok(DataType::Unknown);
                }
                if Self::is_integer_type(left) && Self::is_integer_type(right) {
                    Ok(left.clone())
                } else {
                    Err(type_error(format!(
                        "Bitwise operator '{}' requires integer operands, got {:?} and {:?}",
                        operator, left, right
                    )))
                }
            }
            _ => Ok(DataType::Unknown),
        }
    }

    pub(super) fn is_integer_type(ty: &DataType) -> bool {
        matches!(
            ty,
            DataType::I64
                | DataType::I32
                | DataType::I16
                | DataType::I8
                | DataType::U64
                | DataType::U32
                | DataType::U16
                | DataType::U8
        )
    }


    pub(super) fn is_logical_operator(operator: &str) -> bool {
        matches!(operator, "&&" | "||")
    }

    pub(super) fn is_match_identifier_pattern(expression: &Expression) -> bool {
        matches!(expression, Expression::Identifier(_))
    }

    pub(super) fn literal_type(lit: &Literal) -> DataType {
        match lit {
            Literal::Int(_) => DataType::I64,
            Literal::Float(_) => DataType::F64,
            Literal::Char(_) => DataType::Char,
            Literal::Str(_) => DataType::Str,
            Literal::Bool(_) => DataType::Bool,
            Literal::None => DataType::None,
            Literal::List(_) => DataType::Vector {
                element_type: Box::new(DataType::Unknown),
                dynamic: false,
            },
            Literal::Dict(_) => DataType::Map {
                key_type: Box::new(DataType::Unknown),
                value_type: Box::new(DataType::Unknown),
            },
            Literal::Tuple(_) => DataType::Tuple,
        }
    }

    pub(super) fn validate_int_literal_range(data_type: &DataType, value: i64) -> Result<()> {
        match data_type {
            DataType::I8 if (!(-128..=127).contains(&value)) => {
                return Err(type_error(format!(
                    "Integer literal {} exceeds i8 range (-128 to 127)",
                    value
                )));
            }
            DataType::I16 if (!(-32768..=32767).contains(&value)) => {
                return Err(type_error(format!(
                    "Integer literal {} exceeds i16 range (-32768 to 32767)",
                    value
                )));
            }
            DataType::I32 if (!(-2147483648..=2147483647).contains(&value)) => {
                return Err(type_error(format!(
                    "Integer literal {} exceeds i32 range (-2147483648 to 2147483647)",
                    value
                )));
            }
            DataType::U8 if (!(0..=255).contains(&value)) => {
                return Err(type_error(format!(
                    "Integer literal {} exceeds u8 range (0 to 255)",
                    value
                )));
            }
            DataType::U16 if (!(0..=65535).contains(&value)) => {
                return Err(type_error(format!(
                    "Integer literal {} exceeds u16 range (0 to 65535)",
                    value
                )));
            }
            DataType::U32 if (!(0..=4294967295).contains(&value)) => {
                return Err(type_error(format!(
                    "Integer literal {} exceeds u32 range (0 to 4294967295)",
                    value
                )));
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn mire_value_type(value: &MireValue) -> DataType {
        match value {
            MireValue::I8(_) => DataType::I8,
            MireValue::I16(_) => DataType::I16,
            MireValue::I32(_) => DataType::I32,
            MireValue::I64(_) => DataType::I64,
            MireValue::U8(_) => DataType::U8,
            MireValue::U16(_) => DataType::U16,
            MireValue::U32(_) => DataType::U32,
            MireValue::U64(_) => DataType::U64,
            MireValue::Float(_) => DataType::F64,
            MireValue::F64(_) => DataType::F64,
            MireValue::F32(_) => DataType::F32,
            MireValue::Str(_) => DataType::Str,
            MireValue::Bool(_) => DataType::Bool,
            MireValue::None => DataType::None,
            MireValue::List(values) => {
                let element_type = values
                    .first()
                    .map(Self::mire_value_type)
                    .unwrap_or(DataType::Anything);
                DataType::Vector {
                    element_type: Box::new(element_type),
                    dynamic: false,
                }
            }
            MireValue::Dict(entries) => {
                let key_type = entries
                    .first()
                    .map(|((key, _), _)| Self::mire_value_type(key))
                    .unwrap_or(DataType::Anything);
                let value_type = entries
                    .first()
                    .map(|((_, value), _)| Self::mire_value_type(value))
                    .unwrap_or(DataType::Anything);
                DataType::Map {
                    key_type: Box::new(key_type),
                    value_type: Box::new(value_type),
                }
            }
            MireValue::Tuple(_) => DataType::Tuple,
            MireValue::Function(_) | MireValue::Builtinfn(_) => DataType::Function,
            MireValue::Ref { is_mutable, .. } => {
                if *is_mutable {
                    DataType::RefMut {
                        inner: Box::new(DataType::Anything),
                    }
                } else {
                    DataType::Ref {
                        inner: Box::new(DataType::Anything),
                    }
                }
            }
            MireValue::Box { .. } => DataType::Box,
            MireValue::Array { elements, size } => {
                let element_type = elements
                    .first()
                    .map(Self::mire_value_type)
                    .unwrap_or(DataType::Anything);
                DataType::Array {
                    element_type: Box::new(element_type),
                    size: *size,
                }
            }
            MireValue::Slice { elements } => {
                let element_type = elements
                    .first()
                    .map(Self::mire_value_type)
                    .unwrap_or(DataType::Anything);
                DataType::Slice {
                    element_type: Box::new(element_type),
                }
            }
            MireValue::EnumVariant { enum_name, .. } => DataType::EnumNamed(enum_name.clone()),
        }
    }

    pub(super) fn unify_types(left: &DataType, right: &DataType) -> Result<DataType> {
        if left == right {
            return Ok(left.clone());
        }

        if left.is_struct_like() && right.is_struct_like() {
            return match (left.struct_name(), right.struct_name()) {
                (Some(left_name), Some(right_name)) if left_name != right_name => {
                    Err(type_error(format!(
                        "Cannot unify incompatible struct types {:?} and {:?}",
                        left, right
                    )))
                }
                (Some(_), _) => Ok(left.clone()),
                (_, Some(_)) => Ok(right.clone()),
                _ => Ok(DataType::Struct),
            };
        }

        if left.is_enum_like() && right.is_enum_like() {
            return match (left.enum_name(), right.enum_name()) {
                (Some(left_name), Some(right_name)) if left_name != right_name => {
                    Err(type_error(format!(
                        "Cannot unify incompatible enum types {:?} and {:?}",
                        left, right
                    )))
                }
                (Some(_), _) => Ok(left.clone()),
                (_, Some(_)) => Ok(right.clone()),
                _ => Ok(DataType::Enum),
            };
        }

        if left == &DataType::Unknown || left == &DataType::None {
            return Ok(right.clone());
        }
        if right == &DataType::Unknown || right == &DataType::None {
            return Ok(left.clone());
        }

        if Self::is_numeric(left) && Self::is_numeric(right) {
            return Ok(Self::promote_numeric(left, right));
        }

        match (left, right) {
            (
                DataType::Vector {
                    element_type: left_elem,
                    dynamic: left_dynamic,
                },
                DataType::Vector {
                    element_type: right_elem,
                    dynamic: right_dynamic,
                },
            ) => {
                let element_type = Self::unify_types(left_elem, right_elem)?;
                return Ok(DataType::Vector {
                    element_type: Box::new(element_type),
                    dynamic: *left_dynamic || *right_dynamic,
                });
            }
            (
                DataType::Map {
                    key_type: left_key,
                    value_type: left_value,
                },
                DataType::Map {
                    key_type: right_key,
                    value_type: right_value,
                },
            ) => {
                let key_type = Self::unify_types(left_key, right_key)?;
                let value_type = Self::unify_types(left_value, right_value)?;
                return Ok(DataType::Map {
                    key_type: Box::new(key_type),
                    value_type: Box::new(value_type),
                });
            }
            _ => {}
        }

        match (left, right) {
            (
                DataType::Result {
                    ok: left_ok,
                    err: left_err,
                },
                DataType::Result {
                    ok: right_ok,
                    err: right_err,
                },
            ) => {
                let ok = Self::unify_types(left_ok, right_ok)?;
                let err = Self::unify_types(left_err, right_err)?;
                return Ok(DataType::Result {
                    ok: Box::new(ok),
                    err: Box::new(err),
                });
            }
            (DataType::Result { ok, .. }, _) if *right == DataType::Unknown => {
                return Ok(DataType::Result {
                    ok: ok.clone(),
                    err: Box::new(DataType::Str),
                });
            }
            (_, DataType::Result { ok, .. }) if *left == DataType::Unknown => {
                return Ok(DataType::Result {
                    ok: ok.clone(),
                    err: Box::new(DataType::Str),
                });
            }
            (
                DataType::Ref { inner: left_inner } | DataType::RefMut { inner: left_inner },
                DataType::Ref { inner: right_inner } | DataType::RefMut { inner: right_inner },
            ) => {
                let inner = Self::unify_types(left_inner, right_inner)?;
                let same_kind = std::mem::discriminant(left) == std::mem::discriminant(right);
                return Ok(if same_kind {
                    if matches!(left, DataType::Ref { .. }) {
                        DataType::Ref {
                            inner: Box::new(inner),
                        }
                    } else {
                        DataType::RefMut {
                            inner: Box::new(inner),
                        }
                    }
                } else {
                    DataType::Ref {
                        inner: Box::new(inner),
                    }
                });
            }
            (DataType::Ref { inner } | DataType::RefMut { inner }, other)
            | (other, DataType::Ref { inner } | DataType::RefMut { inner }) => {
                return Self::unify_types(inner, other);
            }
            _ => {}
        }

        Err(type_error(format!(
            "Cannot unify incompatible types {:?} and {:?}",
            left, right
        )))
    }

    pub(super) fn promote_numeric(left: &DataType, right: &DataType) -> DataType {
        if matches!(left, DataType::F64 | DataType::F32)
            || matches!(right, DataType::F64 | DataType::F32)
        {
            DataType::F64
        } else if left == &DataType::I64 || right == &DataType::I64 {
            DataType::I64
        } else {
            left.clone()
        }
    }

    pub(super) fn is_numeric(dtype: &DataType) -> bool {
        matches!(
            dtype,
            DataType::I8
                | DataType::I16
                | DataType::I32
                | DataType::I64
                | DataType::U8
                | DataType::U16
                | DataType::U32
                | DataType::U64
                | DataType::F32
                | DataType::F64
        )
    }

    pub(super) fn is_bool_like(dtype: &DataType) -> bool {
        matches!(
            dtype,
            DataType::Bool | DataType::Anything | DataType::Unknown
        )
    }

    pub(super) fn is_assignable(&self, expected: &DataType, actual: &DataType) -> bool {
        if matches!(expected, DataType::Generic(_)) || matches!(actual, DataType::Generic(_)) {
            return true;
        }
        if expected == actual {
            return true;
        }

        if expected.is_struct_like() && actual.is_struct_like() {
            return match (expected.struct_name(), actual.struct_name()) {
                (Some(expected_name), Some(actual_name)) => expected_name == actual_name,
                _ => true,
            };
        }

        if expected.is_enum_like() && actual.is_enum_like() {
            return match (expected.enum_name(), actual.enum_name()) {
                (Some(expected_name), Some(actual_name)) => expected_name == actual_name,
                _ => true,
            };
        }

        match (expected, actual) {
            (
                DataType::Ref {
                    inner: expected_inner,
                },
                DataType::Ref {
                    inner: actual_inner,
                }
                | DataType::RefMut {
                    inner: actual_inner,
                },
            ) => {
                return self.is_assignable(expected_inner, actual_inner);
            }
            (
                DataType::RefMut {
                    inner: expected_inner,
                },
                DataType::RefMut {
                    inner: actual_inner,
                },
            ) => {
                return self.is_assignable(expected_inner, actual_inner);
            }
            (DataType::RefMut { .. }, DataType::Ref { .. }) => return false,
            (DataType::Ref { inner, .. } | DataType::RefMut { inner, .. }, _) => {
                return self.is_assignable(inner, actual);
            }
            _ => {}
        }

        if expected == &DataType::None {
            return true;
        }
        if expected == &DataType::Anything || actual == &DataType::Unknown {
            return true;
        }

        if expected == &DataType::Unknown {
            return true;
        }

        if expected == &DataType::Dict && actual == &DataType::List {
            return true;
        }

        if matches!(expected, DataType::Map { .. }) && actual == &DataType::Dict {
            return true;
        }

        match (expected, actual) {
            (DataType::Result { ok: expected_ok, err: expected_err },
             DataType::Result { ok: actual_ok, err: actual_err }) => {
                return self.is_assignable(expected_ok, actual_ok)
                    && self.is_assignable(expected_err, actual_err);
            }
            (
                DataType::Array {
                    element_type: expected_elem,
                    ..
                }
                | DataType::Slice {
                    element_type: expected_elem,
                },
                DataType::Vector {
                    element_type: actual_elem,
                    ..
                },
            ) => {
                return self.is_assignable(expected_elem, actual_elem);
            }
            (DataType::Array { .. } | DataType::Slice { .. }, DataType::List) => return true,
            (
                DataType::Vector {
                    element_type: expected_elem,
                    ..
                },
                DataType::Vector {
                    element_type: actual_elem,
                    ..
                },
            ) => {
                return self.is_assignable(expected_elem, actual_elem);
            }
            (DataType::Vector { .. }, DataType::List) => return true,
            _ => {}
        }

        Self::is_numeric(expected) && Self::is_numeric(actual)
    }

}
