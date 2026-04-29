//! World tick result types.

use std::collections::HashMap;

use crate::sim::ic10::{ReferenceId, TickResult};

/// Default IC10 instruction budget for one world tick.
pub const IC10_INSTRUCTIONS_PER_TICK: u32 = 128;

/// Deterministic order used to run IC10 housings during one world tick.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Ic10Schedule {
    /// Run IC10 housings in stable world insertion order.
    #[default]
    Stable,
    /// Rotate the first housing each world tick.
    Rotating,
    /// Shuffle housing order deterministically from a seed and tick number.
    SeededShuffle {
        /// Caller-provided shuffle seed.
        seed: u64,
    },
}

/// Result of running one IC housing during a world tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ic10TickResult {
    /// The housing that ran.
    pub reference_id: ReferenceId,
    /// The IC10 tick result.
    pub tick: TickResult,
}

/// World-facing operation performed by one IC10 instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldAccessOperation {
    /// The IC read from the world.
    Read,
    /// The IC wrote to the world.
    Write,
}

/// Target of a world-facing IC10 operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WorldAccessTarget {
    /// A device logic field such as `Temperature` or `On`.
    DeviceLogic {
        /// Device or housing body that was accessed.
        reference_id: ReferenceId,
        /// Logic field name.
        field: String,
    },
    /// A device or housing stack address.
    DeviceStack {
        /// Device or housing body that was accessed.
        reference_id: ReferenceId,
        /// Stack address.
        address: usize,
    },
}

/// One successful world-facing IC10 access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldAccessEvent {
    /// World tick in which the access happened.
    pub tick: u64,
    /// IC housing that performed the access.
    pub actor: ReferenceId,
    /// Whether the access read or wrote.
    pub operation: WorldAccessOperation,
    /// Accessed target.
    pub target: WorldAccessTarget,
}

/// Debug diagnostic category produced from a world access trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldDiagnosticKind {
    /// More than one IC wrote the same target in one world tick.
    MultipleWritesSameTick,
    /// One IC read a target while another IC wrote it in the same world tick.
    ReadWriteSameTick,
    /// One IC read the same volatile logic field more than once in one tick.
    RepeatedVolatileRead,
}

/// Debug diagnostic produced from world access trace analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldDiagnostic {
    /// Diagnostic category.
    pub kind: WorldDiagnosticKind,
    /// Index of the earlier access in [`WorldTickResult::access`].
    pub first_access: usize,
    /// Index of the later access in [`WorldTickResult::access`].
    pub second_access: usize,
}

/// Result of advancing the world by one tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldTickResult {
    /// Zero-based world tick index that was advanced.
    pub tick: u64,
    /// Per-housing IC10 execution results in stable world order.
    pub ic10: Vec<Ic10TickResult>,
    /// Successful world-facing IC10 accesses performed during this tick.
    pub access: Vec<WorldAccessEvent>,
    /// Debug diagnostics derived from access events.
    pub diagnostics: Vec<WorldDiagnostic>,
}

pub(super) fn diagnostics_for_access(access: &[WorldAccessEvent]) -> Vec<WorldDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut previous_by_target: HashMap<WorldAccessTarget, usize> = HashMap::new();
    let mut previous_read_by_actor_and_target: HashMap<(ReferenceId, WorldAccessTarget), usize> =
        HashMap::new();

    for (index, event) in access.iter().enumerate() {
        if let Some(previous_index) = previous_by_target.get(&event.target).copied() {
            let previous = &access[previous_index];
            if previous.actor != event.actor {
                match (previous.operation, event.operation) {
                    (WorldAccessOperation::Write, WorldAccessOperation::Write) => {
                        diagnostics.push(WorldDiagnostic {
                            kind: WorldDiagnosticKind::MultipleWritesSameTick,
                            first_access: previous_index,
                            second_access: index,
                        });
                    }
                    (WorldAccessOperation::Read, WorldAccessOperation::Write)
                    | (WorldAccessOperation::Write, WorldAccessOperation::Read) => {
                        diagnostics.push(WorldDiagnostic {
                            kind: WorldDiagnosticKind::ReadWriteSameTick,
                            first_access: previous_index,
                            second_access: index,
                        });
                    }
                    (WorldAccessOperation::Read, WorldAccessOperation::Read) => {}
                }
            }
        }

        if event.operation == WorldAccessOperation::Read
            && matches!(event.target, WorldAccessTarget::DeviceLogic { .. })
        {
            let key = (event.actor, event.target.clone());
            if let Some(previous_index) = previous_read_by_actor_and_target.insert(key, index) {
                diagnostics.push(WorldDiagnostic {
                    kind: WorldDiagnosticKind::RepeatedVolatileRead,
                    first_access: previous_index,
                    second_access: index,
                });
            }
        }

        previous_by_target.insert(event.target.clone(), index);
    }

    diagnostics
}
