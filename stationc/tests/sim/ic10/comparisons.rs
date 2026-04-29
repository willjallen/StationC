use super::support::{TestResult, assert_register, run};

#[test]
fn binary_set_comparisons_write_zero_or_one() -> TestResult {
    let output = run("\
seq r0 4 4
sne r1 4 5
sge r2 5 5
sgt r3 6 5
sle r4 5 5
slt r5 4 5
yield
")?;

    assert_register(&output.ic10, 0, 1.0)?;
    assert_register(&output.ic10, 1, 1.0)?;
    assert_register(&output.ic10, 2, 1.0)?;
    assert_register(&output.ic10, 3, 1.0)?;
    assert_register(&output.ic10, 4, 1.0)?;
    assert_register(&output.ic10, 5, 1.0)
}

#[test]
fn zero_set_comparisons_write_zero_or_one() -> TestResult {
    let output = run("\
seqz r0 0
snez r1 2
sgez r2 0
sgtz r3 1
slez r4 0
sltz r5 -1
yield
")?;

    assert_register(&output.ic10, 0, 1.0)?;
    assert_register(&output.ic10, 1, 1.0)?;
    assert_register(&output.ic10, 2, 1.0)?;
    assert_register(&output.ic10, 3, 1.0)?;
    assert_register(&output.ic10, 4, 1.0)?;
    assert_register(&output.ic10, 5, 1.0)
}

#[test]
fn false_comparisons_write_zero() -> TestResult {
    let output = run("\
seq r0 4 5
sne r1 4 4
sge r2 4 5
sgt r3 5 5
sle r4 6 5
slt r5 5 5
yield
")?;

    assert_register(&output.ic10, 0, 0.0)?;
    assert_register(&output.ic10, 1, 0.0)?;
    assert_register(&output.ic10, 2, 0.0)?;
    assert_register(&output.ic10, 3, 0.0)?;
    assert_register(&output.ic10, 4, 0.0)?;
    assert_register(&output.ic10, 5, 0.0)
}

#[test]
fn select_uses_nonzero_condition() -> TestResult {
    let output = run("\
select r0 1 11 22
select r1 0 11 22
yield
")?;

    assert_register(&output.ic10, 0, 11.0)?;
    assert_register(&output.ic10, 1, 22.0)
}

#[test]
fn approximate_set_comparisons() -> TestResult {
    let output = run("\
sap r0 100 101 0.02
sna r1 100 110 0.02
sapz r2 0 0
snaz r3 2 0.5
yield
")?;

    assert_register(&output.ic10, 0, 1.0)?;
    assert_register(&output.ic10, 1, 1.0)?;
    assert_register(&output.ic10, 2, 1.0)?;
    assert_register(&output.ic10, 3, 1.0)
}

#[test]
fn nan_predicates_distinguish_nan_from_numbers() -> TestResult {
    let output = run("\
snan r0 nan
snanz r1 nan
snan r2 4
snanz r3 4
yield
")?;

    assert_register(&output.ic10, 0, 1.0)?;
    assert_register(&output.ic10, 1, 0.0)?;
    assert_register(&output.ic10, 2, 0.0)?;
    assert_register(&output.ic10, 3, 1.0)
}
