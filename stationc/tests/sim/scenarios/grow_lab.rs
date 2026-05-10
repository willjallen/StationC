use std::{error::Error as StdError, io};

use stationc::sim::{
    ic10::ReferenceId,
    world::{Device, World},
};

type TestResult<T = ()> = Result<T, Box<dyn StdError>>;

const GLCTRL_SRC: &str = include_str!("../../../examples/ic10/grow-lab/glctrl.ic10");

const SENSOR_TYPE: f64 = 1_076_425_094.0;
const HYDROPONICS_TYPE: f64 = 1_441_767_298.0;
const LED_TYPE: f64 = 1_944_485_013.0;

#[test]
fn grow_lab_script_fits_ic10_editor_limits() -> TestResult {
    assert_script_limits("glctrl", GLCTRL_SRC)
}

#[test]
fn grow_lab_runs_station_lights_during_window_daylight() -> TestResult {
    let (mut world, ids) = mock_grow_lab(45.0, true, GLCTRL_SRC)?;

    world.tick()?;

    assert_device_logic(&world, ids.sensor, "Mode", 0.0)?;
    assert_matching_stations(&world, &ids, 1.0)?;
    assert_device_logic(&world, ids.other_named_station, "On", 0.0)?;
    assert_device_logic(&world, ids.other_type_same_name, "On", 0.0)
}

#[test]
fn grow_lab_supplements_light_near_night_edges() -> TestResult {
    let (mut world, ids) = mock_grow_lab(100.0, false, GLCTRL_SRC)?;

    world.tick()?;

    assert_matching_stations(&world, &ids, 1.0)?;
    assert_device_logic(&world, ids.other_named_station, "On", 0.0)?;
    assert_device_logic(&world, ids.other_type_same_name, "On", 0.0)
}

#[test]
fn grow_lab_preserves_dark_rest_outside_light_window() -> TestResult {
    let (mut world, ids) = mock_grow_lab(130.0, false, GLCTRL_SRC)?;

    world.tick()?;

    assert_matching_stations(&world, &ids, 0.0)
}

#[test]
fn grow_lab_turns_lights_off_at_light_window_boundary() -> TestResult {
    let (mut world, ids) = mock_grow_lab(112.5, false, GLCTRL_SRC)?;

    world.tick()?;

    assert_matching_stations(&world, &ids, 0.0)
}

#[test]
fn grow_lab_pins_default_light_window_edges() -> TestResult {
    assert_angle_controls_lights(0.0, GLCTRL_SRC, 1.0)?;
    assert_angle_controls_lights(90.0, GLCTRL_SRC, 1.0)?;
    assert_angle_controls_lights(112.49, GLCTRL_SRC, 1.0)?;
    assert_angle_controls_lights(112.5, GLCTRL_SRC, 0.0)?;
    assert_angle_controls_lights(112.51, GLCTRL_SRC, 0.0)?;
    assert_angle_controls_lights(180.0, GLCTRL_SRC, 0.0)
}

#[test]
fn grow_lab_updates_outputs_when_solar_angle_changes() -> TestResult {
    let (mut world, ids) = mock_grow_lab(100.0, false, GLCTRL_SRC)?;

    world.tick()?;
    assert_matching_stations(&world, &ids, 1.0)?;

    set_device_logic(&mut world, ids.sensor, "SolarAngle", 130.0)?;
    world.tick()?;
    assert_matching_stations(&world, &ids, 0.0)?;

    set_device_logic(&mut world, ids.sensor, "SolarAngle", 0.0)?;
    world.tick()?;
    assert_matching_stations(&world, &ids, 1.0)
}

#[test]
fn grow_lab_batch_write_overrides_mixed_station_states() -> TestResult {
    let (mut world, ids) = mock_grow_lab(130.0, false, GLCTRL_SRC)?;
    set_device_logic(&mut world, ids.matching_stations[0], "On", 0.0)?;
    set_device_logic(&mut world, ids.matching_stations[1], "On", 1.0)?;

    world.tick()?;

    assert_matching_stations(&world, &ids, 0.0)?;

    set_device_logic(&mut world, ids.sensor, "SolarAngle", 100.0)?;
    world.tick()?;

    assert_matching_stations(&world, &ids, 1.0)
}

#[test]
fn grow_lab_ignores_activate_when_enforcing_dark_rest() -> TestResult {
    let (mut world, ids) = mock_grow_lab(130.0, true, GLCTRL_SRC)?;

    world.tick()?;

    assert_matching_stations(&world, &ids, 0.0)
}

#[test]
fn grow_lab_keeps_lights_off_without_the_named_daylight_sensor() -> TestResult {
    let (mut world, ids) = mock_grow_lab(100.0, false, GLCTRL_SRC)?;
    set_device_name(&mut world, ids.sensor, "GLDLSNSR_WRONG")?;

    world.tick()?;

    assert_matching_stations(&world, &ids, 0.0)
}

#[test]
fn grow_lab_keeps_lights_off_with_wrong_sensor_type() -> TestResult {
    let (mut world, ids) = mock_grow_lab(100.0, false, GLCTRL_SRC)?;
    set_device_prefab(&mut world, ids.sensor, LED_TYPE)?;

    world.tick()?;

    assert_matching_stations(&world, &ids, 0.0)
}

#[test]
fn grow_lab_light_window_constants_are_configurable() -> TestResult {
    let short_light_source =
        GLCTRL_SRC.replace("define LightMinutes 12.5", "define LightMinutes 10");

    assert_angle_controls_lights(89.99, &short_light_source, 1.0)?;
    assert_angle_controls_lights(90.0, &short_light_source, 0.0)
}

#[test]
fn grow_lab_day_cycle_constant_is_configurable() -> TestResult {
    let long_day_source = GLCTRL_SRC.replace("define DayMinutes 20", "define DayMinutes 25");

    assert_angle_controls_lights(89.99, &long_day_source, 1.0)?;
    assert_angle_controls_lights(90.0, &long_day_source, 0.0)
}

struct MockIds {
    sensor: ReferenceId,
    matching_stations: [ReferenceId; 2],
    other_named_station: ReferenceId,
    other_type_same_name: ReferenceId,
}

fn mock_grow_lab(
    solar_angle: f64,
    sunlight_active: bool,
    source: &str,
) -> TestResult<(World, MockIds)> {
    let mut world = World::new();
    let sensor = world.add_device(
        named_device(SENSOR_TYPE, "GLDLSNSR")
            .with_logic("Mode", 2.0)
            .with_logic("SolarAngle", solar_angle)
            .with_logic("Activate", number_from_bool(sunlight_active)),
    );
    let matching_stations = [
        world.add_device(named_device(HYDROPONICS_TYPE, "GLHPSTN").with_logic("On", 0.0)),
        world.add_device(named_device(HYDROPONICS_TYPE, "GLHPSTN").with_logic("On", 0.0)),
    ];
    let other_named_station =
        world.add_device(named_device(HYDROPONICS_TYPE, "GLHPSTN_OTHER").with_logic("On", 0.0));
    let other_type_same_name =
        world.add_device(named_device(LED_TYPE, "GLHPSTN").with_logic("On", 0.0));
    world.add_ic10_housing(source)?;

    Ok((
        world,
        MockIds {
            sensor,
            matching_stations,
            other_named_station,
            other_type_same_name,
        },
    ))
}

fn assert_matching_stations(world: &World, ids: &MockIds, expected: f64) -> TestResult {
    for id in ids.matching_stations {
        assert_device_logic(world, id, "On", expected)?;
    }
    Ok(())
}

fn assert_angle_controls_lights(solar_angle: f64, source: &str, expected: f64) -> TestResult {
    let (mut world, ids) = mock_grow_lab(solar_angle, false, source)?;

    world.tick()?;

    assert_matching_stations(&world, &ids, expected)
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

fn set_device_name(world: &mut World, reference_id: ReferenceId, name: &str) -> TestResult {
    let device = world
        .device_mut(reference_id)
        .ok_or_else(|| test_error(format!("missing device {}", reference_id.value())))?;
    device.set_name_hash(hash(name));
    Ok(())
}

fn set_device_prefab(world: &mut World, reference_id: ReferenceId, prefab_hash: f64) -> TestResult {
    let device = world
        .device_mut(reference_id)
        .ok_or_else(|| test_error(format!("missing device {}", reference_id.value())))?;
    device.set_prefab_hash(prefab_hash);
    Ok(())
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

fn named_device(prefab_hash: f64, name: &str) -> Device {
    Device::new()
        .with_prefab_hash(prefab_hash)
        .with_name_hash(hash(name))
}

const fn number_from_bool(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
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
