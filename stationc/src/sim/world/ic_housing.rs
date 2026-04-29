//! IC housing model.

use crate::sim::ic10::{DevicePort, Error, Ic10, ReferenceId};

use super::device::Device;
use super::device_logic;

pub(super) const PIN_COUNT: usize = 6;

/// A simulated IC housing with one IC10 program, one housing body, and six pins.
#[derive(Debug)]
pub struct IcHousing {
    pub(super) reference_id: ReferenceId,
    pub(super) ic10: Ic10,
    pub(super) device: Device,
    pub(super) pins: [Option<ReferenceId>; PIN_COUNT],
}

impl IcHousing {
    pub(super) fn from_source(reference_id: ReferenceId, source: &str) -> Result<Self, Error> {
        Ok(Self {
            reference_id,
            ic10: Ic10::from_source(source)?,
            device: Device::ic_housing_body(reference_id),
            pins: [None; PIN_COUNT],
        })
    }

    /// Returns this housing's `ReferenceId`.
    #[must_use]
    pub const fn reference_id(&self) -> ReferenceId {
        self.reference_id
    }

    /// Returns the IC10 simulator installed in this housing.
    #[must_use]
    pub const fn ic10(&self) -> &Ic10 {
        &self.ic10
    }

    /// Returns the mutable IC10 simulator installed in this housing.
    #[must_use]
    pub const fn ic10_mut(&mut self) -> &mut Ic10 {
        &mut self.ic10
    }

    /// Returns the world-facing device body of this IC housing.
    #[must_use]
    pub const fn device(&self) -> &Device {
        &self.device
    }

    /// Returns the mutable world-facing device body of this IC housing.
    #[must_use]
    pub const fn device_mut(&mut self) -> &mut Device {
        &mut self.device
    }

    /// Returns the `ReferenceId` connected to a direct pin.
    #[must_use]
    pub const fn pin(&self, port: DevicePort) -> Option<ReferenceId> {
        match port.pin_index() {
            Some(index) => self.pins[index],
            None => Some(self.reference_id),
        }
    }

    pub(super) const fn set_pin(&mut self, port: DevicePort, target: ReferenceId) {
        if let Some(index) = port.pin_index() {
            self.pins[index] = Some(target);
        }
    }

    pub(super) fn is_on(&self) -> bool {
        self.device
            .logic(device_logic::ON)
            .is_none_or(|value| value >= 1.0)
    }
}
