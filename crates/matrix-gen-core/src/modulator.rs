use std::future::Future;

use crate::agent::AgentId;
use crate::message::Message;

pub trait Modulator: Send + Sync {
    fn route(
        &self,
        msg: &Message,
        candidates: &[AgentId],
    ) -> impl Future<Output = Vec<AgentId>> + Send;
}

pub struct BroadcastModulator;

impl Modulator for BroadcastModulator {
    fn route(
        &self,
        _msg: &Message,
        candidates: &[AgentId],
    ) -> impl Future<Output = Vec<AgentId>> + Send {
        let result = candidates.to_vec();
        async move { result }
    }
}

pub struct DropSelfModulator;

impl Modulator for DropSelfModulator {
    fn route(
        &self,
        msg: &Message,
        candidates: &[AgentId],
    ) -> impl Future<Output = Vec<AgentId>> + Send {
        let result: Vec<AgentId> = candidates
            .iter()
            .copied()
            .filter(|&id| id != msg.from)
            .collect();
        async move { result }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Tick;

    fn make_msg(from: u64, tick: Tick) -> Message {
        Message {
            from: AgentId(from),
            tick,
            content: "test".into(),
        }
    }

    #[tokio::test]
    async fn broadcast_delivers_to_all_candidates() {
        let m = BroadcastModulator;
        let candidates = vec![AgentId(1), AgentId(2), AgentId(3)];
        let msg = make_msg(0, 0);
        let result = m.route(&msg, &candidates).await;
        assert_eq!(result, candidates);
    }

    #[tokio::test]
    async fn drop_self_excludes_sender() {
        let m = DropSelfModulator;
        let candidates = vec![AgentId(0), AgentId(1), AgentId(2)];
        let msg = make_msg(0, 0);
        let result = m.route(&msg, &candidates).await;
        assert_eq!(result, vec![AgentId(1), AgentId(2)]);
    }

    #[tokio::test]
    async fn drop_self_passes_through_when_sender_absent() {
        let m = DropSelfModulator;
        let candidates = vec![AgentId(1), AgentId(2)];
        let msg = make_msg(0, 0);
        let result = m.route(&msg, &candidates).await;
        assert_eq!(result, candidates);
    }
}
