# Introduction

`matrix-gen` is a multi-agent simulator that produces realistic social scenarios as raw material for instruction-tuning data synthesis. It is a from-scratch Rust implementation of the architecture described in [MATRIX (Tang et al., 2024)](https://arxiv.org/abs/2410.14251).

## Architecture

Three layers, mirroring §3 of the paper:

1. **Real-world-grounded agents.** Each agent is anchored to a profile (biographical attributes, interests, prior statements), assigned a life goal, and decomposes that goal into a plan. Agents act proactively — they pursue the plan or react to observations from memory rather than responding to random prompts.

2. **Homophily-guided communication.** Agents are grouped by profile-embedding similarity (constrained K-means). Within a group, an LLM-powered *modulator* routes messages by relevance; across groups, the modulators gate communication semantically. This avoids the quadratic blow-up of all-to-all messaging and respects the sociological observation that similar people cluster.

3. **Simulation loop.** Initialize (anonymize, generate goals, cluster) → execute (agents act, modulators route) → terminate (goals met or scenario quota reached). The output is a corpus of scenarios.

`matrix-gen` then composes those scenarios into instruction data via a downstream synthesis pass.

## Quickstart

```bash
nix develop
cargo run -p matrix-gen -- \
  --profiles crates/matrix-gen/examples/profiles.json \
  --backend mock-scripted \
  --scripted crates/matrix-gen/examples/scripted.json \
  --ticks 6 --pairs 4 --clusters 2 \
  --output out.jsonl
```

Live mode (requires `ANTHROPIC_API_KEY`):

```bash
ANTHROPIC_API_KEY=... cargo run -p matrix-gen -- \
  --profiles crates/matrix-gen/examples/profiles.json \
  --backend live --output out.jsonl
```

## Status

Status: §3 architecture and MATRIX-Gen synthesis layer complete. CLI runs end-to-end against a mocked or live LLM backend.
