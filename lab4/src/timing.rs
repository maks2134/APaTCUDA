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

pub fn fmt_ms(ms: f32) -> String {
    if ms < 1.0 {
        format!("{:.1} µs", ms * 1000.0)
    } else if ms < 1000.0 {
        format!("{ms:.3} ms")
    } else {
        format!("{:.3} s", ms / 1000.0)
    }
}
