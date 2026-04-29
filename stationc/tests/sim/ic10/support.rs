use std::{error::Error as StdError, io};

use stationc::sim::ic10::{
    Error as SimError, ErrorCode, StopReason, TickResult, TraceEvent, TracedTickResult, Vm,
};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn StdError>>;

#[derive(Debug)]
pub(super) struct RunOutput {
    pub(super) vm: Vm,
    pub(super) tick: TickResult,
}

pub(super) fn run(source: &str) -> TestResult<RunOutput> {
    run_with_budget(source, 128)
}

pub(super) fn run_with_budget(source: &str, budget: u32) -> TestResult<RunOutput> {
    let mut vm = Vm::from_source(source)?;
    let tick = vm.run_until_yield_or_budget(budget)?;
    Ok(RunOutput { vm, tick })
}

pub(super) fn run_ticks(
    source: &str,
    ticks: u32,
    budget: u32,
) -> TestResult<(Vm, Vec<TickResult>)> {
    let mut vm = Vm::from_source(source)?;
    let results = vm.run_ticks(ticks, budget)?;
    Ok((vm, results))
}

pub(super) fn run_traced(source: &str, budget: u32) -> TestResult<(Vm, TracedTickResult)> {
    let mut vm = Vm::from_source(source)?;
    let tick = vm.run_until_yield_or_budget_with_trace(budget)?;
    Ok((vm, tick))
}

pub(super) fn parse_failure(
    source: &str,
    expected_code: ErrorCode,
    expected_line: usize,
) -> TestResult {
    match Vm::from_source(source) {
        Ok(_) => Err(test_error("expected parse failure")),
        Err(error) => {
            assert_error_code(&error, expected_code)?;
            assert_error_line(&error, Some(expected_line))?;
            Ok(())
        }
    }
}

pub(super) fn runtime_failure(
    source: &str,
    budget: u32,
    expected_code: ErrorCode,
) -> TestResult<SimError> {
    let mut vm = Vm::from_source(source)?;
    match vm.run_until_yield_or_budget(budget) {
        Ok(tick) => Err(test_error(format!(
            "expected runtime failure, got {tick:?}"
        ))),
        Err(error) => {
            assert_error_code(&error, expected_code)?;
            assert_error_line(&error, None)?;
            Ok(error)
        }
    }
}

pub(super) fn assert_register(vm: &Vm, index: usize, expected: f64) -> TestResult {
    let actual = vm
        .register(index)
        .ok_or_else(|| test_error(format!("missing register r{index}")))?;
    assert_number(actual, expected, &format!("r{index}"))
}

pub(super) fn assert_ra(vm: &Vm, expected: f64) -> TestResult {
    assert_number(vm.return_address(), expected, "ra")
}

pub(super) fn assert_sp(vm: &Vm, expected: f64) -> TestResult {
    assert_number(vm.stack_pointer(), expected, "sp")
}

pub(super) fn assert_pc(vm: &Vm, expected: usize) -> TestResult {
    let actual = vm.program_counter();
    if actual == expected {
        Ok(())
    } else {
        Err(test_error(format!(
            "expected pc={expected}, got pc={actual}"
        )))
    }
}

pub(super) fn assert_stack(vm: &Vm, index: usize, expected: f64) -> TestResult {
    let actual = vm
        .stack_value(index)
        .ok_or_else(|| test_error(format!("missing stack[{index}]")))?;
    assert_number(actual, expected, &format!("stack[{index}]"))
}

pub(super) fn assert_tick(
    tick: TickResult,
    expected_instructions: u32,
    expected_stop: StopReason,
) -> TestResult {
    if tick.instructions_executed != expected_instructions {
        return Err(test_error(format!(
            "expected {} instruction(s), got {}",
            expected_instructions, tick.instructions_executed
        )));
    }
    if tick.stop != expected_stop {
        return Err(test_error(format!(
            "expected stop={expected_stop:?}, got stop={:?}",
            tick.stop
        )));
    }
    Ok(())
}

pub(super) fn assert_trace_event(
    tick: &TracedTickResult,
    index: usize,
    expected: &TraceEvent,
) -> TestResult {
    let actual = tick
        .trace
        .get(index)
        .ok_or_else(|| test_error(format!("missing trace event {index}")))?;
    if actual == expected {
        Ok(())
    } else {
        Err(test_error(format!(
            "expected trace event {expected:?}, got {actual:?}"
        )))
    }
}

pub(super) fn tick(results: &[TickResult], index: usize) -> TestResult<TickResult> {
    results
        .get(index)
        .copied()
        .ok_or_else(|| test_error(format!("missing tick result {index}")))
}

fn assert_error_code(error: &SimError, expected: ErrorCode) -> TestResult {
    let actual = error.code();
    if actual == expected {
        Ok(())
    } else {
        Err(test_error(format!(
            "expected error code {expected:?}, got {actual:?}"
        )))
    }
}

fn assert_error_line(error: &SimError, expected: Option<usize>) -> TestResult {
    let actual = error.line();
    if actual == expected {
        Ok(())
    } else {
        Err(test_error(format!(
            "expected error line {expected:?}, got {actual:?}"
        )))
    }
}

fn assert_number(actual: f64, expected: f64, label: &str) -> TestResult {
    if numbers_close(actual, expected) {
        Ok(())
    } else {
        Err(test_error(format!(
            "expected {label}={expected}, got {label}={actual}"
        )))
    }
}

fn numbers_close(actual: f64, expected: f64) -> bool {
    if actual.is_nan() && expected.is_nan() {
        return true;
    }
    if actual.is_infinite() || expected.is_infinite() {
        return actual.to_bits() == expected.to_bits();
    }
    let tolerance = f64::EPSILON * actual.abs().max(expected.abs()).max(1.0) * 8.0;
    (actual - expected).abs() <= tolerance
}

fn test_error(message: impl Into<String>) -> Box<dyn StdError> {
    Box::new(io::Error::other(message.into()))
}
