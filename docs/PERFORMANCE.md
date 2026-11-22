# Performance

Measurements against the budgets of D-038, on the owner's laptop (i7-1355U, 16 GB RAM, Iris
Xe, Fedora 44 KDE on Wayland) unless noted.

| Date | Path | Budget | Measured | How |
|---|---|---|---|---|
| 2026-11-22 | Code generation, 501 nodes | ≤ 100 ms | 7.6 ms per run | `cargo test -p rustbot-codegen --release -- --ignored budget --nocapture` |
| 2026-11-22 | Installing the private toolchain (rustup + Rust 1.100.0, minimal) | — | 29.2 s | `cargo test -p rustbot-toolchain -- --ignored install --nocapture` |
| 2026-11-22 | First build of `/ping` (djs-rs and all crates, debug) | — | 41.8 s | same test |
| 2026-11-22 | Rebuild after a small change | ≤ 10 s | 1.1 s | same test |
