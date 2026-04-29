//! World device model.

use std::collections::HashMap;

use crate::sim::ic10::{EnvironmentFault, ReferenceId, STACK_SIZE};

use super::device_logic;

/// A simulated world object that exposes logic fields and stack memory.
#[derive(Debug, Clone)]
pub struct Device {
    reference_id: Option<ReferenceId>,
    prefab_hash: f64,
    name_hash: f64,
    logic: HashMap<String, f64>,
    stack: [f64; STACK_SIZE],
}

impl Device {
    /// Creates an empty device with no writable logic fields.
    #[must_use]
    pub fn new() -> Self {
        Self {
            reference_id: None,
            prefab_hash: 0.0,
            name_hash: 0.0,
            logic: HashMap::new(),
            stack: [0.0; STACK_SIZE],
        }
    }

    /// Adds a writable logic field and returns the device.
    #[must_use]
    pub fn with_logic(mut self, field: impl Into<String>, value: f64) -> Self {
        self.set_logic(field, value);
        self
    }

    /// Sets the device prefab hash returned by `PrefabHash`.
    #[must_use]
    pub const fn with_prefab_hash(mut self, value: f64) -> Self {
        self.prefab_hash = value;
        self
    }

    /// Sets the device name hash returned by `NameHash`.
    #[must_use]
    pub const fn with_name_hash(mut self, value: f64) -> Self {
        self.name_hash = value;
        self
    }

    /// Returns the assigned `ReferenceId`, if the device has been added to a world.
    #[must_use]
    pub const fn reference_id(&self) -> Option<ReferenceId> {
        self.reference_id
    }

    /// Reads a writable logic field stored on this device.
    #[must_use]
    pub fn logic(&self, field: &str) -> Option<f64> {
        self.logic.get(field).copied()
    }

    /// Sets or creates a writable logic field.
    pub fn set_logic(&mut self, field: impl Into<String>, value: f64) {
        self.logic.insert(field.into(), value);
    }

    /// Reads a stack value by absolute address.
    #[must_use]
    pub fn stack_value(&self, address: usize) -> Option<f64> {
        self.stack.get(address).copied()
    }

    /// Sets a stack value by absolute address.
    ///
    /// # Errors
    ///
    /// Returns an [`EnvironmentFault`] if `address` is outside device stack memory.
    pub fn set_stack_value(&mut self, address: usize, value: f64) -> Result<(), EnvironmentFault> {
        self.put_stack(address, value)
    }

    pub(super) const fn assign_reference_id(&mut self, reference_id: ReferenceId) {
        self.reference_id = Some(reference_id);
    }

    pub(super) fn ic_housing_body(reference_id: ReferenceId) -> Self {
        let mut device = Self::new()
            .with_logic(device_logic::ON, 1.0)
            .with_logic(device_logic::SETTING, 0.0);
        device.assign_reference_id(reference_id);
        device
    }

    pub(super) fn load_logic(&self, field: &str) -> Result<f64, EnvironmentFault> {
        match field {
            device_logic::REFERENCE_ID => {
                self.reference_id.map(ReferenceId::as_f64).ok_or_else(|| {
                    EnvironmentFault::UnknownLogicField {
                        field: field.to_owned(),
                    }
                })
            }
            device_logic::PREFAB_HASH => Ok(self.prefab_hash),
            device_logic::NAME_HASH => Ok(self.name_hash),
            _ => {
                self.logic
                    .get(field)
                    .copied()
                    .ok_or_else(|| EnvironmentFault::UnknownLogicField {
                        field: field.to_owned(),
                    })
            }
        }
    }

    pub(super) fn store_logic(&mut self, field: &str, value: f64) -> Result<(), EnvironmentFault> {
        if device_logic::is_read_only(field) {
            return Err(EnvironmentFault::ReadOnlyLogicField {
                field: field.to_owned(),
            });
        }
        let logic =
            self.logic
                .get_mut(field)
                .ok_or_else(|| EnvironmentFault::UnknownLogicField {
                    field: field.to_owned(),
                })?;
        *logic = value;
        Ok(())
    }

    pub(super) fn get_stack(&self, address: usize) -> Result<f64, EnvironmentFault> {
        self.stack
            .get(address)
            .copied()
            .ok_or(EnvironmentFault::StackAddressOutOfRange { address })
    }

    pub(super) fn put_stack(&mut self, address: usize, value: f64) -> Result<(), EnvironmentFault> {
        let stack_value = self
            .stack
            .get_mut(address)
            .ok_or(EnvironmentFault::StackAddressOutOfRange { address })?;
        *stack_value = value;
        Ok(())
    }
}

impl Default for Device {
    fn default() -> Self {
        Self::new()
    }
}
