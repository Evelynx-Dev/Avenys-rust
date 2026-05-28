use super::*;

pub(super) fn hash_literal(lit: &Literal, hasher: &mut FxHasher) {
    match lit {
        Literal::Int(value) => {
            hasher.write_u8(0);
            value.hash(hasher);
        }
        Literal::Float(value) => {
            hasher.write_u8(1);
            value.to_bits().hash(hasher);
        }
        Literal::Char(value) => {
            hasher.write_u8(2);
            value.hash(hasher);
        }
        Literal::Str(value) => {
            hasher.write_u8(3);
            value.hash(hasher);
        }
        Literal::Bool(value) => {
            hasher.write_u8(4);
            value.hash(hasher);
        }
        Literal::None => hasher.write_u8(5),
        Literal::List(values) => {
            hasher.write_u8(6);
            hash_expressions(values, hasher);
        }
        Literal::Dict(values) => {
            hasher.write_u8(7);
            values.len().hash(hasher);
            for ((k, v), dt) in values {
                hash_expression(k, hasher);
                hash_expression(v, hasher);
                hash_data_type(dt, hasher);
            }
        }
        Literal::Tuple(values) => {
            hasher.write_u8(8);
            hash_expressions(values, hasher);
        }
    }
}

pub(super) fn hash_identifier(identifier: &Identifier, hasher: &mut FxHasher) {
    identifier.name.hash(hasher);
    hash_data_type(&identifier.data_type, hasher);
    identifier.line.hash(hasher);
    identifier.column.hash(hasher);
}

pub(super) fn hash_visibility(visibility: Visibility, hasher: &mut FxHasher) {
    match visibility {
        Visibility::Public => hasher.write_u8(0),
        Visibility::Private => hasher.write_u8(1),
        Visibility::Protected => hasher.write_u8(2),
    }
}

pub(super) fn hash_trait_methods(methods: &[TraitMethodSig], hasher: &mut FxHasher) {
    methods.len().hash(hasher);
    for method in methods {
        method.name.hash(hasher);
        hash_params(&method.params, hasher);
        hash_data_type(&method.return_type, hasher);
    }
}

pub(super) fn hash_enum_variants(variants: &[EnumVariantDef], hasher: &mut FxHasher) {
    variants.len().hash(hasher);
    for variant in variants {
        variant.enum_name.hash(hasher);
        variant.name.hash(hasher);
        variant.payload_names.hash(hasher);
        hash_data_types(&variant.data_types, hasher);
    }
}

pub(super) fn hash_params(params: &[(String, DataType)], hasher: &mut FxHasher) {
    params.len().hash(hasher);
    for (name, dt) in params {
        name.hash(hasher);
        hash_data_type(dt, hasher);
    }
}
