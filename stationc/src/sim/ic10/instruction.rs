//! Parsed IC10 instruction model.

use std::fmt;

use super::{
    environment::{BatchMode, DevicePort},
    registers::RegisterRef,
};

#[derive(Debug, Clone)]
pub(super) struct ProgramInstruction {
    pub(super) source_line: usize,
    pub(super) text: String,
    pub(super) instruction: Instruction,
}

#[derive(Debug, Clone)]
pub(super) enum Instruction {
    Yield,
    Hcf,
    Move {
        destination: RegisterRef,
        source: ValueOperand,
    },
    Unary {
        operation: UnaryOperation,
        destination: RegisterRef,
        source: ValueOperand,
    },
    Binary {
        operation: BinaryOperation,
        destination: RegisterRef,
        left: ValueOperand,
        right: ValueOperand,
    },
    Ternary {
        operation: TernaryOperation,
        destination: RegisterRef,
        first: ValueOperand,
        second: ValueOperand,
        third: ValueOperand,
    },
    Rand {
        destination: RegisterRef,
    },
    Select {
        destination: RegisterRef,
        condition: ValueOperand,
        if_true: ValueOperand,
        if_false: ValueOperand,
    },
    Jump {
        target: JumpTarget,
        link: bool,
        relative: bool,
    },
    Branch {
        condition: BranchCondition,
        target: JumpTarget,
        link: bool,
        relative: bool,
    },
    Push {
        value: ValueOperand,
    },
    Pop {
        destination: RegisterRef,
    },
    Peek {
        destination: RegisterRef,
    },
    Poke {
        address: ValueOperand,
        value: ValueOperand,
    },
    ClearStack {
        device: DeviceOperand,
    },
    LoadLogic {
        destination: RegisterRef,
        device: DeviceOperand,
        field: LogicFieldOperand,
    },
    BatchLoadLogic {
        destination: RegisterRef,
        prefab_hash: ValueOperand,
        name_hash: Option<ValueOperand>,
        field: LogicFieldOperand,
        mode: BatchModeOperand,
    },
    BatchStoreLogic {
        prefab_hash: ValueOperand,
        name_hash: Option<ValueOperand>,
        field: LogicFieldOperand,
        value: ValueOperand,
    },
    StoreLogic {
        device: DeviceOperand,
        field: LogicFieldOperand,
        value: ValueOperand,
    },
    DeviceSet {
        destination: RegisterRef,
        device: DeviceOperand,
        expected_set: bool,
    },
    GetStack {
        destination: RegisterRef,
        device: DeviceOperand,
        address: ValueOperand,
    },
    PutStack {
        device: DeviceOperand,
        address: ValueOperand,
        value: ValueOperand,
    },
}

#[derive(Debug, Clone)]
pub(super) enum ValueOperand {
    Register(RegisterRef),
    Number(f64),
    Symbol(String),
}

#[derive(Debug, Clone)]
pub(super) enum JumpTarget {
    Number(f64),
    Register(RegisterRef),
    Symbol(String),
}

#[derive(Debug, Clone)]
pub(super) enum DeviceOperand {
    Port(DevicePortOperand),
    Reference(ValueOperand),
}

#[derive(Debug, Clone)]
pub(super) enum LogicFieldOperand {
    Named(String),
    Dynamic(ValueOperand),
}

#[derive(Debug, Clone)]
pub(super) enum BatchModeOperand {
    Direct(BatchMode),
    Dynamic(ValueOperand),
}

#[derive(Debug, Clone, Copy)]
pub(super) enum DevicePortOperand {
    Direct(DevicePort),
    Indirect(RegisterRef),
}

#[derive(Debug, Clone, Copy)]
pub(super) enum UnaryOperation {
    Abs,
    Ceil,
    Exp,
    Floor,
    Log,
    Round,
    Sqrt,
    Trunc,
    Acos,
    Asin,
    Atan,
    Cos,
    Sin,
    Tan,
    Not,
    Seqz,
    Sgez,
    Sgtz,
    Slez,
    Sltz,
    Snan,
    Snanz,
    Snez,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum BinaryOperation {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Max,
    Min,
    Atan2,
    And,
    Or,
    Xor,
    Nor,
    Sla,
    Sll,
    Sra,
    Srl,
    Seq,
    Sne,
    Sge,
    Sgt,
    Sle,
    Slt,
    Sapz,
    Snaz,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum TernaryOperation {
    Lerp,
    Sap,
    Sna,
    Ext,
    Ins,
}

#[derive(Debug, Clone)]
pub(super) enum BranchCondition {
    Compare {
        operation: CompareOperation,
        left: ValueOperand,
        right: ValueOperand,
    },
    CompareZero {
        operation: CompareZeroOperation,
        value: ValueOperand,
    },
    Approx {
        operation: ApproxOperation,
        left: ValueOperand,
        right: ValueOperand,
        tolerance: ValueOperand,
    },
    ApproxZero {
        operation: ApproxZeroOperation,
        value: ValueOperand,
        tolerance: ValueOperand,
    },
    Nan {
        value: ValueOperand,
    },
    DeviceSet {
        device: DeviceOperand,
        expected_set: bool,
    },
    DeviceValid {
        operation: DeviceLogicOperation,
        device: DeviceOperand,
        field: LogicFieldOperand,
        expected_valid: bool,
    },
}

#[derive(Debug, Clone, Copy)]
pub(super) enum DeviceLogicOperation {
    Load,
    Store,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum CompareOperation {
    Eq,
    Ne,
    Ge,
    Gt,
    Le,
    Lt,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum CompareZeroOperation {
    Eq,
    Ne,
    Ge,
    Gt,
    Le,
    Lt,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ApproxOperation {
    Approximately,
    NotApproximately,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ApproxZeroOperation {
    ApproximatelyZero,
    NotApproximatelyZero,
}

impl fmt::Display for Instruction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}
