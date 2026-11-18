# Monade — structure v5, part 2: the parliament (council session 9, 2026-11-18)

**Status:** adds to `claude/tesseract-structure-v5.md`; together they are the design source of truth. **Decided** = confirmed by the author. **Proposed** = council recommendation. **Open** = needs the author.

The way the council works is also the way the Monade will think and learn: many voices, one shared space, a vote, and a voice for the minority.

## 1. Experts are Monadi (Decided)

- Every expert is a Monade (at most 2 MB) with its own point of view on the same space.
- Recursion: each expert may summon up to 8 sub-experts; depth up to 8. 8⁸ ≈ 16.7 million possible experts, which exist in potency: they cost nothing until summoned.
- **8 active per level** (Decided), so at most 64 active at once.
- Address of an expert: a path of 8 digits in base 8 = 24 bits; it fits in a 64-bit reference.
- Parliament (it votes) and swarm (it coordinates through a shared environment) at once.

## 2. The blackboard is a Monade (Decided)

- Experts do not send messages to each other (that cost grows with the square of their number). They write into one shared space and anyone may read it.
- The blackboard of one meeting is a **Monade of 8⁴ bits (512 bytes): one 512-bit block per level**, i.e. 8 cache lines.
- **Writing:** each voice is bound to its own key (XOR) and added to its level's block by counting bit by bit; the block's reading is the majority.
- **Reading:** XOR with a speaker's key, then clean-up against known ideas (Hamming distance), recovers that voice.
- **Constant size**, whether 2 or 8 voices speak.
- **Order-free, hence a-temporal:** XOR and counting commute, so who writes first does not matter. Questions need not precede answers.
- **Consensus is measurable:** agreeing voices reinforce each other; diverging voices each stay weakly similar to the total.
- **Capacity (to verify by test):** 8 voices in 512 bits stay recognisable (about 6 standard deviations above noise); in 64 bits they do not (about 2).
- Precedents: Blackboard architecture (Hearsay-II), Global Workspace (Baars), Pandemonium (Selfridge 1959), stigmergy in swarms.
- The author reads the blackboard through the chronicler, who decodes it on request.

## 3. The dialogue as a graph (Proposed)

- Every contribution is a node with 64-bit references to others: "answers", "objects to", "asks".
- The order is causal, not chronological: an answer may exist before its question.
- Iterate until nothing changes: the fixed point is the consensus; if none is reached, the remaining disagreement is measured.
- Append-only, like point 0. Precedent: IBIS (Rittel): issues, positions, arguments.

## 4. Vote and Mutation

- **Vote (Decided):** the bitwise majority already in the core. **A tie gives 0**: back to point 0, suspension of judgement.
- **Mutation, base form = the residue (Decided):** what the majority discarded, i.e. the difference between each voice and the vote. The minority's voice comes out of the same operation, at no extra cost. Like a parity bit: 8 voices + 1 control.
- **Other forms, at other levels (Decided as possible):** perturbation (flip some bits of the result; keep the change only if it does better on unseen data); a ninth vote that breaks ties.
- **Principle (Decided by the author):** contrary opinions and minority voices are heard and respected, provided they respect others and do no harm to others. To be written into point 0 when the author writes it.

## 5. Superposition that reinforces (Proposed)

- **Counters per bit, not only bits:** the bit is the reading, the counter is the memory. Counting is allowed in the core.
- What repeats grows and becomes certain; isolated noise cancels.
- Disagreement leaves counters near the middle: measurable uncertainty, which is the residue.
- **Two memories** (complementary learning systems, McClelland 1995): a fast one that keeps episodes apart (as the hippocampus does), a slow one that superposes and extracts what is common (as the cortex does). Transfer happens in **sleep**: consolidate what repeated, forget what did not serve.
- Note of the philosopher: *alétheia* (a- + léthe, "not forgotten / not hidden"). In the Monade, truth resembles what survives sleep: repeated, confirmed, not pruned.

## 6. Ex pluribus unum: subsumption (Proposed)

- Two ideas closer than the vigilance threshold merge into a **prototype**: the majority of the two, weighted by how often each was seen.
- The prototype keeps references to its origins as metadata in potency: nothing is forgotten by merging.
- Two paths that lead to similar places get a link (synonymy): the concept is what the paths share.
- Two similar experts merge during sleep and free a place (pruning).
- **Objection (mathematician):** similarity is not transitive (A ~ B, B ~ C, not A ~ C). Choose one prototype per group; never merge in a chain.

## 7. Gluing local views (Proposed, mathematical frame)

- Tensor space exists (the tensor product V⊗W), but "local structures joined by a criterion" is better named a **sheaf**: local data plus a gluing rule on overlaps. If the data glue, there is one global truth; if not, the obstruction measures the disagreement. Used for opinion dynamics among experts (Hansen and Ghrist).
- **Fibre bundle:** a whole space attached to each point of a base (an expert attached to a point).

## 8. Ideas and words (Proposed)

- **Idea:** a binary vector (64 bits to 2 MB); the inner language of the experts (a "language of thought", Fodor).
- **Word:** a readable translation, produced only when a human reads; metadata in potency.
- The message unit matches the bus: a 64-bit word (a reference) or a 512-bit block (one cache line).

## 9. Seeing a Monade (Decided; a test and a monitor tool, outside the core)

**a) Structure: exact, lossless.**
- 8 = 2×2×2: every split into 8 parts is a cube split into 8 octants (an octree).
- A size of 8ᵏ bits is a cube of side 2ᵏ: word = 4×4×4, block = 8×8×8, **2 MB = 8⁸ bits = 256×256×256**, one point per bit.
- Linear position ↔ 3D point by **Morton order** (interleaving the address bits): a bijection.
- Shows density, empty regions, repetitions; not meaning.

**b) Meaning: approximate.**
- Ideas projected to 3D so that similar ones stay close (PCA, MDS, UMAP).
- **Each idea is an arrow from point 0.** Opacity and thickness = local fidelity of the projection: opaque and thick where the drawing respects the true distances, faint and thin where it lies (measures: trustworthiness and continuity, Venna and Kaski; stress per point).
- Relations between ideas: segments, with the same fidelity rule (drawn distance vs true Hamming distance).
- **Colour saturation = certainty of the mind** (from the bit counters): full colour = sure, towards grey = uncertain.
- Two uncertainties, two channels, never mixed: how much the drawing lies (opacity) and how unsure the mind is (saturation). In the brain the analogue is precision weighting in predictive coding: uncertain signals arrive attenuated.
- The topologist adds shape: groups, holes, bridges.

**c) Blackboard.** An 8×8 grid (levels × voices); colour = agreement with the majority; the residue highlighted.

## 10. Natural languages (Proposed)

- **Form and meaning are separate.** "cane", "dog", 狗 (gǒu): three forms bound to nearby concepts. A language acts as a **key**: the same idea bound to different keys gives different forms.
- Levels as already decided: the word is a tensor; phonetics, morphology, syntax, semantics are metatensors.
- **Relations are measurable:** Italian and English share Indo-European roots and Latin loans (notte/night, madre/mother), so their forms are close on phonetic dimensions. Chinese adds tone (an extra phonetic dimension), has no inflection (order and particles do the work) and a recursive script (好 = 女 + 子): the test that forbids assuming alphabets or conjugation. All three are basically subject-verb-object: close in basic syntax, far in morphology.
- **Untranslatables** ("magari", "serendipity", 缘分 yuánfèn): the concept spaces overlap only in part. A sheaf again: each language is a local view; what does not glue is the untranslatable, and it can be measured.
- **Objection (linguist, Wittgenstein):** meaning is use. Concept vectors are learned from contexts, not from a dictionary.

## 11. Programming languages (Decided, on the council's advice)

- **Rust** for the whole core and runtime (already safe, fast, on Windows).
- No C or C++ (they would break the no-`unsafe` rule).
- Python only for external tools (data preparation, analysis of measurements), never inside the mind.
- GPU, if and when needed: `wgpu` from Rust.
- Interface: decided with the designer when the adapters come.

## 12. New tests (Decided)

| # | Test | Pass | Guard |
|---|---|---|---|
| 17 | **Seeing a Monade** | Morton round trip is the identity for every size; same Monade → same image (hash) | property test over all sizes; golden image per version |
| 18 | **Subsumption** | similar ideas merge, dissimilar ones do not; provenance kept | chains A~B~C must not collapse into one; merged prototype recalls both origins |
| 19 | **Reinforcement and blackboard capacity** | repeated patterns grow certain, noise cancels; recall improves with repetitions; capacity curve (voices per block vs recovery) at 64, 512, 4096 bits | single-exposure control; random-voice control; threshold co-signed by the statistician |
| 20 | **Swadesh** (about 200 basic concepts in Italian, English, Chinese) | Italian–English closer than Italian–Chinese in form; all three close in meaning; untranslatables measured | word lists from a public source fixed beforehand; shuffled-pairs control must fail |

## 13. The council

Now 22 in potency; **8 summoned per session, by theme** (Decided: the same rule as the mind). 

## 14. How Monadi communicate: addresses and surprises (Decided)

- Leibniz's monads "have no windows"; ours share a pre-established harmony: the **seed**. Every Monade generates the same basic vectors without transmitting them, so the dictionary is shared from the start. A message says only which vectors and how to combine them.
- **Three message forms**, densest first: **reference** (64 bits, "look there": the thing is already in memory); **recipe** (a few references and operations, "build it this way": regenerated from the seed); **surprise** (XOR between what I have and what you expect, "only this is new"). Zero surprise = silence.
- **Full frame** from time to time (reference or recipe), differences in between, as in video compression; otherwise sender and receiver drift apart (mathematician).
- A recipe regenerates only what is composed from the seed; learned content (counters, prototypes) travels as a reference (shared memory) or as its bits.
- **Who talks to whom:** the tree (each expert with its 8 sub-experts), the blackboard (one block per level), a few lateral bridges between similar Monadi (from subsumption): a small-world network.
- **In silicon:** a 512-bit block is a cache line; each core writes only its own blocks, then XOR and counts are summed; order-free, so no locks and a deterministic result (test 8).
- **Objection (red team):** while predictions are poor, surprises are dense; the gain comes only with learning, and is measured.
- **Test 21:** bits exchanged to reach agreement, full messages vs reference + surprise; expected gain grows with learning.

## 14b. Hash function (Decided)

BLAKE3 through the official `blake3` crate: keyed from the seed (context "monade space v1"), extendable output read in 64-byte blocks (one cache line per generated block, random access, lazy), the same function for content addresses (128 bits) and scars, separated by context. Verified against the official test vectors. Locked once chosen: changing it means a new mind.

## 14c. Three levels: bits, integer GPU, the continuous (Decided 2026-11-18)

| Level | Where | Arithmetic | What |
|---|---|---|---|
| 1. Core | CPU | bits and integers, deterministic | the mind: memory, impulse, Logos |
| 2. Accelerator | GPU (`wgpu`) | **integers**, deterministic | the same operations (XOR, popcount, majority) on large batches: `MATCH` over millions of prototypes, sleep, Mutation evaluating many candidates, the force map; same state hash as the CPU (test 8) |
| 3. The continuous | CPU or GPU | floating point | adapters (sound, images, embeddings), physical worlds, teachers, analysis and maps |

- **Bridge (Charikar 2002):** a continuous vector enters the mind only as the **sign** of its projections on random hyperplanes generated from the seed (`GEN`); Hamming distance then measures the angle between the original vectors. The continuous level is a metatensor with a third algebra, sharing the same generated space.
- **Rule:** floating point never enters the state of the mind nor its hash.
- **Metrologist:** a single impulse (≤ 24 KB) stays on the CPU; the GPU only for large batches, and only when a measurement shows the bottleneck (after step 9).

## 15. The internal language: **Logos** (Decided: adopted and named; written after step 9 of the order of writing)

- **A recipe is a program.** About 8 primitives: `GEN · LOAD · BIND · PERM · BUNDLE · MATCH · AND · CALL`. Each instruction is one 64-bit word (1 byte opcode, 7 bytes operands). **A program is a Monade** (code is data).
- **Moving across levels, tensors and dimensions: the octal address (Decided 2026-11-18; built in v0.3.0 as `monade_core::address::Address`).** No ninth primitive: movement lives in the **address** that `LOAD` (and every reference) takes. An address is a root (which arena, i.e. which tensor) plus a **path of base-8 digits**, one per level, each choosing one of the 8 parts. **Down** = append a digit; **up** = drop the last digit; **another tensor** = another root; **another dimension or role** inside the same size = `PERM`. From a 2 MB Monade to a single bit: 8 digits = 24 bits, the same 24-bit path as an expert address (section 1): one addressing rule for experts, Monadi and bits. Each digit is an octant (bit 0 → x, 1 → y, 2 → z), so the path is the Morton order of the cube (section 9). **Packed form, 56 bits** (the 7 operand bytes of an instruction): digits in bits 0–23 (top digit highest, unused digits 0), depth in bits 24–27, root in bits 28–55, bits 56–63 zero; the top size is not packed (it belongs to the root's arena). Named `Address` rather than `Path` (it would clash with `std::path::Path`). Invariants (property-tested on every size): down/up, offset, point and packed round trips; fast = slow against a recursive reference; Morton is a bijection on 512 and 4096 bits and injective on a sample of 2 MB; lexicographic paths = increasing offsets; never a panic.
- **One interpreter in the core**, shared; no compilation to machine code (it would need `unsafe`); no compiler copied into each Monade.
- **Always terminates:** recursion depth at most 8 and a step budget (cost counted in 64-byte lines touched).
- **Guard (Decided):** programs act only on data, never on the engine, point 0 or the specifications; programs written by the mind run in an enclosure until the author approves them.
- **Compilation goes upward (Decided):** never into the core. Recipes used often become new named words of a higher level (the metalanguage); the primitives stay fixed. Sleep compiles (precedent: DreamCoder's abstraction sleep).
- **Experimental ideas (Decided):** they take experience into account; once falsified they no longer occupy resources.
- **Virtualisations (Decided as direction):** recursive enclosures in which ideas are tried.
- **Mechanisms (Decided): up to 8 different ones**, not a single one. The first is the **XOR of a difference**, which serves surprise, virtualisation (a layer of differences over the real world; recursive, depth at most 8; layers live in −N), inheritance (child = parent ⊕ difference) and experiment (confirmed: apply the layer; falsified: discard it). Each further mechanism must justify itself.
- **Scar (Decided):** a falsified idea leaves 8 bytes (hash and motive) so that it is not retried endlessly.
- **Functional design (Proposed):** pure, immutable, lazy ("in potency"), content-addressed (the hash of a recipe is its name), vector-wise (each instruction on 512 bits, SIMD), total (always terminates), higher-order (programs are Monadi), grows by words (Forth, Lisp; sleep adds words above).
- **Inheritance (Proposed):** structural (a sub-expert inherits everything and keeps its difference; delegation to the parent for what it lacks; point 0 is the common ancestor; the right to change what is protected is never inherited) and genetic (through Mutation; falsified ideas do not reproduce; between minds with different seeds only culture, i.e. named recipes, is inherited).
- **The 8 mechanisms (Decided as proposal):** 1 XOR of a difference · 2 delegation (ask the parent) · 3 mask (AND: inherit only some dimensions) · 4 permutation (same content, other role) · 5 majority (recombine, vote) · 6 shared reference (copy a block only when modified) · 7 counter with budget (life of an experimental idea: gains when it predicts, loses when wrong, freed at zero) · 8 free, reserved. Each added only when needed, with its test and slow reference.
- **Content addresses: 128-bit hashes (Decided)**, or a full comparison when two hashes coincide.
- **Scars in a Bloom filter (Decided):** gathered in sleep into one bit vector (a Monade); answers "already falsified?" in constant memory; rare false positives, never false negatives. A scar is set only with a margin on unseen data and a threshold fixed beforehand (statistician).
- **Two systems on one algebra (Decided):** the network (fast, parallel, learned: recognise, intuit) and Logos (slow, sequential, exact: reason, verify). A ternary unit is a one-line program (a masked `MATCH`). `MATCH` is the bridge: Logos asks the network to recognise; the network's winners become Logos arguments. Intuition proposes, logic verifies (precedent: DreamCoder). Recipes used often become learned units (automatisation). Experimental layers of masks and signs live in −N; a recruited unit starts from its nearest prototype (parent) and learns its difference.
- **The logician's frame (Proposed):** sizes as types; programs as derivations (Curry-Howard: every idea can answer "why?"); paraconsistent (a contradiction does not explode); virtualisations as possible worlds (necessary / possible); belief revision through recipe dependencies (truth maintenance, depth ≤ 8); Gödel: the mind cannot certify itself, so the external judge stays necessary.
- **Missing (to design):** memory reclamation by compaction in sleep; a dictionary of names for the words of Logos; the version of Logos fixed in the specifications; a step-by-step program tracer; an adapter from natural language to recipes.

## 16. Open

1. Which Mutation form at which level.
2. Detailed protocol of tests 17–21.
3. Logos: the further mechanisms (up to 8); the functional design and inheritance rules (Proposed).
4. ~~Octal path~~: decided and built (v0.3.0, `Address`, step 3c).
