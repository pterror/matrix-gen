use rand::SeedableRng;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;

use crate::agent::{Agent, ClusterId};
use crate::embedding::{EmbedError, Embedder};

const KMEANS_MAX_ITERS: usize = 32;

/// Build the text we embed for each agent: profession, interests, bio, and
/// historical statements concatenated. Matches the paper's intuition that
/// clustering should reflect the agent's whole profile, not a single field.
fn profile_text(agent: &Agent) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(p) = &agent.profile.profession {
        parts.push(p.clone());
    }
    parts.extend(agent.profile.interests.iter().cloned());
    if let Some(b) = &agent.profile.bio {
        parts.push(b.clone());
    }
    parts.extend(agent.profile.historical_statements.iter().cloned());
    parts.join(" ")
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn argmax_cosine(point: &[f32], centroids: &[Vec<f32>]) -> usize {
    let mut best = 0usize;
    let mut best_score = f32::NEG_INFINITY;
    for (i, c) in centroids.iter().enumerate() {
        let s = dot(point, c);
        if s > best_score {
            best_score = s;
            best = i;
        }
    }
    best
}

pub async fn cluster_agents<E: Embedder>(
    agents: &mut [Agent],
    embedder: &E,
    k: usize,
    seed: u64,
) -> Result<(), EmbedError> {
    if agents.is_empty() || k == 0 {
        return Ok(());
    }
    let k = k.min(agents.len());

    let mut points: Vec<Vec<f32>> = Vec::with_capacity(agents.len());
    for agent in agents.iter() {
        let text = profile_text(agent);
        points.push(embedder.embed(&text).await?);
    }

    // Initial centroids: random distinct points (k-means++-lite would be nicer
    // but determinism + simplicity matter more than initialization quality here).
    let mut rng = SmallRng::seed_from_u64(seed);
    let mut idx: Vec<usize> = (0..agents.len()).collect();
    idx.shuffle(&mut rng);
    let mut centroids: Vec<Vec<f32>> = idx.iter().take(k).map(|&i| points[i].clone()).collect();

    let mut assignments = vec![0usize; agents.len()];
    for _ in 0..KMEANS_MAX_ITERS {
        let mut changed = false;
        for (i, p) in points.iter().enumerate() {
            let a = argmax_cosine(p, &centroids);
            if assignments[i] != a {
                assignments[i] = a;
                changed = true;
            }
        }
        if !changed {
            break;
        }

        // Recompute centroids as the L2-normalized mean of assigned points.
        let dim = points[0].len();
        let mut sums = vec![vec![0.0f32; dim]; k];
        let mut counts = vec![0usize; k];
        for (i, p) in points.iter().enumerate() {
            let a = assignments[i];
            for (s, x) in sums[a].iter_mut().zip(p) {
                *s += *x;
            }
            counts[a] += 1;
        }
        for (c, (sum, count)) in centroids.iter_mut().zip(sums.iter().zip(counts.iter())) {
            if *count == 0 {
                continue;
            }
            for (slot, s) in c.iter_mut().zip(sum) {
                *slot = *s / *count as f32;
            }
            let norm: f32 = c
                .iter()
                .map(|x| x * x)
                .sum::<f32>()
                .sqrt()
                .max(f32::EPSILON);
            for x in c.iter_mut() {
                *x /= norm;
            }
        }
    }

    for (agent, a) in agents.iter_mut().zip(assignments.iter()) {
        agent.cluster = Some(ClusterId(*a as u32));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentId;
    use crate::embedding::HashEmbedder;
    use crate::profile::{Goal, PlanStep, Profile};

    fn make_agent(id: u64, profession: &str, interests: &[&str]) -> Agent {
        let profile = Profile {
            profession: Some(profession.into()),
            interests: interests.iter().map(|s| s.to_string()).collect(),
            bio: None,
            historical_statements: vec![],
        };
        let goal = Goal {
            description: "do things".into(),
            plan: vec![PlanStep {
                description: "step".into(),
                completed: false,
            }],
        };
        Agent::new(AgentId(id), profile, goal)
    }

    #[tokio::test]
    async fn librarians_and_welders_cluster_separately() {
        let mut agents = vec![
            make_agent(0, "librarian", &["bookbinding", "ferns"]),
            make_agent(1, "librarian", &["bookbinding", "ferns"]),
            make_agent(2, "welder", &["metallurgy", "sparks"]),
            make_agent(3, "welder", &["metallurgy", "sparks"]),
        ];
        let embedder = HashEmbedder::new(32);
        cluster_agents(&mut agents, &embedder, 2, 42).await.unwrap();

        let c0 = agents[0].cluster.unwrap();
        let c1 = agents[1].cluster.unwrap();
        let c2 = agents[2].cluster.unwrap();
        let c3 = agents[3].cluster.unwrap();

        assert_eq!(c0, c1, "librarians should share a cluster");
        assert_eq!(c2, c3, "welders should share a cluster");
        assert_ne!(
            c0, c2,
            "librarians and welders should be in different clusters"
        );
    }

    #[tokio::test]
    async fn empty_slice_is_ok() {
        let mut agents: Vec<Agent> = vec![];
        let embedder = HashEmbedder::new(8);
        cluster_agents(&mut agents, &embedder, 2, 0).await.unwrap();
    }

    #[tokio::test]
    async fn k_zero_assigns_nothing() {
        let mut agents = vec![make_agent(0, "baker", &["bread"])];
        let embedder = HashEmbedder::new(8);
        cluster_agents(&mut agents, &embedder, 0, 0).await.unwrap();
        assert!(agents[0].cluster.is_none());
    }

    #[tokio::test]
    async fn k_larger_than_agents_clamps_gracefully() {
        let mut agents = vec![
            make_agent(0, "painter", &["oils"]),
            make_agent(1, "sculptor", &["clay"]),
        ];
        let embedder = HashEmbedder::new(8);
        cluster_agents(&mut agents, &embedder, 10, 7).await.unwrap();
        for agent in &agents {
            assert!(agent.cluster.is_some());
        }
    }
}
