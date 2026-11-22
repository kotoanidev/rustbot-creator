//! Measurements for the budgets of D-038, on only when `RUSTBOT_MEASURE` is set. They print
//! to stderr: the time from start to the first frame, and with `RUSTBOT_MEASURE=frames`, the
//! time each frame takes while the graph editor redraws without pause.
//!
//! A frame is timed from the start of the window's render to the end of its paint, so the
//! display's refresh rate does not hide or add anything. The time the UI thread actually
//! ran in that span comes from `/proc/thread-self/schedstat`, which tells a slow frame
//! from one where another program had the CPU.

use std::cell::RefCell;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use gpui_kit::{IntoElement, Styled as _, Window, canvas};

static STARTED: OnceLock<Instant> = OnceLock::new();

/// How many frames are timed.
const FRAMES: usize = 600;

fn mode() -> Option<String> {
    std::env::var("RUSTBOT_MEASURE").ok()
}

fn measuring_frames() -> bool {
    mode().as_deref() == Some("frames")
}

/// Notes when the app started.
pub(crate) fn start() {
    STARTED.get_or_init(Instant::now);
}

/// Prints the time from the start to the window's first frame.
pub(crate) fn first_frame(window: &Window) {
    if mode().is_some() {
        window.on_next_frame(|_, _| {
            let started = STARTED.get().copied().unwrap_or_else(Instant::now);
            eprintln!("measure: startup to first frame {:?}", started.elapsed());
        });
    }
}

#[derive(Default)]
struct Frames {
    /// When the current frame started, and the thread's CPU time then.
    start: Option<(Instant, Duration)>,
    /// Per frame: wall time and the CPU time the UI thread got.
    times: Vec<(Duration, Duration)>,
}

thread_local! {
    static FRAMES_TIMED: RefCell<Frames> = RefCell::default();
}

/// The UI thread's time on a CPU so far, from the kernel's scheduler statistics.
fn cpu_time() -> Duration {
    std::fs::read_to_string("/proc/thread-self/schedstat")
        .ok()
        .and_then(|text| text.split_whitespace().next()?.parse().ok())
        .map_or(Duration::ZERO, Duration::from_nanos)
}

/// Called when the window starts rendering a frame.
pub(crate) fn frame_start() {
    if measuring_frames() {
        FRAMES_TIMED.with_borrow_mut(|frames| frames.start = Some((Instant::now(), cpu_time())));
    }
}

/// Keeps the editor redrawing while frames are timed.
pub(crate) fn keep_drawing(window: &Window) {
    if measuring_frames() && FRAMES_TIMED.with_borrow(|frames| frames.times.len() < FRAMES) {
        window.request_animation_frame();
    }
}

/// An empty element painted last in the window, which ends the frame's timing. After
/// [`FRAMES`] frames it prints the median, the 99th percentile and the worst frame.
pub(crate) fn frame_end() -> impl IntoElement {
    canvas(
        |_, _, _| (),
        |_, (), _, _| {
            if !measuring_frames() {
                return;
            }
            FRAMES_TIMED.with_borrow_mut(|frames| {
                let Some((wall, cpu)) = frames.start.take() else {
                    return;
                };
                if frames.times.len() >= FRAMES {
                    return;
                }
                frames
                    .times
                    .push((wall.elapsed(), cpu_time().saturating_sub(cpu)));
                if frames.times.len() == FRAMES {
                    let (walls, cpus): (Vec<_>, Vec<_>) = frames.times.iter().copied().unzip();
                    eprintln!("measure: {FRAMES} frames, wall {}", spread(walls));
                    eprintln!("measure: {FRAMES} frames, on CPU {}", spread(cpus));
                }
            });
        },
    )
    .size_0()
}

fn spread(mut times: Vec<Duration>) -> String {
    times.sort();
    let at = |q: usize| times[(times.len() - 1) * q / 100];
    format!("p50 {:?}, p99 {:?}, max {:?}", at(50), at(99), at(100))
}
