# Monade — product brief v0 (consultants' session 10, 2026-11-18)

**Status:** first product thinking by the consultants (C1–C8, see `claude/agents/consiglio.md`). **Decided** = confirmed by the Demiurge. **Proposed** = consultants' recommendation. Market facts carry their sources; verify before using them outside the team.

## Concept (Decided, the Demiurge's words)

**A universal representation machine.** One algebra (XOR, majority, permutation, comparison) and one generated space represent anything (a word, an atom, a scene, a program) and compose it; Logos computes on it; the network learns it.

Logician's note: universal in principle (vector-symbolic architectures can encode any compositional structure), bounded in practice by dimension and capacity. Every claim of universality is backed by a test.

## Principles of the product team (Proposed)

- Start from the product: what someone can use, see and measure.
- Efficient, powerful, intelligent, capable, in that order of proof: each claim measured against a baseline.
- Reuse open source where possible and efficient (Decided as a criterion of good programming).

## Landscape (C2, analyst; first scan, 2026-11-18)

- **Neuro-symbolic AI is funded and growing.** AUI raised $20M at a $750M valuation cap for a neuro-symbolic model (Apollo-1); Lanyon AI raised $10.6M for provably correct scientific computing; sci2sci (Berlin) raised €1.2M for a neuro-symbolic memory layer for regulated industries. Most of them pair a large language model with symbolic reasoning.
- **Hyperdimensional computing is mostly hardware and sensors.** Examples: an FPGA chip that learns inside sensors (MINDS); a TUM chip that recognises patterns without millions of training samples.
- **Binary vectors at the edge are proven for search.** Binary quantisation with Hamming search: about 30× less memory and 15–45× faster than float32, keeping over 95% retrieval accuracy with rescoring. Embedded, deterministic vector search for offline devices exists (MonaVec, Qdrant Edge).
- **Where the Monade differs (to prove):** software on an ordinary CPU, no GPU, no large language model inside; learns continuously; deterministic; every idea carries its derivation (explainable); composes structure, not only similarity.

## Product candidates (C3, product manager)

| # | Product | For whom | Built on steps |
|---|---|---|---|
| 1 | **Memory that explains:** an embeddable library + command line + HTML report. Store anything as Monadi, recall by similarity, compose with roles, answer "why" with the recipe. Local, deterministic, on CPU | developers of local and offline AI; regulated sectors that need explanations | 4–6, 11 |
| 2 | Learning classifier and anomaly detector for signals and sensors, learning on the device | industry, IoT | 7–10 |
| 3 | Open research platform and benchmark: binary predictive coding, continual learning | universities, research | all |

**Recommendation (Proposed):** product 1 first: the smallest thing that is useful, visible and measurable, and it is exactly what steps 3b–6 and 11 build.

## How to make it more efficient, powerful, intelligent, capable (Proposed)

- **Efficient:** reuse proven libraries (`blake3`; later parallelism, memory-mapped files, JSON); measure joules per answer and show it in the report.
- **Powerful:** composition with roles (step 11), then Logos (step 12).
- **Intelligent:** continual learning with sleep (step 14); always compared with baselines (nearest neighbour, float vector search, small language model).
- **Capable:** adapters, text first, then signals, then images.

## Risks (C1, C2, red team)

- **Name:** "MonaVec" by "Mona" already exists in edge vector search; check the trademark "Monade" (C1).
- **Overclaiming:** "universal" must be shown, not stated; hyperdimensional methods trail deep networks on complex perception.
- **Legal questions for C1 (Italy, EU, USA):** ownership and protection of code written with Claude's help; licences of dependencies; copyright of training texts (EU text and data mining rules); GDPR for any personal data; AI Act classification for each intended use; licence of the Monade itself (permissive, copyleft or open core).

## Sources

- AUI: https://pulse2.com/aui-20-million-at-750-million-valuation-raised-following-breakthrough-in-neuro-symbolic-ai/amp/
- Lanyon AI: https://thesaasnews.com/news/lanyon-ai-emerges-from-stealth-to-build-the-future-of-scientific-and-technical-computing
- sci2sci: https://www.startupresearcher.com/news/sci2sci-raises-eur1-2-million-in-pre-seed-funding
- MINDS: https://techconnectworld.com/World2026/showcase/innovation.php?a=2223
- TUM chip: https://www.mirmi.tum.de/en/mirmi/news/research/article/neuartiger-ki-chip-arbeitet-dezentral-und-energiesparend
- Binary quantisation: https://app.readytensor.ai/publications/scaling-semantic-search-to-41-million-documents-with-binary-quantized-embeddings-BMKDEIBrpL9l
- MonaVec: https://arxiv.org/html/2606.19458
- BLAKE3 licence: https://docs.rs/crate/blake3/1.8.4/source/Cargo.toml
