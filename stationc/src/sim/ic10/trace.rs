//! IC10 execution trace data.

use super::instruction::ProgramInstruction;

#[derive(Debug)]
pub(super) enum TraceSink {
    Disabled,
    Stdout,
    Capture(Vec<TraceEvent>),
}

impl TraceSink {
    pub(super) const fn disabled() -> Self {
        Self::Disabled
    }

    pub(super) const fn stdout(enabled: bool) -> Self {
        if enabled {
            Self::Stdout
        } else {
            Self::Disabled
        }
    }

    pub(super) const fn capture() -> Self {
        Self::Capture(Vec::new())
    }

    pub(super) fn instruction(&mut self, pc: usize, instruction: &ProgramInstruction) {
        match self {
            Self::Disabled => {}
            Self::Stdout => {
                println!(
                    "{pc:04} line {}: {}",
                    instruction.source_line, instruction.text
                );
            }
            Self::Capture(events) => events.push(TraceEvent {
                program_counter: pc,
                source_line: instruction.source_line,
                instruction: instruction.text.clone(),
            }),
        }
    }

    pub(super) fn into_events(self) -> Vec<TraceEvent> {
        match self {
            Self::Disabled | Self::Stdout => Vec::new(),
            Self::Capture(events) => events,
        }
    }
}

/// A structured record for one executed IC10 instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceEvent {
    /// Program counter before the instruction executed.
    pub program_counter: usize,
    /// One-based source line number from the IC10 source file.
    pub source_line: usize,
    /// Normalized instruction text after comment stripping and whitespace tokenization.
    pub instruction: String,
}
