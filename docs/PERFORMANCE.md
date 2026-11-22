# Performance

Measurements against the budgets of D-038, on the owner's laptop (i7-1355U, 16 GB RAM, Iris
Xe, Fedora 44 KDE on Wayland) unless noted.

| Date | Path | Budget | Measured | How |
|---|---|---|---|---|
| 2026-11-22 | Code generation, 501 nodes | ≤ 100 ms | 7.6 ms per run | `cargo test -p rustbot-codegen --release -- --ignored budget --nocapture` |
| 2026-11-22 | Installing the private toolchain (rustup + Rust 1.100.0, minimal) | — | 29.2 s | `cargo test -p rustbot-toolchain -- --ignored install --nocapture` |
| 2026-11-22 | First build of `/ping` (djs-rs and all crates, debug) | — | 41.8 s | same test |
| 2026-11-22 | Rebuild after a small change | ≤ 10 s | 1.1 s | same test |
| 2026-11-23 | Startup to the first frame, release, warm (3 runs) | ≤ 500 ms | 192–218 ms | `RUSTBOT_MEASURE=1 target/release/rustbot-creator <project>` |
| 2026-11-23 | Startup to the first frame, release, first run after a build (cold caches) | ≤ 500 ms | 1.09 s ✗ | same |
| 2026-11-23 | Memory (RSS), one project: 2 nodes / 500 nodes | ≤ 200 MB | 108 MB / 130 MB | `VmRSS` in `/proc/<pid>/status` after 8 s |
| 2026-11-23 | Graph of 500 nodes (35 on screen), frame interval while redrawing without pause | ≤ 16.6 ms p99 | p50 17.2–17.4 ms, p99 26–33 ms ✗ | `RUSTBOT_MEASURE=frames target/release/rustbot-creator <project>` |
| 2026-11-23 | The same with a 2-node graph, as a baseline | — | p50 16.6 ms, p99 18–21 ms | same |
| 2026-11-23 | Graph of 500 nodes, CPU work per frame (editor render → its paint) | — | p50 3.7 ms, p99 5.4 ms, max ~40 ms | a temporary probe, removed |

## Notes

- **500-node graph (2026-11-23).** At first every frame drew all 500 nodes and 499 wires,
  each wire as its own GPU path, at ~50 ms a frame. Three changes helped. The editor now
  draws only the nodes and wires that touch the screen (35 here). Wires of one colour
  are one path, because gpui rasterizes each path on its own and ~35 paths cost ~8 ms.
  Nodes have no shadow, which weighed on p99. Frames now arrive at 60 FPS at the median,
  but p99 still misses the budget. Removing the pins changes nothing, so it is not the
  element count. The CPU probe shows rare spikes up to ~40 ms against a median of 3.7 ms,
  and these spikes decide p99. Their cause is open; `perf` cannot run here
  (`perf_event_paranoid` = 2).
- **Cold start.** The first start after a build reads the binary and fonts from disk and
  prepares the GPU pipeline. It is measured, but the budget is meant for a normal start.
