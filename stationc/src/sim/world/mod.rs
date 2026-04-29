//! Deterministic world simulator for IC10 integration tests.

mod device;
mod device_logic;
mod ic_housing;
mod tick;
#[allow(clippy::module_inception)]
mod world;

pub use device::Device;
pub use ic_housing::IcHousing;
pub use tick::{IC10_INSTRUCTIONS_PER_TICK, Ic10TickResult, WorldTickResult};
pub use world::{World, WorldError};
