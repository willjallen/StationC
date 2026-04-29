use stationc::sim::ic10::ErrorCode;

use super::support::{TestResult, assert_register, run, runtime_failure};

#[test]
fn not_zero_is_negative_one() -> TestResult {
    let output = run("\
not r0 0
yield
")?;

    assert_register(&output.vm, 0, -1.0)
}

#[test]
fn bitwise_and_or_xor_and_nor() -> TestResult {
    let output = run("\
and r0 6 3
or r1 4 1
xor r2 7 3
nor r3 0 0
yield
")?;

    assert_register(&output.vm, 0, 2.0)?;
    assert_register(&output.vm, 1, 5.0)?;
    assert_register(&output.vm, 2, 4.0)?;
    assert_register(&output.vm, 3, -1.0)
}

#[test]
fn left_shifts_are_supported_by_both_mnemonics() -> TestResult {
    let output = run("\
sla r0 3 2
sll r1 3 2
yield
")?;

    assert_register(&output.vm, 0, 12.0)?;
    assert_register(&output.vm, 1, 12.0)
}

#[test]
fn arithmetic_and_logical_right_shifts() -> TestResult {
    let output = run("\
sra r0 -8 1
srl r1 8 1
yield
")?;

    assert_register(&output.vm, 0, -4.0)?;
    assert_register(&output.vm, 1, 4.0)
}

#[test]
fn non_integer_bitwise_operand_faults() -> TestResult {
    runtime_failure(
        "\
and r0 1.5 1
yield
",
        128,
        ErrorCode::InvalidIntegerOperand,
    )?;

    Ok(())
}
