use crate::agent::Agent;
use crate::memory::MemoryEntry;
use crate::oracle::OracleRequest;

const RECENT_MEMORY_LIMIT: usize = 8;

pub fn build_actor_request(agent: &Agent) -> OracleRequest {
    let system = build_system_prompt(agent);
    let user = build_user_prompt(agent);
    OracleRequest {
        system: Some(system),
        user,
    }
}

fn build_system_prompt(agent: &Agent) -> String {
    let mut s = String::new();
    if let Some(p) = &agent.profile.profession {
        s.push_str(&format!("You are a {p}. "));
    }
    if !agent.profile.interests.is_empty() {
        s.push_str(&format!(
            "Interests: {}. ",
            agent.profile.interests.join(", ")
        ));
    }
    if let Some(bio) = &agent.profile.bio {
        s.push_str(&format!("Bio: {bio}. "));
    }
    s.push_str(&format!("Your life goal: {}. ", agent.goal.description));
    s.push_str("You act proactively to pursue your goal, or react to what you observe.");
    s
}

fn build_user_prompt(agent: &Agent) -> String {
    let mut s = String::new();
    s.push_str("Plan:\n");
    for (i, step) in agent.goal.plan.iter().enumerate() {
        let mark = if step.completed { "x" } else { " " };
        s.push_str(&format!("  [{mark}] {i}. {}\n", step.description));
    }
    s.push_str("\nRecent memory:\n");
    let recent: Vec<&MemoryEntry> = agent
        .memory
        .entries
        .iter()
        .rev()
        .take(RECENT_MEMORY_LIMIT)
        .collect();
    for entry in recent.iter().rev() {
        s.push_str(&format!("  t={}: {}\n", entry.tick, entry.content));
    }
    s.push_str(
        "\nRespond with a single JSON object describing your next action. Schema:\n\
         {\"type\": \"speak\", \"content\": \"...\"}\n\
         {\"type\": \"react\", \"to\": <agent_id_u64>, \"content\": \"...\"}\n\
         {\"type\": \"advance_plan\", \"step\": <index>, \"content\": \"...\"}\n\
         {\"type\": \"reflect\", \"content\": \"...\"}\n\
         JSON only, no prose.",
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentId;
    use crate::memory::{MemoryEntry, MemorySource};
    use crate::profile::{Goal, PlanStep, Profile};

    fn make_agent() -> Agent {
        Agent::new(
            AgentId(1),
            Profile {
                profession: Some("botanist".into()),
                interests: vec!["fungi".into()],
                bio: Some("grew up near a forest".into()),
                historical_statements: vec![],
            },
            Goal {
                description: "catalog all local species".into(),
                plan: vec![
                    PlanStep {
                        description: "collect samples".into(),
                        completed: false,
                    },
                    PlanStep {
                        description: "write report".into(),
                        completed: true,
                    },
                ],
            },
        )
    }

    #[test]
    fn prompt_mentions_goal() {
        let agent = make_agent();
        let req = build_actor_request(&agent);
        assert!(req.system.unwrap().contains("catalog all local species"));
    }

    #[test]
    fn prompt_includes_plan_steps_with_indices() {
        let agent = make_agent();
        let req = build_actor_request(&agent);
        assert!(req.user.contains("0. collect samples"));
        assert!(req.user.contains("1. write report"));
    }

    #[test]
    fn prompt_includes_recent_memory_section() {
        let mut agent = make_agent();
        agent.memory.push(MemoryEntry {
            tick: 5,
            content: "observed something".into(),
            source: MemorySource::SelfAction,
        });
        let req = build_actor_request(&agent);
        assert!(req.user.contains("Recent memory:"));
        assert!(req.user.contains("t=5: observed something"));
    }
}
