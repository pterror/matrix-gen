use crate::action::Action;
use crate::agent::{Agent, AgentId, ClusterId};
use crate::memory::{MemoryEntry, MemorySource, Tick};
use crate::message::Message;
use crate::modulator::Modulator;
use crate::oracle::{Oracle, OracleError};
use crate::prompt::build_actor_request;
use crate::scenario::Scenario;

pub enum Termination {
    MaxTicks(Tick),
    ScenarioMessageQuota(usize),
}

pub struct Simulation<O: Oracle, M: Modulator> {
    agents: Vec<Agent>,
    oracle: O,
    modulator: M,
    tick: Tick,
    scenario: Scenario,
}

impl<O: Oracle, M: Modulator> Simulation<O, M> {
    pub fn new(agents: Vec<Agent>, oracle: O, modulator: M) -> Self {
        Self {
            agents,
            oracle,
            modulator,
            tick: 0,
            scenario: Scenario::new(),
        }
    }

    pub fn tick(&self) -> Tick {
        self.tick
    }

    pub fn scenario(&self) -> &Scenario {
        &self.scenario
    }

    pub async fn step(&mut self) -> Result<(), OracleError> {
        let actor_idx = (self.tick as usize) % self.agents.len();
        let actor_id = self.agents[actor_idx].id;

        let req = build_actor_request(&self.agents[actor_idx]);
        let raw = self.oracle.complete(&req).await?;

        let action: Action = serde_json::from_str(raw.trim()).unwrap_or_else(|_| Action::Speak {
            content: raw.trim().to_string(),
        });

        if let Action::AdvancePlan { step, .. } = &action {
            let step = *step;
            if step < self.agents[actor_idx].goal.plan.len() {
                self.agents[actor_idx].goal.plan[step].completed = true;
            }
        }

        let tick = self.tick;
        self.agents[actor_idx].memory.push(MemoryEntry {
            tick,
            content: action.content().to_string(),
            source: MemorySource::SelfAction,
        });

        if action.is_broadcast() {
            let msg = Message {
                from: actor_id,
                tick,
                content: action.content().to_string(),
            };

            self.scenario.record(msg.clone());

            let sender_cluster = self.agents[actor_idx].cluster;
            let candidates: Vec<(AgentId, Option<ClusterId>)> = self
                .agents
                .iter()
                .filter(|a| a.id != actor_id)
                .map(|a| (a.id, a.cluster))
                .collect();

            let recipients = self
                .modulator
                .route(&msg, sender_cluster, &candidates)
                .await;

            for recipient_id in recipients {
                if let Some(agent) = self.agents.iter_mut().find(|a| a.id == recipient_id) {
                    agent.memory.push(MemoryEntry {
                        tick,
                        content: msg.content.clone(),
                        source: MemorySource::Observed { from: actor_id },
                    });
                }
            }
        }

        self.tick += 1;
        Ok(())
    }

    pub async fn run(&mut self, termination: Termination) -> Result<(), OracleError> {
        loop {
            match &termination {
                Termination::MaxTicks(n) => {
                    if self.tick >= *n {
                        break;
                    }
                }
                Termination::ScenarioMessageQuota(n) => {
                    if self.scenario.messages.len() >= *n {
                        break;
                    }
                }
            }
            self.step().await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Agent;
    use crate::memory::MemorySource;
    use crate::modulator::{BroadcastModulator, DropSelfModulator};
    use crate::oracle::ScriptedOracle;
    use crate::profile::{Goal, Profile};

    fn make_agents() -> Vec<Agent> {
        (0..3)
            .map(|i| {
                Agent::new(
                    AgentId(i),
                    Profile {
                        profession: None,
                        interests: vec![],
                        bio: None,
                        historical_statements: vec![],
                    },
                    Goal {
                        description: format!("goal {i}"),
                        plan: vec![],
                    },
                )
            })
            .collect()
    }

    #[tokio::test]
    async fn broadcast_three_ticks_round_robin() {
        let mut sim = Simulation::new(
            make_agents(),
            ScriptedOracle::new(["act A", "act B", "act C", "act D"]),
            BroadcastModulator,
        );
        sim.run(Termination::MaxTicks(3)).await.unwrap();

        assert_eq!(sim.tick(), 3);

        let msgs = &sim.scenario().messages;
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].content, "act A");
        assert_eq!(msgs[1].content, "act B");
        assert_eq!(msgs[2].content, "act C");

        assert_eq!(msgs[0].from, AgentId(0));
        assert_eq!(msgs[1].from, AgentId(1));
        assert_eq!(msgs[2].from, AgentId(2));

        let agent0 = &sim.agents[0];
        let self_actions: Vec<_> = agent0
            .memory
            .entries
            .iter()
            .filter(|e| matches!(e.source, MemorySource::SelfAction))
            .collect();
        let observed: Vec<_> = agent0
            .memory
            .entries
            .iter()
            .filter(|e| matches!(e.source, MemorySource::Observed { .. }))
            .collect();
        assert_eq!(self_actions.len(), 1);
        assert_eq!(observed.len(), 2);
    }

    #[tokio::test]
    async fn drop_self_modulator_same_outcome() {
        let mut sim = Simulation::new(
            make_agents(),
            ScriptedOracle::new(["act A", "act B", "act C", "act D"]),
            DropSelfModulator,
        );
        sim.run(Termination::MaxTicks(3)).await.unwrap();

        assert_eq!(sim.tick(), 3);
        assert_eq!(sim.scenario().messages.len(), 3);

        let agent0 = &sim.agents[0];
        let self_actions: Vec<_> = agent0
            .memory
            .entries
            .iter()
            .filter(|e| matches!(e.source, MemorySource::SelfAction))
            .collect();
        let observed: Vec<_> = agent0
            .memory
            .entries
            .iter()
            .filter(|e| matches!(e.source, MemorySource::Observed { .. }))
            .collect();
        assert_eq!(self_actions.len(), 1);
        assert_eq!(observed.len(), 2);
    }

    #[tokio::test]
    async fn reflect_stays_private() {
        let mut sim = Simulation::new(
            make_agents(),
            ScriptedOracle::new([r#"{"type":"reflect","content":"hmm"}"#]),
            BroadcastModulator,
        );
        sim.step().await.unwrap();

        assert!(sim.scenario().messages.is_empty());

        let self_actions: Vec<_> = sim.agents[0]
            .memory
            .entries
            .iter()
            .filter(|e| matches!(e.source, MemorySource::SelfAction))
            .collect();
        assert_eq!(self_actions.len(), 1);
        assert_eq!(self_actions[0].content, "hmm");

        for agent in &sim.agents[1..] {
            assert!(agent.memory.entries.is_empty());
        }
    }

    #[tokio::test]
    async fn advance_plan_marks_step_complete() {
        let agents = vec![Agent::new(
            AgentId(0),
            Profile {
                profession: None,
                interests: vec![],
                bio: None,
                historical_statements: vec![],
            },
            Goal {
                description: "finish project".into(),
                plan: vec![crate::profile::PlanStep {
                    description: "do the thing".into(),
                    completed: false,
                }],
            },
        )];
        let mut sim = Simulation::new(
            agents,
            ScriptedOracle::new([r#"{"type":"advance_plan","step":0,"content":"did it"}"#]),
            BroadcastModulator,
        );
        sim.step().await.unwrap();

        assert!(sim.agents[0].goal.plan[0].completed);
    }

    #[tokio::test]
    async fn malformed_json_falls_back_to_speak() {
        let mut sim = Simulation::new(
            make_agents(),
            ScriptedOracle::new(["not json at all"]),
            BroadcastModulator,
        );
        sim.step().await.unwrap();

        assert_eq!(sim.scenario().messages.len(), 1);
        assert_eq!(sim.scenario().messages[0].content, "not json at all");
    }

    #[tokio::test]
    async fn quota_termination_stops_early() {
        let mut sim = Simulation::new(
            make_agents(),
            ScriptedOracle::new(["act A", "act B", "act C", "act D"]),
            BroadcastModulator,
        );
        sim.run(Termination::ScenarioMessageQuota(2)).await.unwrap();

        assert_eq!(sim.scenario().messages.len(), 2);
        assert_eq!(sim.tick(), 2);
    }
}
