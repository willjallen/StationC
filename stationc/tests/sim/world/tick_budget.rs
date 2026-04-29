use stationc::sim::{
    ic10::{DevicePort, StopReason},
    world::{Device, IC10_INSTRUCTIONS_PER_TICK, World},
};

use super::support::{
    TestResult, assert_housing_logic, assert_housing_register, assert_tick, housing,
};

#[test]
fn each_ic_housing_gets_its_own_instruction_budget() -> TestResult {
    let mut world = World::new();
    let first = world.add_ic10_housing(
        "\
move r0 0
loop:
add r0 r0 1
j loop
",
    )?;
    let second = world.add_ic10_housing(
        "\
move r0 100
loop:
add r0 r0 1
j loop
",
    )?;

    let tick = world.tick()?;

    assert_eq!(tick.tick, 0);
    assert_eq!(tick.ic10.len(), 2);
    assert_tick(
        tick.ic10[0].tick,
        IC10_INSTRUCTIONS_PER_TICK,
        StopReason::Budget,
    )?;
    assert_tick(
        tick.ic10[1].tick,
        IC10_INSTRUCTIONS_PER_TICK,
        StopReason::Budget,
    )?;
    assert_housing_register(&world, first, 0, 64.0)?;
    assert_housing_register(&world, second, 0, 164.0)
}

#[test]
fn yield_stops_one_housing_without_stopping_the_world() -> TestResult {
    let mut world = World::new();
    let first = world.add_ic10_housing(
        "\
move r0 1
yield
move r0 2
yield
",
    )?;
    let second = world.add_ic10_housing(
        "\
move r0 10
add r0 r0 5
yield
",
    )?;

    let tick = world.tick()?;

    assert_tick(tick.ic10[0].tick, 2, StopReason::Yield)?;
    assert_tick(tick.ic10[1].tick, 3, StopReason::Yield)?;
    assert_housing_register(&world, first, 0, 1.0)?;
    assert_housing_register(&world, second, 0, 15.0)
}

#[test]
fn db_targets_the_current_ic_housing_body() -> TestResult {
    let mut world = World::new();
    let housing_id = world.add_ic10_housing(
        "\
s db Setting 137
l r0 db Setting
yield
",
    )?;

    world.tick()?;
    let housing = housing(&world, housing_id)?;

    assert_housing_register(&world, housing_id, 0, 137.0)?;
    assert_housing_logic(housing, "Setting", 137.0)
}

#[test]
fn direct_pin_connection_is_visible_on_the_housing() -> TestResult {
    let mut world = World::new();
    let device_id = world.add_device(Device::new().with_logic("On", 0.0));
    let housing_id = world.add_ic10_housing("yield")?;

    world.connect_pin(housing_id, DevicePort::D0, device_id)?;

    let housing = housing(&world, housing_id)?;
    assert_eq!(housing.pin(DevicePort::D0), Some(device_id));
    assert_eq!(housing.pin(DevicePort::Db), Some(housing_id));
    Ok(())
}
