use std::{error::Error as StdError, io};

use stationc::sim::{
    ic10::{ReferenceId, StopReason, TickResult},
    world::{IcHousing, World},
};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn StdError>>;

pub(super) fn assert_number(actual: f64, expected: f64, label: &str) -> TestResult {
    if numbers_close(actual, expected) {
        Ok(())
    } else {
        Err(test_error(format!(
            "expected {label}={expected}, got {label}={actual}"
        )))
    }
}

pub(super) fn assert_device_logic(
    world: &World,
    reference_id: ReferenceId,
    field: &str,
    expected: f64,
) -> TestResult {
    let device = world
        .device(reference_id)
        .ok_or_else(|| test_error(format!("missing device {}", reference_id.value())))?;
    let actual = device.logic(field).ok_or_else(|| {
        test_error(format!(
            "missing logic field {field} on device {}",
            reference_id.value()
        ))
    })?;
    assert_number(actual, expected, field)
}

pub(super) fn assert_device_slot_logic(
    world: &World,
    reference_id: ReferenceId,
    slot_index: usize,
    field: &str,
    expected: f64,
) -> TestResult {
    let device = world
        .device(reference_id)
        .ok_or_else(|| test_error(format!("missing device {}", reference_id.value())))?;
    let slot = device.slot(slot_index).ok_or_else(|| {
        test_error(format!(
            "missing slot {slot_index} on device {}",
            reference_id.value()
        ))
    })?;
    let actual = slot.logic(field).ok_or_else(|| {
        test_error(format!(
            "missing logic field {field} on slot {slot_index} of device {}",
            reference_id.value()
        ))
    })?;
    assert_number(actual, expected, field)
}

pub(super) fn assert_device_stack(
    world: &World,
    reference_id: ReferenceId,
    address: usize,
    expected: f64,
) -> TestResult {
    let device = world
        .device(reference_id)
        .ok_or_else(|| test_error(format!("missing device {}", reference_id.value())))?;
    let actual = device
        .stack_value(address)
        .ok_or_else(|| test_error(format!("missing device stack[{address}]")))?;
    assert_number(actual, expected, &format!("stack[{address}]"))
}

pub(super) fn assert_housing_logic(housing: &IcHousing, field: &str, expected: f64) -> TestResult {
    let actual = housing.device().logic(field).ok_or_else(|| {
        test_error(format!(
            "missing logic field {field} on housing {}",
            housing.reference_id().value()
        ))
    })?;
    assert_number(actual, expected, field)
}

pub(super) fn assert_housing_register(
    world: &World,
    reference_id: ReferenceId,
    register: usize,
    expected: f64,
) -> TestResult {
    let housing = housing(world, reference_id)?;
    let actual = housing
        .ic10()
        .register(register)
        .ok_or_else(|| test_error(format!("missing register r{register}")))?;
    assert_number(actual, expected, &format!("r{register}"))
}

pub(super) fn assert_housing_stack(
    world: &World,
    reference_id: ReferenceId,
    address: usize,
    expected: f64,
) -> TestResult {
    let housing = housing(world, reference_id)?;
    let actual = housing
        .device()
        .stack_value(address)
        .ok_or_else(|| test_error(format!("missing housing stack[{address}]")))?;
    assert_number(actual, expected, &format!("stack[{address}]"))
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

pub(super) fn housing(world: &World, reference_id: ReferenceId) -> TestResult<&IcHousing> {
    world
        .ic10_housing(reference_id)
        .ok_or_else(|| test_error(format!("missing housing {}", reference_id.value())))
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
