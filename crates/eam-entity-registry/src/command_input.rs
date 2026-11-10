//! Command input: what the UI or automation engine asks the registry to execute.

use eam_domain::{EntityCommand, EntityId};

/// A request to run a [`EntityCommand`] on one entity.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandInput {
    /// Target entity.
    pub entity_id: EntityId,
    /// Command to run.
    pub command: EntityCommand,
}
