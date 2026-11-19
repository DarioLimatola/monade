# worlds

Worlds the Monade represents, read through adapters.

## stars

The stars around the Sun, from the HYG database (Hipparcos, Yale, Gliese; about
120 000 stars), downloadable as a CSV from https://github.com/astronexus/HYG-Database
(`hyg/CURRENT/hygdata_v41.csv`). The catalog is not kept in this repository.

```
cargo run --release -p monade-run -- stars --catalog hygdata_v41.csv
```

Every star becomes one exact octal address of 21 digits (a Morton code of 63 bits:
cube of 16 777 pc around the Sun, cells of 0.008 pc); only the stars are stored, and
any region can become a dense 2 MB Monade on demand. Writes `stars.html`: a 3D map at
any radius from 2 to 8 000 pc, colours and light to scale, and the Hertzsprung–Russell
diagram. Prints the dense Monadi near the Sun at five depths, writes the sky (`stars.sky`, read
by Specchio's Tree of the sky) and learns the HR relation (`prereg/hr-relation.md`).

A second voice: `--second hyglike_from_athyg_v32.csv.gz` (AT-HYG, Gaia DR3 distances,
in the same repository under `hyg/athyg_v3/`; gzip is read directly) compares the
two catalogs star by star and learns the HR relation from both.
