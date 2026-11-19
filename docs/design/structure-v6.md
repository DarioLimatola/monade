<!-- Copy of the technical design from the claude.ai project "Tesseract" (2026-11-19). -->
# Monade — structure, version 6 (2026-11-18)

**Status.**
- The single source of truth for the design. It gathers structure v5, the parliament (council session 9), Logos, and councils 11–16.
- Those documents remain as records with the details and the reasons: `claude/tesseract-structure-v5.md`, `claude/monade-parliament.md`, `claude/monade-council-1x-*.md`.
- When two documents disagree, **this one holds**.
- **Decided** = confirmed by the author · **Built** = in the code (version) · **Proposed** = council recommendation · **Open** = needs the author.
- This copy in the repository holds the technical design only (product, business and legal matters stay outside the repository).

## 1. Principles (Decided)

1. **Simplicity, elegance, recursion.** Little code, clear, the same rule at every scale. Do much with little.
2. **Count and compare.** Bits and integers in the core; no floating point in the core.
3. **Growth by 8 as a criterion of elegance**, derived from the hardware (byte, 64-bit word, 64-byte cache line), never hard-coded as n = 8 and always checked by data.
4. **An ordinary laptop first** (Windows, AVX2), then scale.
5. **Reuse open source** with permissive licences; write our own only when nothing fits. Using good libraries is good programming.
6. **Conventional technical terms** whenever they exist (e.g. *seed*).
7. **The author designs and decides;** the mind may propose; only the author writes point 0 and the specifications.
8. **The method of the council is handed down to the mind:** many voices, one shared space, a vote, the minority heard (the residue), an independent judge, objections that can block, sealed tests.
9. **Minorities are heard and respected**, provided they respect others and do no harm (to be written into point 0).
10. **Good, beautiful, true** are regulative ideas, not rules reducible to strings. No ethical speculation until an ethics specialist joins.
11. **The repository holds only the engine and its technical design.**
12. **Design before code:** each step is outlined, then built, with a green `cargo all`.

## 2. Vocabulary (Decided)

| Term | Meaning |
|---|---|
| **Point 0** | the origin: empty coordinate (v ⊕ 0 = v) and the log of foundations; inputs pass through it, never stay |
| **Vector = axis** | a binary vector, standing for one axis |
| **Dimension** | a vector + its meaning |
| **Measure** | the position of a point along a dimension; in ℤ, axis by axis, zero at the centre of the scale (signed: N and −N) |
| **Coordinates** | measures + metadimensions |
| **Tensor** | vectors joined; free number of axes, at most 64; beyond, a metatensor |
| **Metatensor** | a group of tensors or vectors: at most 64 members, depth at most 8 |
| **Monade** | the recursive structure from 64 bits to 2 MB (8² … 8⁸ bits) |
| **Reference** | a 64-bit word that points to something already in memory |
| **Idea** | a binary vector (64 bits … 2 MB), the inner language of the mind |
| **Word** (linguistic) | a readable translation of an idea, produced when a human reads |
| **Metadata** | in potency: up to 8 per datum, costing nothing until used |

The names value / domain / entity / scene are arbitrary. Dimensions and metadimensions take their meaning from what is represented.

## 3. Point 0 and the specifications (Decided; mechanism Built v0.1)

- **Point 0 as object:** empty until the end. When written, its principles are immutable ("the tablets"). Only the author writes; the mind may propose amendments; limits and values may be extended, never deleted ("the numbers can change, not the moral foundation").
- **Mechanism (Built):** append-only, readable log; ids never reused; amendments by a new entry; canonical form; a hand edit is refused.
- **Specifications:** a separate, versioned registry, mutable only by the author. Two entries are **locked**: the seed `0x4d4f4e4144450001` and the hash version 1. A new seed is a new mind.

## 4. The generated space (Decided; Built v0.4)

Basic vectors are computed, never stored: BLAKE3 keyed from the seed (context "monade space v1"), extendable output read in 64-byte blocks (random access), from an id or a name. The same on every machine; golden fingerprints fixed. The same hash serves content addresses (128 bits) and scars, separated by context.

## 5. The Monade (Decided; Built v0.1–v0.3)

| Size | Bits | Role |
|---|---|---|
| word | 64 | measures, references, counters |
| block | 512 | comparison; one cache line; one blackboard level |
| symbol | 4096 | composition, large memories |
| … | 8⁵ … 8⁷ | |
| thought | 2 MB (8⁸) | the largest single Monade; beyond: references |

- **Operations:** XOR (binding), AND (overlap), popcount, Hamming distance, majority (a tie gives 0, suspension of judgement), on any size.
- **Octal address (Built v0.3):** a root + up to 8 base-8 digits; down / up / another root; each digit an octant, so the path is the Morton order of the cube (2 MB = 256³). Packed in 56 bits. One addressing rule for experts, Monadi and bits.
- **Recursion beyond 2 MB (Proposed):** a cell can be the root of another Monade (a metatensor). This is how positions finer than one cell, nested scales (a solar system in a star's cell) and large data are reached.
- **Two algebras** (Proposed): bit-wise and byte-wise, declared per tensor. Dense vectors for binding; sparse for search and memory.

## 6. Three levels of computation (Decided)

| Level | Where | Arithmetic | What |
|---|---|---|---|
| 1 Core | CPU | bits, integers, deterministic | the mind |
| 2 Accelerator | GPU (`wgpu`) | integers, deterministic | the same operations on large batches, only when measured necessary |
| 3 The continuous | CPU/GPU | floating point | adapters, worlds, teachers, analyses, rendering |

The continuous enters the mind only as the **sign of random projections** generated from the seed (Charikar). Floating point never enters the mind's state or its hash.

## 7. Memory

- **Counters per bit, the tally (Built v0.5; Decided):** the bit is the reading, the counter is the memory.
  - `i8`, symmetric ±127, saturating, never wrapping.
  - add / subtract / read (sign; a tie reads 0); certainty; uncertain mask; residue (voice ⊕ reading); merge; decay (towards 0, never across).
  - `is_exact()` says whether anything was clamped or decayed.
- **Weights (Decided):**
  - powers of 2: 1, 2, 4, 8;
  - reliability measured against verified outcomes, per domain, never against consensus nor on test data;
  - echoes discounted by the similarity of residues;
  - no negative weights (subtract instead).
- **Counter width = how long memory lasts (Decided):** it is not precision. Variants (width, decay, vote probability) are genes, compared with pre-registered margins on unseen data; at least one variant of each family kept alive.
- **Fast and slow tallies (Proposed, step 5d):** fast for episodes and change; slow for what is stable; transfer in sleep (complementary learning systems).
- **Memory as an ecosystem (Proposed):** prototypes with niches, guilds, succession; salience; **niche reserve** on every forgetting (the last of its kind is demoted, never deleted).
- **Permanence of the object and of the self (Proposed):** stable index and predicted trace for objects; a protected self vector; identity = chain of versions (state hash + previous) + the seed.
- **Persistence (Proposed, step 6c):** snapshots of the mind with their state hash and a journal of lessons.
  - Large data in **tiles** with an index, read with **positioned reads** (safe Rust), BLAKE3 per tile, writes via a temporary file and a rename.
  - No memory map unless a measurement demands it (one reviewed exception).

## 8. Engine and learning (Proposed)

- **Ternary unit:** popcount((x XNOR sign) AND mask) ≥ threshold; homeostatic thresholds; a counter per connection, flipping at a threshold (as in binary networks trained without latent weights).
- **Vigilance (ART-1):** if nothing matches, recruit; recruitment guarantees a minimum distance.
- **Impulse:** sparse expansion ×8, k winners, descent with predictions until levels agree; at most 24 KB touched (inside L1).
- **Learning cycle:** local, no backpropagation. Up (signature), down (prediction), error = XOR (surprise), silence when small, three factors (trace × signed modulator), recruitment, structural change only by Mutation.
- **Four states:** wake, quiet, play, sleep ("Sogno"). Energy rules per state; sleep consolidates, merges, prunes and respects the niche reserve.
- **Mutation:** base form = the **residue** (Decided). Other forms at other levels: perturbation; a ninth vote that breaks ties (Decided as possible). Promotion only with a margin on unseen data.

## 9. Language (Decided direction; organisation Proposed)

**Decided (2026-11-16):**
- No learning by predicting the next token.
- Language emerges from a series of **virtualisations**, each level giving the one above a simpler world.

| Level | Works on | Produces |
|---|---|---|
| 0 Signals | bits from adapters | — |
| 1 Symbols | recurring patterns | a stable code per family of inputs |
| 2 Associations | co-occurring symbols, across modalities | pairs, records |
| 3 Sequences and events | ordered symbols | episodes |
| 4 Schemas | many episodes | records with roles (who does what to whom): variables |
| 5 Language | role records and word sequences | understanding (words → structure) and production (structure → words) |

Learning signals: association by co-occurrence (counting); completion; local prediction of each level's own code; chunking (a recurring combination becomes a symbol above); task feedback.

**Decided (session 9):**
- Form and meaning are separate.
- A language acts as a **key**: the same idea bound to different keys gives different forms.
- The word is a tensor; phonetics, morphology, syntax and semantics are metatensors.

**Proposed: how a word is organised.** One tensor per word of a language, with metatensors:

| Metatensor | Members (512 or 4096 bits each) | Learned from |
|---|---|---|
| Form | letters with their positions (`PERM`); phonemes (IPA); morphemes | text, NorthEuraLex |
| Meaning | the **context vector**: a tally of the words around it, read by sign (random indexing / BEAGLE family, here with integer counters) | text (meaning is use) |
| Relations | is-a, part-of, synonym, translation, bound with role vectors | WordNet, Wikidata |
| Grounding | links to the worlds (an element, a star, a micro-world object) | the worlds |
| Syntax | category, roles it takes | sentences |

- **Concept** = what the forms of several languages share.
- **Untranslatable** = what does not glue across languages (a sheaf: measured).
- **Production** = unrolling a role record into words, role by role.

**What is not yet known (Open, a council session of its own):**
- how roles and variables are learned from episodes (level 4: an open research problem);
- how sentences are produced fluently;
- how far this route can go.

Expectation: a small grounded language and language tasks (identification, similarity, analogies, simple questions) are feasible; free conversation at the level of a language model is not a near-term outcome.

**Option (Open):** an open-source language model or embeddings as a **teacher at level 3**, entering only through the sign bridge. Red team: then the intelligence is borrowed. If used, measure what the Monade adds on its own.

## 9b. The ladder of eight levels, from signal to thought (Decided 2026-11-19, with the objections below)

Three ladders existed and did not match:
- the eight learning levels of structure v5 (retention, association, error correction, consolidation, abstraction, structure, mutation, metacognition);
- the six language levels of section 9;
- the seven sizes (where "word" means 64 bits and "thought" means 2 MB: sizes, not levels of the mind).

**Proposal:** one ladder of **8 levels of representation**, and **mechanisms that work at every level**, the same rule at every level.

| Level | Name | What it holds | Where it is in the plan | Status |
|---|---|---|---|---|
| 0 | Signal | bits from the adapters | gate (6b) | not built |
| 1 | Symbol | stable codes, prototypes (vigilance, recruitment) | tally (5), `MATCH` (6b), ternary unit (7) | **begun** (tally) |
| 2 | Association | things bound together (role : filler, co-occurrence) | binding, tally | **begun** (`space.sentence`) |
| 3 | Sequence and event | order, episodes, surprise | `PERM` (6b), fast tally (5d), D1 | not built |
| 4 | Schema | records with roles and variables (who does what to whom) | blackboard (6), composition (11) | **open research** |
| 5 | Language | words ↔ structure, in three languages | the language track, D2, test 20 | not built |
| 6 | Thought | reasoning on structures: composing, inferring, verifying, imagining (play, sleep); "why" | Logos (12), layers (13), fertile dream test 5, Peirce test 7 | designed, not built |
| 7 | Metacognition | knowing what it knows: certainty, calibration, choosing what to read or ask, proposing to the author, the self vector | certainty of tallies (5, begun); test 4; self vector (8.1); experts and routing | **seed only** |

- **Mechanisms (the old eight learning levels) work at every level:** retention, association, error correction (prediction ⊕ reality), consolidation (sleep), abstraction (chunking: a recurring combination becomes a symbol of the level above), mutation (residue).
- **The blackboard has one 512-bit block per level:** its 8 levels are these 8 levels.
- **Every level needs a test that passes only if that level works** (ablation: switch it off and the result must collapse; test 2 is the model).
- **Logician:** thought is not language; level 6 works on structures, and language (5) is one of its inputs and outputs.
- **Neuroscientist:** metacognition in the brain rests on confidence signals; here the counters already provide them. The rest (deciding what to do with doubt) is to be built.
- **Red team:** no level is claimed until its test passes.

## 10. The parliament (Decided unless marked)

- **Experts are Monadi:** up to 8 sub-experts each, depth up to 8, **8 active per level**; addresses of 24 bits.
- **The blackboard is a Monade of 8⁴ bits:** one 512-bit block per level.
  - Voices are bound to keys and counted bit by bit (tally); the reading is the majority.
  - Order-free; constant size; consensus measurable.
  - Capacity measured in v0.5: 8 voices in 512 bits 99.6% at 6.2 σ; 64 bits too small.
- **Vote** = majority; a tie = point 0. **Residue** = Mutation's base form.
- **Superposition that reinforces** (Built v0.5). **Subsumption** (Proposed): similar ideas merge into a prototype, provenance kept, never in a chain.
- **Communication between Monadi (Decided): addresses and surprises.**
  - **Reference:** "look there".
  - **Recipe:** "build it this way", regenerated from the shared seed.
  - **Surprise:** XOR, only what is new.
  - Full frames from time to time.
- **Logos (Decided):**
  - the internal language; a recipe is a program, a program is a Monade;
  - about 8 primitives (`GEN LOAD BIND PERM BUNDLE MATCH AND CALL`), one 64-bit word per instruction;
  - depth ≤ 8 and a step budget; one interpreter in the core;
  - programs act only on data, in an enclosure until the author approves; compilation goes upward;
  - up to 8 mechanisms, the first being the XOR of a difference;
  - 128-bit content hashes; scars of falsified ideas in a Bloom filter;
  - two systems: the network intuits, Logos verifies.
  - Written after step 9. Details in the parliament, §15.
- **Seeing a Monade (Decided):**
  - **Structure** (exact): an octree, Morton order.
  - **Meaning** (approximate): arrows from point 0. Opacity = how much the drawing lies; saturation = how sure the mind is.

## 11. Inputs, data and lessons

- **Data (Decided):**
  - in a folder outside the repo (e.g. `D:\monade-data`);
  - a **dataset card** for each set (source, licence, version, hash, splits);
  - train / validation / **sealed test**, the seal kept by the author, opened once per version;
  - a retention test for forgetting;
  - catalogs and sources as **voices**;
  - no personal data, no social media.
- **The gate (Proposed, step 6b):**
  - **Envelope** per input: voice, domain, time, modality, weight exponent, split tag, card hash.
  - **Payload:** one Monade.
  - The gate **refuses to learn from test data**.
  - Surprise decides what is stored; recent traces are tagged for replay.
- **Encoders (Proposed; versioned and locked like the hash):**
  - name → generated space;
  - number → level code (near values give near vectors); cyclic values → cyclic code;
  - position → `PERM`; set → tally; record → bind + bundle;
  - point in space → Morton;
  - continuous vector → sign bridge.
- **`MATCH` (clean-up memory, nearest prototype) and `PERM` come in step 6b** (Decided order).
- **A lesson (Proposed)** = data + order + teacher + budget, logged like checks; same seed + same lessons = same mind hash.
- **Curriculum (Proposed)** — from the concrete to the abstract, variety before quantity:
  - 0 generated micro-worlds;
  - 1 facts (Wikidata CC0, periodic table, family trees);
  - 2 words (Swadesh, NorthEuraLex, WordNet);
  - 3 text as sequence (21 European languages; Tatoeba);
  - 4 the small sky (stars within 100 pc);
  - 5 tables and continuity (UCI, OpenML-CC18, Split/Permuted MNIST);
  - 6 literature: the author's readings (section 11b).
- **Second voice (Decided):** the whole universe; each voice signed; reliability per domain (now: weights, section 7).

## 11b. Literature as nourishment (Decided direction, 2026-11-18; details Proposed)

- **Decided:**
  - The author feeds the mind with a great deal of literature.
  - Primary works lead to secondary literature, which refers to other authors, and so on.
  - Reading goes on until memory resources are used; then the mind forgets (sleep, with the niche reserve) and keeps receiving input.
  - The name "corpus of beauty" is dropped.
- **Proposed:**
  - **Languages (Decided 2026-11-18): Italian, English, German.** (Chinese and Arabic: not now.)
  - **Authors (Decided):** Svevo, Gramsci, Leone Ginzburg; Thomas Mann, Goethe, Nietzsche, Kant, Wittgenstein; Keats, Byron (original texts from free editions).
  - **Teaching sources (Decided):** the most recent high-quality open textbooks: history, geography, physics, chemistry, mathematics, computer science, linguistics, philosophy, biology, ethics, engineering, art history (OpenStax, Serlo, Wikibooks, MIT OpenCourseWare…).
  - **An automatic gatherer, the crawler (Decided; ingestion pipeline `ingest`, command `monade crawl`; design Proposed):**
    - in `tools/`, in the background;
    - discovers, checks licences, fetches politely, extracts, structures (works, sentences, entities, references), writes Parquet and cards, queues lessons;
    - an external open-source language model may help at level 3 (classify, extract references, propose readings), every output signed and verified against the source, never writing what the mind learns. It runs locally on the GPU (GeForce RTX 4060, 8 GB).
  - **A library for every expert (Proposed):** one shared library, many shelves; an expert = references + tallies + prototype; recruited by vigilance, merged in sleep; references and questions routed to experts by `MATCH`; inheritance from the parent; quotas of critics against echo chambers.
    - Details: `claude/monade-council-17-library.md`.
  - **Reading as a graph:** authors, works, quotations and references become a graph of references (64-bit) that the mind represents natively. The path of reading is recorded (provenance).
  - **Against the snowball:**
    - quotas per language, period and voice, so that the most cited do not crowd out the rest;
    - the niche reserve keeps the last author of a kind;
    - what survives sleep is what returns across many sources.
  - **Sources:** public-domain and openly licensed works (Wikisource, Liber Liber, Project Gutenberg, TextGrid, Deutsches Textarchiv); every source has its dataset card.

## 12. Verification and tests

- **Rules (Decided):**
  - every kernel has a slow reference (oracle) and fast = slow property tests;
  - thresholds computed before the first run, each check with a control that must fail;
  - new checks appended; golden hashes never change;
  - CI on Windows and Linux;
  - results "that speak": why, how, numbers, charts (`monade-run check` / `report`, Built).
- **Tests:**
  - inner: 1 atom · 2 credit between levels · 3 composition and capacity · 4 knows when it does not know · 5 fertile dream · 6 impulse in L1 · 7 Peirce · 8 determinism;
  - parliament tests: 17 seeing a Monade · 18 subsumption · 19 reinforcement and capacity (begun, v0.5) · 20 Swadesh · 21 communication;
  - outer 8 to define in detail.
- **Demonstrations (Decided order for D1–D2):**
  - D1 language identification;
  - D2 analogies and knowledge bases;
  - D3 UCI / OpenML-CC18;
  - D4 continual learning;
  - D5 few examples;
  - D6 sequences;
  - D7 efficiency (NeuroBench);
  - D8 the same mind on two machines;
  - D9 ARC-AGI subset.
- **Milestones:**
  - M1 stars and the HR relation;
  - M2 languages;
  - M3 periodic table;
  - M4 continual learning;
  - M5 reasoning;
  - M6 efficiency.
- **Reuse for suites:** torchhd, openml-python, Avalanche, NeuroBench, ARC-AGI (in `tools/`).
- **Honest claims (Proposed):** accuracy per joule and per byte, learning online, explanations by certainty and residue. Not the top accuracy.

## 13. Software architecture (Decided where marked)

| Part | Language | What |
|---|---|---|
| `monade-core` (the mind) | Rust | words, point 0, specs, sizes, address, space, tally; later gate, engine, Logos |
| `monade-run` (the conductor) | Rust | commands; check, report; worlds; lessons; serves Specchio on localhost |
| `tools/` | Python with **uv** | downloads, baselines, analyses; write Parquet / CSV |
| **Specchio** (Decided name) | HTML + JS (three.js) | the interface, in the browser; later a desktop window (Tauri) |

- **Contracts:** files (Parquet, JSON) and commands.
- **Rules (Decided):**
  - Rust everywhere in core and runtime; no C/C++; Python only for tools, never inside the mind;
  - no `unsafe` in our code;
  - errors are values;
  - documentation and an example on every public item;
  - Rust pinned (1.97.0).
- **Large data:** section 7, persistence.

## 14. The interface: Specchio (Proposed)

- **Principles:** mostly grey; colour only with meaning (Okabe-Ito); shape for state; one typeface, tabular digits; dark by default; no gauges, no decorative motion, no rotating tesseract; numbers always printed.
- **Rooms:**
  1. Mind (checks, report, certainty);
  2. Worlds;
  3. Lessons (cards, runs, learning curves);
  4. Blackboard (8×8 grid, residue);
  5. Map (the inside of the mind in 3D: Monadi as points, references as lines; any point drawn as a cube of its bits; certainty as colour when the tallies are shown);
  6. Foundations (read-only).
- **Header always visible:** state of the mind, held-out score vs baseline, budget per impulse, joules per answer, memory vs ceiling.
- **The sky at full scale:** Gaia Sky (MPL-2.0) as an external viewer, fed with our layers.

## 15. Worlds

- **Stars v1 (Built v0.4.2):** HYG in four 2 MB Monadi, one bit per cell, a 3D map. Known limits: grid (cell centres), two brightness levels, five colour classes.
- **Stars v2 (step 5c):**
  - Built v0.6.4: every star an exact Morton code of 63 bits (21 octal digits, cube of 16 777 pc, cells of 0.008 pc), sorted, only the stars stored (8 bytes a star); a region is a prefix and can become a dense 2 MB Monade on demand; brightness and colour to scale; the map at any radius from 2 to 8 000 pc; the HR diagram drawn.
  - Still to do (5c-2): Gaia as a second voice (AT-HYG, Gaia DR3 distances, in the same repository as HYG); counts and light per cell; the HR relation as the first learning test (Decided).
- **The universe at full scale: postponed (Decided).** Kept possible: tiles, levels of detail, recursion of addresses. Ladder R1–R7 in `claude/monade-council-14-universe.md`. Data: measured, inferred and modelled layers, never mixed. All of Gaia ≈ 24 GB in our format.
- **Later worlds (Proposed):** the atom (test 1); a galaxy; a brain (C. elegans).

## 16. Order of writing (Decided)

| # | Step | Status |
|---|---|---|
| 1–3 | word, point 0 and specifications, sizes, arenas | Built v0.1 |
| 3b | check and report | Built v0.2–v0.3.1 |
| 3c | octal address | Built v0.3 |
| 4 | generated space | Built v0.4 |
| world | stars v1 | Built v0.4.2 |
| 5 | tally | Built v0.5, confirmed on Windows |
| **5b** | weights 1/2/4/8; tally golden hash; capacity with noisy voices; housekeeping (`runs/` ignored, stray patch removed); `tools/` with uv; Specchio skeleton | next |
| 5b-crawler | the crawler v0 (discover, licences, fetch, extract, cards, reference graph); runs in parallel from here on | |
| 5c | stars v2 within 100 pc; HR relation | |
| 5d | fast and slow tallies; stochastic votes | |
| 6 | blackboard (weights, redundancy, niche reserve) | |
| 6b | the gate: envelope, encoders, `PERM`, `MATCH`, lesson runner | |
| D1 + D2 | language identification; analogies and knowledge bases | |
| 6c | persistence (snapshots, journal, tiles) | |
| 7 | ternary unit with homeostasis; rungs 0–4 | |
| D3–D5 | tables, continual learning, few examples | |
| 8 | test 8, determinism across lessons and machines | |
| 9 | impulse with budget and metrology (test 6) | |
| 10 | learning cycle with two levels (test 2), on real data | |
| 11 | composition and capacity (test 3) | |
| 12 | Logos v1 | |
| 13 | layers and scars | |
| 14 | sleep and quiet (test 5), subsumption (test 18) | |
| 15 | calibration (4), atom (1), Peirce (7), Swadesh (20), communication (21); adapters; Specchio rooms | |

## 17. Realistic expectations

- **Within a year:** associative memory and recall; classification from few examples; continual learning with little forgetting; analogies; language identification and word similarity; large savings in memory and time.
- **Maybe:** two levels learning together; role-filler sentences in a closed domain; a small grounded language.
- **Not now:** free conversation at the level of a language model.
- The Intelligence: let the first milestone be small and true.

## 19. Open

1. Language: how roles and variables are learned (level 4); production; whether to use a language model as a level-3 teacher.
2. Content of point 0 (at the end); whether the seed moves into point 0.
3. Which voices come first, through which adapters.
4. The outer 8 tests in detail.
5. Memory-mapped files: only if measured necessary.
6. Disk space for data; Gaia Sky as an external viewer.
7. First works and the order of languages (section 11b).
