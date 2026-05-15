use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use matrix_gen_core::{
    Agent, AgentId, BroadcastModulator, DropSelfModulator, EchoOracle, Goal, HashEmbedder,
    InstructionPair, Modulator, Oracle, OracleError, OracleModulator, OracleRequest, Profile,
    Scenario, ScriptedOracle, Simulation, Termination, cluster_agents, synthesize_instructions,
};
use serde::{Deserialize, Serialize};

/// Multi-agent social simulator that synthesizes instruction data.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Path to a JSON file containing an array of AgentSeed entries.
    #[arg(long)]
    profiles: PathBuf,

    /// How many simulation steps to run.
    #[arg(long, default_value_t = 12)]
    ticks: u64,

    /// How many instruction pairs to synthesize from the resulting scenario.
    #[arg(long, default_value_t = 4)]
    pairs: usize,

    /// Where to write the JSONL output (one InstructionPair per line).
    #[arg(long)]
    output: PathBuf,

    /// Optional: also write the raw scenario transcript here (JSON).
    #[arg(long)]
    scenario_out: Option<PathBuf>,

    /// Number of homophily clusters. 0 disables clustering.
    #[arg(long, default_value_t = 0)]
    clusters: usize,

    /// Backend mode.
    #[arg(long, value_enum, default_value_t = Backend::Mock)]
    backend: Backend,

    /// rig model id when --backend live. Default: claude-haiku-4-5-20251001.
    #[arg(long, default_value = "claude-haiku-4-5-20251001")]
    model: String,

    /// Path to a JSON array of scripted oracle responses (used when --backend mock-scripted).
    #[arg(long)]
    scripted: Option<PathBuf>,

    /// Use the LLM-driven modulator instead of broadcast.
    #[arg(long, default_value_t = false)]
    oracle_modulator: bool,

    /// Seed for K-means clustering.
    #[arg(long, default_value_t = 0)]
    seed: u64,
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq, Eq)]
enum Backend {
    /// EchoOracle — always returns the user prompt. Useful for smoke tests.
    Mock,
    /// ScriptedOracle from --scripted path.
    MockScripted,
    /// rig + Anthropic (requires ANTHROPIC_API_KEY).
    Live,
}

#[derive(Debug, Deserialize, Serialize)]
struct AgentSeed {
    id: u64,
    profile: Profile,
    goal: Goal,
}

/// Thin newtype so Arc<ScriptedOracle> satisfies the Oracle trait bound.
struct SharedOracle(Arc<ScriptedOracle>);

impl Oracle for SharedOracle {
    async fn complete(&self, request: &OracleRequest) -> Result<String, OracleError> {
        self.0.complete(request).await
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let seeds: Vec<AgentSeed> = serde_json::from_str(
        &std::fs::read_to_string(&args.profiles)
            .with_context(|| format!("read profiles {}", args.profiles.display()))?,
    )
    .context("parse profiles JSON")?;

    let mut agents: Vec<Agent> = seeds
        .into_iter()
        .map(|s| Agent::new(AgentId(s.id), s.profile, s.goal))
        .collect();

    if args.clusters > 0 {
        let embedder = HashEmbedder::new(64);
        cluster_agents(&mut agents, &embedder, args.clusters, args.seed)
            .await
            .context("cluster agents")?;
    }

    let scripted_responses: Option<Vec<String>> = match &args.backend {
        Backend::MockScripted => {
            let path = args
                .scripted
                .as_ref()
                .context("--scripted is required when --backend mock-scripted")?;
            let raw = std::fs::read_to_string(path)
                .with_context(|| format!("read scripted {}", path.display()))?;
            Some(serde_json::from_str(&raw).context("parse scripted JSON array")?)
        }
        _ => None,
    };

    // For MockScripted, the simulation and synthesis share one ScriptedOracle so
    // responses are consumed in sequence: sim ticks first, then synthesis.
    let shared_scripted: Option<Arc<ScriptedOracle>> = scripted_responses
        .as_ref()
        .map(|r| Arc::new(ScriptedOracle::new(r.clone())));

    let scenario = match (&args.backend, &args.oracle_modulator) {
        (Backend::Mock, false) => {
            run_sim(agents, EchoOracle, BroadcastModulator, args.ticks).await?
        }
        (Backend::Mock, true) => {
            run_sim(
                agents,
                EchoOracle,
                OracleModulator::new(EchoOracle),
                args.ticks,
            )
            .await?
        }
        (Backend::MockScripted, false) => {
            let arc = shared_scripted.clone().unwrap();
            run_sim(
                agents,
                SharedOracle(arc.clone()),
                DropSelfModulator,
                args.ticks,
            )
            .await?
        }
        (Backend::MockScripted, true) => {
            let arc = shared_scripted.clone().unwrap();
            run_sim(
                agents,
                SharedOracle(arc.clone()),
                OracleModulator::new(SharedOracle(arc.clone())),
                args.ticks,
            )
            .await?
        }
        (Backend::Live, oracle_modulator) => {
            run_live(agents, &args.model, args.ticks, *oracle_modulator).await?
        }
    };

    if let Some(out) = &args.scenario_out {
        std::fs::write(out, serde_json::to_string_pretty(&scenario)?)
            .with_context(|| format!("write scenario {}", out.display()))?;
    }

    let pairs = match &args.backend {
        Backend::Live => synth_live(&scenario, &args.model, args.pairs).await?,
        Backend::Mock => synthesize_instructions(&scenario, &EchoOracle, args.pairs).await?,
        Backend::MockScripted => {
            // shared_scripted continues from where the simulation left off.
            let arc = shared_scripted.unwrap();
            synthesize_instructions(&scenario, &SharedOracle(arc), args.pairs).await?
        }
    };

    write_jsonl(&args.output, &pairs)?;
    eprintln!(
        "wrote {} instruction pair(s) to {}",
        pairs.len(),
        args.output.display()
    );
    Ok(())
}

async fn run_sim<O: Oracle, M: Modulator>(
    agents: Vec<Agent>,
    oracle: O,
    modulator: M,
    ticks: u64,
) -> Result<Scenario> {
    let mut sim = Simulation::new(agents, oracle, modulator);
    sim.run(Termination::MaxTicks(ticks)).await?;
    Ok(sim.scenario().clone())
}

async fn run_live(
    agents: Vec<Agent>,
    model_id: &str,
    ticks: u64,
    oracle_modulator: bool,
) -> Result<Scenario> {
    use matrix_gen_rig::RigOracle;
    use matrix_gen_rig::rig::client::{CompletionClient, ProviderClient};
    use matrix_gen_rig::rig::providers::anthropic::Client;

    let client = Client::from_env();
    let oracle = RigOracle::new(client.completion_model(model_id));

    if oracle_modulator {
        let mod_client = Client::from_env();
        let mod_oracle = RigOracle::new(mod_client.completion_model(model_id));
        run_sim(agents, oracle, OracleModulator::new(mod_oracle), ticks).await
    } else {
        run_sim(agents, oracle, BroadcastModulator, ticks).await
    }
}

async fn synth_live(
    scenario: &Scenario,
    model_id: &str,
    count: usize,
) -> Result<Vec<InstructionPair>> {
    use matrix_gen_rig::RigOracle;
    use matrix_gen_rig::rig::client::{CompletionClient, ProviderClient};
    use matrix_gen_rig::rig::providers::anthropic::Client;
    let client = Client::from_env();
    let oracle = RigOracle::new(client.completion_model(model_id));
    Ok(synthesize_instructions(scenario, &oracle, count).await?)
}

fn write_jsonl(path: &std::path::Path, pairs: &[InstructionPair]) -> Result<()> {
    use std::io::Write;
    let mut f =
        std::fs::File::create(path).with_context(|| format!("create {}", path.display()))?;
    for p in pairs {
        writeln!(f, "{}", serde_json::to_string(p)?)?;
    }
    Ok(())
}
