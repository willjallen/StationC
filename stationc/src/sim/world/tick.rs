//! World tick result types.

use crate::sim::ic10::{ReferenceId, TickResult};

/// Default IC10 instruction budget for one world tick.
pub const IC10_INSTRUCTIONS_PER_TICK: u32 = 128;

/// Result of running one IC housing during a world tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ic10TickResult {
    /// The housing that ran.
    pub reference_id: ReferenceId,
    /// The IC10 tick result.
    pub tick: TickResult,
}

/// Result of advancing the world by one tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldTickResult {
    /// Zero-based world tick index that was advanced.
    pub tick: u64,
    /// Per-housing IC10 execution results in stable world order.
    pub ic10: Vec<Ic10TickResult>,
}
