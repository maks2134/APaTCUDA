#[inline(always)]
pub fn read_cycles() -> u64 {
    #[cfg(target_arch = "x86_64")]
    {
        unsafe { core::arch::x86_64::_rdtsc() }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        use std::sync::OnceLock;
        static START: OnceLock<std::time::Instant> = OnceLock::new();
        let start = START.get_or_init(std::time::Instant::now);
        start.elapsed().as_nanos() as u64
    }
}

#[inline]
pub fn instant_now() -> std::time::Instant {
    std::time::Instant::now()
}

pub struct Timed<T> {
    pub value: T,
    pub cycles: u64,
    pub elapsed: std::time::Duration,
}

pub fn time_call<T, F: FnOnce() -> T>(f: F) -> Timed<T> {
    let wall0 = instant_now();
    let c0 = read_cycles();
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    let value = f();
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    let c1 = read_cycles();
    let elapsed = wall0.elapsed();
    Timed {
        value,
        cycles: c1.saturating_sub(c0),
        elapsed,
    }
}

#[inline]
pub fn cycles_label() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    {
        "cycles"
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        "ns"
    }
}
