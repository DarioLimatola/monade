# worlds

Worlds the Monade represents, read through adapters.

## stars

The stars around the Sun, from the HYG database (Hipparcos, Yale, Gliese; about
120 000 stars), downloadable as a CSV from https://github.com/astronexus/HYG-Database
(`hyg/CURRENT/hygdata_v41.csv`). The catalog is not kept in this repository.

```
cargo run --release -p monade-run -- stars --catalog hygdata_v41.csv
```

Writes `stars.html`, a 3D map of four 2 MB Monadi (cubes of 32, 128, 512 and 2048
parsecs around the Sun, 256³ cells each, one bit per cell).
