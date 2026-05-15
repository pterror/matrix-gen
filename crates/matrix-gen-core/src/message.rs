use serde::{Deserialize, Serialize};

use crate::agent::AgentId;
use crate::memory::Tick;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub from: AgentId,
    pub tick: Tick,
    pub content: String,
}
