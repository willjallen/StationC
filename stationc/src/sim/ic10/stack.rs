//! IC10 stack memory.

use std::fmt;

pub(super) const STACK_SIZE: usize = 512;

#[derive(Debug)]
pub(super) struct Stack {
    values: [f64; STACK_SIZE],
}

impl Stack {
    pub(super) const fn new() -> Self {
        Self {
            values: [0.0; STACK_SIZE],
        }
    }

    pub(super) fn push(&mut self, stack_pointer: f64, value: f64) -> Result<f64, StackFault> {
        let address = stack_pointer_for_write(stack_pointer)?;
        self.values[address] = value;
        let next = u32::try_from(address + 1).map_err(|_| StackFault::AddressOutOfRange {
            address: stack_pointer,
            operation: StackOperation::Push,
        })?;
        Ok(f64::from(next))
    }

    pub(super) fn pop(&self, stack_pointer: f64) -> Result<(f64, f64), StackFault> {
        let address = stack_pointer_for_read(stack_pointer, StackOperation::Pop)?;
        let next = u32::try_from(address).map_err(|_| StackFault::AddressOutOfRange {
            address: stack_pointer,
            operation: StackOperation::Pop,
        })?;
        Ok((self.values[address], f64::from(next)))
    }

    pub(super) fn peek(&self, stack_pointer: f64) -> Result<f64, StackFault> {
        let address = stack_pointer_for_read(stack_pointer, StackOperation::Peek)?;
        Ok(self.values[address])
    }

    pub(super) fn poke(&mut self, address: f64, value: f64) -> Result<(), StackFault> {
        let address = stack_address(address, StackOperation::Poke)?;
        self.values[address] = value;
        Ok(())
    }

    pub(super) const fn values(&self) -> &[f64; STACK_SIZE] {
        &self.values
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StackOperation {
    Push,
    Pop,
    Peek,
    Poke,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum StackFault {
    AddressOutOfRange {
        address: f64,
        operation: StackOperation,
    },
}

impl fmt::Display for StackFault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AddressOutOfRange { address, operation } => {
                write!(
                    formatter,
                    "stack {operation:?} address out of range: {address}"
                )
            }
        }
    }
}

fn stack_pointer_for_write(stack_pointer: f64) -> Result<usize, StackFault> {
    let address = stack_address(stack_pointer, StackOperation::Push)?;
    if address < STACK_SIZE {
        Ok(address)
    } else {
        Err(StackFault::AddressOutOfRange {
            address: stack_pointer,
            operation: StackOperation::Push,
        })
    }
}

fn stack_pointer_for_read(
    stack_pointer: f64,
    operation: StackOperation,
) -> Result<usize, StackFault> {
    if stack_pointer < 1.0 {
        return Err(StackFault::AddressOutOfRange {
            address: stack_pointer,
            operation,
        });
    }
    stack_address(stack_pointer - 1.0, operation)
}

fn stack_address(address: f64, operation: StackOperation) -> Result<usize, StackFault> {
    if !address.is_finite() || address.fract() != 0.0 || address < 0.0 {
        return Err(StackFault::AddressOutOfRange { address, operation });
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let address_as_usize = address as usize;
    if address_as_usize >= STACK_SIZE {
        return Err(StackFault::AddressOutOfRange { address, operation });
    }
    Ok(address_as_usize)
}
