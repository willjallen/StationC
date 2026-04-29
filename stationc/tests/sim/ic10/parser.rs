use stationc::sim::ic10::{ErrorCode, TraceEvent};

use super::support::{
    TestResult, assert_register, assert_trace_event, parse_failure, run, run_traced,
    runtime_failure,
};

#[test]
fn comments_blank_lines_and_inline_labels_are_accepted() -> TestResult {
    let output = run("\
# leading comment

start: move r0 1 # inline comment
add r0 r0 2
yield
")?;

    assert_register(&output.ic10, 0, 3.0)
}

#[test]
fn multiple_labels_can_target_one_instruction() -> TestResult {
    let output = run("\
entry: start: move r0 4
j done
move r0 99
done:
yield
")?;

    assert_register(&output.ic10, 0, 4.0)
}

#[test]
fn numeric_literals_include_hex_binary_and_special_values() -> TestResult {
    let output = run("\
move r0 $10
move r1 %1010
move r2 pinf
move r3 ninf
snan r4 nan
yield
")?;

    assert_register(&output.ic10, 0, 16.0)?;
    assert_register(&output.ic10, 1, 10.0)?;
    assert_register(&output.ic10, 2, f64::INFINITY)?;
    assert_register(&output.ic10, 3, f64::NEG_INFINITY)?;
    assert_register(&output.ic10, 4, 1.0)
}

#[test]
fn label_symbols_can_be_moved_as_numeric_values() -> TestResult {
    let output = run("\
move r0 done
j done
move r0 99
done:
yield
")?;

    assert_register(&output.ic10, 0, 3.0)
}

#[test]
fn trace_preserves_source_line_numbers() -> TestResult {
    let (_vm, tick) = run_traced("\n\nmove r0 1\nyield\n", 128)?;

    assert_trace_event(
        &tick,
        0,
        &TraceEvent {
            program_counter: 0,
            source_line: 3,
            instruction: "move r0 1".to_owned(),
        },
    )
}

#[test]
fn duplicate_label_is_parse_error() -> TestResult {
    parse_failure(
        "\
loop:
move r0 1
loop:
yield
",
        ErrorCode::DuplicateLabel,
        3,
    )
}

#[test]
fn wrong_arity_is_parse_error() -> TestResult {
    parse_failure(
        "\
add r0 1
yield
",
        ErrorCode::WrongArity,
        1,
    )
}

#[test]
fn unknown_instruction_is_parse_error() -> TestResult {
    parse_failure(
        "\
frobnicate r0 1
yield
",
        ErrorCode::UnsupportedInstruction,
        1,
    )
}

#[test]
fn bad_register_alias_is_parse_error() -> TestResult {
    parse_failure(
        "\
alias total 42
yield
",
        ErrorCode::AliasTargetMustBeRegisterOrDevice,
        1,
    )
}

#[test]
fn define_requires_numeric_value() -> TestResult {
    parse_failure(
        "\
define answer nope
yield
",
        ErrorCode::DefineValueMustBeNumeric,
        1,
    )
}

#[test]
fn invalid_destination_register_is_parse_error() -> TestResult {
    parse_failure(
        "\
move nope 1
yield
",
        ErrorCode::ExpectedRegister,
        1,
    )
}

#[test]
fn invalid_register_number_is_parse_error() -> TestResult {
    parse_failure(
        "\
move r16 1
yield
",
        ErrorCode::ExpectedRegister,
        1,
    )
}

#[test]
fn unknown_symbol_is_runtime_error() -> TestResult {
    runtime_failure(
        "\
move r0 missing_symbol
yield
",
        128,
        ErrorCode::UnknownSymbol,
    )?;

    Ok(())
}
