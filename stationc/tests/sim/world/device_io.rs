use stationc::sim::{
    ic10::{DevicePort, Error as Ic10Error, ErrorCode, Ic10, ReferenceId, StopReason},
    world::{Device, World, WorldError},
};

use super::support::{
    TestResult, assert_device_logic, assert_housing_register, assert_number, assert_tick,
};

#[test]
fn ic10_loads_and_stores_direct_device_logic() -> TestResult {
    let mut world = World::new();
    let sensor = world.add_device(Device::new().with_logic("Temperature", 301.25));
    let light = world.add_device(Device::new().with_logic("On", 0.0));
    let housing = world.add_ic10_housing(
        "\
l r0 d0 Temperature
s d1 On r0
yield
",
    )?;
    world.connect_pin(housing, DevicePort::D0, sensor)?;
    world.connect_pin(housing, DevicePort::D1, light)?;

    let tick = world.tick()?;

    assert_tick(tick.ic10[0].tick, 3, StopReason::Yield)?;
    assert_housing_register(&world, housing, 0, 301.25)?;
    assert_device_logic(&world, light, "On", 301.25)
}

#[test]
fn device_pin_aliases_are_accepted_by_world_instructions() -> TestResult {
    let mut world = World::new();
    let sensor = world.add_device(Device::new().with_logic("Temperature", 290.0));
    let light = world.add_device(Device::new().with_logic("On", 0.0));
    let housing = world.add_ic10_housing(
        "\
alias sensor d0
alias light d1
l r0 sensor Temperature
s light On 1
yield
",
    )?;
    world.connect_pin(housing, DevicePort::D0, sensor)?;
    world.connect_pin(housing, DevicePort::D1, light)?;

    world.tick()?;

    assert_housing_register(&world, housing, 0, 290.0)?;
    assert_device_logic(&world, light, "On", 1.0)
}

#[test]
fn reference_id_logic_can_be_used_for_direct_load_and_store() -> TestResult {
    let mut world = World::new();
    let sensor = world.add_device(Device::new().with_logic("Temperature", 287.5));
    let light = world.add_device(Device::new().with_logic("On", 0.0));
    let housing = world.add_ic10_housing(&format!(
        "\
l r0 d0 ReferenceId
ld r1 r0 Temperature
sd {} On 1
yield
",
        light.value()
    ))?;
    world.connect_pin(housing, DevicePort::D0, sensor)?;

    world.tick()?;

    assert_housing_register(&world, housing, 0, sensor.as_f64())?;
    assert_housing_register(&world, housing, 1, 287.5)?;
    assert_device_logic(&world, light, "On", 1.0)
}

#[test]
fn device_indirection_uses_register_value_as_pin_index() -> TestResult {
    let mut world = World::new();
    let sensor = world.add_device(Device::new().with_logic("Temperature", 293.0));
    let housing = world.add_ic10_housing(
        "\
move r0 2
l r1 dr0 Temperature
yield
",
    )?;
    world.connect_pin(housing, DevicePort::D2, sensor)?;

    world.tick()?;

    assert_housing_register(&world, housing, 1, 293.0)
}

#[test]
fn unbound_device_pin_is_typed_runtime_error() -> TestResult {
    let mut world = World::new();
    world.add_ic10_housing(
        "\
l r0 d0 Temperature
yield
",
    )?;

    let error = tick_error(&mut world)?;

    assert_ic10_error_code(error, ErrorCode::DevicePortUnbound)
}

#[test]
fn standalone_ic10_requires_world_context_for_device_io() -> TestResult {
    let mut ic10 = Ic10::from_source(
        "\
l r0 d0 Temperature
yield
",
    )?;

    let error = ic10_tick_error(&mut ic10)?;

    assert_eq!(error.code(), ErrorCode::WorldContextRequired);
    Ok(())
}

#[test]
fn direct_reference_to_missing_device_is_typed_runtime_error() -> TestResult {
    let mut world = World::new();
    world.add_ic10_housing(
        "\
ld r0 999 Temperature
yield
",
    )?;

    let error = tick_error(&mut world)?;

    assert_ic10_error_code(error, ErrorCode::UnknownReferenceId)
}

#[test]
fn read_only_logic_field_cannot_be_written() -> TestResult {
    let mut world = World::new();
    let device = world.add_device(Device::new().with_logic("On", 0.0));
    let housing = world.add_ic10_housing(
        "\
s d0 ReferenceId 10
yield
",
    )?;
    world.connect_pin(housing, DevicePort::D0, device)?;

    let error = tick_error(&mut world)?;

    assert_ic10_error_code(error, ErrorCode::ReadOnlyLogicField)
}

#[test]
fn device_metadata_logic_fields_are_available() -> TestResult {
    let mut world = World::new();
    let device = world.add_device(
        Device::new()
            .with_prefab_hash(1234.0)
            .with_name_hash(5678.0)
            .with_logic("On", 0.0),
    );
    let housing = world.add_ic10_housing(
        "\
l r0 d0 PrefabHash
l r1 d0 NameHash
yield
",
    )?;
    world.connect_pin(housing, DevicePort::D0, device)?;

    world.tick()?;

    assert_housing_register(&world, housing, 0, 1234.0)?;
    assert_housing_register(&world, housing, 1, 5678.0)
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

fn ic10_tick_error(ic10: &mut Ic10) -> TestResult<Ic10Error> {
    match ic10.run_until_yield_or_budget(128) {
        Ok(result) => Err(format!("expected IC10 tick error, got {result:?}").into()),
        Err(error) => Ok(error),
    }
}

#[test]
fn reference_ids_are_explicit_numeric_values() -> TestResult {
    let reference_id = ReferenceId::new(42);

    assert_eq!(reference_id.value(), 42);
    assert_number(reference_id.as_f64(), 42.0, "ReferenceId")
}
