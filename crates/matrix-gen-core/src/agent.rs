use serde::{Deserialize, Serialize};

use crate::memory::Memory;
use crate::profile::{Goal, Profile};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClusterId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub profile: Profile,
    pub goal: Goal,
    pub memory: Memory,
    pub cluster: Option<ClusterId>,
}

impl Agent {
    pub fn new(id: AgentId, profile: Profile, goal: Goal) -> Self {
        let memory = Memory::with_history(profile.historical_statements.iter().map(String::as_str));
        Self {
            id,
            profile,
            goal,
            memory,
            cluster: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::PlanStep;

    #[test]
    fn new_populates_memory_from_history() {
        let profile = Profile {
            profession: Some("archivist".into()),
            interests: vec!["maps".into()],
            bio: None,
            historical_statements: vec!["catalogued the atlas collection".into()],
        };
        let goal = Goal {
            description: "digitize the archive".into(),
            plan: vec![PlanStep {
                description: "scan first batch".into(),
                completed: false,
            }],
        };
        let agent = Agent::new(AgentId(1), profile, goal);
        assert_eq!(agent.memory.entries.len(), 1);
        assert_eq!(
            agent.memory.entries[0].content,
            "catalogued the atlas collection"
        );
        assert_eq!(agent.cluster, None);

        let json = serde_json::to_string(&agent).unwrap();
        let back: Agent = serde_json::from_str(&json).unwrap();
        assert_eq!(agent, back);
    }
}
