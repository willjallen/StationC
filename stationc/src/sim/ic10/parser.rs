//! IC10 source parser.

use std::{collections::HashMap, fmt};

use super::{
    environment::{BatchMode, DevicePort},
    instruction::{
        ApproxOperation, ApproxZeroOperation, BatchModeOperand, BinaryOperation, BranchCondition,
        CompareOperation, CompareZeroOperation, DeviceLogicOperation, DeviceOperand,
        DevicePortOperand, Instruction, JumpTarget, LogicFieldOperand, ProgramInstruction,
        TernaryOperation, UnaryOperation, ValueOperand,
    },
    logic_types,
    program::Program,
    registers::{RegisterIndex, RegisterRef},
};

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

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "line {}: {}", self.line, self.message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ParseErrorCode {
    DuplicateLabel,
    WrongArity,
    AliasTargetMustBeRegisterOrDevice,
    DefineValueMustBeNumeric,
    UnsupportedInstruction,
    ExpectedRegister,
    ExpectedDevicePin,
}

#[derive(Debug, Clone, Copy)]
struct ParseContext<'source> {
    line_number: usize,
    aliases: &'source HashMap<String, AliasTarget>,
    constants: &'source HashMap<String, f64>,
}

impl<'source> ParseContext<'source> {
    const fn new(
        line_number: usize,
        aliases: &'source HashMap<String, AliasTarget>,
        constants: &'source HashMap<String, f64>,
    ) -> Self {
        Self {
            line_number,
            aliases,
            constants,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum AliasTarget {
    Register(RegisterRef),
    Device(DevicePortOperand),
}

#[derive(Debug, Clone, Copy)]
struct BranchFlags {
    link: bool,
    relative: bool,
}

#[derive(Debug, Clone, Copy)]
enum BranchShape {
    Compare(CompareOperation),
    CompareZero(CompareZeroOperation),
    Approx(ApproxOperation),
    ApproxZero(ApproxZeroOperation),
    Nan,
}

impl BranchShape {
    const fn target_index(self) -> usize {
        match self {
            Self::CompareZero(_) | Self::Nan => 2,
            Self::Compare(_) | Self::ApproxZero(_) => 3,
            Self::Approx(_) => 4,
        }
    }

    const fn expected_tokens(self) -> usize {
        self.target_index() + 1
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
        let token_refs: Vec<&str> = tokens.iter().map(String::as_str).collect();

        if parse_directive(&token_refs, line_number, &mut aliases, &mut constants)? {
            continue;
        }

        let context = ParseContext::new(line_number, &aliases, &constants);
        instructions.push(ProgramInstruction {
            source_line: line_number,
            text: tokens.join(" "),
            instruction: parse_instruction(&token_refs, context)?,
        });
    }

    Ok(Program::new(instructions, labels, constants))
}

fn strip_comment(line: &str) -> &str {
    line.split_once('#').map_or(line, |(before, _)| before)
}

fn parse_labels<'line>(
    mut line: &'line str,
    line_number: usize,
    instruction_index: usize,
    labels: &mut HashMap<String, usize>,
) -> Result<&'line str, ParseError> {
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

fn tokenize(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;

    for character in line.chars() {
        match character {
            '"' => {
                in_quote = !in_quote;
                current.push(character);
            }
            character if character.is_whitespace() && !in_quote => {
                if !current.is_empty() {
                    tokens.push(current);
                    current = String::new();
                }
            }
            character => current.push(character),
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn parse_directive(
    tokens: &[&str],
    line_number: usize,
    aliases: &mut HashMap<String, AliasTarget>,
    constants: &mut HashMap<String, f64>,
) -> Result<bool, ParseError> {
    match tokens[0] {
        "alias" => {
            parse_alias(tokens, line_number, aliases)?;
            Ok(true)
        }
        "define" => {
            parse_define(tokens, line_number, constants)?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn parse_alias(
    tokens: &[&str],
    line_number: usize,
    aliases: &mut HashMap<String, AliasTarget>,
) -> Result<(), ParseError> {
    require_len(tokens, 3, line_number)?;
    let name = tokens[1];
    let target = parse_alias_target(tokens[2]).ok_or_else(|| {
        ParseError::new(
            ParseErrorCode::AliasTargetMustBeRegisterOrDevice,
            line_number,
            "alias target must be a register or device pin",
        )
    })?;
    aliases.insert(name.to_owned(), target);
    Ok(())
}

fn parse_alias_target(token: &str) -> Option<AliasTarget> {
    parse_register_token(token)
        .map(AliasTarget::Register)
        .or_else(|| parse_device_port_token(token).map(AliasTarget::Device))
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
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    match tokens[0] {
        "yield" | "sleep" | "hcf" => parse_control_instruction(tokens, context),
        "move" | "rand" | "select" => parse_register_instruction(tokens, context),
        "push" | "pop" | "peek" | "poke" => parse_local_stack_instruction(tokens, context),
        "l" | "s" | "ld" | "sd" => parse_device_logic_instruction(tokens, context),
        "ls" | "ss" => parse_slot_logic_instruction(tokens, context),
        "sdns" | "sdse" => parse_device_predicate_instruction(tokens, context),
        "lb" | "lbn" | "lbs" | "lbns" | "sb" | "sbn" | "sbs" => {
            parse_batch_logic_instruction(tokens, context)
        }
        "clr" | "clrd" | "get" | "put" | "getd" | "putd" => {
            parse_device_stack_instruction(tokens, context)
        }
        "j" | "jal" | "jr" => parse_jump_instruction(tokens, context),
        mnemonic => parse_math_or_branch_instruction(mnemonic, tokens, context),
    }
}

fn parse_control_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    match tokens[0] {
        "yield" => {
            require_len(tokens, 1, context.line_number)?;
            Ok(Instruction::Yield)
        }
        "sleep" => {
            require_len(tokens, 2, context.line_number)?;
            Ok(Instruction::Sleep {
                duration: parse_value(tokens[1], context),
            })
        }
        "hcf" => {
            require_len(tokens, 1, context.line_number)?;
            Ok(Instruction::Hcf)
        }
        mnemonic => unsupported_instruction(mnemonic, context.line_number),
    }
}

fn parse_register_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    match tokens[0] {
        "move" => parse_move_instruction(tokens, context),
        "rand" => parse_rand_instruction(tokens, context),
        "select" => parse_select_instruction(tokens, context),
        mnemonic => unsupported_instruction(mnemonic, context.line_number),
    }
}

fn parse_move_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 3, context.line_number)?;
    Ok(Instruction::Move {
        destination: parse_register(tokens[1], context)?,
        source: parse_value(tokens[2], context),
    })
}

fn parse_rand_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 2, context.line_number)?;
    Ok(Instruction::Rand {
        destination: parse_register(tokens[1], context)?,
    })
}

fn parse_select_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 5, context.line_number)?;
    Ok(Instruction::Select {
        destination: parse_register(tokens[1], context)?,
        condition: parse_value(tokens[2], context),
        if_true: parse_value(tokens[3], context),
        if_false: parse_value(tokens[4], context),
    })
}

fn parse_local_stack_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    match tokens[0] {
        "push" => parse_push_instruction(tokens, context),
        "pop" => parse_pop_instruction(tokens, context),
        "peek" => parse_peek_instruction(tokens, context),
        "poke" => parse_poke_instruction(tokens, context),
        mnemonic => unsupported_instruction(mnemonic, context.line_number),
    }
}

fn parse_push_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 2, context.line_number)?;
    Ok(Instruction::Push {
        value: parse_value(tokens[1], context),
    })
}

fn parse_pop_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 2, context.line_number)?;
    Ok(Instruction::Pop {
        destination: parse_register(tokens[1], context)?,
    })
}

fn parse_peek_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 2, context.line_number)?;
    Ok(Instruction::Peek {
        destination: parse_register(tokens[1], context)?,
    })
}

fn parse_poke_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 3, context.line_number)?;
    Ok(Instruction::Poke {
        address: parse_value(tokens[1], context),
        value: parse_value(tokens[2], context),
    })
}

fn parse_device_logic_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    match tokens[0] {
        "l" | "ld" => parse_load_logic_instruction(tokens, context),
        "s" | "sd" => parse_store_logic_instruction(tokens, context),
        mnemonic => unsupported_instruction(mnemonic, context.line_number),
    }
}

fn parse_load_logic_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 4, context.line_number)?;
    let device = parse_load_or_store_device(tokens[0], tokens[2], context);
    Ok(Instruction::LoadLogic {
        destination: parse_register(tokens[1], context)?,
        device,
        field: parse_logic_field(tokens[3], context),
    })
}

fn parse_store_logic_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 4, context.line_number)?;
    let device = parse_load_or_store_device(tokens[0], tokens[1], context);
    Ok(Instruction::StoreLogic {
        device,
        field: parse_logic_field(tokens[2], context),
        value: parse_value(tokens[3], context),
    })
}

fn parse_slot_logic_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    match tokens[0] {
        "ls" => parse_load_slot_logic_instruction(tokens, context),
        "ss" => parse_store_slot_logic_instruction(tokens, context),
        mnemonic => unsupported_instruction(mnemonic, context.line_number),
    }
}

fn parse_load_slot_logic_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 5, context.line_number)?;
    Ok(Instruction::LoadSlotLogic {
        destination: parse_register(tokens[1], context)?,
        device: parse_slot_device(tokens[2], context)?,
        slot: parse_value(tokens[3], context),
        field: parse_logic_field(tokens[4], context),
    })
}

fn parse_store_slot_logic_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 5, context.line_number)?;
    Ok(Instruction::StoreSlotLogic {
        device: parse_slot_device(tokens[1], context)?,
        slot: parse_value(tokens[2], context),
        field: parse_logic_field(tokens[3], context),
        value: parse_value(tokens[4], context),
    })
}

fn parse_device_predicate_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 3, context.line_number)?;
    let expected_set = match tokens[0] {
        "sdse" => true,
        "sdns" => false,
        mnemonic => return unsupported_instruction(mnemonic, context.line_number),
    };
    Ok(Instruction::DeviceSet {
        destination: parse_register(tokens[1], context)?,
        device: parse_device_operand(tokens[2], context),
        expected_set,
    })
}

fn parse_batch_logic_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    match tokens[0] {
        "lb" => parse_batch_load_logic_instruction(tokens, context),
        "lbn" => parse_batch_load_logic_by_name_instruction(tokens, context),
        "lbs" => parse_batch_load_slot_logic_instruction(tokens, context),
        "lbns" => parse_batch_load_slot_logic_by_name_instruction(tokens, context),
        "sb" => parse_batch_store_logic_instruction(tokens, context),
        "sbn" => parse_batch_store_logic_by_name_instruction(tokens, context),
        "sbs" => parse_batch_store_slot_logic_instruction(tokens, context),
        mnemonic => unsupported_instruction(mnemonic, context.line_number),
    }
}

fn parse_batch_load_logic_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 5, context.line_number)?;
    Ok(Instruction::BatchLoadLogic {
        destination: parse_register(tokens[1], context)?,
        prefab_hash: parse_value(tokens[2], context),
        name_hash: None,
        field: parse_logic_field(tokens[3], context),
        mode: parse_batch_mode(tokens[4], context),
    })
}

fn parse_batch_load_logic_by_name_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 6, context.line_number)?;
    Ok(Instruction::BatchLoadLogic {
        destination: parse_register(tokens[1], context)?,
        prefab_hash: parse_value(tokens[2], context),
        name_hash: Some(parse_value(tokens[3], context)),
        field: parse_logic_field(tokens[4], context),
        mode: parse_batch_mode(tokens[5], context),
    })
}

fn parse_batch_load_slot_logic_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 6, context.line_number)?;
    Ok(Instruction::BatchLoadSlotLogic {
        destination: parse_register(tokens[1], context)?,
        prefab_hash: parse_value(tokens[2], context),
        name_hash: None,
        slot: parse_value(tokens[3], context),
        field: parse_logic_field(tokens[4], context),
        mode: parse_batch_mode(tokens[5], context),
    })
}

fn parse_batch_load_slot_logic_by_name_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 7, context.line_number)?;
    Ok(Instruction::BatchLoadSlotLogic {
        destination: parse_register(tokens[1], context)?,
        prefab_hash: parse_value(tokens[2], context),
        name_hash: Some(parse_value(tokens[3], context)),
        slot: parse_value(tokens[4], context),
        field: parse_logic_field(tokens[5], context),
        mode: parse_batch_mode(tokens[6], context),
    })
}

fn parse_batch_store_logic_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 4, context.line_number)?;
    Ok(Instruction::BatchStoreLogic {
        prefab_hash: parse_value(tokens[1], context),
        name_hash: None,
        field: parse_logic_field(tokens[2], context),
        value: parse_value(tokens[3], context),
    })
}

fn parse_batch_store_logic_by_name_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 5, context.line_number)?;
    Ok(Instruction::BatchStoreLogic {
        prefab_hash: parse_value(tokens[1], context),
        name_hash: Some(parse_value(tokens[2], context)),
        field: parse_logic_field(tokens[3], context),
        value: parse_value(tokens[4], context),
    })
}

fn parse_batch_store_slot_logic_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 5, context.line_number)?;
    Ok(Instruction::BatchStoreSlotLogic {
        prefab_hash: parse_value(tokens[1], context),
        slot: parse_value(tokens[2], context),
        field: parse_logic_field(tokens[3], context),
        value: parse_value(tokens[4], context),
    })
}

fn parse_device_stack_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    match tokens[0] {
        "clr" | "clrd" => parse_clear_stack_instruction(tokens, context),
        "get" | "getd" => parse_get_stack_instruction(tokens, context),
        "put" | "putd" => parse_put_stack_instruction(tokens, context),
        mnemonic => unsupported_instruction(mnemonic, context.line_number),
    }
}

fn parse_clear_stack_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 2, context.line_number)?;
    Ok(Instruction::ClearStack {
        device: parse_clear_stack_device(tokens[0], tokens[1], context),
    })
}

fn parse_get_stack_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 4, context.line_number)?;
    let device = parse_get_or_put_device(tokens[0], tokens[2], context);
    Ok(Instruction::GetStack {
        destination: parse_register(tokens[1], context)?,
        device,
        address: parse_value(tokens[3], context),
    })
}

fn parse_put_stack_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 4, context.line_number)?;
    let device = parse_get_or_put_device(tokens[0], tokens[1], context);
    Ok(Instruction::PutStack {
        device,
        address: parse_value(tokens[2], context),
        value: parse_value(tokens[3], context),
    })
}

fn parse_load_or_store_device(
    mnemonic: &str,
    token: &str,
    context: ParseContext<'_>,
) -> DeviceOperand {
    if matches!(mnemonic, "ld" | "sd") {
        DeviceOperand::Reference(parse_value(token, context))
    } else {
        parse_device_operand(token, context)
    }
}

fn parse_get_or_put_device(
    mnemonic: &str,
    token: &str,
    context: ParseContext<'_>,
) -> DeviceOperand {
    if matches!(mnemonic, "getd" | "putd") {
        DeviceOperand::Reference(parse_value(token, context))
    } else {
        parse_device_operand(token, context)
    }
}

fn parse_clear_stack_device(
    mnemonic: &str,
    token: &str,
    context: ParseContext<'_>,
) -> DeviceOperand {
    if mnemonic == "clrd" {
        DeviceOperand::Reference(parse_value(token, context))
    } else {
        parse_device_operand(token, context)
    }
}

fn parse_jump_instruction(
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    require_len(tokens, 2, context.line_number)?;
    let (link, relative) = match tokens[0] {
        "j" => (false, false),
        "jal" => (true, false),
        "jr" => (false, true),
        mnemonic => return unsupported_instruction(mnemonic, context.line_number),
    };
    Ok(Instruction::Jump {
        target: parse_jump_target(tokens[1], context),
        link,
        relative,
    })
}

fn parse_math_or_branch_instruction(
    mnemonic: &str,
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Instruction, ParseError> {
    if let Some(instruction) = parse_math_instruction(mnemonic, tokens, context)? {
        return Ok(instruction);
    }
    if let Some(instruction) = parse_branch_instruction(mnemonic, tokens, context)? {
        return Ok(instruction);
    }
    unsupported_instruction(mnemonic, context.line_number)
}

fn parse_math_instruction(
    mnemonic: &str,
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Option<Instruction>, ParseError> {
    if let Some(operation) = unary_operation(mnemonic) {
        require_len(tokens, 3, context.line_number)?;
        return Ok(Some(Instruction::Unary {
            operation,
            destination: parse_register(tokens[1], context)?,
            source: parse_value(tokens[2], context),
        }));
    }

    if let Some(operation) = binary_operation(mnemonic) {
        require_len(tokens, 4, context.line_number)?;
        return Ok(Some(Instruction::Binary {
            operation,
            destination: parse_register(tokens[1], context)?,
            left: parse_value(tokens[2], context),
            right: parse_value(tokens[3], context),
        }));
    }

    if let Some(operation) = ternary_operation(mnemonic) {
        require_len(tokens, 5, context.line_number)?;
        return Ok(Some(Instruction::Ternary {
            operation,
            destination: parse_register(tokens[1], context)?,
            first: parse_value(tokens[2], context),
            second: parse_value(tokens[3], context),
            third: parse_value(tokens[4], context),
        }));
    }

    Ok(None)
}

fn parse_branch_instruction(
    mnemonic: &str,
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Option<Instruction>, ParseError> {
    if let Some(instruction) = parse_device_branch_instruction(mnemonic, tokens, context)? {
        return Ok(Some(instruction));
    }

    let Some((base, flags)) = branch_family(mnemonic) else {
        return Ok(None);
    };
    let Some(shape) = branch_shape(base) else {
        return Ok(None);
    };

    require_len(tokens, shape.expected_tokens(), context.line_number)?;
    Ok(Some(Instruction::Branch {
        condition: parse_branch_condition(shape, tokens, context),
        target: parse_jump_target(tokens[shape.target_index()], context),
        link: flags.link,
        relative: flags.relative,
    }))
}

fn parse_device_branch_instruction(
    mnemonic: &str,
    tokens: &[&str],
    context: ParseContext<'_>,
) -> Result<Option<Instruction>, ParseError> {
    if let Some((expected_set, flags)) = device_set_branch(mnemonic) {
        require_len(tokens, 3, context.line_number)?;
        return Ok(Some(Instruction::Branch {
            condition: BranchCondition::DeviceSet {
                device: parse_device_operand(tokens[1], context),
                expected_set,
            },
            target: parse_jump_target(tokens[2], context),
            link: flags.link,
            relative: flags.relative,
        }));
    }

    if let Some(operation) = device_valid_branch(mnemonic) {
        require_len(tokens, 4, context.line_number)?;
        return Ok(Some(Instruction::Branch {
            condition: BranchCondition::DeviceValid {
                operation,
                device: parse_device_operand(tokens[1], context),
                field: parse_logic_field(tokens[2], context),
                expected_valid: false,
            },
            target: parse_jump_target(tokens[3], context),
            link: false,
            relative: false,
        }));
    }

    Ok(None)
}

fn parse_branch_condition(
    shape: BranchShape,
    tokens: &[&str],
    context: ParseContext<'_>,
) -> BranchCondition {
    match shape {
        BranchShape::Compare(operation) => BranchCondition::Compare {
            operation,
            left: parse_value(tokens[1], context),
            right: parse_value(tokens[2], context),
        },
        BranchShape::CompareZero(operation) => BranchCondition::CompareZero {
            operation,
            value: parse_value(tokens[1], context),
        },
        BranchShape::Approx(operation) => BranchCondition::Approx {
            operation,
            left: parse_value(tokens[1], context),
            right: parse_value(tokens[2], context),
            tolerance: parse_value(tokens[3], context),
        },
        BranchShape::ApproxZero(operation) => BranchCondition::ApproxZero {
            operation,
            value: parse_value(tokens[1], context),
            tolerance: parse_value(tokens[2], context),
        },
        BranchShape::Nan => BranchCondition::Nan {
            value: parse_value(tokens[1], context),
        },
    }
}

fn device_set_branch(mnemonic: &str) -> Option<(bool, BranchFlags)> {
    match mnemonic {
        "bdns" => Some((
            false,
            BranchFlags {
                link: false,
                relative: false,
            },
        )),
        "bdnsal" => Some((
            false,
            BranchFlags {
                link: true,
                relative: false,
            },
        )),
        "brdns" => Some((
            false,
            BranchFlags {
                link: false,
                relative: true,
            },
        )),
        "bdse" => Some((
            true,
            BranchFlags {
                link: false,
                relative: false,
            },
        )),
        "bdseal" => Some((
            true,
            BranchFlags {
                link: true,
                relative: false,
            },
        )),
        "brdse" => Some((
            true,
            BranchFlags {
                link: false,
                relative: true,
            },
        )),
        _ => None,
    }
}

fn device_valid_branch(mnemonic: &str) -> Option<DeviceLogicOperation> {
    match mnemonic {
        "bdnvl" => Some(DeviceLogicOperation::Load),
        "bdnvs" => Some(DeviceLogicOperation::Store),
        _ => None,
    }
}

fn branch_family(mnemonic: &str) -> Option<(&str, BranchFlags)> {
    if let Some(stripped) = mnemonic.strip_prefix("br") {
        return Some((
            relative_branch_base(stripped)?,
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

fn relative_branch_base(stripped_relative: &str) -> Option<&str> {
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

fn branch_shape(base: &str) -> Option<BranchShape> {
    compare_operation(base)
        .map(BranchShape::Compare)
        .or_else(|| compare_zero_operation(base).map(BranchShape::CompareZero))
        .or_else(|| approximate_operation(base).map(BranchShape::Approx))
        .or_else(|| approximate_zero_operation(base).map(BranchShape::ApproxZero))
        .or_else(|| (base == "bnan").then_some(BranchShape::Nan))
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
        "ext" => Some(TernaryOperation::Ext),
        "ins" => Some(TernaryOperation::Ins),
        _ => None,
    }
}

fn parse_register(token: &str, context: ParseContext<'_>) -> Result<RegisterRef, ParseError> {
    if let Some(AliasTarget::Register(register)) = context.aliases.get(token).copied() {
        return Ok(register);
    }
    parse_register_token(token).ok_or_else(|| {
        ParseError::new(
            ParseErrorCode::ExpectedRegister,
            context.line_number,
            format!("expected register, found `{token}`"),
        )
    })
}

fn parse_value(token: &str, context: ParseContext<'_>) -> ValueOperand {
    match context.aliases.get(token).copied() {
        Some(AliasTarget::Register(register)) => ValueOperand::Register(register),
        Some(AliasTarget::Device(_)) | None => parse_register_token(token).map_or_else(
            || parse_non_register_value(token, context.constants),
            ValueOperand::Register,
        ),
    }
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

fn parse_logic_field(token: &str, context: ParseContext<'_>) -> LogicFieldOperand {
    if is_dynamic_logic_field(token, context) {
        LogicFieldOperand::Dynamic(parse_value(token, context))
    } else {
        LogicFieldOperand::Named(token.to_owned())
    }
}

fn parse_batch_mode(token: &str, context: ParseContext<'_>) -> BatchModeOperand {
    match token {
        "Average" => BatchModeOperand::Direct(BatchMode::Average),
        "Sum" => BatchModeOperand::Direct(BatchMode::Sum),
        "Minimum" => BatchModeOperand::Direct(BatchMode::Minimum),
        "Maximum" => BatchModeOperand::Direct(BatchMode::Maximum),
        _ => BatchModeOperand::Dynamic(parse_value(token, context)),
    }
}

fn is_dynamic_logic_field(token: &str, context: ParseContext<'_>) -> bool {
    matches!(context.aliases.get(token), Some(AliasTarget::Register(_)))
        || parse_register_token(token).is_some()
        || context.constants.contains_key(token)
        || parse_number(token).is_some()
}

fn parse_jump_target(token: &str, context: ParseContext<'_>) -> JumpTarget {
    match context.aliases.get(token).copied() {
        Some(AliasTarget::Register(register)) => JumpTarget::Register(register),
        Some(AliasTarget::Device(_)) | None => parse_register_token(token).map_or_else(
            || {
                parse_number(token)
                    .map_or_else(|| JumpTarget::Symbol(token.to_owned()), JumpTarget::Number)
            },
            JumpTarget::Register,
        ),
    }
}

fn parse_device_operand(token: &str, context: ParseContext<'_>) -> DeviceOperand {
    match context.aliases.get(token).copied() {
        Some(AliasTarget::Device(device)) => DeviceOperand::Port(device),
        Some(AliasTarget::Register(register)) => {
            DeviceOperand::Reference(ValueOperand::Register(register))
        }
        None => parse_device_port_token(token).map_or_else(
            || DeviceOperand::Reference(parse_value(token, context)),
            DeviceOperand::Port,
        ),
    }
}

fn parse_slot_device(
    token: &str,
    context: ParseContext<'_>,
) -> Result<DevicePortOperand, ParseError> {
    match context.aliases.get(token).copied() {
        Some(AliasTarget::Device(device)) => Ok(device),
        Some(AliasTarget::Register(_)) | None => parse_device_port_token(token).ok_or_else(|| {
            ParseError::new(
                ParseErrorCode::ExpectedDevicePin,
                context.line_number,
                format!("expected device pin, found `{token}`"),
            )
        }),
    }
}

fn parse_device_port_token(token: &str) -> Option<DevicePortOperand> {
    if token == "db" {
        return Some(DevicePortOperand::Direct(DevicePort::Db));
    }
    let pin = token
        .strip_prefix('d')
        .and_then(|digits| digits.parse::<u8>().ok())
        .and_then(DevicePort::from_pin_index);
    if let Some(pin) = pin {
        return Some(DevicePortOperand::Direct(pin));
    }
    token
        .strip_prefix('d')
        .and_then(parse_register_token)
        .map(DevicePortOperand::Indirect)
}

fn parse_register_token(token: &str) -> Option<RegisterRef> {
    if token == "ra" {
        return Some(RegisterRef::return_address());
    }
    if token == "sp" {
        return Some(RegisterRef::stack_pointer());
    }

    let r_count = token.bytes().take_while(|byte| *byte == b'r').count();
    if r_count == 0 {
        return None;
    }
    let digits = token.get(r_count..)?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    let parsed = digits.parse::<u8>().ok()?;
    let base = RegisterIndex::new(parsed)?;
    if r_count == 1 {
        Some(RegisterRef::direct(base))
    } else {
        let depth = u8::try_from(r_count - 1).ok()?;
        Some(RegisterRef::indirect(base, depth))
    }
}

fn parse_number(token: &str) -> Option<f64> {
    match token {
        "nan" => Some(f64::NAN),
        "pinf" => Some(f64::INFINITY),
        "ninf" => Some(f64::NEG_INFINITY),
        _ => parse_hash_literal(token)
            .or_else(|| parse_numeric_literal(token))
            .or_else(|| logic_types::value_from_symbol(token)),
    }
}

fn parse_hash_literal(token: &str) -> Option<f64> {
    let name = token.strip_prefix("HASH(\"")?.strip_suffix("\")")?;
    Some(f64::from(crc32(name.as_bytes())))
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xEDB8_8320_u32 & mask);
        }
    }
    !crc
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

fn unsupported_instruction<T>(mnemonic: &str, line_number: usize) -> Result<T, ParseError> {
    Err(ParseError::new(
        ParseErrorCode::UnsupportedInstruction,
        line_number,
        format!("unsupported instruction `{mnemonic}`"),
    ))
}
