# Monade

A binary, multi-dimensional AI architecture — a universal representation machine:
count and compare, sizes from 64 bits to 2 MB in powers of 8. Read `CONTEXT.md` first.

```
cargo test                            # all tests
cargo all                             # every check, as in CI
cargo run -p monade-run -- show       # point 0 and the specifications
cargo run -p monade-run -- check      # verify, in words and numbers; saved in runs/
cargo run -p monade-run -- report     # report.html: open it in a browser
cargo run --release -p monade-run -- specchio   # Specchio, the interface (or double click Specchio.cmd)
cargo run -p monade-run -- tools doctor         # the Python tools (needs uv)
```
