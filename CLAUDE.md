# CLAUDE.md

Behavioral rules for Claude Code in the matrix-gen repository.

## Project Overview

Multi-agent social simulator for synthesizing diverse instruction data

Part of the [rhi ecosystem](https://rhi.zone).

## Origin

A from-scratch Rust implementation of the architecture in [MATRIX (Tang et al., 2024)](https://arxiv.org/abs/2410.14251) — specifically §3 — extended into the full data-synthesis pipeline (MATRIX-Gen).

The paper's framework is named MATRIX. This project is named `matrix-gen` because the bare name "matrix" collides with the Matrix chat protocol (matrix.org) — search, package registries, and conversations all become ambiguous. The `-gen` suffix doubles as a nod to MATRIX-Gen, the scenario-driven instruction generator the paper layers on top of the simulator. Both the simulator substrate and the generator live in this repo.

Key design commitments:
- **Rust core.** The simulation loop is hot-path: K-means clustering over thousands of agent embeddings, modulator gating on every message. Rust over Python here.
- **Full pipeline, one repo.** Simulator + MATRIX-Gen synthesis + data export. The scope was chosen deliberately over a slimmer "just the substrate" framing; splitting later is cheaper than merging.
- **LLM as oracle, not agent.** Following the paper: agents are profile-driven state machines that consult an LLM for action/reaction generation. The LLM does not drive the loop.

## Architecture

Three layers, mirroring §3 of the paper:

1. **Profile-grounded agents** — biographical profile + life goal + plan + memory bank. Agents either pursue their plan or react to observations.
2. **Homophily-clustered communication** — constrained K-means over profile embeddings; LLM-powered modulators gate intra- and inter-cluster messages.
3. **Simulation loop** — initialize (anonymize, goal-generate, cluster) → execute (act, route) → terminate (goals met or scenario quota reached).

Scenarios produced by the simulator feed the MATRIX-Gen instruction synthesis layer.

Crate layout:
- `crates/matrix-gen-core/` — simulator substrate (agents, clusters, modulators, loop).

## Development

```bash
nix develop        # Enter dev shell
cargo test         # Run tests
cargo clippy       # Lint
cd docs && bun dev # Local docs
```

If a tool appears missing, you are outside `nix develop`. Do not assume the tool is unavailable to the project.

## Context Is The Only Scarce Resource

Every byte that enters the main session stays in the main session for its entire lifetime. File contents, command output, search results, page text — once read, it lingers in cache and shapes every downstream token. There is no "just looking."

**All exploration runs in subagents.** Investigations, audits, deep dives, surveys, "let me check," "let me find" — if the purpose of a tool sequence is to find out something you don't yet know, it runs in a subagent. Renaming the activity does not change what it is. The subagent returns a distilled summary; the raw output stays in the subagent.

The main session holds only the durable artifacts you are producing: the edit, the commit, the doc update.

**Subagent model tiers:**
- Opus — design, architecture, any subagent that itself spawns subagents.
- Sonnet — implementation, mechanical multi-file work, default exploration.

Use Opus for exploration only when the search requires architectural judgment, not lookup.

## Durability

Subagent reports, mid-session realizations, "I'll remember this" — none of these outlast the session. Anything worth keeping goes into CLAUDE.md, code, docs, or a commit. If it isn't written down, it is gone.

**Commit completed work immediately.** After tests pass, commit. After each phase of a multi-phase plan, commit. Uncommitted work is lost work, and accumulated uncommitted phases lose isolation as well.

**Docs change in the same commit as the code.** New pages enter the sidebar in that commit. There is no follow-up.

## Authenticity

When asked to analyze X, read X. Do not synthesize from conversation memory, prior summaries, or what the file probably says. Claims must correspond to evidence produced this session.

**Something unexpected is a signal.** Surprising output, anomalous numbers, a file containing what it shouldn't — stop and find out why. Do not accept the anomaly and proceed.

## Discipline

Corrections from the user are conversation, not material for new rules. A single correction does not warrant a CLAUDE.md edit. Rules are added when a failure mode is observed repeatedly and the rule names the failure it prevents.

Do not announce actions ("I will now…"). Act.

## Workflow

Batch checks to minimize round-trips:
```bash
cargo clippy --all-targets --all-features -- -D warnings && cargo test
```

After editing multiple files, run the full check once. `cargo fmt` runs in the pre-commit hook.

When the same change spans multiple crates, edit all files first, then build once.

`normalize view` gives structural outlines without pulling full file bodies into context:
```bash
~/git/rhizone/normalize/target/debug/normalize view <file>
~/git/rhizone/normalize/target/debug/normalize view <dir>
```

## Commit Convention

Conventional commits: `type(scope): message`

Types: `feat`, `fix`, `refactor`, `docs`, `chore`, `test`. Scope is optional but recommended for multi-crate repos.

## Hard Constraints

- No `--no-verify`. Fix the issue or fix the hook.
- No path dependencies in `Cargo.toml` — they couple repos and break independent publishing.
- No interactive git (`git add -p`, `git add -i`, `git rebase -i`) — these block on stdin and hang.
- No assuming a tool is missing without checking `nix develop`.
