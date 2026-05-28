use super::*;

pub(super) fn hash_mire_value(value: &MireValue, hasher: &mut FxHasher) {
    match value {
        MireValue::I8(v) => {
            hasher.write_u8(0);
            v.hash(hasher);
        }
        MireValue::I16(v) => {
            hasher.write_u8(1);
            v.hash(hasher);
        }
        MireValue::I32(v) => {
            hasher.write_u8(2);
            v.hash(hasher);
        }
        MireValue::I64(v) => {
            hasher.write_u8(3);
            v.hash(hasher);
        }
        MireValue::U8(v) => {
            hasher.write_u8(4);
            v.hash(hasher);
        }
        MireValue::U16(v) => {
            hasher.write_u8(5);
            v.hash(hasher);
        }
        MireValue::U32(v) => {
            hasher.write_u8(6);
            v.hash(hasher);
        }
        MireValue::U64(v) => {
            hasher.write_u8(7);
            v.hash(hasher);
        }
        MireValue::Float(v) => {
            hasher.write_u8(8);
            v.to_bits().hash(hasher);
        }
        MireValue::F32(v) => {
            hasher.write_u8(9);
            v.to_bits().hash(hasher);
        }
        MireValue::F64(v) => {
            hasher.write_u8(10);
            v.to_bits().hash(hasher);
        }
        MireValue::Str(v) => {
            hasher.write_u8(11);
            v.hash(hasher);
        }
        MireValue::Bool(v) => {
            hasher.write_u8(12);
            v.hash(hasher);
        }
        MireValue::None => hasher.write_u8(13),
        MireValue::List(values) => {
            hasher.write_u8(14);
            values.len().hash(hasher);
            for value in values {
                hash_mire_value(value, hasher);
            }
        }
        MireValue::Dict(values) => {
            hasher.write_u8(15);
            values.len().hash(hasher);
            for ((k, v), dt) in values {
                hash_mire_value(k, hasher);
                hash_mire_value(v, hasher);
                hash_data_type(dt, hasher);
            }
        }
        MireValue::Tuple(values) => {
            hasher.write_u8(16);
            values.len().hash(hasher);
            for value in values {
                hash_mire_value(value, hasher);
            }
        }
        MireValue::Function(function) => {
            hasher.write_u8(17);
            hash_function_def(function, hasher);
        }
        MireValue::Builtinfn(name) => {
            hasher.write_u8(18);
            name.hash(hasher);
        }
        MireValue::Ref { value, is_mutable } => {
            hasher.write_u8(22);
            hash_mire_value(value, hasher);
            is_mutable.hash(hasher);
        }
        MireValue::Box { value } => {
            hasher.write_u8(23);
            hash_mire_value(value, hasher);
        }
        MireValue::Array { elements, size } => {
            hasher.write_u8(24);
            size.hash(hasher);
            elements.len().hash(hasher);
            for value in elements {
                hash_mire_value(value, hasher);
            }
        }
        MireValue::Slice { elements } => {
            hasher.write_u8(25);
            elements.len().hash(hasher);
            for value in elements {
                hash_mire_value(value, hasher);
            }
        }
        MireValue::EnumVariant {
            enum_name,
            variant_name,
            data,
        } => {
            hasher.write_u8(26);
            enum_name.hash(hasher);
            variant_name.hash(hasher);
            data.len().hash(hasher);
            for value in data {
                hash_mire_value(value, hasher);
            }
        }
    }
}
