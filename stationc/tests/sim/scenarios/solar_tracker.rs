use std::{error::Error as StdError, io};

use stationc::sim::{
    ic10::ReferenceId,
    world::{Device, World},
};

type TestResult<T = ()> = Result<T, Box<dyn StdError>>;

const STTRACK_SRC: &str = include_str!("../../../examples/ic10/solar-tracker/sttrack.ic10");
const SENSOR_TYPE: f64 = 1_076_425_094.0;

#[test]
fn solar_tracker_script_fits_ic10_editor_limits() -> TestResult {
    assert_script_limits("sttrack", STTRACK_SRC)
}

#[test]
fn solar_tracker_tracks_named_normal_and_dual_panels() -> TestResult {
    let mut world = World::new();
    let panel_type = hash("StructureSolarPanel");
    let panel_dual_type = hash("StructureSolarPanelDual");
    let sensor = named_device(SENSOR_TYPE, "STDLSNSR")
        .with_logic("Horizontal", 33.0)
        .with_logic("Vertical", 20.0);
    world.add_device(sensor);
    let normal = world.add_device(solar_panel(panel_type));
    let dual = world.add_device(solar_panel(panel_dual_type));
    let housing = world.add_ic10_housing(STTRACK_SRC)?;

    world.tick()?;

    assert_device_logic(&world, normal, "On", 1.0)?;
    assert_device_logic(&world, normal, "Horizontal", 33.0)?;
    assert_device_logic(&world, normal, "Vertical", 110.0)?;
    assert_device_logic(&world, dual, "On", 1.0)?;
    assert_device_logic(&world, dual, "Horizontal", 33.0)?;
    assert_device_logic(&world, dual, "Vertical", 110.0)?;
    assert_housing_logic(&world, housing, "Setting", 110.0)
}

#[test]
fn solar_tracker_ignores_panels_outside_the_named_batch() -> TestResult {
    let mut world = World::new();
    let panel_type = hash("StructureSolarPanel");
    world.add_device(
        named_device(SENSOR_TYPE, "STDLSNSR")
            .with_logic("Horizontal", 75.0)
            .with_logic("Vertical", 15.0),
    );
    let matching = world.add_device(solar_panel(panel_type));
    let wrong_name = world.add_device(
        named_device(panel_type, "OTHERSP")
            .with_logic("On", 0.0)
            .with_logic("Horizontal", 5.0)
            .with_logic("Vertical", 5.0),
    );
    let wrong_type = world.add_device(
        named_device(hash("StructureBattery"), "STSP")
            .with_logic("On", 0.0)
            .with_logic("Horizontal", 8.0)
            .with_logic("Vertical", 8.0),
    );
    world.add_ic10_housing(STTRACK_SRC)?;

    world.tick()?;

    assert_device_logic(&world, matching, "On", 1.0)?;
    assert_device_logic(&world, matching, "Horizontal", 75.0)?;
    assert_device_logic(&world, matching, "Vertical", 105.0)?;
    assert_device_logic(&world, wrong_name, "On", 0.0)?;
    assert_device_logic(&world, wrong_name, "Horizontal", 5.0)?;
    assert_device_logic(&world, wrong_name, "Vertical", 5.0)?;
    assert_device_logic(&world, wrong_type, "On", 0.0)?;
    assert_device_logic(&world, wrong_type, "Horizontal", 8.0)?;
    assert_device_logic(&world, wrong_type, "Vertical", 8.0)
}

#[test]
fn solar_tracker_clamps_panel_vertical_to_safe_travel_limits() -> TestResult {
    assert_sensor_vertical_applies_panel_vertical(-100.0, 15.0)?;
    assert_sensor_vertical_applies_panel_vertical(100.0, 165.0)
}

#[test]
fn solar_tracker_keeps_panels_unchanged_without_the_named_daylight_sensor() -> TestResult {
    let mut world = World::new();
    let panel_type = hash("StructureSolarPanel");
    world.add_device(
        named_device(SENSOR_TYPE, "WRONGSNSR")
            .with_logic("Horizontal", 75.0)
            .with_logic("Vertical", 15.0),
    );
    world.add_device(
        named_device(hash("StructureBattery"), "STDLSNSR")
            .with_logic("Horizontal", 75.0)
            .with_logic("Vertical", 15.0),
    );
    let panel = world.add_device(
        named_device(panel_type, "STSP")
            .with_logic("On", 0.0)
            .with_logic("Horizontal", 12.0)
            .with_logic("Vertical", 34.0),
    );
    let housing = world.add_ic10_housing(STTRACK_SRC)?;

    world.tick()?;

    assert_device_logic(&world, panel, "On", 0.0)?;
    assert_device_logic(&world, panel, "Horizontal", 12.0)?;
    assert_device_logic(&world, panel, "Vertical", 34.0)?;
    assert_housing_logic(&world, housing, "Setting", -1.0)
}

#[test]
fn solar_tracker_updates_panel_angles_when_sensor_readings_change() -> TestResult {
    let mut world = World::new();
    let panel_type = hash("StructureSolarPanel");
    let sensor = world.add_device(
        named_device(SENSOR_TYPE, "STDLSNSR")
            .with_logic("Horizontal", 10.0)
            .with_logic("Vertical", 0.0),
    );
    let panel = world.add_device(solar_panel(panel_type));
    let housing = world.add_ic10_housing(STTRACK_SRC)?;

    world.tick()?;
    assert_device_logic(&world, panel, "Horizontal", 10.0)?;
    assert_device_logic(&world, panel, "Vertical", 90.0)?;
    assert_housing_logic(&world, housing, "Setting", 90.0)?;

    set_device_logic(&mut world, sensor, "Horizontal", 170.0)?;
    set_device_logic(&mut world, sensor, "Vertical", 40.0)?;
    world.tick()?;

    assert_device_logic(&world, panel, "Horizontal", 170.0)?;
    assert_device_logic(&world, panel, "Vertical", 130.0)?;
    assert_housing_logic(&world, housing, "Setting", 130.0)
}

fn assert_sensor_vertical_applies_panel_vertical(
    sensor_vertical: f64,
    expected_panel_vertical: f64,
) -> TestResult {
    let mut world = World::new();
    let panel_type = hash("StructureSolarPanel");
    world.add_device(
        named_device(SENSOR_TYPE, "STDLSNSR")
            .with_logic("Horizontal", 0.0)
            .with_logic("Vertical", sensor_vertical),
    );
    let panel = world.add_device(solar_panel(panel_type));
    let housing = world.add_ic10_housing(STTRACK_SRC)?;

    world.tick()?;

    assert_device_logic(&world, panel, "Vertical", expected_panel_vertical)?;
    assert_housing_logic(&world, housing, "Setting", expected_panel_vertical)
}

fn solar_panel(prefab_hash: f64) -> Device {
    named_device(prefab_hash, "STSP")
        .with_logic("On", 0.0)
        .with_logic("Horizontal", 0.0)
        .with_logic("Vertical", 0.0)
}

fn named_device(prefab_hash: f64, name: &str) -> Device {
    Device::new()
        .with_prefab_hash(prefab_hash)
        .with_name_hash(hash(name))
}

fn assert_script_limits(name: &str, source: &str) -> TestResult {
    let line_count = source.lines().count();
    if line_count > 128 {
        return Err(test_error(format!("{name} has {line_count} lines")));
    }
    for (index, line) in source.lines().enumerate() {
        if line.len() > 90 {
            return Err(test_error(format!(
                "{name} line {} has {} chars",
                index + 1,
                line.len()
            )));
        }
    }
    Ok(())
}

fn assert_device_logic(
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

fn assert_housing_logic(
    world: &World,
    reference_id: ReferenceId,
    field: &str,
    expected: f64,
) -> TestResult {
    let housing = world
        .ic10_housing(reference_id)
        .ok_or_else(|| test_error(format!("missing housing {}", reference_id.value())))?;
    let actual = housing.device().logic(field).ok_or_else(|| {
        test_error(format!(
            "missing logic field {field} on housing {}",
            reference_id.value()
        ))
    })?;
    assert_number(actual, expected, field)
}

fn set_device_logic(
    world: &mut World,
    reference_id: ReferenceId,
    field: &str,
    value: f64,
) -> TestResult {
    let device = world
        .device_mut(reference_id)
        .ok_or_else(|| test_error(format!("missing device {}", reference_id.value())))?;
    device.set_logic(field, value);
    Ok(())
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
    let tolerance = f64::EPSILON * actual.abs().max(expected.abs()).max(1.0) * 8.0;
    (actual - expected).abs() <= tolerance
}

fn hash(value: &str) -> f64 {
    f64::from(crc32(value.as_bytes()))
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xEDB8_8320_u32 & mask);
        }
    }
    !crc
}

fn test_error(message: impl Into<String>) -> Box<dyn StdError> {
    Box::new(io::Error::other(message.into()))
}
