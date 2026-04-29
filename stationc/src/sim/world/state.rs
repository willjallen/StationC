//! World simulator state and tick loop.

use std::{error::Error as StdError, fmt};

use crate::sim::ic10::{DevicePort, DeviceTarget, EnvironmentFault, Ic10Environment, ReferenceId};

use super::{
    device::Device,
    ic_housing::{IcHousing, PIN_COUNT},
    tick::{IC10_INSTRUCTIONS_PER_TICK, Ic10TickResult, WorldTickResult},
};

/// A deterministic world containing devices and IC housings.
#[derive(Debug)]
pub struct World {
    tick: u64,
    next_reference_id: u32,
    devices: Vec<Device>,
    ic_housings: Vec<IcHousing>,
}

impl World {
    /// Creates an empty world.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            tick: 0,
            next_reference_id: 1,
            devices: Vec::new(),
            ic_housings: Vec::new(),
        }
    }

    /// Returns the current world tick count.
    #[must_use]
    pub const fn tick_count(&self) -> u64 {
        self.tick
    }

    /// Adds a world device and returns its assigned `ReferenceId`.
    pub fn add_device(&mut self, mut device: Device) -> ReferenceId {
        let reference_id = self.allocate_reference_id();
        device.assign_reference_id(reference_id);
        self.devices.push(device);
        reference_id
    }

    /// Adds an IC10 housing and returns its assigned `ReferenceId`.
    ///
    /// # Errors
    ///
    /// Returns [`WorldError::Ic10`] if the IC10 source cannot be parsed.
    pub fn add_ic10_housing(&mut self, source: &str) -> Result<ReferenceId, WorldError> {
        let reference_id = self.allocate_reference_id();
        let housing =
            IcHousing::from_source(reference_id, source).map_err(|source| WorldError::Ic10 {
                reference_id,
                source,
            })?;
        self.ic_housings.push(housing);
        Ok(reference_id)
    }

    /// Connects one IC housing pin to another world object.
    ///
    /// # Errors
    ///
    /// Returns [`WorldError`] if either reference id is unknown or `port` is `db`.
    pub fn connect_pin(
        &mut self,
        housing_id: ReferenceId,
        port: DevicePort,
        target_id: ReferenceId,
    ) -> Result<(), WorldError> {
        if port.pin_index().is_none() {
            return Err(WorldError::InvalidPin { port });
        }
        self.require_reference_id(target_id)?;
        let housing = self
            .ic10_housing_mut(housing_id)
            .ok_or(WorldError::UnknownReferenceId {
                reference_id: housing_id,
            })?;
        housing.set_pin(port, target_id);
        Ok(())
    }

    /// Returns a device by `ReferenceId`.
    #[must_use]
    pub fn device(&self, reference_id: ReferenceId) -> Option<&Device> {
        self.devices
            .iter()
            .find(|device| device.reference_id() == Some(reference_id))
    }

    /// Returns a mutable device by `ReferenceId`.
    #[must_use]
    pub fn device_mut(&mut self, reference_id: ReferenceId) -> Option<&mut Device> {
        self.devices
            .iter_mut()
            .find(|device| device.reference_id() == Some(reference_id))
    }

    /// Returns an IC housing by `ReferenceId`.
    #[must_use]
    pub fn ic10_housing(&self, reference_id: ReferenceId) -> Option<&IcHousing> {
        self.ic_housings
            .iter()
            .find(|housing| housing.reference_id() == reference_id)
    }

    /// Returns a mutable IC housing by `ReferenceId`.
    #[must_use]
    pub fn ic10_housing_mut(&mut self, reference_id: ReferenceId) -> Option<&mut IcHousing> {
        self.ic_housings
            .iter_mut()
            .find(|housing| housing.reference_id() == reference_id)
    }

    /// Advances the world by one tick using the default IC10 budget.
    ///
    /// # Errors
    ///
    /// Returns [`WorldError`] if any IC10 housing faults while running.
    pub fn tick(&mut self) -> Result<WorldTickResult, WorldError> {
        self.tick_with_budget(IC10_INSTRUCTIONS_PER_TICK)
    }

    /// Advances the world by one tick using a caller-provided IC10 budget.
    ///
    /// # Errors
    ///
    /// Returns [`WorldError`] if any IC10 housing faults while running.
    pub fn tick_with_budget(&mut self, budget: u32) -> Result<WorldTickResult, WorldError> {
        let tick = self.tick;
        let mut ic10 = Vec::with_capacity(self.ic_housings.len());
        let housing_count = self.ic_housings.len();

        for index in 0..housing_count {
            let mut housing = self.ic_housings.remove(index);
            let reference_id = housing.reference_id();
            let result = {
                let mut environment = WorldIc10Context {
                    current_reference_id: reference_id,
                    current_device: &mut housing.device,
                    pins: housing.pins,
                    devices: &mut self.devices,
                    ic_housings: &mut self.ic_housings,
                };
                housing
                    .vm
                    .run_until_yield_or_budget_with_environment(budget, &mut environment)
            };
            self.ic_housings.insert(index, housing);

            let tick_result = result.map_err(|source| WorldError::Ic10 {
                reference_id,
                source,
            })?;
            ic10.push(Ic10TickResult {
                reference_id,
                tick: tick_result,
            });
        }

        self.tick = self.tick.saturating_add(1);
        Ok(WorldTickResult { tick, ic10 })
    }

    const fn allocate_reference_id(&mut self) -> ReferenceId {
        let reference_id = ReferenceId::new(self.next_reference_id);
        self.next_reference_id = self.next_reference_id.saturating_add(1);
        reference_id
    }

    fn require_reference_id(&self, reference_id: ReferenceId) -> Result<(), WorldError> {
        if self.device(reference_id).is_some() || self.ic10_housing(reference_id).is_some() {
            Ok(())
        } else {
            Err(WorldError::UnknownReferenceId { reference_id })
        }
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

struct WorldIc10Context<'a> {
    current_reference_id: ReferenceId,
    current_device: &'a mut Device,
    pins: [Option<ReferenceId>; PIN_COUNT],
    devices: &'a mut [Device],
    ic_housings: &'a mut [IcHousing],
}

impl Ic10Environment for WorldIc10Context<'_> {
    fn load_logic(&mut self, target: DeviceTarget, field: &str) -> Result<f64, EnvironmentFault> {
        self.resolve_device_mut(target)?.load_logic(field)
    }

    fn store_logic(
        &mut self,
        target: DeviceTarget,
        field: &str,
        value: f64,
    ) -> Result<(), EnvironmentFault> {
        self.resolve_device_mut(target)?.store_logic(field, value)
    }

    fn get_stack(&mut self, target: DeviceTarget, address: usize) -> Result<f64, EnvironmentFault> {
        self.resolve_device_mut(target)?.get_stack(address)
    }

    fn put_stack(
        &mut self,
        target: DeviceTarget,
        address: usize,
        value: f64,
    ) -> Result<(), EnvironmentFault> {
        self.resolve_device_mut(target)?.put_stack(address, value)
    }
}

impl WorldIc10Context<'_> {
    fn resolve_device_mut(
        &mut self,
        target: DeviceTarget,
    ) -> Result<&mut Device, EnvironmentFault> {
        let reference_id = self.reference_id_for_target(target)?;
        self.resolve_reference_mut(reference_id)
    }

    fn reference_id_for_target(
        &self,
        target: DeviceTarget,
    ) -> Result<ReferenceId, EnvironmentFault> {
        match target {
            DeviceTarget::ReferenceId(reference_id) => Ok(reference_id),
            DeviceTarget::Port(DevicePort::Db) => Ok(self.current_reference_id),
            DeviceTarget::Port(port) => {
                let index = port
                    .pin_index()
                    .ok_or(EnvironmentFault::DevicePortUnbound { port })?;
                self.pins[index].ok_or(EnvironmentFault::DevicePortUnbound { port })
            }
        }
    }

    fn resolve_reference_mut(
        &mut self,
        reference_id: ReferenceId,
    ) -> Result<&mut Device, EnvironmentFault> {
        if reference_id == self.current_reference_id {
            return Ok(self.current_device);
        }
        if let Some(device) = self
            .devices
            .iter_mut()
            .find(|device| device.reference_id() == Some(reference_id))
        {
            return Ok(device);
        }
        if let Some(housing) = self
            .ic_housings
            .iter_mut()
            .find(|housing| housing.reference_id() == reference_id)
        {
            return Ok(housing.device_mut());
        }
        Err(EnvironmentFault::UnknownReferenceId { reference_id })
    }
}

/// Error reported by the world simulator.
#[derive(Debug)]
pub enum WorldError {
    /// A `ReferenceId` was not present in the world.
    UnknownReferenceId {
        /// The missing reference id.
        reference_id: ReferenceId,
    },
    /// A caller tried to connect a non-configurable pin such as `db`.
    InvalidPin {
        /// The invalid pin.
        port: DevicePort,
    },
    /// An IC10 housing failed while parsing or executing.
    Ic10 {
        /// The housing that failed.
        reference_id: ReferenceId,
        /// The IC10 error.
        source: crate::sim::ic10::Error,
    },
}

impl fmt::Display for WorldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownReferenceId { reference_id } => {
                write!(formatter, "unknown ReferenceId `{}`", reference_id.value())
            }
            Self::InvalidPin { port } => write!(formatter, "cannot connect `{port}`"),
            Self::Ic10 {
                reference_id,
                source,
            } => write!(
                formatter,
                "IC10 housing {} failed: {source}",
                reference_id.value()
            ),
        }
    }
}

impl StdError for WorldError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Ic10 { source, .. } => Some(source),
            Self::UnknownReferenceId { .. } | Self::InvalidPin { .. } => None,
        }
    }
}
