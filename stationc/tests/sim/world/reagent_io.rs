use stationc::sim::{
    ic10::{DevicePort, ReagentMode},
    world::{Device, World},
};

use super::support::{TestResult, assert_housing_register};

#[test]
fn lr_reads_reagent_contents_from_mock_device() -> TestResult {
    let mut world = World::new();
    let furnace = world.add_device(
        Device::new()
            .with_reagent(ReagentMode::Contents, hash("Iron"), 49.0)
            .with_reagent(ReagentMode::Contents, hash("Copper"), 12.0),
    );
    let housing = world.add_ic10_housing(
        "\
lr r0 d0 Contents HASH(\"Iron\")
lr r1 d0 Contents HASH(\"Copper\")
lr r2 d0 Contents HASH(\"Nickel\")
yield
",
    )?;
    world.connect_pin(housing, DevicePort::D0, furnace)?;

    world.tick()?;

    assert_housing_register(&world, housing, 0, 49.0)?;
    assert_housing_register(&world, housing, 1, 12.0)?;
    assert_housing_register(&world, housing, 2, 0.0)
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
