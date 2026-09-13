use std::arch::x86_64::_rdtsc;

#[inline(always)]
pub fn read_cycles() -> u64 {
    unsafe { _rdtsc() }
}

pub struct Timed<T> {
    pub value: T,
    pub cycles: u64,
    pub elapsed: std::time::Duration,
}

pub fn time_call<T, F: FnOnce() -> T>(f: F) -> Timed<T> {
    let wall0 = std::time::Instant::now();
    let c0 = read_cycles();
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    let value = f();
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    let c1 = read_cycles();
    Timed {
        value,
        cycles: c1.saturating_sub(c0),
        elapsed: wall0.elapsed(),
    }
}
