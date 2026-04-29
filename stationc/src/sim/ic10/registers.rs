//! IC10 register file.

use std::fmt;

pub(super) const REGISTER_COUNT: usize = 16;
const REGISTER_COUNT_U8: u8 = 16;

#[derive(Debug)]
pub(super) struct Registers {
    values: [f64; REGISTER_COUNT],
    return_address: f64,
    stack_pointer: f64,
}

impl Registers {
    pub(super) const fn new() -> Self {
        Self {
            values: [0.0; REGISTER_COUNT],
            return_address: 0.0,
            stack_pointer: 0.0,
        }
    }

    pub(super) fn read(&self, reference: RegisterRef) -> Result<f64, RegisterFault> {
        match reference {
            RegisterRef::Direct(register) => Ok(self.values[register.as_usize()]),
            RegisterRef::ReturnAddress => Ok(self.return_address),
            RegisterRef::StackPointer => Ok(self.stack_pointer),
            RegisterRef::Indirect { base, depth } => {
                let register = self.resolve_indirect(base, depth)?;
                Ok(self.values[register.as_usize()])
            }
        }
    }

    pub(super) fn write(
        &mut self,
        reference: RegisterRef,
        value: f64,
    ) -> Result<(), RegisterFault> {
        match reference {
            RegisterRef::Direct(register) => self.values[register.as_usize()] = value,
            RegisterRef::ReturnAddress => self.return_address = value,
            RegisterRef::StackPointer => self.stack_pointer = value,
            RegisterRef::Indirect { base, depth } => {
                let register = self.resolve_indirect(base, depth)?;
                self.values[register.as_usize()] = value;
            }
        }
        Ok(())
    }

    fn resolve_indirect(
        &self,
        base: RegisterIndex,
        depth: u8,
    ) -> Result<RegisterIndex, RegisterFault> {
        let mut register = base;
        for _ in 0..depth {
            let value = self.values[register.as_usize()];
            register = RegisterIndex::from_numeric(value)?;
        }
        Ok(register)
    }

    pub(super) const fn stack_pointer(&self) -> f64 {
        self.stack_pointer
    }

    pub(super) const fn set_stack_pointer(&mut self, value: f64) {
        self.stack_pointer = value;
    }

    pub(super) const fn direct_values(&self) -> [f64; REGISTER_COUNT] {
        self.values
    }

    pub(super) const fn return_address(&self) -> f64 {
        self.return_address
    }
}

impl fmt::Display for Registers {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, value) in self.values.iter().enumerate() {
            write!(formatter, "r{index}={value}")?;
            if index + 1 < REGISTER_COUNT {
                formatter.write_str(" ")?;
            }
        }
        write!(
            formatter,
            " ra={} sp={}",
            self.return_address, self.stack_pointer
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RegisterRef {
    Direct(RegisterIndex),
    Indirect { base: RegisterIndex, depth: u8 },
    ReturnAddress,
    StackPointer,
}

impl RegisterRef {
    pub(super) fn parse(token: &str) -> Option<Self> {
        if token == "ra" {
            return Some(Self::ReturnAddress);
        }
        if token == "sp" {
            return Some(Self::StackPointer);
        }

        let r_count = token.bytes().take_while(|byte| *byte == b'r').count();
        if r_count == 0 {
            return None;
        }
        let digits = token.get(r_count..)?;
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        let parsed = digits.parse::<u8>().ok()?;
        let base = RegisterIndex::new(parsed)?;
        if r_count == 1 {
            Some(Self::Direct(base))
        } else {
            let depth = u8::try_from(r_count - 1).ok()?;
            Some(Self::Indirect { base, depth })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RegisterIndex {
    value: u8,
}

impl RegisterIndex {
    const fn new(value: u8) -> Option<Self> {
        if value < REGISTER_COUNT_U8 {
            Some(Self { value })
        } else {
            None
        }
    }

    fn from_numeric(value: f64) -> Result<Self, RegisterFault> {
        if !value.is_finite() || value.fract() != 0.0 {
            return Err(RegisterFault::InvalidIndirectIndex(value));
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let index = value as usize;
        if index >= REGISTER_COUNT {
            return Err(RegisterFault::InvalidIndirectIndex(value));
        }
        let value = u8::try_from(index).map_err(|_| RegisterFault::InvalidIndirectIndex(value))?;
        Ok(Self { value })
    }

    fn as_usize(self) -> usize {
        usize::from(self.value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum RegisterFault {
    InvalidIndirectIndex(f64),
}

impl fmt::Display for RegisterFault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIndirectIndex(value) => {
                write!(formatter, "invalid indirect register index `{value}`")
            }
        }
    }
}
