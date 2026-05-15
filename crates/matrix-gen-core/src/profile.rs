use serde::{Deserialize, Serialize};

/// Anonymized real-world-grounded profile.
///
/// The paper draws these from public X (Twitter) profiles; the type is
/// source-agnostic and only requires the attributes the simulator uses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub profession: Option<String>,
    #[serde(default)]
    pub interests: Vec<String>,
    pub bio: Option<String>,
    /// Past statements grounding the agent's voice and prior behavior.
    #[serde(default)]
    pub historical_statements: Vec<String>,
}

/// A life goal plus a step-by-step plan to achieve it.
///
/// Generated once at initialization from the profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Goal {
    pub description: String,
    pub plan: Vec<PlanStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanStep {
    pub description: String,
    #[serde(default)]
    pub completed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_roundtrip() {
        let p = Profile {
            profession: Some("librarian".into()),
            interests: vec!["bookbinding".into(), "ferns".into()],
            bio: None,
            historical_statements: vec!["reshelved the entire 800s today".into()],
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: Profile = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn profile_missing_optional_fields() {
        let json = r#"{"profession":null,"bio":null}"#;
        let p: Profile = serde_json::from_str(json).unwrap();
        assert!(p.interests.is_empty());
        assert!(p.historical_statements.is_empty());
    }

    #[test]
    fn goal_with_plan() {
        let g = Goal {
            description: "publish a field guide".into(),
            plan: vec![
                PlanStep {
                    description: "draft outline".into(),
                    completed: true,
                },
                PlanStep {
                    description: "collect specimens".into(),
                    completed: false,
                },
            ],
        };
        let json = serde_json::to_string(&g).unwrap();
        let back: Goal = serde_json::from_str(&json).unwrap();
        assert_eq!(g, back);
    }
}
