use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstructionPair {
    pub instruction: String,
    pub response: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_round_trip_single() {
        let pair = InstructionPair {
            instruction: "What is bookbinding?".into(),
            response: "It is the craft of assembling books.".into(),
        };
        let json = serde_json::to_string(&pair).unwrap();
        let decoded: InstructionPair = serde_json::from_str(&json).unwrap();
        assert_eq!(pair, decoded);
    }

    #[test]
    fn serde_round_trip_vec() {
        let pairs = vec![
            InstructionPair {
                instruction: "What happened?".into(),
                response: "An agent greeted another.".into(),
            },
            InstructionPair {
                instruction: "Who spoke first?".into(),
                response: "Agent 0.".into(),
            },
        ];
        let json = serde_json::to_string(&pairs).unwrap();
        let decoded: Vec<InstructionPair> = serde_json::from_str(&json).unwrap();
        assert_eq!(pairs, decoded);
    }
}
