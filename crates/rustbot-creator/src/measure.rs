//! Measurements for the budgets of D-038, on only when `RUSTBOT_MEASURE` is set. They print
//! to stderr: the time from start to the first frame, and with `RUSTBOT_MEASURE=frames`, the
//! intervals between frames while the graph editor redraws without pause.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use gpui_kit::Window;

static STARTED: OnceLock<Instant> = OnceLock::new();

/// How many frames are timed.
const FRAMES: usize = 600;

fn mode() -> Option<String> {
    std::env::var("RUSTBOT_MEASURE").ok()
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

/// The editor's frame intervals.
#[derive(Default)]
pub(crate) struct Frames {
    last: Option<Instant>,
    intervals: Vec<Duration>,
}

impl Frames {
    /// Called on every render of the editor: keeps it redrawing until enough frames are
    /// timed, then prints the median and the 99th percentile.
    pub(crate) fn tick(&mut self, window: &Window) {
        if mode().as_deref() != Some("frames") || self.intervals.len() >= FRAMES {
            return;
        }
        let now = Instant::now();
        if let Some(last) = self.last.replace(now) {
            self.intervals.push(now - last);
        }
        if self.intervals.len() < FRAMES {
            window.request_animation_frame();
            return;
        }
        let mut sorted = self.intervals.clone();
        sorted.sort();
        let at = |q: usize| sorted[(sorted.len() - 1) * q / 100];
        eprintln!(
            "measure: {FRAMES} frames, interval p50 {:?}, p99 {:?}",
            at(50),
            at(99)
        );
    }
}
