//! Parsed IC10 program storage.

use std::collections::HashMap;

use super::instruction::ProgramInstruction;

#[derive(Debug)]
pub(super) struct Program {
    instructions: Vec<ProgramInstruction>,
    labels: HashMap<String, usize>,
    constants: HashMap<String, f64>,
}

impl Program {
    pub(super) const fn new(
        instructions: Vec<ProgramInstruction>,
        labels: HashMap<String, usize>,
        constants: HashMap<String, f64>,
    ) -> Self {
        Self {
            instructions,
            labels,
            constants,
        }
    }

    pub(super) fn instruction(&self, pc: usize) -> Option<&ProgramInstruction> {
        self.instructions.get(pc)
    }

    pub(super) fn label(&self, name: &str) -> Option<usize> {
        self.labels.get(name).copied()
    }

    pub(super) fn constant(&self, name: &str) -> Option<f64> {
        self.constants.get(name).copied()
    }

    // Keep this non-const for stable toolchains where `Vec::len` is not const.
    #[allow(clippy::missing_const_for_fn)]
    pub(super) fn len(&self) -> usize {
        self.instructions.len()
    }
}
