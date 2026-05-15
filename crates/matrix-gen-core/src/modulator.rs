use std::future::Future;

use crate::agent::{AgentId, ClusterId};
use crate::message::Message;
use crate::oracle::{Oracle, OracleRequest};

pub trait Modulator: Send + Sync {
    fn route<'a>(
        &'a self,
        msg: &'a Message,
        sender_cluster: Option<ClusterId>,
        candidates: &'a [(AgentId, Option<ClusterId>)],
    ) -> impl Future<Output = Vec<AgentId>> + Send + 'a;
}

pub struct BroadcastModulator;

impl Modulator for BroadcastModulator {
    fn route<'a>(
        &'a self,
        _msg: &'a Message,
        _sender_cluster: Option<ClusterId>,
        candidates: &'a [(AgentId, Option<ClusterId>)],
    ) -> impl Future<Output = Vec<AgentId>> + Send + 'a {
        let result: Vec<AgentId> = candidates.iter().map(|(id, _)| *id).collect();
        async move { result }
    }
}

pub struct DropSelfModulator;

impl Modulator for DropSelfModulator {
    fn route<'a>(
        &'a self,
        msg: &'a Message,
        _sender_cluster: Option<ClusterId>,
        candidates: &'a [(AgentId, Option<ClusterId>)],
    ) -> impl Future<Output = Vec<AgentId>> + Send + 'a {
        let result: Vec<AgentId> = candidates
            .iter()
            .filter(|(id, _)| *id != msg.from)
            .map(|(id, _)| *id)
            .collect();
        async move { result }
    }
}

pub struct OracleModulator<O: Oracle> {
    oracle: O,
}

impl<O: Oracle> OracleModulator<O> {
    pub fn new(oracle: O) -> Self {
        Self { oracle }
    }
}

impl<O: Oracle> OracleModulator<O> {
    async fn route_inner(
        &self,
        msg: &Message,
        sender_cluster: Option<ClusterId>,
        candidates: &[(AgentId, Option<ClusterId>)],
    ) -> Vec<AgentId> {
        let mut delivered = Vec::new();
        for (cand_id, cand_cluster) in candidates {
            let same_cluster = matches!(
                (sender_cluster, *cand_cluster),
                (Some(a), Some(b)) if a.0 == b.0
            );
            let relationship = if same_cluster {
                "same cluster"
            } else {
                "different cluster"
            };
            let user = format!(
                "Sender (agent {}) and recipient (agent {}) are in the {}.\n\
                 Message: {:?}\n\
                 Should the recipient receive this message? Reply with exactly YES or NO.",
                msg.from.0, cand_id.0, relationship, msg.content
            );
            let req = OracleRequest {
                system: Some(
                    "You are a message-routing gatekeeper. Deliver messages that are \
                     relevant to the recipient given the cluster relationship. Within a \
                     cluster, most messages should pass. Across clusters, only highly \
                     relevant messages should pass."
                        .to_string(),
                ),
                user,
            };
            match self.oracle.complete(&req).await {
                Ok(resp) if resp.trim().to_ascii_uppercase().starts_with("YES") => {
                    delivered.push(*cand_id);
                }
                Ok(_) => {}
                // On backend error, default to delivery (paper's modulators are advisory).
                Err(_) => delivered.push(*cand_id),
            }
        }
        delivered
    }
}

impl<O: Oracle> Modulator for OracleModulator<O> {
    fn route<'a>(
        &'a self,
        msg: &'a Message,
        sender_cluster: Option<ClusterId>,
        candidates: &'a [(AgentId, Option<ClusterId>)],
    ) -> impl Future<Output = Vec<AgentId>> + Send + 'a {
        self.route_inner(msg, sender_cluster, candidates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Tick;
    use crate::oracle::ScriptedOracle;

    fn make_msg(from: u64, tick: Tick) -> Message {
        Message {
            from: AgentId(from),
            tick,
            content: "test".into(),
        }
    }

    fn no_cluster(ids: &[u64]) -> Vec<(AgentId, Option<ClusterId>)> {
        ids.iter().map(|&i| (AgentId(i), None)).collect()
    }

    #[tokio::test]
    async fn broadcast_delivers_to_all_candidates() {
        let m = BroadcastModulator;
        let candidates = no_cluster(&[1, 2, 3]);
        let msg = make_msg(0, 0);
        let result = m.route(&msg, None, &candidates).await;
        assert_eq!(result, vec![AgentId(1), AgentId(2), AgentId(3)]);
    }

    #[tokio::test]
    async fn drop_self_excludes_sender() {
        let m = DropSelfModulator;
        let candidates = no_cluster(&[0, 1, 2]);
        let msg = make_msg(0, 0);
        let result = m.route(&msg, None, &candidates).await;
        assert_eq!(result, vec![AgentId(1), AgentId(2)]);
    }

    #[tokio::test]
    async fn drop_self_passes_through_when_sender_absent() {
        let m = DropSelfModulator;
        let candidates = no_cluster(&[1, 2]);
        let msg = make_msg(0, 0);
        let result = m.route(&msg, None, &candidates).await;
        assert_eq!(result, vec![AgentId(1), AgentId(2)]);
    }

    #[tokio::test]
    async fn oracle_modulator_yes_no_yes() {
        let m = OracleModulator::new(ScriptedOracle::new(["YES", "NO", "YES"]));
        let candidates = no_cluster(&[1, 2, 3]);
        let msg = make_msg(0, 0);
        let result = m.route(&msg, None, &candidates).await;
        assert_eq!(result, vec![AgentId(1), AgentId(3)]);
    }

    #[tokio::test]
    async fn oracle_modulator_case_insensitive_yes() {
        let m = OracleModulator::new(ScriptedOracle::new(["yes"]));
        let candidates = no_cluster(&[1]);
        let msg = make_msg(0, 0);
        let result = m.route(&msg, None, &candidates).await;
        assert_eq!(result, vec![AgentId(1)]);
    }

    #[tokio::test]
    async fn oracle_modulator_exhausted_falls_back_to_delivery() {
        let m = OracleModulator::new(ScriptedOracle::new(["YES"]));
        let candidates = no_cluster(&[1, 2]);
        let msg = make_msg(0, 0);
        let result = m.route(&msg, None, &candidates).await;
        assert_eq!(result, vec![AgentId(1), AgentId(2)]);
    }
}
