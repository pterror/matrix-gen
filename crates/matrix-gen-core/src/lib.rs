//! Multi-agent social simulator core.
//!
//! Type backbone for profile-grounded agents per MATRIX (Tang et al., 2024) §3.

pub mod agent;
pub mod memory;
pub mod oracle;
pub mod profile;

pub use agent::{Agent, AgentId, ClusterId};
pub use memory::{Memory, MemoryEntry, MemorySource};
pub use oracle::{EchoOracle, Oracle, OracleError, OracleRequest, ScriptedOracle};
pub use profile::{Goal, PlanStep, Profile};
