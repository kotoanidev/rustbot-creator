# Performance

Measurements against the budgets of D-038, on the owner's laptop (i7-1355U, 16 GB RAM, Iris
Xe, Fedora 44 KDE on Wayland) unless noted.

| Date | Path | Budget | Measured | How |
|---|---|---|---|---|
| 2026-11-22 | Code generation, 501 nodes | ≤ 100 ms | 7.6 ms per run | `cargo test -p rustbot-codegen --release -- --ignored budget --nocapture` |
