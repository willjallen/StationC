use std::{error::Error as StdError, io};

use stationc::sim::{
    ic10::{ReagentMode, ReferenceId},
    world::{Device, IcHousing, World},
};

type TestResult<T = ()> = Result<T, Box<dyn StdError>>;

const AFCTRL: &str = include_str!("../../../examples/ic10/advanced-furnace/afctrl.ic10");
const AFVIZLED: &str = include_str!("../../../examples/ic10/advanced-furnace/afvizled.ic10");
const AFVIZNUM: &str = include_str!("../../../examples/ic10/advanced-furnace/afviznum.ic10");

const FURNACE_TYPE: f64 = 545_937_711.0;
const BUTTON_TYPE: f64 = 491_845_673.0;
const LEVER_TYPE: f64 = 1_220_484_876.0;
const SWITCH_TYPE: f64 = 321_604_921.0;
const DIAL_TYPE: f64 = 554_524_804.0;
const HOUSING_TYPE: f64 = -128_473_777.0;
const LED_TYPE: f64 = 1_944_485_013.0;
const KIT_LIGHT_TYPE: f64 = -1_860_064_656.0;

#[test]
fn advanced_furnace_panel_scripts_fit_ic10_editor_limits() -> TestResult {
    assert_script_limits("afctrl", AFCTRL)?;
    assert_script_limits("afvizled", AFVIZLED)?;
    assert_script_limits("afviznum", AFVIZNUM)
}

#[test]
fn advanced_furnace_panel_displays_four_reagent_rows() -> TestResult {
    let reagents = [
        MockReagent::new("Iron", 49.0),
        MockReagent::new("Copper", 12.0),
        MockReagent::new("Cobalt", 7.0),
        MockReagent::new("Silicon", 3.0),
    ];
    let (mut world, ids) = mock_panel_world(Controls::all_on(), &reagents)?;

    tick_panel_cycle(&mut world)?;

    assert_device_logic(&world, ids.furnace, "On", 1.0)?;
    assert_device_logic(&world, ids.furnace, "Activate", 1.0)?;
    assert_device_logic(&world, ids.furnace, "Open", 1.0)?;
    assert_device_logic(&world, ids.furnace, "SettingInput", 27.0)?;
    assert_device_logic(&world, ids.furnace, "SettingOutput", 8.0)?;
    assert_device_logic(&world, ids.vent, "On", 1.0)?;
    assert_housing_logic(housing(&world, ids.viz_led_housing)?, "On", 1.0)?;
    assert_housing_logic(housing(&world, ids.viz_num_housing)?, "On", 1.0)?;
    assert_reagent_row(&world, &ids, 0, 49.0, 4.0)?;
    assert_reagent_row(&world, &ids, 1, 12.0, 3.0)?;
    assert_reagent_row(&world, &ids, 2, 7.0, 0.0)?;
    assert_reagent_row(&world, &ids, 3, 3.0, 9.0)?;
    assert_status_displays(&world, &ids)?;
    assert_device_logic(&world, ids.led_room_light, "On", 1.0)?;
    assert_device_logic(&world, ids.kit_room_light, "On", 1.0)
}

#[test]
fn advanced_furnace_panel_displays_partial_reagent_rows() -> TestResult {
    assert_reagent_layout(
        &[
            MockReagent::new("Iron", 49.0),
            MockReagent::new("Carbon", 5.0),
        ],
        &[(49.0, 4.0), (5.0, 8.0)],
    )?;
    assert_reagent_layout(
        &[
            MockReagent::new("Hydrocarbon", 2.0),
            MockReagent::new("Nickel", 3.0),
            MockReagent::new("Silver", 4.0),
        ],
        &[(2.0, 7.0), (3.0, 2.0), (4.0, 6.0)],
    )
}

#[test]
fn advanced_furnace_panel_master_on_respects_released_controls() -> TestResult {
    let controls = Controls {
        master_on: true,
        activate: false,
        mold_open: false,
        vent_on: false,
    };
    let reagents = [MockReagent::new("Iron", 49.0)];
    let (mut world, ids) = mock_panel_world(controls, &reagents)?;

    tick_panel_cycle(&mut world)?;

    assert_device_logic(&world, ids.furnace, "On", 1.0)?;
    assert_device_logic(&world, ids.furnace, "Activate", 0.0)?;
    assert_device_logic(&world, ids.furnace, "Open", 0.0)?;
    assert_device_logic(&world, ids.vent, "On", 0.0)?;
    assert_housing_logic(housing(&world, ids.viz_led_housing)?, "On", 1.0)?;
    assert_housing_logic(housing(&world, ids.viz_num_housing)?, "On", 1.0)?;
    assert_reagent_row(&world, &ids, 0, 49.0, 4.0)?;
    assert_empty_row(&world, &ids, 1)?;
    assert_device_logic(&world, ids.led_room_light, "On", 1.0)?;
    assert_device_logic(&world, ids.kit_room_light, "On", 1.0)
}

#[test]
fn advanced_furnace_panel_mock_master_off_shuts_outputs_down() -> TestResult {
    let reagents = [
        MockReagent::new("Iron", 49.0),
        MockReagent::new("Copper", 12.0),
        MockReagent::new("Cobalt", 7.0),
        MockReagent::new("Silicon", 3.0),
    ];
    let controls = Controls {
        master_on: false,
        ..Controls::all_on()
    };
    let (mut world, ids) = mock_panel_world(controls, &reagents)?;

    tick_panel_cycle(&mut world)?;

    assert_named_device_logic(&world, ids.furnace, "AF", "On", 0.0)?;
    assert_named_device_logic(&world, ids.furnace, "AF", "Activate", 0.0)?;
    assert_named_device_logic(&world, ids.furnace, "AF", "Open", 0.0)?;
    assert_named_device_logic(&world, ids.furnace, "AF", "SettingInput", 27.0)?;
    assert_named_device_logic(&world, ids.furnace, "AF", "SettingOutput", 8.0)?;
    assert_named_device_logic(&world, ids.vent, "AFVNT", "On", 0.0)?;
    assert_housing_logic(housing(&world, ids.viz_led_housing)?, "On", 0.0)?;
    assert_housing_logic(housing(&world, ids.viz_num_housing)?, "On", 0.0)?;
    for row in 0..4 {
        assert_empty_row(&world, &ids, row)?;
    }
    assert_status_displays_off(&world, &ids)?;
    assert_named_device_logic(&world, ids.led_room_light, "AFLIGHT", "On", 0.0)?;
    assert_named_device_logic(&world, ids.kit_room_light, "AFLIGHT2", "On", 0.0)
}

#[derive(Debug, Clone, Copy)]
struct Controls {
    master_on: bool,
    activate: bool,
    mold_open: bool,
    vent_on: bool,
}

impl Controls {
    const fn all_on() -> Self {
        Self {
            master_on: true,
            activate: true,
            mold_open: true,
            vent_on: true,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct MockReagent {
    name: &'static str,
    amount: f64,
}

impl MockReagent {
    const fn new(name: &'static str, amount: f64) -> Self {
        Self { name, amount }
    }
}

#[derive(Debug, Clone, Copy)]
struct MockIds {
    furnace: ReferenceId,
    vent: ReferenceId,
    reagent_leds: [ReferenceId; 4],
    quantity_displays: [ReferenceId; 4],
    temperature: ReferenceId,
    pressure: ReferenceId,
    input: ReferenceId,
    output: ReferenceId,
    led_room_light: ReferenceId,
    kit_room_light: ReferenceId,
    viz_led_housing: ReferenceId,
    viz_num_housing: ReferenceId,
}

fn mock_panel_world(controls: Controls, reagents: &[MockReagent]) -> TestResult<(World, MockIds)> {
    let mut world = World::new();
    let mut furnace_device = named_device(FURNACE_TYPE, "AF")
        .with_logic("On", 1.0)
        .with_logic("Activate", 1.0)
        .with_logic("Open", 1.0)
        .with_logic("SettingInput", 0.0)
        .with_logic("SettingOutput", 0.0)
        .with_logic("Temperature", 1873.0)
        .with_logic("Pressure", 123_456.0);
    for reagent in reagents {
        furnace_device.set_reagent(ReagentMode::Contents, hash(reagent.name), reagent.amount);
    }
    let furnace = world.add_device(furnace_device);
    world.add_device(
        named_device(SWITCH_TYPE, "AFMASTER")
            .with_logic("Open", number_from_bool(controls.master_on)),
    );
    world.add_device(
        named_device(BUTTON_TYPE, "AFACT")
            .with_logic("Activate", number_from_bool(controls.activate)),
    );
    world.add_device(
        named_device(LEVER_TYPE, "AFMOLD").with_logic("Open", number_from_bool(controls.mold_open)),
    );
    world.add_device(
        named_device(LEVER_TYPE, "AFVNTL").with_logic("Open", number_from_bool(controls.vent_on)),
    );
    world.add_device(named_device(DIAL_TYPE, "AFGIN").with_logic("Setting", 27.0));
    world.add_device(named_device(DIAL_TYPE, "AFGOUT").with_logic("Setting", 8.0));
    let vent =
        world.add_device(named_device(hash("StructureActiveVent"), "AFVNT").with_logic("On", 1.0));

    let reagent_leds = [
        world.add_device(reagent_led("AF1L", 1.0)),
        world.add_device(reagent_led("AF2L", 1.0)),
        world.add_device(reagent_led("AF3L", 1.0)),
        world.add_device(reagent_led("AF4L", 1.0)),
    ];
    let quantity_displays = [
        world.add_device(display("AF1Q", 1.0)),
        world.add_device(display("AF2Q", 1.0)),
        world.add_device(display("AF3Q", 1.0)),
        world.add_device(display("AF4Q", 1.0)),
    ];
    let temperature = world.add_device(display("AFT", 1.0));
    let pressure = world.add_device(display("AFP", 1.0));
    let input = world.add_device(display("AFIN", 1.0));
    let output = world.add_device(display("AFOUT", 1.0));
    let led_room_light = world.add_device(named_device(LED_TYPE, "AFLIGHT").with_logic("On", 1.0));
    let kit_room_light =
        world.add_device(named_device(KIT_LIGHT_TYPE, "AFLIGHT2").with_logic("On", 1.0));

    let ctrl_housing = world.add_ic10_housing(AFCTRL)?;
    let viz_led_housing = world.add_ic10_housing(AFVIZLED)?;
    let viz_num_housing = world.add_ic10_housing(AFVIZNUM)?;
    set_housing_identity(&mut world, ctrl_housing, "AFCTRL")?;
    set_housing_identity(&mut world, viz_led_housing, "AFVIZLED")?;
    set_housing_identity(&mut world, viz_num_housing, "AFVIZNUM")?;

    Ok((
        world,
        MockIds {
            furnace,
            vent,
            reagent_leds,
            quantity_displays,
            temperature,
            pressure,
            input,
            output,
            led_room_light,
            kit_room_light,
            viz_led_housing,
            viz_num_housing,
        },
    ))
}

fn assert_reagent_layout(reagents: &[MockReagent], expected_rows: &[(f64, f64)]) -> TestResult {
    let (mut world, ids) = mock_panel_world(Controls::all_on(), reagents)?;

    tick_panel_cycle(&mut world)?;

    for (row, (amount, color)) in expected_rows.iter().copied().enumerate() {
        assert_reagent_row(&world, &ids, row, amount, color)?;
    }
    for row in expected_rows.len()..4 {
        assert_empty_row(&world, &ids, row)?;
    }
    Ok(())
}

fn tick_panel_cycle(world: &mut World) -> TestResult {
    world.tick()?;
    world.tick()?;
    Ok(())
}

fn assert_reagent_row(
    world: &World,
    ids: &MockIds,
    row: usize,
    amount: f64,
    color: f64,
) -> TestResult {
    assert_device_logic(world, ids.reagent_leds[row], "On", 1.0)?;
    assert_device_logic(world, ids.reagent_leds[row], "Color", color)?;
    assert_device_logic(world, ids.quantity_displays[row], "On", 1.0)?;
    assert_device_logic(world, ids.quantity_displays[row], "Setting", amount)?;
    assert_device_logic(world, ids.quantity_displays[row], "Mode", 0.0)
}

fn assert_empty_row(world: &World, ids: &MockIds, row: usize) -> TestResult {
    assert_device_logic(world, ids.reagent_leds[row], "On", 0.0)?;
    assert_device_logic(world, ids.quantity_displays[row], "On", 0.0)
}

fn assert_status_displays(world: &World, ids: &MockIds) -> TestResult {
    assert_device_logic(world, ids.temperature, "On", 1.0)?;
    assert_device_logic(world, ids.temperature, "Setting", 1873.0)?;
    assert_device_logic(world, ids.temperature, "Mode", 3.0)?;
    assert_device_logic(world, ids.pressure, "On", 1.0)?;
    assert_device_logic(world, ids.pressure, "Setting", 123_456.0)?;
    assert_device_logic(world, ids.pressure, "Mode", 14.0)?;
    assert_device_logic(world, ids.input, "On", 1.0)?;
    assert_device_logic(world, ids.input, "Setting", 27.0)?;
    assert_device_logic(world, ids.input, "Mode", 12.0)?;
    assert_device_logic(world, ids.output, "On", 1.0)?;
    assert_device_logic(world, ids.output, "Setting", 8.0)?;
    assert_device_logic(world, ids.output, "Mode", 12.0)
}

fn assert_status_displays_off(world: &World, ids: &MockIds) -> TestResult {
    assert_device_logic(world, ids.temperature, "On", 0.0)?;
    assert_device_logic(world, ids.pressure, "On", 0.0)?;
    assert_device_logic(world, ids.input, "On", 0.0)?;
    assert_device_logic(world, ids.output, "On", 0.0)
}

fn assert_script_limits(name: &str, source: &str) -> TestResult {
    let line_count = source.lines().count();
    if line_count > 128 {
        return Err(format!("{name} has {line_count} lines").into());
    }
    for (index, line) in source.lines().enumerate() {
        if line.len() > 90 {
            return Err(format!("{name} line {} has {} chars", index + 1, line.len()).into());
        }
    }
    Ok(())
}

fn assert_named_device_logic(
    world: &World,
    reference_id: ReferenceId,
    name: &str,
    field: &str,
    expected: f64,
) -> TestResult {
    let device = world
        .device(reference_id)
        .ok_or_else(|| test_error(format!("missing {name} device {}", reference_id.value())))?;
    let actual = device.logic(field).ok_or_else(|| {
        test_error(format!(
            "missing {name}.{field} on device {}",
            reference_id.value()
        ))
    })?;
    if numbers_close(actual, expected) {
        Ok(())
    } else {
        Err(test_error(format!(
            "expected {name}.{field}={expected}, got {actual}"
        )))
    }
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

fn assert_housing_logic(housing: &IcHousing, field: &str, expected: f64) -> TestResult {
    let actual = housing.device().logic(field).ok_or_else(|| {
        test_error(format!(
            "missing logic field {field} on housing {}",
            housing.reference_id().value()
        ))
    })?;
    assert_number(actual, expected, field)
}

fn housing(world: &World, reference_id: ReferenceId) -> TestResult<&IcHousing> {
    world
        .ic10_housing(reference_id)
        .ok_or_else(|| test_error(format!("missing housing {}", reference_id.value())))
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

fn set_housing_identity(world: &mut World, id: ReferenceId, name: &str) -> TestResult {
    let housing = world
        .ic10_housing_mut(id)
        .ok_or_else(|| test_error(format!("missing IC housing {}", id.value())))?;
    housing.device_mut().set_prefab_hash(HOUSING_TYPE);
    housing.device_mut().set_name_hash(hash(name));
    Ok(())
}

fn test_error(message: impl Into<String>) -> Box<dyn StdError> {
    Box::new(io::Error::other(message.into()))
}

fn named_device(prefab_hash: f64, name: &str) -> Device {
    Device::new()
        .with_prefab_hash(prefab_hash)
        .with_name_hash(hash(name))
}

fn reagent_led(name: &str, on: f64) -> Device {
    named_device(LED_TYPE, name)
        .with_logic("On", on)
        .with_logic("Color", 0.0)
}

fn display(name: &str, on: f64) -> Device {
    named_device(hash("StructureConsoleLED1x2"), name)
        .with_logic("On", on)
        .with_logic("Setting", -1.0)
        .with_logic("Mode", -1.0)
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
