use stationc::sim::{
    ic10::{DevicePort, ErrorCode, StopReason},
    world::{Device, World, WorldAccessOperation, WorldAccessTarget, WorldError},
};

use super::support::{TestResult, assert_device_logic, assert_housing_register, assert_tick};

#[test]
fn batch_load_modes_aggregate_matching_prefab_devices() -> TestResult {
    let mut world = World::new();
    world.add_device(Device::new().with_prefab_hash(100.0).with_logic("On", 1.0));
    world.add_device(Device::new().with_prefab_hash(100.0).with_logic("On", 3.0));
    world.add_device(Device::new().with_prefab_hash(200.0).with_logic("On", 9.0));
    let housing = world.add_ic10_housing(
        "\
lb r0 100 On Sum
lb r1 100 On Average
lb r2 100 On Minimum
lb r3 100 On Maximum
lb r4 100 On 1
yield
",
    )?;

    let tick = world.tick()?;

    assert_tick(tick.ic10[0].tick, 6, StopReason::Yield)?;
    assert_housing_register(&world, housing, 0, 4.0)?;
    assert_housing_register(&world, housing, 1, 2.0)?;
    assert_housing_register(&world, housing, 2, 1.0)?;
    assert_housing_register(&world, housing, 3, 3.0)?;
    assert_housing_register(&world, housing, 4, 4.0)
}

#[test]
fn batch_mode_names_do_not_shadow_named_logic_fields() -> TestResult {
    let mut world = World::new();
    let device = world.add_device(Device::new().with_logic("Maximum", 42.0));
    let housing = world.add_ic10_housing(
        "\
l r0 d0 Maximum
yield
",
    )?;
    world.connect_pin(housing, DevicePort::D0, device)?;

    world.tick()?;

    assert_housing_register(&world, housing, 0, 42.0)
}

#[test]
fn dynamic_batch_mode_can_come_from_register() -> TestResult {
    let mut world = World::new();
    world.add_device(Device::new().with_prefab_hash(100.0).with_logic("On", 2.0));
    world.add_device(Device::new().with_prefab_hash(100.0).with_logic("On", 5.0));
    let housing = world.add_ic10_housing(
        "\
move r15 3
lb r0 100 On r15
yield
",
    )?;

    let tick = world.tick()?;

    assert_tick(tick.ic10[0].tick, 3, StopReason::Yield)?;
    assert_housing_register(&world, housing, 0, 5.0)
}

#[test]
fn dynamic_batch_prefab_and_logic_type_can_come_from_registers() -> TestResult {
    let prefab_hash = hash_literal_value("StructureGasSensor");

    let mut world = World::new();
    world.add_device(
        Device::new()
            .with_prefab_hash(prefab_hash)
            .with_logic("Temperature", 280.0)
            .with_logic("Pressure", 100.0),
    );
    world.add_device(
        Device::new()
            .with_prefab_hash(prefab_hash)
            .with_logic("Temperature", 300.0)
            .with_logic("Pressure", 200.0),
    );
    let housing = world.add_ic10_housing(
        "\
move r1 HASH(\"StructureGasSensor\")
move r2 LogicType.Temperature
lb r0 r1 r2 Average
yield
",
    )?;

    let tick = world.tick()?;

    assert_tick(tick.ic10[0].tick, 4, StopReason::Yield)?;
    assert_housing_register(&world, housing, 0, 290.0)
}

#[test]
fn batch_load_by_name_uses_hash_literals_with_quoted_spaces() -> TestResult {
    let prefab_hash = hash_literal_value("StructureLogicSorter");
    let target_name_hash = hash_literal_value("Sorter Corn");
    let other_name_hash = hash_literal_value("Sorter Wheat");

    let mut world = World::new();
    let target = world.add_device(
        Device::new()
            .with_prefab_hash(prefab_hash)
            .with_name_hash(target_name_hash)
            .with_logic("Mode", 0.0),
    );
    let other = world.add_device(
        Device::new()
            .with_prefab_hash(prefab_hash)
            .with_name_hash(other_name_hash)
            .with_logic("Mode", 0.0),
    );
    let housing = world.add_ic10_housing(
        "\
lbn r0 HASH(\"StructureLogicSorter\") HASH(\"Sorter Corn\") ReferenceId Maximum
sd r0 Mode 7
yield
",
    )?;

    let tick = world.tick()?;

    assert_tick(tick.ic10[0].tick, 3, StopReason::Yield)?;
    assert_housing_register(&world, housing, 0, target.as_f64())?;
    assert_device_logic(&world, target, "Mode", 7.0)?;
    assert_device_logic(&world, other, "Mode", 0.0)?;
    assert_eq!(tick.access.len(), 2);
    assert_eq!(tick.access[0].operation, WorldAccessOperation::Read);
    assert_eq!(
        tick.access[0].target,
        WorldAccessTarget::DeviceLogic {
            reference_id: target,
            field: "ReferenceId".to_owned(),
        }
    );
    assert_eq!(tick.access[1].operation, WorldAccessOperation::Write);
    assert_eq!(
        tick.access[1].target,
        WorldAccessTarget::DeviceLogic {
            reference_id: target,
            field: "Mode".to_owned(),
        }
    );
    Ok(())
}

#[test]
fn batch_load_by_name_still_aggregates_all_matching_names() -> TestResult {
    let mut world = World::new();
    world.add_device(
        Device::new()
            .with_prefab_hash(500.0)
            .with_name_hash(10.0)
            .with_logic("Temperature", 280.0),
    );
    world.add_device(
        Device::new()
            .with_prefab_hash(500.0)
            .with_name_hash(10.0)
            .with_logic("Temperature", 320.0),
    );
    world.add_device(
        Device::new()
            .with_prefab_hash(500.0)
            .with_name_hash(20.0)
            .with_logic("Temperature", 100.0),
    );
    let housing = world.add_ic10_housing(
        "\
lbn r0 500 10 Temperature Average
yield
",
    )?;

    let tick = world.tick()?;

    assert_tick(tick.ic10[0].tick, 2, StopReason::Yield)?;
    assert_housing_register(&world, housing, 0, 300.0)
}

#[test]
fn batch_reference_id_maximum_selects_largest_matching_reference() -> TestResult {
    let mut world = World::new();
    let first = world.add_device(Device::new().with_prefab_hash(100.0).with_logic("On", 1.0));
    let second = world.add_device(Device::new().with_prefab_hash(100.0).with_logic("On", 1.0));
    world.add_device(Device::new().with_prefab_hash(200.0).with_logic("On", 1.0));
    let housing = world.add_ic10_housing(
        "\
lb r0 100 ReferenceId Maximum
yield
",
    )?;

    let tick = world.tick()?;

    assert_tick(tick.ic10[0].tick, 2, StopReason::Yield)?;
    assert_housing_register(&world, housing, 0, second.as_f64())?;
    assert_eq!(tick.access.len(), 2);
    assert_eq!(
        tick.access[0].target,
        WorldAccessTarget::DeviceLogic {
            reference_id: first,
            field: "ReferenceId".to_owned(),
        }
    );
    assert_eq!(
        tick.access[1].target,
        WorldAccessTarget::DeviceLogic {
            reference_id: second,
            field: "ReferenceId".to_owned(),
        }
    );
    Ok(())
}

#[test]
fn batch_load_no_matches_returns_documented_empty_values() -> TestResult {
    let mut world = World::new();
    let housing = world.add_ic10_housing(
        "\
lb r0 999 On Average
lb r1 999 On Sum
lb r2 999 On Minimum
lb r3 999 On Maximum
yield
",
    )?;

    let tick = world.tick()?;

    assert_tick(tick.ic10[0].tick, 5, StopReason::Yield)?;
    assert_housing_register(&world, housing, 0, f64::NAN)?;
    assert_housing_register(&world, housing, 1, 0.0)?;
    assert_housing_register(&world, housing, 2, 0.0)?;
    assert_housing_register(&world, housing, 3, f64::NEG_INFINITY)?;
    assert_eq!(tick.access.len(), 0);
    Ok(())
}

#[test]
fn batch_load_missing_field_on_matching_device_is_typed_error() -> TestResult {
    let mut world = World::new();
    world.add_device(Device::new().with_prefab_hash(100.0).with_logic("On", 1.0));
    world.add_ic10_housing(
        "\
lb r0 100 Temperature Average
yield
",
    )?;

    let error = tick_error(&mut world)?;

    assert_ic10_error_code(error, ErrorCode::UnknownLogicField)
}

#[test]
fn invalid_numeric_batch_mode_is_typed_error() -> TestResult {
    let mut world = World::new();
    world.add_ic10_housing(
        "\
lb r0 100 On 4
yield
",
    )?;

    let error = tick_error(&mut world)?;

    assert_ic10_error_code(error, ErrorCode::InvalidBatchMode)
}

fn assert_ic10_error_code(error: WorldError, expected: ErrorCode) -> TestResult {
    match error {
        WorldError::Ic10 { source, .. } => {
            assert_eq!(source.code(), expected);
            Ok(())
        }
        other => Err(format!("expected IC10 error, got {other:?}").into()),
    }
}

fn tick_error(world: &mut World) -> TestResult<WorldError> {
    match world.tick() {
        Ok(result) => Err(format!("expected world tick error, got {result:?}").into()),
        Err(error) => Ok(error),
    }
}

fn hash_literal_value(value: &str) -> f64 {
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
