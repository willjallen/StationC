//! Deterministic world simulator for IC10 integration tests.

mod device;
mod device_logic;
mod ic_housing;
mod tick;
#[allow(clippy::module_inception)]
mod world;

pub use device::{Device, DeviceSlot};
pub use ic_housing::IcHousing;
pub use tick::{
    IC10_INSTRUCTIONS_PER_TICK, Ic10Schedule, Ic10TickResult, WorldAccessEvent,
    WorldAccessOperation, WorldAccessTarget, WorldDiagnostic, WorldDiagnosticKind, WorldTickResult,
};
pub use world::{World, WorldError};
