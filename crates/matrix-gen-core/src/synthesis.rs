use crate::instruction::InstructionPair;
use crate::oracle::{Oracle, OracleError, OracleRequest};
use crate::scenario::Scenario;

const SYSTEM_PROMPT: &str = "You are a data-synthesis assistant. Given a transcript of a multi-agent social simulation, produce realistic instruction-following examples that a human user might ask, grounded in the scenario context. Each example is an {instruction, response} pair.";

pub fn build_synthesis_request(scenario: &Scenario, count: usize) -> OracleRequest {
    let mut transcript = String::new();
    for msg in &scenario.messages {
        transcript.push_str(&format!(
            "[t={} agent={}] {}\n",
            msg.tick, msg.from.0, msg.content
        ));
    }
    let user = format!(
        "Scenario transcript:\n{transcript}\n\
         Produce exactly {count} instruction-response pairs that a user might plausibly send to an assistant, with each response grounded in the scenario above.\n\
         Reply with a JSON array of objects: [{{\"instruction\": \"...\", \"response\": \"...\"}}, ...]\n\
         JSON only, no prose."
    );
    OracleRequest {
        system: Some(SYSTEM_PROMPT.to_string()),
        user,
    }
}

pub async fn synthesize_instructions<O: Oracle>(
    scenario: &Scenario,
    oracle: &O,
    count: usize,
) -> Result<Vec<InstructionPair>, OracleError> {
    if count == 0 || scenario.messages.is_empty() {
        return Ok(Vec::new());
    }
    let req = build_synthesis_request(scenario, count);
    let raw = oracle.complete(&req).await?;
    parse_pairs(&raw).map_err(|e| OracleError::Backend(format!("synthesis parse: {e}")))
}

fn parse_pairs(raw: &str) -> Result<Vec<InstructionPair>, serde_json::Error> {
    let trimmed = raw.trim();
    // Tolerate a code-fence wrapper: ```json\n...\n```
    let inner = strip_fence(trimmed).unwrap_or(trimmed);
    serde_json::from_str(inner)
}

fn strip_fence(s: &str) -> Option<&str> {
    let s = s
        .strip_prefix("```json")
        .or_else(|| s.strip_prefix("```"))?;
    let s = s.trim_start_matches('\n');
    s.strip_suffix("```").map(|s| s.trim_end_matches('\n'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentId;
    use crate::message::Message;
    use crate::oracle::ScriptedOracle;

    fn make_scenario(content: &str) -> Scenario {
        let mut s = Scenario::new();
        s.record(Message {
            from: AgentId(0),
            tick: 1,
            content: content.into(),
        });
        s
    }

    #[test]
    fn build_request_contains_transcript_content() {
        let scenario = make_scenario("hello from agent zero");
        let req = build_synthesis_request(&scenario, 2);
        assert!(req.system.is_some());
        assert!(req.user.contains("hello from agent zero"));
        assert!(req.user.contains("agent=0"));
        assert!(req.user.contains("t=1"));
    }

    #[tokio::test]
    async fn empty_scenario_returns_empty() {
        let oracle = ScriptedOracle::new([] as [&str; 0]);
        let result = synthesize_instructions(&Scenario::new(), &oracle, 3)
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn zero_count_returns_empty() {
        let oracle = ScriptedOracle::new([] as [&str; 0]);
        let scenario = make_scenario("some content");
        let result = synthesize_instructions(&scenario, &oracle, 0)
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn scripted_oracle_returns_parsed_pairs() {
        let oracle = ScriptedOracle::new([
            r#"[{"instruction":"What is bookbinding?","response":"It is the craft of assembling books."}]"#,
        ]);
        let scenario = make_scenario("agents discuss craft");
        let pairs = synthesize_instructions(&scenario, &oracle, 1)
            .await
            .unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].instruction, "What is bookbinding?");
        assert_eq!(pairs[0].response, "It is the craft of assembling books.");
    }

    #[tokio::test]
    async fn code_fenced_response_parses_correctly() {
        let fenced = "```json\n[{\"instruction\":\"x\",\"response\":\"y\"}]\n```";
        let oracle = ScriptedOracle::new([fenced]);
        let scenario = make_scenario("some content");
        let pairs = synthesize_instructions(&scenario, &oracle, 1)
            .await
            .unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].instruction, "x");
        assert_eq!(pairs[0].response, "y");
    }

    #[tokio::test]
    async fn malformed_json_returns_backend_error() {
        let oracle = ScriptedOracle::new(["not valid json {"]);
        let scenario = make_scenario("some content");
        let err = synthesize_instructions(&scenario, &oracle, 1)
            .await
            .unwrap_err();
        assert!(matches!(err, OracleError::Backend(_)));
        if let OracleError::Backend(msg) = err {
            assert!(msg.contains("synthesis parse:"));
        }
    }
}
