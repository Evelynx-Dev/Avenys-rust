pub mod codegen;
pub mod inline;
pub mod lower;
pub mod optimize;

use crate::parser::ast::DataType;
use std::collections::HashMap;

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

fn fnv_hash(bytes: &[u8]) -> u64 {
    let mut state = FNV_OFFSET_BASIS;
    for &b in bytes {
        state = state.wrapping_mul(FNV_PRIME) ^ (b as u64);
    }
    state
}

#[derive(Debug, Clone)]
pub struct MirProgram {
    pub functions: Vec<MirFunction>,
    pub entry_point: Option<String>,
    pub extern_functions: Vec<MirExternFunction>,
    pub struct_types: HashMap<String, Vec<(String, DataType)>>,
}

#[derive(Debug, Clone)]
pub struct MirExternFunction {
    pub name: String,
    pub params: Vec<DataType>,
    pub return_type: DataType,
}

#[derive(Debug, Clone)]
pub struct MirFunction {
    pub name: String,
    pub params: Vec<MirParam>,
    pub ret_type: DataType,
    pub blocks: Vec<MirBlock>,
    pub body_hash: u64,
}

#[derive(Debug, Clone)]
pub struct MirParam {
    pub name: String,
    pub data_type: DataType,
}

#[derive(Debug, Clone)]
pub struct MirBlock {
    pub id: usize,
    pub label: String,
    pub insts: Vec<MirInst>,
    pub terminator: MirTerminator,
}

#[derive(Clone, Debug)]
pub enum MirValue {
    Const(MirConst),
    Temp(usize),
    Param(String),
    Global(String),
}

#[derive(Clone, Debug)]
pub enum MirConst {
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    Str(String),
    None,
}

#[derive(Clone, Debug)]
pub struct MirType {
    pub data_type: DataType,
}

#[derive(Debug, Clone)]
pub struct MirInst {
    pub result: Option<usize>,
    pub op: MirOp,
    pub loc: (usize, usize),
}

#[derive(Debug, Clone)]
pub enum MirOp {
    Alloca(MirType),
    Load(MirValue, MirType),
    Store(MirValue, MirValue),
    Add(MirValue, MirValue),
    Sub(MirValue, MirValue),
    Mul(MirValue, MirValue),
    SDiv(MirValue, MirValue),
    Shl(MirValue, MirValue),
    And(MirValue, MirValue),
    Or(MirValue, MirValue),
    ICmp(MirCmp, MirValue, MirValue),
    FCmp(MirCmp, MirValue, MirValue),
    Call(String, Vec<MirValue>, MirType),
    Gep(MirValue, Vec<MirValue>, String),
    PtrToInt(MirValue, MirType),
    IntToPtr(MirValue, MirType),
    BitCast(MirValue, MirType),
    ZExt(MirValue, MirType),
    Trunc(MirValue, MirType),
    Sitofp(MirValue, MirType),
    Fptosi(MirValue, MirType),
    Phi(Vec<(MirValue, usize)>, MirType),
    Select(MirValue, MirValue, MirValue),
    Copy(MirValue),
}

#[derive(Debug, Clone)]
pub enum MirCmp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone)]
pub enum MirTerminator {
    Br(usize),
    BrCond(MirValue, usize, usize),
    Ret(Option<MirValue>),
    Unreachable,
}

impl MirProgram {
    pub fn new(functions: Vec<MirFunction>, entry_point: Option<String>) -> Self {
        Self {
            functions,
            entry_point,
            extern_functions: Vec::new(),
            struct_types: HashMap::new(),
        }
    }
}

impl MirFunction {
    pub fn new(name: String, params: Vec<MirParam>, ret_type: DataType) -> Self {
        Self {
            name,
            params,
            ret_type,
            blocks: Vec::new(),
            body_hash: 0,
        }
    }

    pub fn next_temp(&self) -> usize {
        self.blocks
            .iter()
            .flat_map(|b| b.insts.iter())
            .filter_map(|inst| inst.result)
            .max()
            .map(|m| m + 1)
            .unwrap_or(0)
    }

    pub fn compute_hash(&self) -> u64 {
        let mut buf = Vec::new();
        for block in &self.blocks {
            buf.extend_from_slice(&block.id.to_le_bytes());
            for inst in &block.insts {
                buf.push(inst.result.unwrap_or(255) as u8);
            }
            match &block.terminator {
                MirTerminator::Br(t) => {
                    buf.extend_from_slice(&(*t as u64).to_le_bytes());
                    buf.push(0);
                }
                MirTerminator::BrCond(_, t, f) => {
                    buf.extend_from_slice(&(*t as u64).to_le_bytes());
                    buf.extend_from_slice(&(*f as u64).to_le_bytes());
                    buf.push(1);
                }
                MirTerminator::Ret(_) => buf.push(2),
                MirTerminator::Unreachable => buf.push(3),
            }
        }
        fnv_hash(&buf)
    }

    pub fn push_block(&mut self, label: String) -> usize {
        let id = self.blocks.len();
        self.blocks.push(MirBlock {
            id,
            label,
            insts: Vec::new(),
            terminator: MirTerminator::Unreachable,
        });
        id
    }
}

impl MirBlock {
    pub fn push(&mut self, result: Option<usize>, op: MirOp, loc: (usize, usize)) {
        self.insts.push(MirInst { result, op, loc });
    }
}

impl MirValue {
    pub fn temp(id: usize) -> Self {
        MirValue::Temp(id)
    }
}
