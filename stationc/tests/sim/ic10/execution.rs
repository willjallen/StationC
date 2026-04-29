use stationc::sim::ic10::{ErrorCode, StopReason, TraceEvent};

use super::support::{
    TestResult, assert_pc, assert_register, assert_tick, assert_trace_event, run, run_ticks,
    run_traced, runtime_failure, tick,
};

#[test]
fn yield_stops_current_tick() -> TestResult {
    let output = run("\
move r0 1
yield
move r0 99
")?;

    assert_tick(output.tick, 2, StopReason::Yield)?;
    assert_pc(&output.vm, 2)?;
    assert_register(&output.vm, 0, 1.0)
}

#[test]
fn second_tick_continues_after_yield() -> TestResult {
    let (vm, results) = run_ticks(
        "\
move r0 1
yield
move r0 2
yield
",
        2,
        128,
    )?;

    assert_tick(tick(&results, 0)?, 2, StopReason::Yield)?;
    assert_tick(tick(&results, 1)?, 2, StopReason::Yield)?;
    assert_pc(&vm, 4)?;
    assert_register(&vm, 0, 2.0)
}

#[test]
fn program_end_reports_halt() -> TestResult {
    let (vm, results) = run_ticks(
        "\
move r0 1
",
        2,
        128,
    )?;

    assert_tick(tick(&results, 0)?, 1, StopReason::Halt)?;
    assert_pc(&vm, 1)?;
    assert_register(&vm, 0, 1.0)
}

#[test]
fn hcf_is_runtime_fault() -> TestResult {
    runtime_failure(
        "\
move r0 1
hcf
",
        128,
        ErrorCode::HaltAndCatchFire,
    )?;

    Ok(())
}

#[test]
fn trace_includes_executed_source_lines() -> TestResult {
    let (_vm, tick) = run_traced(
        "\
move r0 1
move r1 2
add r2 r0 r1
yield
",
        128,
    )?;

    assert_trace_event(
        &tick,
        0,
        &TraceEvent {
            program_counter: 0,
            source_line: 1,
            instruction: "move r0 1".to_owned(),
        },
    )?;
    assert_trace_event(
        &tick,
        2,
        &TraceEvent {
            program_counter: 2,
            source_line: 3,
            instruction: "add r2 r0 r1".to_owned(),
        },
    )
}

#[test]
fn indirect_register_write_uses_base_register_as_index() -> TestResult {
    let output = run("\
move r0 3
move rr0 99
yield
")?;

    assert_register(&output.vm, 3, 99.0)
}

#[test]
fn indirect_register_read_uses_base_register_as_index() -> TestResult {
    let output = run("\
move r0 4
move r4 88
move r1 rr0
yield
")?;

    assert_register(&output.vm, 1, 88.0)
}

#[test]
fn invalid_indirect_register_index_faults() -> TestResult {
    runtime_failure(
        "\
move r0 16
move r1 rr0
yield
",
        128,
        ErrorCode::InvalidIndirectRegisterIndex,
    )?;

    Ok(())
}

#[test]
fn random_instruction_is_deterministic_for_fresh_vm() -> TestResult {
    let first = run("\
rand r0
yield
")?;
    let second = run("\
rand r0
yield
")?;

    assert_register(&first.vm, 0, 0.996_522_255_776_848_2)?;
    assert_register(&second.vm, 0, 0.996_522_255_776_848_2)
}
