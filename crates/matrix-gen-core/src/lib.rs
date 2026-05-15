//! Multi-agent social simulator core.
//!
//! Type backbone for profile-grounded agents per MATRIX (Tang et al., 2024) §3.

pub mod action;
pub mod agent;
pub mod cluster;
pub mod embedding;
pub mod memory;
pub mod message;
pub mod modulator;
pub mod oracle;
pub mod profile;
pub mod prompt;
pub mod scenario;
pub mod simulation;

pub use action::Action;
pub use agent::{Agent, AgentId, ClusterId};
pub use cluster::cluster_agents;
pub use embedding::{EmbedError, Embedder, HashEmbedder};
pub use memory::{Memory, MemoryEntry, MemorySource};
pub use message::Message;
pub use modulator::{BroadcastModulator, DropSelfModulator, Modulator, OracleModulator};
pub use oracle::{EchoOracle, Oracle, OracleError, OracleRequest, ScriptedOracle};
pub use profile::{Goal, PlanStep, Profile};
pub use prompt::build_actor_request;
pub use scenario::Scenario;
pub use simulation::{Simulation, Termination};
