# matrix-gen

Multi-agent social simulator that synthesizes instruction-tuning data from realistic social scenarios. Rust implementation of [MATRIX (Tang et al., 2024, arXiv:2410.14251)](https://arxiv.org/abs/2410.14251) §3.

## Crate layout

| Crate | Role |
|---|---|
| `matrix-gen-core` | Agents, profiles, memory, oracle/embedder traits, simulation loop, K-means, synthesis |
| `matrix-gen-rig` | `RigOracle<M>` / `RigEmbedder<E>` adapting rig-core to core traits |
| `matrix-gen` | CLI binary — loads profiles, runs simulation, synthesizes instruction pairs |

## Quickstart (mocked, no API key)

```bash
nix develop
cargo run -p matrix-gen -- \
  --profiles crates/matrix-gen/examples/profiles.json \
  --backend mock-scripted \
  --scripted crates/matrix-gen/examples/scripted.json \
  --ticks 6 --pairs 4 --clusters 2 \
  --output out.jsonl
```

## Live (Anthropic)

```bash
ANTHROPIC_API_KEY=... cargo run -p matrix-gen -- \
  --profiles crates/matrix-gen/examples/profiles.json \
  --backend live \
  --ticks 12 --pairs 8 \
  --output out.jsonl
```

## Tests

```bash
cargo test --workspace
```
