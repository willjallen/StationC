use stationc::sim::ic10::ErrorCode;

use super::support::{TestResult, assert_register, run, runtime_failure};

#[test]
fn not_zero_is_negative_one() -> TestResult {
    let output = run("\
not r0 0
yield
")?;

    assert_register(&output.ic10, 0, -1.0)
}

#[test]
fn not_one_is_negative_two() -> TestResult {
    let output = run("\
not r0 1
yield
")?;

    assert_register(&output.ic10, 0, -2.0)
}

#[test]
fn binary_literals_allow_underscore_separators() -> TestResult {
    let output = run("\
move r0 %0000_1111
move r1 %1010_0101
and r2 r0 r1
yield
")?;

    assert_register(&output.ic10, 0, 15.0)?;
    assert_register(&output.ic10, 1, 165.0)?;
    assert_register(&output.ic10, 2, 5.0)
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

    assert_register(&output.ic10, 0, 2.0)?;
    assert_register(&output.ic10, 1, 5.0)?;
    assert_register(&output.ic10, 2, 4.0)?;
    assert_register(&output.ic10, 3, -1.0)
}

#[test]
fn left_shifts_are_supported_by_both_mnemonics() -> TestResult {
    let output = run("\
sla r0 3 2
sll r1 3 2
yield
")?;

    assert_register(&output.ic10, 0, 12.0)?;
    assert_register(&output.ic10, 1, 12.0)
}

#[test]
fn arithmetic_and_logical_right_shifts() -> TestResult {
    let output = run("\
sra r0 -8 1
srl r1 8 1
yield
")?;

    assert_register(&output.ic10, 0, -4.0)?;
    assert_register(&output.ic10, 1, 4.0)
}

#[test]
fn ext_extracts_right_aligned_bit_field() -> TestResult {
    let output = run("\
ext r0 $DEADBEEF 8 16
ext r1 -1 60 4
yield
")?;

    assert_register(&output.ic10, 0, f64::from(0xADBE_u32))?;
    assert_register(&output.ic10, 1, 15.0)
}

#[test]
fn ins_inserts_low_field_bits_into_destination_register() -> TestResult {
    let output = run("\
move r0 $DE0000EF
move r1 $ADBE
ins r0 r1 8 16
move r2 0
ins r2 -1 4 4
yield
")?;

    assert_register(&output.ic10, 0, f64::from(0xDEAD_BEEF_u32))?;
    assert_register(&output.ic10, 2, 0xF0 as f64)
}

#[test]
fn zero_length_bit_fields_are_noops() -> TestResult {
    let output = run("\
ext r0 255 4 0
move r1 123
ins r1 255 4 0
yield
")?;

    assert_register(&output.ic10, 0, 0.0)?;
    assert_register(&output.ic10, 1, 123.0)
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

#[test]
fn invalid_bit_field_range_faults() -> TestResult {
    runtime_failure(
        "\
ext r0 1 0 54
yield
",
        128,
        ErrorCode::InvalidBitFieldRange,
    )?;

    runtime_failure(
        "\
ins r0 1 63 2
yield
",
        128,
        ErrorCode::InvalidBitFieldRange,
    )?;

    Ok(())
}

#[test]
fn negative_shift_operand_faults() -> TestResult {
    runtime_failure(
        "\
sll r0 1 -1
yield
",
        128,
        ErrorCode::InvalidShiftOperand,
    )?;

    Ok(())
}

#[test]
fn oversized_unsigned_shift_result_faults() -> TestResult {
    runtime_failure(
        "\
srl r0 -1 1
yield
",
        128,
        ErrorCode::UnsignedIntegerNotExactlyRepresentable,
    )?;

    Ok(())
}
