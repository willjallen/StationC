//! IC10 source parser.

use std::{collections::HashMap, fmt};

use super::{
    instruction::{
        ApproxOperation, ApproxZeroOperation, BinaryOperation, BranchCondition, CompareOperation,
        CompareZeroOperation, Instruction, JumpTarget, ProgramInstruction, TernaryOperation,
        UnaryOperation, ValueOperand,
    },
    program::Program,
    registers::RegisterRef,
};

#[derive(Debug, Clone, Copy)]
struct BranchFlags {
    link: bool,
    relative: bool,
}

#[derive(Debug)]
pub(super) struct ParseError {
    code: ParseErrorCode,
    line: usize,
    message: String,
}

impl ParseError {
    fn new(code: ParseErrorCode, line: usize, message: impl Into<String>) -> Self {
        Self {
            code,
            line,
            message: message.into(),
        }
    }

    pub(super) const fn code(&self) -> ParseErrorCode {
        self.code
    }

    pub(super) const fn line(&self) -> usize {
        self.line
    }

    pub(super) fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ParseErrorCode {
    DuplicateLabel,
    WrongArity,
    AliasTargetMustBeRegister,
    DefineValueMustBeNumeric,
    UnsupportedInstruction,
    ExpectedRegister,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "line {}: {}", self.line, self.message)
    }
}

pub(super) fn parse_program(source: &str) -> Result<Program, ParseError> {
    let mut labels = HashMap::new();
    let mut aliases = HashMap::new();
    let mut constants = HashMap::new();
    let mut instructions = Vec::new();

    for (line_index, raw_line) in source.lines().enumerate() {
        let line_number = line_index + 1;
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        let remainder = parse_labels(line, line_number, instructions.len(), &mut labels)?;
        let tokens = tokenize(remainder);
        if tokens.is_empty() {
            continue;
        }

        match tokens[0] {
            "alias" => parse_alias(&tokens, line_number, &mut aliases)?,
            "define" => parse_define(&tokens, line_number, &mut constants)?,
            _ => {
                let instruction = parse_instruction(&tokens, line_number, &aliases, &constants)?;
                instructions.push(ProgramInstruction {
                    source_line: line_number,
                    text: tokens.join(" "),
                    instruction,
                });
            }
        }
    }

    Ok(Program::new(instructions, labels, constants))
}

fn strip_comment(line: &str) -> &str {
    line.split_once('#').map_or(line, |(before, _)| before)
}

fn parse_labels<'a>(
    mut line: &'a str,
    line_number: usize,
    instruction_index: usize,
    labels: &mut HashMap<String, usize>,
) -> Result<&'a str, ParseError> {
    while let Some((candidate, after)) = line.split_once(':') {
        let label = candidate.trim();
        if label.contains(char::is_whitespace) || label.is_empty() {
            break;
        }
        if labels.insert(label.to_owned(), instruction_index).is_some() {
            return Err(ParseError::new(
                ParseErrorCode::DuplicateLabel,
                line_number,
                format!("duplicate label `{label}`"),
            ));
        }
        line = after.trim();
        if line.is_empty() {
            break;
        }
    }
    Ok(line)
}

fn tokenize(line: &str) -> Vec<&str> {
    line.split_whitespace().collect()
}

fn parse_alias(
    tokens: &[&str],
    line_number: usize,
    aliases: &mut HashMap<String, RegisterRef>,
) -> Result<(), ParseError> {
    require_len(tokens, 3, line_number)?;
    let name = tokens[1];
    let register = RegisterRef::parse(tokens[2]).ok_or_else(|| {
        ParseError::new(
            ParseErrorCode::AliasTargetMustBeRegister,
            line_number,
            "phase 1 aliases must target registers",
        )
    })?;
    aliases.insert(name.to_owned(), register);
    Ok(())
}

fn parse_define(
    tokens: &[&str],
    line_number: usize,
    constants: &mut HashMap<String, f64>,
) -> Result<(), ParseError> {
    require_len(tokens, 3, line_number)?;
    let value = parse_number(tokens[2]).ok_or_else(|| {
        ParseError::new(
            ParseErrorCode::DefineValueMustBeNumeric,
            line_number,
            "define value must be numeric",
        )
    })?;
    constants.insert(tokens[1].to_owned(), value);
    Ok(())
}

fn parse_instruction(
    tokens: &[&str],
    line_number: usize,
    aliases: &HashMap<String, RegisterRef>,
    constants: &HashMap<String, f64>,
) -> Result<Instruction, ParseError> {
    match tokens[0] {
        "yield" => {
            require_len(tokens, 1, line_number)?;
            Ok(Instruction::Yield)
        }
        "hcf" => {
            require_len(tokens, 1, line_number)?;
            Ok(Instruction::Hcf)
        }
        "move" => parse_move(tokens, line_number, aliases, constants),
        "rand" => {
            require_len(tokens, 2, line_number)?;
            Ok(Instruction::Rand {
                destination: parse_register(tokens[1], line_number, aliases)?,
            })
        }
        "select" => {
            require_len(tokens, 5, line_number)?;
            Ok(Instruction::Select {
                destination: parse_register(tokens[1], line_number, aliases)?,
                condition: parse_value(tokens[2], aliases, constants),
                if_true: parse_value(tokens[3], aliases, constants),
                if_false: parse_value(tokens[4], aliases, constants),
            })
        }
        "push" => {
            require_len(tokens, 2, line_number)?;
            Ok(Instruction::Push {
                value: parse_value(tokens[1], aliases, constants),
            })
        }
        "pop" => {
            require_len(tokens, 2, line_number)?;
            Ok(Instruction::Pop {
                destination: parse_register(tokens[1], line_number, aliases)?,
            })
        }
        "peek" => {
            require_len(tokens, 2, line_number)?;
            Ok(Instruction::Peek {
                destination: parse_register(tokens[1], line_number, aliases)?,
            })
        }
        "poke" => {
            require_len(tokens, 3, line_number)?;
            Ok(Instruction::Poke {
                address: parse_value(tokens[1], aliases, constants),
                value: parse_value(tokens[2], aliases, constants),
            })
        }
        "j" => parse_jump(tokens, line_number, aliases, false, false),
        "jal" => parse_jump(tokens, line_number, aliases, true, false),
        "jr" => parse_jump(tokens, line_number, aliases, false, true),
        mnemonic => parse_operation(mnemonic, tokens, line_number, aliases, constants),
    }
}

fn parse_move(
    tokens: &[&str],
    line_number: usize,
    aliases: &HashMap<String, RegisterRef>,
    constants: &HashMap<String, f64>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 3, line_number)?;
    Ok(Instruction::Move {
        destination: parse_register(tokens[1], line_number, aliases)?,
        source: parse_value(tokens[2], aliases, constants),
    })
}

fn parse_jump(
    tokens: &[&str],
    line_number: usize,
    aliases: &HashMap<String, RegisterRef>,
    link: bool,
    relative: bool,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 2, line_number)?;
    Ok(Instruction::Jump {
        target: parse_jump_target(tokens[1], aliases),
        link,
        relative,
    })
}

fn parse_operation(
    mnemonic: &str,
    tokens: &[&str],
    line_number: usize,
    aliases: &HashMap<String, RegisterRef>,
    constants: &HashMap<String, f64>,
) -> Result<Instruction, ParseError> {
    if let Some(operation) = unary_operation(mnemonic) {
        require_len(tokens, 3, line_number)?;
        return Ok(Instruction::Unary {
            operation,
            destination: parse_register(tokens[1], line_number, aliases)?,
            source: parse_value(tokens[2], aliases, constants),
        });
    }

    if let Some(operation) = binary_operation(mnemonic) {
        require_len(tokens, 4, line_number)?;
        return Ok(Instruction::Binary {
            operation,
            destination: parse_register(tokens[1], line_number, aliases)?,
            left: parse_value(tokens[2], aliases, constants),
            right: parse_value(tokens[3], aliases, constants),
        });
    }

    if let Some(operation) = ternary_operation(mnemonic) {
        require_len(tokens, 5, line_number)?;
        return Ok(Instruction::Ternary {
            operation,
            destination: parse_register(tokens[1], line_number, aliases)?,
            first: parse_value(tokens[2], aliases, constants),
            second: parse_value(tokens[3], aliases, constants),
            third: parse_value(tokens[4], aliases, constants),
        });
    }

    if let Some((condition, target_index, flags)) =
        parse_branch_condition(mnemonic, tokens, aliases, constants)
    {
        require_len(tokens, target_index + 1, line_number)?;
        return Ok(Instruction::Branch {
            condition,
            target: parse_jump_target(tokens[target_index], aliases),
            link: flags.link,
            relative: flags.relative,
        });
    }

    Err(ParseError::new(
        ParseErrorCode::UnsupportedInstruction,
        line_number,
        format!("unsupported instruction `{mnemonic}`"),
    ))
}

fn parse_branch_condition(
    mnemonic: &str,
    tokens: &[&str],
    aliases: &HashMap<String, RegisterRef>,
    constants: &HashMap<String, f64>,
) -> Option<(BranchCondition, usize, BranchFlags)> {
    let (base, flags) = branch_family(mnemonic)?;
    if let Some(operation) = compare_operation(base) {
        return Some(compare_branch(operation, tokens, aliases, constants, flags));
    }
    if let Some(operation) = compare_zero_operation(base) {
        return Some(compare_zero_branch(
            operation, tokens, aliases, constants, flags,
        ));
    }
    if let Some(operation) = approximate_operation(base) {
        return Some(approx_branch(operation, tokens, aliases, constants, flags));
    }
    if let Some(operation) = approximate_zero_operation(base) {
        return Some(approx_zero_branch(
            operation, tokens, aliases, constants, flags,
        ));
    }
    parse_nan_branch(base, tokens, aliases, constants, flags)
}

fn compare_branch(
    operation: CompareOperation,
    tokens: &[&str],
    aliases: &HashMap<String, RegisterRef>,
    constants: &HashMap<String, f64>,
    flags: BranchFlags,
) -> (BranchCondition, usize, BranchFlags) {
    (
        BranchCondition::Compare {
            operation,
            left: parse_value(
                tokens.get(1).copied().unwrap_or_default(),
                aliases,
                constants,
            ),
            right: parse_value(
                tokens.get(2).copied().unwrap_or_default(),
                aliases,
                constants,
            ),
        },
        3,
        flags,
    )
}

fn compare_zero_branch(
    operation: CompareZeroOperation,
    tokens: &[&str],
    aliases: &HashMap<String, RegisterRef>,
    constants: &HashMap<String, f64>,
    flags: BranchFlags,
) -> (BranchCondition, usize, BranchFlags) {
    (
        BranchCondition::CompareZero {
            operation,
            value: parse_value(
                tokens.get(1).copied().unwrap_or_default(),
                aliases,
                constants,
            ),
        },
        2,
        flags,
    )
}

fn approx_branch(
    operation: ApproxOperation,
    tokens: &[&str],
    aliases: &HashMap<String, RegisterRef>,
    constants: &HashMap<String, f64>,
    flags: BranchFlags,
) -> (BranchCondition, usize, BranchFlags) {
    (
        BranchCondition::Approx {
            operation,
            left: parse_value(
                tokens.get(1).copied().unwrap_or_default(),
                aliases,
                constants,
            ),
            right: parse_value(
                tokens.get(2).copied().unwrap_or_default(),
                aliases,
                constants,
            ),
            tolerance: parse_value(
                tokens.get(3).copied().unwrap_or_default(),
                aliases,
                constants,
            ),
        },
        4,
        flags,
    )
}

fn approx_zero_branch(
    operation: ApproxZeroOperation,
    tokens: &[&str],
    aliases: &HashMap<String, RegisterRef>,
    constants: &HashMap<String, f64>,
    flags: BranchFlags,
) -> (BranchCondition, usize, BranchFlags) {
    (
        BranchCondition::ApproxZero {
            operation,
            value: parse_value(
                tokens.get(1).copied().unwrap_or_default(),
                aliases,
                constants,
            ),
            tolerance: parse_value(
                tokens.get(2).copied().unwrap_or_default(),
                aliases,
                constants,
            ),
        },
        3,
        flags,
    )
}

fn parse_nan_branch(
    base: &str,
    tokens: &[&str],
    aliases: &HashMap<String, RegisterRef>,
    constants: &HashMap<String, f64>,
    flags: BranchFlags,
) -> Option<(BranchCondition, usize, BranchFlags)> {
    if base == "bnan" {
        Some((
            BranchCondition::Nan {
                value: parse_value(
                    tokens.get(1).copied().unwrap_or_default(),
                    aliases,
                    constants,
                ),
            },
            2,
            flags,
        ))
    } else {
        None
    }
}

fn compare_operation(base: &str) -> Option<CompareOperation> {
    match base {
        "beq" => Some(CompareOperation::Eq),
        "bne" => Some(CompareOperation::Ne),
        "bge" => Some(CompareOperation::Ge),
        "bgt" => Some(CompareOperation::Gt),
        "ble" => Some(CompareOperation::Le),
        "blt" => Some(CompareOperation::Lt),
        _ => None,
    }
}

fn compare_zero_operation(base: &str) -> Option<CompareZeroOperation> {
    match base {
        "beqz" => Some(CompareZeroOperation::Eq),
        "bnez" => Some(CompareZeroOperation::Ne),
        "bgez" => Some(CompareZeroOperation::Ge),
        "bgtz" => Some(CompareZeroOperation::Gt),
        "blez" => Some(CompareZeroOperation::Le),
        "bltz" => Some(CompareZeroOperation::Lt),
        _ => None,
    }
}

fn approximate_operation(base: &str) -> Option<ApproxOperation> {
    match base {
        "bap" => Some(ApproxOperation::Approximately),
        "bna" => Some(ApproxOperation::NotApproximately),
        _ => None,
    }
}

fn approximate_zero_operation(base: &str) -> Option<ApproxZeroOperation> {
    match base {
        "bapz" => Some(ApproxZeroOperation::ApproximatelyZero),
        "bnaz" => Some(ApproxZeroOperation::NotApproximatelyZero),
        _ => None,
    }
}

fn branch_family(mnemonic: &str) -> Option<(&str, BranchFlags)> {
    if let Some(stripped) = mnemonic.strip_prefix("br") {
        return Some((
            branch_base(stripped)?,
            BranchFlags {
                link: false,
                relative: true,
            },
        ));
    }
    if let Some(stripped) = mnemonic.strip_suffix("al") {
        return Some((
            stripped,
            BranchFlags {
                link: true,
                relative: false,
            },
        ));
    }
    if mnemonic.starts_with('b') {
        return Some((
            mnemonic,
            BranchFlags {
                link: false,
                relative: false,
            },
        ));
    }
    None
}

fn branch_base(stripped_relative: &str) -> Option<&str> {
    match stripped_relative {
        "eq" => Some("beq"),
        "ne" => Some("bne"),
        "ge" => Some("bge"),
        "gt" => Some("bgt"),
        "le" => Some("ble"),
        "lt" => Some("blt"),
        "eqz" => Some("beqz"),
        "nez" => Some("bnez"),
        "gez" => Some("bgez"),
        "gtz" => Some("bgtz"),
        "lez" => Some("blez"),
        "ltz" => Some("bltz"),
        "ap" => Some("bap"),
        "na" => Some("bna"),
        "apz" => Some("bapz"),
        "naz" => Some("bnaz"),
        "nan" => Some("bnan"),
        _ => None,
    }
}

fn unary_operation(mnemonic: &str) -> Option<UnaryOperation> {
    match mnemonic {
        "abs" => Some(UnaryOperation::Abs),
        "ceil" => Some(UnaryOperation::Ceil),
        "exp" => Some(UnaryOperation::Exp),
        "floor" => Some(UnaryOperation::Floor),
        "log" => Some(UnaryOperation::Log),
        "round" => Some(UnaryOperation::Round),
        "sqrt" => Some(UnaryOperation::Sqrt),
        "trunc" => Some(UnaryOperation::Trunc),
        "acos" => Some(UnaryOperation::Acos),
        "asin" => Some(UnaryOperation::Asin),
        "atan" => Some(UnaryOperation::Atan),
        "cos" => Some(UnaryOperation::Cos),
        "sin" => Some(UnaryOperation::Sin),
        "tan" => Some(UnaryOperation::Tan),
        "not" => Some(UnaryOperation::Not),
        "seqz" => Some(UnaryOperation::Seqz),
        "sgez" => Some(UnaryOperation::Sgez),
        "sgtz" => Some(UnaryOperation::Sgtz),
        "slez" => Some(UnaryOperation::Slez),
        "sltz" => Some(UnaryOperation::Sltz),
        "snan" => Some(UnaryOperation::Snan),
        "snanz" => Some(UnaryOperation::Snanz),
        "snez" => Some(UnaryOperation::Snez),
        _ => None,
    }
}

fn binary_operation(mnemonic: &str) -> Option<BinaryOperation> {
    match mnemonic {
        "add" => Some(BinaryOperation::Add),
        "sub" => Some(BinaryOperation::Sub),
        "mul" => Some(BinaryOperation::Mul),
        "div" => Some(BinaryOperation::Div),
        "mod" => Some(BinaryOperation::Mod),
        "pow" => Some(BinaryOperation::Pow),
        "max" => Some(BinaryOperation::Max),
        "min" => Some(BinaryOperation::Min),
        "atan2" => Some(BinaryOperation::Atan2),
        "and" => Some(BinaryOperation::And),
        "or" => Some(BinaryOperation::Or),
        "xor" => Some(BinaryOperation::Xor),
        "nor" => Some(BinaryOperation::Nor),
        "sla" => Some(BinaryOperation::Sla),
        "sll" => Some(BinaryOperation::Sll),
        "sra" => Some(BinaryOperation::Sra),
        "srl" => Some(BinaryOperation::Srl),
        "seq" => Some(BinaryOperation::Seq),
        "sne" => Some(BinaryOperation::Sne),
        "sge" => Some(BinaryOperation::Sge),
        "sgt" => Some(BinaryOperation::Sgt),
        "sle" => Some(BinaryOperation::Sle),
        "slt" => Some(BinaryOperation::Slt),
        "sapz" => Some(BinaryOperation::Sapz),
        "snaz" => Some(BinaryOperation::Snaz),
        _ => None,
    }
}

fn ternary_operation(mnemonic: &str) -> Option<TernaryOperation> {
    match mnemonic {
        "lerp" => Some(TernaryOperation::Lerp),
        "sap" => Some(TernaryOperation::Sap),
        "sna" => Some(TernaryOperation::Sna),
        _ => None,
    }
}

fn parse_register(
    token: &str,
    line_number: usize,
    aliases: &HashMap<String, RegisterRef>,
) -> Result<RegisterRef, ParseError> {
    aliases
        .get(token)
        .copied()
        .or_else(|| RegisterRef::parse(token))
        .ok_or_else(|| {
            ParseError::new(
                ParseErrorCode::ExpectedRegister,
                line_number,
                format!("expected register, found `{token}`"),
            )
        })
}

fn parse_value(
    token: &str,
    aliases: &HashMap<String, RegisterRef>,
    constants: &HashMap<String, f64>,
) -> ValueOperand {
    aliases
        .get(token)
        .copied()
        .or_else(|| RegisterRef::parse(token))
        .map_or_else(
            || parse_non_register_value(token, constants),
            ValueOperand::Register,
        )
}

fn parse_non_register_value(token: &str, constants: &HashMap<String, f64>) -> ValueOperand {
    constants.get(token).map_or_else(
        || {
            parse_number(token).map_or_else(
                || ValueOperand::Symbol(token.to_owned()),
                ValueOperand::Number,
            )
        },
        |value| ValueOperand::Number(*value),
    )
}

fn parse_jump_target(token: &str, aliases: &HashMap<String, RegisterRef>) -> JumpTarget {
    aliases
        .get(token)
        .copied()
        .or_else(|| RegisterRef::parse(token))
        .map_or_else(
            || {
                parse_number(token)
                    .map_or_else(|| JumpTarget::Symbol(token.to_owned()), JumpTarget::Number)
            },
            JumpTarget::Register,
        )
}

fn parse_number(token: &str) -> Option<f64> {
    match token {
        "nan" => Some(f64::NAN),
        "pinf" => Some(f64::INFINITY),
        "ninf" => Some(f64::NEG_INFINITY),
        _ => parse_numeric_literal(token),
    }
}

fn parse_numeric_literal(token: &str) -> Option<f64> {
    if let Some(hex) = token.strip_prefix('$') {
        let parsed = u64::from_str_radix(hex, 16).ok()?;
        #[allow(clippy::cast_precision_loss)]
        return Some(parsed as f64);
    }
    if let Some(binary) = token.strip_prefix('%') {
        let cleaned = binary.replace('_', "");
        let parsed = i64::from_str_radix(&cleaned, 2).ok()?;
        #[allow(clippy::cast_precision_loss)]
        return Some(parsed as f64);
    }
    token.parse::<f64>().ok()
}

fn require_len(tokens: &[&str], expected: usize, line_number: usize) -> Result<(), ParseError> {
    if tokens.len() == expected {
        Ok(())
    } else {
        Err(ParseError::new(
            ParseErrorCode::WrongArity,
            line_number,
            format!(
                "`{}` expects {} token(s), got {}",
                tokens.first().copied().unwrap_or("<empty>"),
                expected,
                tokens.len()
            ),
        ))
    }
}
