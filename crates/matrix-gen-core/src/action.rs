use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Public statement broadcast to other agents.
    Speak { content: String },
    /// Direct reaction to a specific observation, broadcast as a statement.
    React { to: u64, content: String },
    /// Mark a plan step complete; content is a short narration of doing it.
    AdvancePlan { step: usize, content: String },
    /// Silent internal note. Stored in own memory, not broadcast.
    Reflect { content: String },
}

impl Action {
    pub fn content(&self) -> &str {
        match self {
            Action::Speak { content }
            | Action::React { content, .. }
            | Action::AdvancePlan { content, .. }
            | Action::Reflect { content } => content,
        }
    }

    pub fn is_broadcast(&self) -> bool {
        !matches!(self, Action::Reflect { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speak_roundtrip() {
        let a = Action::Speak {
            content: "hello".into(),
        };
        let json = serde_json::to_string(&a).unwrap();
        let back: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(a, back);
    }

    #[test]
    fn react_roundtrip() {
        let a = Action::React {
            to: 7,
            content: "agreed".into(),
        };
        let json = serde_json::to_string(&a).unwrap();
        let back: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(a, back);
    }

    #[test]
    fn advance_plan_roundtrip() {
        let a = Action::AdvancePlan {
            step: 2,
            content: "did it".into(),
        };
        let json = serde_json::to_string(&a).unwrap();
        let back: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(a, back);
    }

    #[test]
    fn reflect_roundtrip() {
        let a = Action::Reflect {
            content: "hmm".into(),
        };
        let json = serde_json::to_string(&a).unwrap();
        let back: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(a, back);
    }

    #[test]
    fn content_returns_inner_string() {
        assert_eq!(
            Action::Speak {
                content: "s".into()
            }
            .content(),
            "s"
        );
        assert_eq!(
            Action::React {
                to: 0,
                content: "r".into()
            }
            .content(),
            "r"
        );
        assert_eq!(
            Action::AdvancePlan {
                step: 0,
                content: "a".into()
            }
            .content(),
            "a"
        );
        assert_eq!(
            Action::Reflect {
                content: "f".into()
            }
            .content(),
            "f"
        );
    }

    #[test]
    fn is_broadcast_reflect_is_false() {
        assert!(
            !Action::Reflect {
                content: "x".into()
            }
            .is_broadcast()
        );
        assert!(
            Action::Speak {
                content: "x".into()
            }
            .is_broadcast()
        );
        assert!(
            Action::React {
                to: 0,
                content: "x".into()
            }
            .is_broadcast()
        );
        assert!(
            Action::AdvancePlan {
                step: 0,
                content: "x".into()
            }
            .is_broadcast()
        );
    }
}
