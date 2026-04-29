use stationc::sim::{
    ic10::{DevicePort, StopReason},
    world::World,
};

use super::support::{
    TestResult, assert_housing_register, assert_housing_stack, assert_tick, housing,
};

#[test]
fn one_ic_can_write_another_ic_housing_stack() -> TestResult {
    let mut world = World::new();
    let writer = world.add_ic10_housing(
        "\
put d0 10 42
yield
",
    )?;
    let reader = world.add_ic10_housing("yield")?;
    world.connect_pin(writer, DevicePort::D0, reader)?;

    let tick = world.tick()?;

    assert_tick(tick.ic10[0].tick, 2, StopReason::Yield)?;
    assert_housing_stack(&world, reader, 10, 42.0)
}

#[test]
fn one_ic_can_read_another_ic_housing_stack_in_the_same_tick() -> TestResult {
    let mut world = World::new();
    let writer = world.add_ic10_housing(
        "\
put db 12 99
yield
",
    )?;
    let reader = world.add_ic10_housing(
        "\
get r0 d0 12
yield
",
    )?;
    world.connect_pin(reader, DevicePort::D0, writer)?;

    let tick = world.tick()?;

    assert_tick(tick.ic10[0].tick, 2, StopReason::Yield)?;
    assert_tick(tick.ic10[1].tick, 2, StopReason::Yield)?;
    assert_housing_register(&world, reader, 0, 99.0)
}

#[test]
fn getd_and_putd_access_other_housings_by_reference_id() -> TestResult {
    let mut world = World::new();
    let target = world.add_ic10_housing("yield")?;
    let writer = world.add_ic10_housing(&format!(
        "\
putd {} 5 77
getd r0 {} 5
yield
",
        target.value(),
        target.value()
    ))?;

    world.tick()?;

    assert_housing_stack(&world, target, 5, 77.0)?;
    assert_housing_register(&world, writer, 0, 77.0)
}

#[test]
fn ic_housing_db_stack_can_be_used_as_local_world_stack() -> TestResult {
    let mut world = World::new();
    let housing_id = world.add_ic10_housing(
        "\
put db 20 123
get r0 db 20
yield
",
    )?;

    world.tick()?;
    let housing = housing(&world, housing_id)?;

    assert_eq!(housing.device().stack_value(20), Some(123.0));
    assert_housing_register(&world, housing_id, 0, 123.0)
}
