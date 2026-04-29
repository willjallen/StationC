use super::support::{TestResult, assert_register, run};

#[test]
fn moves_and_adds_registers() -> TestResult {
    let output = run("\
move r0 1
move r1 2
add r2 r0 r1
yield
")?;

    assert_register(&output.vm, 2, 3.0)
}

#[test]
fn define_and_alias_feed_arithmetic() -> TestResult {
    let output = run("\
alias total r0
define increment 7
move total 5
add total total increment
yield
")?;

    assert_register(&output.vm, 0, 12.0)
}

#[test]
fn subtraction_multiplication_and_division() -> TestResult {
    let output = run("\
sub r0 9 4
mul r1 r0 3
div r2 r1 5
yield
")?;

    assert_register(&output.vm, 0, 5.0)?;
    assert_register(&output.vm, 1, 15.0)?;
    assert_register(&output.vm, 2, 3.0)
}

#[test]
fn modulo_uses_euclidean_remainder() -> TestResult {
    let output = run("\
mod r0 -7 4
mod r1 22 -20
yield
")?;

    assert_register(&output.vm, 0, 1.0)?;
    assert_register(&output.vm, 1, 2.0)
}

#[test]
fn min_max_abs_and_sqrt() -> TestResult {
    let output = run("\
min r0 8 3
max r1 8 3
abs r2 -12
sqrt r3 81
yield
")?;

    assert_register(&output.vm, 0, 3.0)?;
    assert_register(&output.vm, 1, 8.0)?;
    assert_register(&output.vm, 2, 12.0)?;
    assert_register(&output.vm, 3, 9.0)
}

#[test]
fn rounding_family_is_exercised() -> TestResult {
    let output = run("\
floor r0 2.9
ceil r1 2.1
trunc r2 -2.9
round r3 2.5
yield
")?;

    assert_register(&output.vm, 0, 2.0)?;
    assert_register(&output.vm, 1, 3.0)?;
    assert_register(&output.vm, 2, -2.0)?;
    assert_register(&output.vm, 3, 3.0)
}

#[test]
fn exponential_logarithm_and_power_identity_cases() -> TestResult {
    let output = run("\
exp r0 0
log r1 1
pow r2 2 8
yield
")?;

    assert_register(&output.vm, 0, 1.0)?;
    assert_register(&output.vm, 1, 0.0)?;
    assert_register(&output.vm, 2, 256.0)
}

#[test]
fn trigonometric_identity_cases() -> TestResult {
    let output = run("\
sin r0 0
cos r1 0
tan r2 0
atan r3 0
atan2 r4 0 1
yield
")?;

    assert_register(&output.vm, 0, 0.0)?;
    assert_register(&output.vm, 1, 1.0)?;
    assert_register(&output.vm, 2, 0.0)?;
    assert_register(&output.vm, 3, 0.0)?;
    assert_register(&output.vm, 4, 0.0)
}

#[test]
fn lerp_clamps_ratio_to_unit_interval() -> TestResult {
    let output = run("\
lerp r0 10 20 0.25
lerp r1 10 20 -1
lerp r2 10 20 2
yield
")?;

    assert_register(&output.vm, 0, 12.5)?;
    assert_register(&output.vm, 1, 10.0)?;
    assert_register(&output.vm, 2, 20.0)
}
