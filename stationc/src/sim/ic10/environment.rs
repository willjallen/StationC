//! Environment interface for IC10 world-facing instructions.

use std::fmt;

/// A stable in-world device identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ReferenceId {
    value: u32,
}

impl ReferenceId {
    /// Creates a reference id from its numeric IC10 representation.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self { value }
    }

    /// Returns the raw numeric identifier.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.value
    }

    /// Returns the value IC10 code sees when reading `ReferenceId`.
    #[must_use]
    pub fn as_f64(self) -> f64 {
        f64::from(self.value)
    }
}

/// One of an IC housing's directly configurable device pins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DevicePort {
    /// Device pin `d0`.
    D0,
    /// Device pin `d1`.
    D1,
    /// Device pin `d2`.
    D2,
    /// Device pin `d3`.
    D3,
    /// Device pin `d4`.
    D4,
    /// Device pin `d5`.
    D5,
    /// The device or housing the IC is installed in.
    Db,
}

impl DevicePort {
    /// Parses a direct device pin token.
    #[must_use]
    pub const fn from_pin_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::D0),
            1 => Some(Self::D1),
            2 => Some(Self::D2),
            3 => Some(Self::D3),
            4 => Some(Self::D4),
            5 => Some(Self::D5),
            _ => None,
        }
    }

    /// Returns the `d0` through `d5` array index, or `None` for `db`.
    #[must_use]
    pub const fn pin_index(self) -> Option<usize> {
        match self {
            Self::D0 => Some(0),
            Self::D1 => Some(1),
            Self::D2 => Some(2),
            Self::D3 => Some(3),
            Self::D4 => Some(4),
            Self::D5 => Some(5),
            Self::Db => None,
        }
    }
}

impl fmt::Display for DevicePort {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::D0 => formatter.write_str("d0"),
            Self::D1 => formatter.write_str("d1"),
            Self::D2 => formatter.write_str("d2"),
            Self::D3 => formatter.write_str("d3"),
            Self::D4 => formatter.write_str("d4"),
            Self::D5 => formatter.write_str("d5"),
            Self::Db => formatter.write_str("db"),
        }
    }
}

/// A resolved IC10 device target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceTarget {
    /// A housing pin such as `d0`, `d5`, or `db`.
    Port(DevicePort),
    /// A direct `ReferenceId`.
    ReferenceId(ReferenceId),
}

/// Kind of operation requested from an IC10 environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvironmentOperation {
    /// Read a device logic field.
    LoadLogic,
    /// Write a device logic field.
    StoreLogic,
    /// Read device stack memory.
    GetStack,
    /// Write device stack memory.
    PutStack,
}

impl fmt::Display for EnvironmentOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LoadLogic => formatter.write_str("load logic"),
            Self::StoreLogic => formatter.write_str("store logic"),
            Self::GetStack => formatter.write_str("get stack"),
            Self::PutStack => formatter.write_str("put stack"),
        }
    }
}

/// Runtime failure reported by an IC10 environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvironmentFault {
    /// A world-facing instruction ran without a world environment.
    WorldContextRequired {
        /// The operation that needed world context.
        operation: EnvironmentOperation,
    },
    /// A direct housing pin was not connected to a device.
    DevicePortUnbound {
        /// The unbound pin.
        port: DevicePort,
    },
    /// No world object had the requested reference id.
    UnknownReferenceId {
        /// The missing reference id.
        reference_id: ReferenceId,
    },
    /// The selected device does not expose the requested logic field.
    UnknownLogicField {
        /// The missing field name.
        field: String,
    },
    /// The selected logic field can be read but not written.
    ReadOnlyLogicField {
        /// The read-only field name.
        field: String,
    },
    /// A device stack operation addressed outside stack memory.
    StackAddressOutOfRange {
        /// The invalid stack address.
        address: usize,
    },
}

impl fmt::Display for EnvironmentFault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorldContextRequired { operation } => {
                write!(formatter, "`{operation}` requires a world context")
            }
            Self::DevicePortUnbound { port } => write!(formatter, "device port `{port}` is unset"),
            Self::UnknownReferenceId { reference_id } => {
                write!(formatter, "unknown ReferenceId `{}`", reference_id.value())
            }
            Self::UnknownLogicField { field } => {
                write!(formatter, "unknown logic field `{field}`")
            }
            Self::ReadOnlyLogicField { field } => {
                write!(formatter, "logic field `{field}` is read-only")
            }
            Self::StackAddressOutOfRange { address } => {
                write!(formatter, "device stack address out of range: {address}")
            }
        }
    }
}

/// Host-provided behavior for IC10 instructions that touch the outside world.
pub trait Ic10Environment {
    /// Reads a logic field from a device target.
    ///
    /// # Errors
    ///
    /// Returns an [`EnvironmentFault`] if the target or field cannot be read.
    fn load_logic(&mut self, target: DeviceTarget, field: &str) -> Result<f64, EnvironmentFault>;

    /// Writes a logic field on a device target.
    ///
    /// # Errors
    ///
    /// Returns an [`EnvironmentFault`] if the target or field cannot be written.
    fn store_logic(
        &mut self,
        target: DeviceTarget,
        field: &str,
        value: f64,
    ) -> Result<(), EnvironmentFault>;

    /// Reads a value from device stack memory.
    ///
    /// # Errors
    ///
    /// Returns an [`EnvironmentFault`] if the target or address cannot be read.
    fn get_stack(&mut self, target: DeviceTarget, address: usize) -> Result<f64, EnvironmentFault>;

    /// Writes a value to device stack memory.
    ///
    /// # Errors
    ///
    /// Returns an [`EnvironmentFault`] if the target or address cannot be written.
    fn put_stack(
        &mut self,
        target: DeviceTarget,
        address: usize,
        value: f64,
    ) -> Result<(), EnvironmentFault>;

    /// Returns whether execution should suspend after the current instruction.
    #[must_use]
    fn should_suspend_execution(&self) -> bool {
        false
    }
}

/// Environment used by standalone IC10 execution.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoEnvironment;

impl Ic10Environment for NoEnvironment {
    fn load_logic(&mut self, _target: DeviceTarget, _field: &str) -> Result<f64, EnvironmentFault> {
        Err(EnvironmentFault::WorldContextRequired {
            operation: EnvironmentOperation::LoadLogic,
        })
    }

    fn store_logic(
        &mut self,
        _target: DeviceTarget,
        _field: &str,
        _value: f64,
    ) -> Result<(), EnvironmentFault> {
        Err(EnvironmentFault::WorldContextRequired {
            operation: EnvironmentOperation::StoreLogic,
        })
    }

    fn get_stack(
        &mut self,
        _target: DeviceTarget,
        _address: usize,
    ) -> Result<f64, EnvironmentFault> {
        Err(EnvironmentFault::WorldContextRequired {
            operation: EnvironmentOperation::GetStack,
        })
    }

    fn put_stack(
        &mut self,
        _target: DeviceTarget,
        _address: usize,
        _value: f64,
    ) -> Result<(), EnvironmentFault> {
        Err(EnvironmentFault::WorldContextRequired {
            operation: EnvironmentOperation::PutStack,
        })
    }
}
