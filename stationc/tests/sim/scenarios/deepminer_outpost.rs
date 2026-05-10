use std::{error::Error as StdError, io};

use stationc::sim::{
    ic10::{DevicePort, ReferenceId},
    world::{Device, World},
};

type TestResult<T = ()> = Result<T, Box<dyn StdError>>;

const DMOCENT_SRC: &str =
    include_str!("../../../examples/ic10/3-ore-deepminer-outpost/dmocent.ic10");
const DMOSTATION_SRC: &str =
    include_str!("../../../examples/ic10/3-ore-deepminer-outpost/dmostation.ic10");

const LED_TYPE: f64 = 1_944_485_013.0;
const KIT_LIGHT_TYPE: f64 = -1_860_064_656.0;

#[test]
fn deepminer_outpost_scripts_fit_ic10_editor_limits() -> TestResult {
    assert_script_limits("dmocent", DMOCENT_SRC)?;
    assert_script_limits("dmostation", DMOSTATION_SRC)
}

#[test]
fn centrifuge_script_opens_first_full_closed_centrifuge() -> TestResult {
    let (mut world, ids) = mock_centrifuge_world([100.0, 390.0, 399.0], [0.0, 0.0, 0.0])?;

    world.tick()?;

    assert_device_logic(&world, ids.centrifuges[0], "Open", 0.0)?;
    assert_device_logic(&world, ids.centrifuges[1], "Open", 1.0)?;
    assert_device_logic(&world, ids.centrifuges[2], "Open", 0.0)?;
    assert_housing_logic(&world, ids.housing, "Setting", 11.0)
}

#[test]
fn centrifuge_script_closes_open_empty_centrifuge() -> TestResult {
    let (mut world, ids) = mock_centrifuge_world([100.0, 0.0, 399.0], [0.0, 1.0, 0.0])?;

    world.tick()?;

    assert_device_logic(&world, ids.centrifuges[1], "Open", 0.0)?;
    assert_device_logic(&world, ids.centrifuges[2], "Open", 0.0)?;
    assert_housing_logic(&world, ids.housing, "Setting", 0.0)
}

#[test]
fn station_script_turns_named_lights_on_when_proximity_active() -> TestResult {
    let (mut world, ids) = mock_station_world(1.0)?;

    world.tick()?;

    assert_device_logic(&world, ids.proximity, "Setting", 50.0)?;
    assert_device_logic(&world, ids.led_light, "On", 1.0)?;
    assert_device_logic(&world, ids.kit_light, "On", 1.0)?;
    assert_housing_logic(&world, ids.housing, "Setting", 1.0)
}

#[test]
fn station_script_turns_named_lights_off_when_proximity_inactive() -> TestResult {
    let (mut world, ids) = mock_station_world(0.0)?;

    world.tick()?;

    assert_device_logic(&world, ids.proximity, "Setting", 50.0)?;
    assert_device_logic(&world, ids.led_light, "On", 0.0)?;
    assert_device_logic(&world, ids.kit_light, "On", 0.0)?;
    assert_housing_logic(&world, ids.housing, "Setting", 0.0)
}

struct CentrifugeIds {
    centrifuges: [ReferenceId; 3],
    housing: ReferenceId,
}

struct StationIds {
    proximity: ReferenceId,
    led_light: ReferenceId,
    kit_light: ReferenceId,
    housing: ReferenceId,
}

fn mock_centrifuge_world(reagents: [f64; 3], open: [f64; 3]) -> TestResult<(World, CentrifugeIds)> {
    let mut world = World::new();
    let centrifuges = [
        world.add_device(centrifuge(reagents[0], open[0])),
        world.add_device(centrifuge(reagents[1], open[1])),
        world.add_device(centrifuge(reagents[2], open[2])),
    ];
    let housing = world.add_ic10_housing(DMOCENT_SRC)?;
    world.connect_pin(housing, DevicePort::D0, centrifuges[0])?;
    world.connect_pin(housing, DevicePort::D1, centrifuges[1])?;
    world.connect_pin(housing, DevicePort::D2, centrifuges[2])?;
    Ok((
        world,
        CentrifugeIds {
            centrifuges,
            housing,
        },
    ))
}

fn mock_station_world(activate: f64) -> TestResult<(World, StationIds)> {
    let mut world = World::new();
    let proximity = world.add_device(
        named_device(hash("StructureProximitySensor"), "DMOPROX")
            .with_logic("Activate", activate)
            .with_logic("Setting", 0.0),
    );
    let led_light = world.add_device(named_device(LED_TYPE, "DMOLIGHT").with_logic("On", 0.0));
    let kit_light =
        world.add_device(named_device(KIT_LIGHT_TYPE, "DMOLIGHT").with_logic("On", 0.0));
    let housing = world.add_ic10_housing(DMOSTATION_SRC)?;
    world.connect_pin(housing, DevicePort::D0, proximity)?;
    Ok((
        world,
        StationIds {
            proximity,
            led_light,
            kit_light,
            housing,
        },
    ))
}

fn centrifuge(reagents: f64, open: f64) -> Device {
    named_device(hash("StructureCentrifuge"), "DMOCF")
        .with_logic("On", 0.0)
        .with_logic("Open", open)
        .with_logic("Reagents", reagents)
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
    if actual.is_nan() && expected.is_nan() {
        return true;
    }
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
