use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum LlType {
    I8,
    I64,
    I1,
    F64,
    Ptr,
    Struct(Vec<LlType>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LlValue {
    pub(super) ty: LlType,
    pub(super) repr: String,
    pub(super) owned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VarInfo {
    pub(super) ptr: String,
    pub(super) ty: LlType,
    pub(super) data_type: DataType,
    pub(super) owns_heap_string: bool,
    pub(super) struct_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FnInfo {
    pub(super) llvm_name: String,
    pub(super) params: Vec<LlType>,
    pub(super) ret: LlType,
    pub(super) returns_value: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LoopLabels {
    pub(super) break_label: String,
    pub(super) continue_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StructInfo {
    pub(super) fields: Vec<LlType>,
    pub(super) field_data_types: Vec<DataType>,
    pub(super) field_indices: HashMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EnumInfo {
    pub(super) llvm_type: String,
    pub(super) variants: HashMap<String, VariantInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VariantInfo {
    pub(super) tag: u32,
    pub(super) payload_types: Vec<LlType>,
}
