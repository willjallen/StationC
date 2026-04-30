use stationc::sim::{
    ic10::{DevicePort, StopReason},
    world::{
        Device, DeviceSlot, World, WorldAccessEvent, WorldAccessOperation, WorldAccessTarget,
        WorldDiagnosticKind,
    },
};

use super::support::{TestResult, assert_device_logic, assert_housing_register, assert_tick};

#[test]
fn access_trace_records_logic_and_stack_targets_in_execution_order() -> TestResult {
    let mut device = Device::new()
        .with_logic("On", 0.0)
        .with_logic("Temperature", 301.0);
    device
        .set_stack_value(3, 44.0)
        .map_err(|error| std::io::Error::other(error.to_string()))?;

    let mut world = World::new();
    let device = world.add_device(device);
    let housing = world.add_ic10_housing(
        "\
l r0 d0 Temperature
s d0 On 1
get r1 d0 3
put d0 4 55
yield
",
    )?;
    world.connect_pin(housing, DevicePort::D0, device)?;

    let tick = world.tick()?;

    assert_tick(tick.ic10[0].tick, 5, StopReason::Yield)?;
    assert_eq!(
        tick.access,
        vec![
            WorldAccessEvent {
                tick: 0,
                actor: housing,
                operation: WorldAccessOperation::Read,
                target: WorldAccessTarget::DeviceLogic {
                    reference_id: device,
                    field: "Temperature".to_owned(),
                },
            },
            WorldAccessEvent {
                tick: 0,
                actor: housing,
                operation: WorldAccessOperation::Write,
                target: WorldAccessTarget::DeviceLogic {
                    reference_id: device,
                    field: "On".to_owned(),
                },
            },
            WorldAccessEvent {
                tick: 0,
                actor: housing,
                operation: WorldAccessOperation::Read,
                target: WorldAccessTarget::DeviceStack {
                    reference_id: device,
                    address: 3,
                },
            },
            WorldAccessEvent {
                tick: 0,
                actor: housing,
                operation: WorldAccessOperation::Write,
                target: WorldAccessTarget::DeviceStack {
                    reference_id: device,
                    address: 4,
                },
            },
        ]
    );
    assert!(tick.diagnostics.is_empty());
    assert_housing_register(&world, housing, 0, 301.0)?;
    assert_housing_register(&world, housing, 1, 44.0)?;
    assert_device_logic(&world, device, "On", 1.0)
}

#[test]
fn access_trace_records_slot_logic_targets() -> TestResult {
    let mut world = World::new();
    let device = world.add_device(
        Device::new().with_slot(
            2,
            DeviceSlot::new()
                .with_logic("Occupied", 1.0)
                .with_logic("Quantity", 0.0),
        ),
    );
    let housing = world.add_ic10_housing(
        "\
ls r0 d0 2 Occupied
ss d0 2 Quantity 4
yield
",
    )?;
    world.connect_pin(housing, DevicePort::D0, device)?;

    let tick = world.tick()?;

    assert_tick(tick.ic10[0].tick, 3, StopReason::Yield)?;
    assert_eq!(
        tick.access,
        vec![
            WorldAccessEvent {
                tick: 0,
                actor: housing,
                operation: WorldAccessOperation::Read,
                target: WorldAccessTarget::DeviceSlotLogic {
                    reference_id: device,
                    slot: 2,
                    field: "Occupied".to_owned(),
                },
            },
            WorldAccessEvent {
                tick: 0,
                actor: housing,
                operation: WorldAccessOperation::Write,
                target: WorldAccessTarget::DeviceSlotLogic {
                    reference_id: device,
                    slot: 2,
                    field: "Quantity".to_owned(),
                },
            },
        ]
    );
    assert!(tick.diagnostics.is_empty());
    Ok(())
}

#[test]
fn diagnostics_flag_repeated_slot_logic_reads() -> TestResult {
    let mut world = World::new();
    let device =
        world.add_device(Device::new().with_slot(0, DeviceSlot::new().with_logic("Charge", 8.0)));
    let housing = world.add_ic10_housing(
        "\
ls r0 d0 0 Charge
ls r1 d0 0 Charge
yield
",
    )?;
    world.connect_pin(housing, DevicePort::D0, device)?;

    let tick = world.tick()?;

    assert_tick(tick.ic10[0].tick, 3, StopReason::Yield)?;
    assert_eq!(tick.access.len(), 2);
    assert_eq!(tick.diagnostics.len(), 1);
    assert_eq!(
        tick.diagnostics[0].kind,
        WorldDiagnosticKind::RepeatedVolatileRead
    );
    assert_housing_register(&world, housing, 0, 8.0)?;
    assert_housing_register(&world, housing, 1, 8.0)
}

#[test]
fn access_trace_records_db_and_direct_reference_targets_after_resolution() -> TestResult {
    let mut world = World::new();
    let device = world.add_device(Device::new().with_logic("Temperature", 288.0));
    let housing = world.add_ic10_housing(&format!(
        "\
s db Setting 12
ld r0 {} Temperature
yield
",
        device.value()
    ))?;

    let tick = world.tick()?;

    assert_tick(tick.ic10[0].tick, 3, StopReason::Yield)?;
    assert_eq!(
        tick.access,
        vec![
            WorldAccessEvent {
                tick: 0,
                actor: housing,
                operation: WorldAccessOperation::Write,
                target: WorldAccessTarget::DeviceLogic {
                    reference_id: housing,
                    field: "Setting".to_owned(),
                },
            },
            WorldAccessEvent {
                tick: 0,
                actor: housing,
                operation: WorldAccessOperation::Read,
                target: WorldAccessTarget::DeviceLogic {
                    reference_id: device,
                    field: "Temperature".to_owned(),
                },
            },
        ]
    );
    assert!(tick.diagnostics.is_empty());
    assert_housing_register(&world, housing, 0, 288.0)
}

#[test]
fn diagnostics_flag_multiple_writes_to_same_target_in_one_tick() -> TestResult {
    let mut world = World::new();
    let device = world.add_device(Device::new().with_logic("On", 0.0));
    let first = world.add_ic10_housing(
        "\
s d0 On 1
yield
",
    )?;
    let second = world.add_ic10_housing(
        "\
s d0 On 0
yield
",
    )?;
    world.connect_pin(first, DevicePort::D0, device)?;
    world.connect_pin(second, DevicePort::D0, device)?;

    let tick = world.tick()?;

    assert_eq!(tick.access.len(), 2);
    assert_eq!(tick.diagnostics.len(), 1);
    assert_eq!(
        tick.diagnostics[0].kind,
        WorldDiagnosticKind::MultipleWritesSameTick
    );
    assert_eq!(tick.diagnostics[0].first_access, 0);
    assert_eq!(tick.diagnostics[0].second_access, 1);
    assert_device_logic(&world, device, "On", 0.0)
}

#[test]
fn diagnostics_flag_read_write_overlap_to_same_target_in_one_tick() -> TestResult {
    let mut world = World::new();
    let device = world.add_device(Device::new().with_logic("On", 0.0));
    let reader = world.add_ic10_housing(
        "\
l r0 d0 On
yield
",
    )?;
    let writer = world.add_ic10_housing(
        "\
s d0 On 1
yield
",
    )?;
    world.connect_pin(reader, DevicePort::D0, device)?;
    world.connect_pin(writer, DevicePort::D0, device)?;

    let tick = world.tick()?;

    assert_eq!(tick.access.len(), 2);
    assert_eq!(tick.diagnostics.len(), 1);
    assert_eq!(
        tick.diagnostics[0].kind,
        WorldDiagnosticKind::ReadWriteSameTick
    );
    assert_eq!(tick.diagnostics[0].first_access, 0);
    assert_eq!(tick.diagnostics[0].second_access, 1);
    assert_housing_register(&world, reader, 0, 0.0)?;
    assert_device_logic(&world, device, "On", 1.0)
}
