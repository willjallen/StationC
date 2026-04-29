use stationc::sim::ic10::StopReason;

use super::support::{
    TestResult, assert_pc, assert_ra, assert_register, assert_tick, run, run_with_budget,
};

#[test]
fn label_loop_runs_until_comparison_fails() -> TestResult {
    let output = run("\
move r0 0
start:
add r0 r0 1
blt r0 3 start
yield
")?;

    assert_register(&output.vm, 0, 3.0)?;
    assert_tick(output.tick, 8, StopReason::Yield)
}

#[test]
fn unconditional_jump_skips_instruction() -> TestResult {
    let output = run("\
move r0 1
j done
move r0 99
done:
yield
")?;

    assert_register(&output.vm, 0, 1.0)?;
    assert_pc(&output.vm, 4)
}

#[test]
fn jump_and_link_sets_return_address() -> TestResult {
    let output = run("\
jal subroutine
move r0 7
yield
subroutine:
move r1 ra
j ra
")?;

    assert_register(&output.vm, 0, 7.0)?;
    assert_register(&output.vm, 1, 1.0)?;
    assert_ra(&output.vm, 1.0)
}

#[test]
fn equality_and_inequality_branches() -> TestResult {
    let output = run("\
move r0 0
beq 3 3 equal
move r0 99
equal:
add r0 r0 1
bne 3 4 different
move r0 99
different:
add r0 r0 1
yield
")?;

    assert_register(&output.vm, 0, 2.0)
}

#[test]
fn ordering_branches_take_true_paths() -> TestResult {
    let output = run("\
move r0 0
bge 5 5 ge
move r0 99
ge:
add r0 r0 1
bgt 6 5 gt
move r0 99
gt:
add r0 r0 1
ble 5 5 le
move r0 99
le:
add r0 r0 1
blt 4 5 lt
move r0 99
lt:
add r0 r0 1
yield
")?;

    assert_register(&output.vm, 0, 4.0)
}

#[test]
fn zero_branches_take_true_paths() -> TestResult {
    let output = run("\
move r0 0
beqz 0 eqz
move r0 99
eqz:
add r0 r0 1
bnez 2 nez
move r0 99
nez:
add r0 r0 1
bgez 0 gez
move r0 99
gez:
add r0 r0 1
bgtz 1 gtz
move r0 99
gtz:
add r0 r0 1
blez 0 lez
move r0 99
lez:
add r0 r0 1
bltz -1 ltz
move r0 99
ltz:
add r0 r0 1
yield
")?;

    assert_register(&output.vm, 0, 6.0)
}

#[test]
fn relative_branch_uses_current_instruction_as_base() -> TestResult {
    let output = run("\
move r0 0
add r0 r0 1
brlt r0 3 -1
yield
")?;

    assert_register(&output.vm, 0, 3.0)
}

#[test]
fn relative_jump_can_skip_forward() -> TestResult {
    let output = run("\
move r0 1
jr 2
move r0 99
move r1 7
yield
")?;

    assert_register(&output.vm, 0, 1.0)?;
    assert_register(&output.vm, 1, 7.0)
}

#[test]
fn branch_and_link_records_return_address() -> TestResult {
    let output = run("\
beqal 1 1 linked
move r0 99
linked:
move r0 ra
yield
")?;

    assert_register(&output.vm, 0, 1.0)?;
    assert_ra(&output.vm, 1.0)
}

#[test]
fn nan_branch_takes_nan_path() -> TestResult {
    let output = run("\
move r0 0
bnan nan saw_nan
move r0 99
saw_nan:
move r0 1
yield
")?;

    assert_register(&output.vm, 0, 1.0)
}

#[test]
fn budget_exhaustion_reports_budget_stop() -> TestResult {
    let output = run_with_budget(
        "\
move r0 0
loop:
add r0 r0 1
j loop
",
        5,
    )?;

    assert_tick(output.tick, 5, StopReason::Budget)?;
    assert_register(&output.vm, 0, 2.0)
}
