use stationc::sim::ic10::ErrorCode;

use super::support::{TestResult, assert_register, assert_sp, assert_stack, run, runtime_failure};

#[test]
fn push_and_pop_are_lifo_and_update_sp() -> TestResult {
    let output = run("\
push 11
push 22
pop r0
pop r1
yield
")?;

    assert_register(&output.ic10, 0, 22.0)?;
    assert_register(&output.ic10, 1, 11.0)?;
    assert_sp(&output.ic10, 0.0)
}

#[test]
fn peek_reads_top_without_decrementing_sp() -> TestResult {
    let output = run("\
push 33
peek r0
peek r1
yield
")?;

    assert_register(&output.ic10, 0, 33.0)?;
    assert_register(&output.ic10, 1, 33.0)?;
    assert_sp(&output.ic10, 1.0)
}

#[test]
fn stack_pointer_at_upper_read_bound_can_peek_and_pop() -> TestResult {
    let output = run("\
poke 511 44
move sp 512
peek r0
pop r1
yield
")?;

    assert_register(&output.ic10, 0, 44.0)?;
    assert_register(&output.ic10, 1, 44.0)?;
    assert_sp(&output.ic10, 511.0)
}

#[test]
fn poke_writes_absolute_stack_address() -> TestResult {
    let output = run("\
poke 0 77
move sp 1
peek r0
yield
")?;

    assert_register(&output.ic10, 0, 77.0)?;
    assert_stack(&output.ic10, 0, 77.0)?;
    assert_sp(&output.ic10, 1.0)
}

#[test]
fn pop_empty_stack_faults() -> TestResult {
    runtime_failure(
        "\
pop r0
yield
",
        128,
        ErrorCode::StackAddressOutOfRange,
    )?;

    Ok(())
}

#[test]
fn peek_empty_stack_faults() -> TestResult {
    runtime_failure(
        "\
peek r0
yield
",
        128,
        ErrorCode::StackAddressOutOfRange,
    )?;

    Ok(())
}

#[test]
fn push_past_stack_limit_faults() -> TestResult {
    let pushes = "push 1\n".repeat(513);
    let source = format!("{pushes}yield\n");
    runtime_failure(&source, 600, ErrorCode::StackAddressOutOfRange)?;

    Ok(())
}

#[test]
fn push_with_stack_pointer_at_limit_faults() -> TestResult {
    runtime_failure(
        "\
move sp 512
push 1
yield
",
        128,
        ErrorCode::StackAddressOutOfRange,
    )?;

    Ok(())
}

#[test]
fn poke_rejects_fractional_address() -> TestResult {
    runtime_failure(
        "\
poke 1.5 9
yield
",
        128,
        ErrorCode::StackAddressOutOfRange,
    )?;

    Ok(())
}
