# HR relation, first learning test on the stars (step 5c)

Written 2026-11-19, before the first run; the code is `crates/monade-run/src/hr.rs`.

**Question.** Can the mind learn, by counting only, the absolute magnitude of a star
from its colour (B-V)?

**Method.** 64 colour bins of 0.05; per bin a tally of 64 bits; each teacher votes a
thermometer code of its absolute magnitude (levels of 0.5 mag from -10); reading a
tally gives the median level. Even places in the catalog teach, odd places are asked.
Baseline: one tally for every star (colour ignored). Control: tallies per colour fed
with shuffled colours.

**Thresholds (pre-registered).**
- learned error ≤ 60 % of the baseline error;
- control error ≥ 90 % of the baseline error (the control must not learn).

## Results (2026-11-19)

First run, teachers in catalog order: the tally of all the stars read level 42 where
the median was 23. HYG ends with thousands of faint red dwarfs, and a saturated 8-bit
tally follows its most recent votes (the known limit of the counters). The baseline
was wrong, so the run says nothing about learning. Fix to the procedure, not to the
thresholds: teachers in a shuffled order (a fixed permutation).

Second run:

| catalog | taught | error learned | baseline | control | exact median per colour (reference) | learned ≤ 60 % | control ≥ 90 % |
|---|---|---|---|---|---|---|---|
| HYG v4.1 | 53 930 | 1.17 mag | 1.70 mag | 1.70 mag | 1.17 mag | FAIL (69 %) | PASS |
| HYGLike v3.2 (AT-HYG, Gaia DR3 distances) | 58 611 | 1.08 mag | 1.73 mag | 1.74 mag | 1.08 mag | FAIL (62 %) | PASS |

**Reading.** The test fails against its pre-registered threshold. The tallies learn
exactly as much as a perfect count can (the error equals the exact median's), and the
control shows nothing is learned from noise: the shortfall is in the prediction, not in
the mind. Colour alone does not fix brightness: at the same colour a giant is up to
10 magnitudes brighter than a dwarf. The better voice teaches better: with Gaia
distances the error drops from 1.17 to 1.08 mag.

**Next (to pre-register before running):** a second feature that separates giants
from dwarfs (luminosity class from the spectral type), with the threshold set against
the exact reference.
