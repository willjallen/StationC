//! IC10 simulator support.

mod environment;
#[allow(clippy::module_inception)]
mod ic10;
mod instruction;
mod logic_types;
mod parser;
mod program;
mod registers;
mod stack;
mod trace;

use std::{env, error::Error as StdError, fmt, fs, path::PathBuf, process::ExitCode};

pub use environment::{
    BatchMode, DevicePort, DeviceTarget, EnvironmentFault, EnvironmentOperation, Ic10Environment,
    NoEnvironment, ReferenceId,
};
use ic10::{Ic10 as CoreIc10, RunStop};
use parser::parse_program;
use registers::REGISTER_COUNT as INTERNAL_REGISTER_COUNT;
use stack::STACK_SIZE as INTERNAL_STACK_SIZE;
pub use trace::TraceEvent;
use trace::TraceSink;

const DEFAULT_TICKS: u32 = 1;
const DEFAULT_BUDGET: u32 = 128;

/// Number of internal IC10 registers, `r0` through `r15`.
pub const REGISTER_COUNT: usize = INTERNAL_REGISTER_COUNT;

/// Number of values in IC10 stack memory.
pub const STACK_SIZE: usize = INTERNAL_STACK_SIZE;

/// A standalone IC10 simulator with parsed program and mutable state.
#[derive(Debug)]
pub struct Ic10 {
    inner: CoreIc10,
}

impl Ic10 {
    /// Parses IC10 source and creates a fresh simulator.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when the IC10 source cannot be parsed.
    pub fn from_source(source: &str) -> Result<Self, Error> {
        let program = parse_program(source)?;
        Ok(Self {
            inner: CoreIc10::new(program),
        })
    }

    /// Runs one IC10 tick until `yield`, halt, or the instruction budget is reached.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when execution encounters a runtime fault.
    pub fn run_until_yield_or_budget(&mut self, budget: u32) -> Result<TickResult, Error> {
        let mut trace_sink = TraceSink::disabled();
        let result = self
            .inner
            .run_until_yield_or_budget(budget, &mut trace_sink)?;
        Ok(result.into())
    }

    /// Runs one IC10 tick with a caller-provided world environment.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when execution encounters a runtime fault.
    pub fn run_until_yield_or_budget_with_environment<E: Ic10Environment>(
        &mut self,
        budget: u32,
        environment: &mut E,
    ) -> Result<TickResult, Error> {
        let mut trace_sink = TraceSink::disabled();
        let result = self.inner.run_until_yield_or_budget_with_environment(
            budget,
            &mut trace_sink,
            environment,
        )?;
        Ok(result.into())
    }

    /// Runs one IC10 tick and returns structured trace events for executed instructions.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when execution encounters a runtime fault.
    pub fn run_until_yield_or_budget_with_trace(
        &mut self,
        budget: u32,
    ) -> Result<TracedTickResult, Error> {
        let mut trace_sink = TraceSink::capture();
        let result = self
            .inner
            .run_until_yield_or_budget(budget, &mut trace_sink)?;
        Ok(TracedTickResult {
            tick: result.into(),
            trace: trace_sink.into_events(),
        })
    }

    /// Runs multiple ticks and stops early if the program halts.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] when any tick encounters a runtime fault.
    pub fn run_ticks(&mut self, ticks: u32, budget: u32) -> Result<Vec<TickResult>, Error> {
        let mut results = Vec::new();
        for _ in 0..ticks {
            let result = self.run_until_yield_or_budget(budget)?;
            let stop = result.stop;
            results.push(result);
            if stop == StopReason::Halt {
                break;
            }
        }
        Ok(results)
    }

    /// Returns the current program counter.
    #[must_use]
    pub const fn program_counter(&self) -> usize {
        self.inner.program_counter()
    }

    /// Returns the value of direct register `r0` through `r15`.
    #[must_use]
    pub fn register(&self, index: usize) -> Option<f64> {
        self.inner.registers().direct_values().get(index).copied()
    }

    /// Returns the current return-address register value.
    #[must_use]
    pub const fn return_address(&self) -> f64 {
        self.inner.registers().return_address()
    }

    /// Returns the current stack-pointer register value.
    #[must_use]
    pub const fn stack_pointer(&self) -> f64 {
        self.inner.registers().stack_pointer()
    }

    /// Returns the stack value at an absolute stack address.
    #[must_use]
    pub fn stack_value(&self, index: usize) -> Option<f64> {
        self.inner.stack().values().get(index).copied()
    }

    /// Returns a copy of the current IC10 state.
    #[must_use]
    pub const fn snapshot(&self) -> Snapshot {
        Snapshot {
            program_counter: self.program_counter(),
            registers: self.inner.registers().direct_values(),
            return_address: self.return_address(),
            stack_pointer: self.stack_pointer(),
            stack: *self.inner.stack().values(),
        }
    }
}

/// A copy of observable IC10 state.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    /// Current program counter.
    pub program_counter: usize,
    /// Direct internal registers `r0` through `r15`.
    pub registers: [f64; REGISTER_COUNT],
    /// Return-address register `ra`.
    pub return_address: f64,
    /// Stack-pointer register `sp`.
    pub stack_pointer: f64,
    /// Complete stack memory.
    pub stack: [f64; STACK_SIZE],
}

/// Result of running one IC10 tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickResult {
    /// Number of instructions executed during the tick.
    pub instructions_executed: u32,
    /// Reason the tick stopped.
    pub stop: StopReason,
}

impl From<ic10::RunResult> for TickResult {
    fn from(value: ic10::RunResult) -> Self {
        Self {
            instructions_executed: value.instructions_executed,
            stop: value.stop.into(),
        }
    }
}

/// Result of running one IC10 tick with structured trace capture enabled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TracedTickResult {
    /// Tick execution result.
    pub tick: TickResult,
    /// One event for each executed instruction.
    pub trace: Vec<TraceEvent>,
}

/// Reason an IC10 tick stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// The program executed `yield`.
    Yield,
    /// The world environment suspended execution, such as when an IC housing is turned off.
    Disabled,
    /// The instruction budget was exhausted.
    Budget,
    /// The program counter reached the end of the program.
    Halt,
}

impl From<RunStop> for StopReason {
    fn from(value: RunStop) -> Self {
        match value {
            RunStop::Yielded => Self::Yield,
            RunStop::Disabled => Self::Disabled,
            RunStop::BudgetExhausted => Self::Budget,
            RunStop::Halted => Self::Halt,
        }
    }
}

/// Stable category for an IC10 parse or runtime error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// A label was declared more than once.
    DuplicateLabel,
    /// An instruction or directive received the wrong number of tokens.
    WrongArity,
    /// An `alias` directive targeted something other than a register.
    AliasTargetMustBeRegister,
    /// An `alias` directive targeted something other than a register or device pin.
    AliasTargetMustBeRegisterOrDevice,
    /// A `define` directive used a non-numeric value.
    DefineValueMustBeNumeric,
    /// The parser does not support the instruction mnemonic.
    UnsupportedInstruction,
    /// An operand that must be a register was not a register.
    ExpectedRegister,
    /// A symbol was referenced but not defined.
    UnknownSymbol,
    /// A numeric logic type did not resolve to a known logic field.
    UnknownLogicType,
    /// A jump target resolved outside the valid program range.
    InvalidJumpTarget,
    /// The program counter exceeded the representable range.
    ProgramCounterTooLarge,
    /// A numeric value could not be used as an instruction or stack index.
    InvalidNumericIndex,
    /// A numeric value could not be used as a direct `ReferenceId`.
    InvalidReferenceId,
    /// A numeric value could not be used as an indirect device pin.
    InvalidDevicePortIndex,
    /// A numeric value could not be used as a batch mode.
    InvalidBatchMode,
    /// An operand expected to be an integer was not an integer.
    InvalidIntegerOperand,
    /// A shift count was invalid.
    InvalidShiftOperand,
    /// A relative jump offset could not be represented.
    RelativeJumpOutOfRange,
    /// A signed integer result could not be exactly represented as an IC10 number.
    IntegerNotExactlyRepresentable,
    /// An unsigned integer result could not be exactly represented as an IC10 number.
    UnsignedIntegerNotExactlyRepresentable,
    /// An indirect register reference resolved to an invalid register index.
    InvalidIndirectRegisterIndex,
    /// A stack operation addressed outside stack memory.
    StackAddressOutOfRange,
    /// A world-facing instruction ran without world context.
    WorldContextRequired,
    /// A device pin was not bound in the current world context.
    DevicePortUnbound,
    /// A direct `ReferenceId` did not resolve in the current world context.
    UnknownReferenceId,
    /// A device did not expose the requested logic field.
    UnknownLogicField,
    /// A device logic field can be read but not written.
    ReadOnlyLogicField,
    /// A device stack operation addressed outside device stack memory.
    DeviceStackAddressOutOfRange,
    /// The program executed `hcf`.
    HaltAndCatchFire,
}

/// Parse or runtime error from the IC10 simulator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    code: ErrorCode,
    line: Option<usize>,
    message: String,
}

impl Error {
    /// Returns the stable error category.
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        self.code
    }

    /// Returns the source line for parse errors when one is available.
    #[must_use]
    pub const fn line(&self) -> Option<usize> {
        self.line
    }

    /// Returns the human-readable diagnostic message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl StdError for Error {}

impl From<parser::ParseError> for Error {
    fn from(value: parser::ParseError) -> Self {
        Self {
            code: parse_error_code(value.code()),
            line: Some(value.line()),
            message: value.message().to_owned(),
        }
    }
}

const fn parse_error_code(error: parser::ParseErrorCode) -> ErrorCode {
    match error {
        parser::ParseErrorCode::DuplicateLabel => ErrorCode::DuplicateLabel,
        parser::ParseErrorCode::WrongArity => ErrorCode::WrongArity,
        parser::ParseErrorCode::AliasTargetMustBeRegisterOrDevice => {
            ErrorCode::AliasTargetMustBeRegisterOrDevice
        }
        parser::ParseErrorCode::DefineValueMustBeNumeric => ErrorCode::DefineValueMustBeNumeric,
        parser::ParseErrorCode::UnsupportedInstruction => ErrorCode::UnsupportedInstruction,
        parser::ParseErrorCode::ExpectedRegister => ErrorCode::ExpectedRegister,
    }
}

impl From<ic10::Ic10Fault> for Error {
    fn from(value: ic10::Ic10Fault) -> Self {
        let code = ic10_fault_code(&value);
        Self {
            code,
            line: None,
            message: value.to_string(),
        }
    }
}

const fn ic10_fault_code(error: &ic10::Ic10Fault) -> ErrorCode {
    match error {
        ic10::Ic10Fault::UnknownSymbol(_) => ErrorCode::UnknownSymbol,
        ic10::Ic10Fault::UnknownLogicType(_) => ErrorCode::UnknownLogicType,
        ic10::Ic10Fault::InvalidJumpTarget(_) => ErrorCode::InvalidJumpTarget,
        ic10::Ic10Fault::ProgramCounterTooLarge(_) => ErrorCode::ProgramCounterTooLarge,
        ic10::Ic10Fault::InvalidNumericIndex(_) => ErrorCode::InvalidNumericIndex,
        ic10::Ic10Fault::InvalidReferenceId(_) => ErrorCode::InvalidReferenceId,
        ic10::Ic10Fault::InvalidDevicePortIndex(_) => ErrorCode::InvalidDevicePortIndex,
        ic10::Ic10Fault::InvalidBatchMode(_) => ErrorCode::InvalidBatchMode,
        ic10::Ic10Fault::InvalidIntegerOperand(_) => ErrorCode::InvalidIntegerOperand,
        ic10::Ic10Fault::InvalidShiftOperand(_) => ErrorCode::InvalidShiftOperand,
        ic10::Ic10Fault::RelativeJumpOutOfRange(_) => ErrorCode::RelativeJumpOutOfRange,
        ic10::Ic10Fault::IntegerNotExactlyRepresentable(_) => {
            ErrorCode::IntegerNotExactlyRepresentable
        }
        ic10::Ic10Fault::UnsignedIntegerNotExactlyRepresentable(_) => {
            ErrorCode::UnsignedIntegerNotExactlyRepresentable
        }
        ic10::Ic10Fault::Register(registers::RegisterFault::InvalidIndirectIndex(_)) => {
            ErrorCode::InvalidIndirectRegisterIndex
        }
        ic10::Ic10Fault::Stack(stack::StackFault::AddressOutOfRange { .. }) => {
            ErrorCode::StackAddressOutOfRange
        }
        ic10::Ic10Fault::Environment(environment::EnvironmentFault::WorldContextRequired {
            ..
        }) => ErrorCode::WorldContextRequired,
        ic10::Ic10Fault::Environment(environment::EnvironmentFault::DevicePortUnbound {
            ..
        }) => ErrorCode::DevicePortUnbound,
        ic10::Ic10Fault::Environment(environment::EnvironmentFault::UnknownReferenceId {
            ..
        }) => ErrorCode::UnknownReferenceId,
        ic10::Ic10Fault::Environment(environment::EnvironmentFault::UnknownLogicField {
            ..
        }) => ErrorCode::UnknownLogicField,
        ic10::Ic10Fault::Environment(environment::EnvironmentFault::ReadOnlyLogicField {
            ..
        }) => ErrorCode::ReadOnlyLogicField,
        ic10::Ic10Fault::Environment(environment::EnvironmentFault::StackAddressOutOfRange {
            ..
        }) => ErrorCode::DeviceStackAddressOutOfRange,
        ic10::Ic10Fault::HaltAndCatchFire { .. } => ErrorCode::HaltAndCatchFire,
    }
}

#[derive(Debug)]
enum SimError {
    Cli(String),
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse(parser::ParseError),
    Runtime(ic10::Ic10Fault),
}

impl fmt::Display for SimError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cli(message) => formatter.write_str(message),
            Self::Io { path, source } => {
                write!(formatter, "failed to read {}: {source}", path.display())
            }
            Self::Parse(error) => write!(formatter, "{error}"),
            Self::Runtime(error) => write!(formatter, "{error}"),
        }
    }
}

impl From<parser::ParseError> for SimError {
    fn from(value: parser::ParseError) -> Self {
        Self::Parse(value)
    }
}

impl From<ic10::Ic10Fault> for SimError {
    fn from(value: ic10::Ic10Fault) -> Self {
        Self::Runtime(value)
    }
}

#[derive(Debug)]
struct Command {
    source_path: PathBuf,
    ticks: u32,
    budget: u32,
    trace: bool,
}

pub(super) fn run() -> ExitCode {
    match run_from_env() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn run_from_env() -> Result<(), SimError> {
    let command = parse_args(env::args().skip(1))?;
    let source = fs::read_to_string(&command.source_path).map_err(|source| SimError::Io {
        path: command.source_path.clone(),
        source,
    })?;
    let program = parse_program(&source)?;
    let mut ic10 = CoreIc10::new(program);
    let mut trace_sink = TraceSink::stdout(command.trace);

    for tick in 0..command.ticks {
        let result = ic10.run_until_yield_or_budget(command.budget, &mut trace_sink)?;
        if command.trace {
            println!(
                "tick {tick}: executed {} instruction(s), pc={}, stop={}",
                result.instructions_executed,
                ic10.program_counter(),
                result.stop
            );
        }
        if result.stop == RunStop::Halted {
            break;
        }
    }

    if command.trace {
        println!("{}", ic10.registers());
    }

    Ok(())
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Command, SimError> {
    match (args.next().as_deref(), args.next().as_deref()) {
        (Some("sim"), Some("ic10")) => parse_ic10_args(args),
        _ => Err(SimError::Cli(
            "expected `stationc sim ic10 <path>`".to_owned(),
        )),
    }
}

fn parse_ic10_args(mut args: impl Iterator<Item = String>) -> Result<Command, SimError> {
    let Some(source_path) = args.next() else {
        return Err(SimError::Cli("missing IC10 source path".to_owned()));
    };

    let mut command = Command {
        source_path: PathBuf::from(source_path),
        ticks: DEFAULT_TICKS,
        budget: DEFAULT_BUDGET,
        trace: false,
    };

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--trace" => command.trace = true,
            "--ticks" => {
                let Some(value) = args.next() else {
                    return Err(SimError::Cli("missing value after --ticks".to_owned()));
                };
                command.ticks = parse_u32_flag("--ticks", &value)?;
            }
            "--budget" => {
                let Some(value) = args.next() else {
                    return Err(SimError::Cli("missing value after --budget".to_owned()));
                };
                command.budget = parse_u32_flag("--budget", &value)?;
            }
            unknown => {
                return Err(SimError::Cli(format!("unknown argument `{unknown}`")));
            }
        }
    }

    Ok(command)
}

fn parse_u32_flag(name: &str, value: &str) -> Result<u32, SimError> {
    value
        .parse::<u32>()
        .map_err(|error| SimError::Cli(format!("invalid {name} value `{value}`: {error}")))
}

const USAGE: &str = "\
usage:
  stationc sim ic10 <path> [--ticks N] [--budget N] [--trace]";
