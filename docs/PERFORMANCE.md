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
| 2026-11-23 | Frame time, graph of 500 nodes (35 on screen), 600 frames × 3 runs | ≤ 16.6 ms p99 | p50 2.9–4.6 ms, p99 6.8–10.0 ms, max 10–17 ms | `RUSTBOT_MEASURE=frames target/release/rustbot-creator <project>` |
| 2026-11-23 | Frame time, graph of 2 nodes, as a baseline | — | p50 1.9 ms, p99 4.1 ms | same |

## Notes

- **How frames are measured.** `RUSTBOT_MEASURE=frames` keeps the editor redrawing and
  times 600 frames from the start of the window's render to the end of its paint. It also
  reads how long the UI thread was on a CPU (`/proc/thread-self/schedstat`), and the two
  agree, so the numbers are the app's own work, not waiting for other programs. Timing
  the interval between frames was tried first and dropped. It depends on the display's
  refresh rate and on the compositor (KDE gave p50 8.4–16.7 ms for the same graph), and
  even a 2-node graph had a p99 up to 43 ms. The GPU's share is not in these numbers.
- **500-node graph (2026-11-23).** At first every frame drew all 500 nodes and 499 wires,
  each wire as its own GPU path, at ~50 ms between frames. These changes fixed it:
  - The editor draws only the nodes and wires that touch the screen.
  - Wires of one colour are one path. gpui rasterizes each path on its own, and ~35
    single-wire paths cost ~8 ms.
  - Nodes have no shadow.
  - The first frame draws nothing until the canvas knows its size. Before, that frame
    drew all 500 nodes and took 31–36 ms.
- **Cold start.** The first start after a build reads the binary and fonts from disk and
  prepares the GPU pipeline, so it is slower (1.09 s once). A normal start is 150–280 ms.
