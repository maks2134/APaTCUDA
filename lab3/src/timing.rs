use std::time::{Duration, Instant};

pub struct Timed<T> {
    pub value: T,
    pub elapsed: Duration,
}

pub fn time_call<T, F: FnOnce() -> T>(f: F) -> Timed<T> {
    let t0 = Instant::now();
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    let value = f();
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    Timed {
        value,
        elapsed: t0.elapsed(),
    }
}

/// Unified human-readable duration (µs / ms / s).
#[allow(dead_code)]
pub fn fmt_duration(d: Duration) -> String {
    let us = d.as_secs_f64() * 1_000_000.0;
    if us < 1000.0 {
        format!("{us:.1} µs")
    } else if us < 1_000_000.0 {
        format!("{:.3} ms", us / 1000.0)
    } else {
        format!("{:.3} s", us / 1_000_000.0)
    }
}

pub fn fmt_ms(ms: f32) -> String {
    if ms < 1.0 {
        format!("{:.1} µs", ms * 1000.0)
    } else if ms < 1000.0 {
        format!("{ms:.3} ms")
    } else {
        format!("{:.3} s", ms / 1000.0)
    }
}
