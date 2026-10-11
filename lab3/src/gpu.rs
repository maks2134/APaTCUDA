use std::sync::Arc;

use cudarc::driver::sys::CUevent_flags;
use cudarc::driver::{
    CudaContext, CudaFunction, CudaModule, CudaSlice, CudaStream, LaunchConfig, PushKernelArg,
};
use cudarc::nvrtc::Ptx;

use crate::matrix::MatrixI32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuVariant {
    Basic,
    Naive,
    Coalesced,
    Streams,
    Vram,
    Oom,
}

impl GpuVariant {
    pub fn label(self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Naive => "naive",
            Self::Coalesced => "coalesced",
            Self::Streams => "streams",
            Self::Vram => "vram",
            Self::Oom => "oom",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "basic" => Self::Basic,
            "naive" => Self::Naive,
            "coalesced" => Self::Coalesced,
            "streams" => Self::Streams,
            "vram" => Self::Vram,
            "oom" => Self::Oom,
            _ => return None,
        })
    }

    pub fn all() -> &'static [GpuVariant] {
        &[
            Self::Basic,
            Self::Naive,
            Self::Coalesced,
            Self::Streams,
            Self::Vram,
            Self::Oom,
        ]
    }
}

pub struct GpuEngine {
    ctx: Arc<CudaContext>,
    stream: Arc<CudaStream>,
    #[allow(dead_code)]
    module: Arc<CudaModule>,
    basic: CudaFunction,
    naive: CudaFunction,
    coalesced: CudaFunction,
    coalesced_offset: CudaFunction,
}

pub struct GpuTimedResult {
    pub a1: MatrixI32,
    pub a2: MatrixI32,
    pub variant: GpuVariant,
    pub kernel_ms: f32,
    pub h2d_ms: f32,
    pub d2h_ms: f32,
    pub total_ms: f32,
    pub note: String,
}

fn timing_flag() -> Option<CUevent_flags> {
    Some(CUevent_flags::CU_EVENT_DEFAULT)
}

fn cfg_2d(k: usize, m: usize, block_x: u32, block_y: u32) -> LaunchConfig {
    LaunchConfig {
        grid_dim: (
            m.div_ceil(block_x as usize) as u32,
            k.div_ceil(block_y as usize) as u32,
            1,
        ),
        block_dim: (block_x, block_y, 1),
        shared_mem_bytes: 0,
    }
}

fn cfg_coalesced(k: usize, m: usize) -> LaunchConfig {
    let block_x = 16u32;
    let block_y = 16u32;
    let cols = m.div_ceil(4);
    LaunchConfig {
        grid_dim: (
            cols.div_ceil(block_x as usize) as u32,
            k.div_ceil(block_y as usize) as u32,
            1,
        ),
        block_dim: (block_x, block_y, 1),
        shared_mem_bytes: 0,
    }
}

fn cfg_naive(k: usize, m: usize) -> LaunchConfig {
    let block_x = 16u32;
    let block_y = 16u32;
    LaunchConfig {
        grid_dim: (
            k.div_ceil(block_x as usize) as u32,
            m.div_ceil(block_y as usize) as u32,
            1,
        ),
        block_dim: (block_x, block_y, 1),
        shared_mem_bytes: 0,
    }
}

impl GpuEngine {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let ctx = CudaContext::new(0)?;
        const CUBIN: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/deinterleave.cubin"));
        let module = ctx.load_module(Ptx::from_binary(CUBIN.to_vec()))?;
        let basic = module.load_function("deinterleave_basic")?;
        let naive = module.load_function("deinterleave_naive")?;
        let coalesced = module.load_function("deinterleave_coalesced")?;
        let coalesced_offset = module.load_function("deinterleave_coalesced_offset")?;
        let stream = ctx.default_stream();
        Ok(Self {
            ctx,
            stream,
            module,
            basic,
            naive,
            coalesced,
            coalesced_offset,
        })
    }

    pub fn device_name(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(self.ctx.name()?)
    }

    fn kernel_fn(&self, v: GpuVariant) -> &CudaFunction {
        match v {
            GpuVariant::Basic => &self.basic,
            GpuVariant::Naive => &self.naive,
            _ => &self.coalesced,
        }
    }

    fn launch_cfg(v: GpuVariant, k: usize, m: usize) -> LaunchConfig {
        match v {
            GpuVariant::Naive => cfg_naive(k, m),
            GpuVariant::Basic => cfg_2d(k, m, 16, 16),
            _ => cfg_coalesced(k, m),
        }
    }

    pub fn run(
        &self,
        variant: GpuVariant,
        b: &MatrixI32,
        reps: usize,
    ) -> Result<GpuTimedResult, Box<dyn std::error::Error>> {
        match variant {
            GpuVariant::Basic | GpuVariant::Naive | GpuVariant::Coalesced => {
                self.run_simple(variant, b, reps)
            }
            GpuVariant::Streams => self.run_streams(b, reps),
            GpuVariant::Vram => self.run_vram(b, reps),
            GpuVariant::Oom => self.run_oom(b, reps),
        }
    }

    fn run_simple(
        &self,
        variant: GpuVariant,
        b: &MatrixI32,
        reps: usize,
    ) -> Result<GpuTimedResult, Box<dyn std::error::Error>> {
        assert_eq!(b.rows % 2, 0);
        let k = b.rows / 2;
        let m = b.cols;
        let k_i = k as i32;
        let m_i = m as i32;
        let out_len = k * m;
        let func = self.kernel_fn(variant);
        let cfg = Self::launch_cfg(variant, k, m);

        let ev_h2d0 = self.ctx.new_event(timing_flag())?;
        let ev_h2d1 = self.ctx.new_event(timing_flag())?;
        let ev_k0 = self.ctx.new_event(timing_flag())?;
        let ev_k1 = self.ctx.new_event(timing_flag())?;
        let ev_d2h0 = self.ctx.new_event(timing_flag())?;
        let ev_d2h1 = self.ctx.new_event(timing_flag())?;

        ev_h2d0.record(&self.stream)?;
        let b_dev = self.stream.clone_htod(&b.data)?;
        let mut a1_dev = self.stream.alloc_zeros::<i32>(out_len)?;
        let mut a2_dev = self.stream.alloc_zeros::<i32>(out_len)?;
        ev_h2d1.record(&self.stream)?;
        ev_h2d1.synchronize()?;
        let h2d_ms = ev_h2d0.elapsed_ms(&ev_h2d1)?;

        {
            let mut launch = self.stream.launch_builder(func);
            launch.arg(&b_dev);
            launch.arg(&mut a1_dev);
            launch.arg(&mut a2_dev);
            launch.arg(&k_i);
            launch.arg(&m_i);
            unsafe { launch.launch(cfg) }?;
            self.stream.synchronize()?;
        }

        ev_k0.record(&self.stream)?;
        for _ in 0..reps {
            let mut launch = self.stream.launch_builder(func);
            launch.arg(&b_dev);
            launch.arg(&mut a1_dev);
            launch.arg(&mut a2_dev);
            launch.arg(&k_i);
            launch.arg(&m_i);
            unsafe { launch.launch(cfg) }?;
        }
        ev_k1.record(&self.stream)?;
        ev_k1.synchronize()?;
        let kernel_ms = ev_k0.elapsed_ms(&ev_k1)? / reps as f32;

        ev_d2h0.record(&self.stream)?;
        let a1_host = self.stream.clone_dtoh(&a1_dev)?;
        let a2_host = self.stream.clone_dtoh(&a2_dev)?;
        ev_d2h1.record(&self.stream)?;
        ev_d2h1.synchronize()?;
        let d2h_ms = ev_d2h0.elapsed_ms(&ev_d2h1)?;
        let total_ms = h2d_ms + kernel_ms + d2h_ms;

        Ok(GpuTimedResult {
            a1: MatrixI32 {
                rows: k,
                cols: m,
                data: a1_host,
            },
            a2: MatrixI32 {
                rows: k,
                cols: m,
                data: a2_host,
            },
            variant,
            kernel_ms,
            h2d_ms,
            d2h_ms,
            total_ms,
            note: String::new(),
        })
    }

    /// One H2D, then K-range kernels on 3 streams in parallel, then one D2H.
    fn run_streams(
        &self,
        b: &MatrixI32,
        reps: usize,
    ) -> Result<GpuTimedResult, Box<dyn std::error::Error>> {
        assert_eq!(b.rows % 2, 0);
        let k = b.rows / 2;
        let m = b.cols;
        let n_streams = 3usize;
        let out_len = k * m;
        let m_i = m as i32;

        let streams: Vec<Arc<CudaStream>> = (0..n_streams)
            .map(|_| self.ctx.new_stream())
            .collect::<Result<_, _>>()?;

        let ev_h2d0 = self.ctx.new_event(timing_flag())?;
        let ev_h2d1 = self.ctx.new_event(timing_flag())?;
        let ev_k0 = self.ctx.new_event(timing_flag())?;
        let ev_k1 = self.ctx.new_event(timing_flag())?;
        let ev_d2h0 = self.ctx.new_event(timing_flag())?;
        let ev_d2h1 = self.ctx.new_event(timing_flag())?;

        ev_h2d0.record(&self.stream)?;
        let b_dev = self.stream.clone_htod(&b.data)?;
        let mut a1_dev = self.stream.alloc_zeros::<i32>(out_len)?;
        let mut a2_dev = self.stream.alloc_zeros::<i32>(out_len)?;
        self.stream.synchronize()?;
        ev_h2d1.record(&self.stream)?;
        ev_h2d1.synchronize()?;
        let h2d_ms = ev_h2d0.elapsed_ms(&ev_h2d1)?;

        let ranges: Vec<(usize, usize)> = {
            let mut v = Vec::new();
            let chunk = k.div_ceil(n_streams).max(1);
            let mut k0 = 0usize;
            while k0 < k {
                let tk = (k - k0).min(chunk);
                v.push((k0, tk));
                k0 += tk;
            }
            v
        };

        let func = &self.coalesced_offset;

        // Warmup: launch on all streams once
        {
            for (i, &(k0, tk)) in ranges.iter().enumerate() {
                let s = &streams[i % n_streams];
                let tk_i = tk as i32;
                let k0_i = k0 as i32;
                let cfg = cfg_coalesced(tk, m);
                let mut launch = s.launch_builder(func);
                launch.arg(&b_dev);
                launch.arg(&mut a1_dev);
                launch.arg(&mut a2_dev);
                launch.arg(&tk_i);
                launch.arg(&m_i);
                launch.arg(&k0_i);
                unsafe { launch.launch(cfg) }?;
            }
            for s in &streams {
                s.synchronize()?;
            }
        }

        ev_k0.record(&streams[0])?;
        for _ in 0..reps {
            for (i, &(k0, tk)) in ranges.iter().enumerate() {
                let s = &streams[i % n_streams];
                let tk_i = tk as i32;
                let k0_i = k0 as i32;
                let cfg = cfg_coalesced(tk, m);
                let mut launch = s.launch_builder(func);
                launch.arg(&b_dev);
                launch.arg(&mut a1_dev);
                launch.arg(&mut a2_dev);
                launch.arg(&tk_i);
                launch.arg(&m_i);
                launch.arg(&k0_i);
                unsafe { launch.launch(cfg) }?;
            }
            for s in &streams {
                s.synchronize()?;
            }
        }
        ev_k1.record(&streams[0])?;
        ev_k1.synchronize()?;
        let kernel_ms = ev_k0.elapsed_ms(&ev_k1)? / reps as f32;

        ev_d2h0.record(&self.stream)?;
        for s in &streams {
            self.stream.join(s)?;
        }
        let a1_host = self.stream.clone_dtoh(&a1_dev)?;
        let a2_host = self.stream.clone_dtoh(&a2_dev)?;
        ev_d2h1.record(&self.stream)?;
        ev_d2h1.synchronize()?;
        let d2h_ms = ev_d2h0.elapsed_ms(&ev_d2h1)?;
        let total_ms = h2d_ms + kernel_ms + d2h_ms;

        Ok(GpuTimedResult {
            a1: MatrixI32 {
                rows: k,
                cols: m,
                data: a1_host,
            },
            a2: MatrixI32 {
                rows: k,
                cols: m,
                data: a2_host,
            },
            variant: GpuVariant::Streams,
            kernel_ms,
            h2d_ms,
            d2h_ms,
            total_ms,
            note: format!(
                "{} K-slices on {} streams (parallel kernels)",
                ranges.len(),
                n_streams
            ),
        })
    }

    fn run_vram(
        &self,
        b: &MatrixI32,
        reps: usize,
    ) -> Result<GpuTimedResult, Box<dyn std::error::Error>> {
        let (free0, total) = self.ctx.mem_get_info()?;
        let target = (free0 as f64 * 0.90) as usize;
        let i32_bytes = std::mem::size_of::<i32>();

        let k = b.rows / 2;
        let m = b.cols;
        let out_len = k * m;
        let work_bytes = (b.data.len() + 2 * out_len) * i32_bytes;
        let filler_elems = target.saturating_sub(work_bytes) / i32_bytes;

        let mut fillers: Vec<CudaSlice<i32>> = Vec::new();
        let chunk = (64 * 1024 * 1024) / i32_bytes;
        let mut left = filler_elems;
        while left > 0 {
            let n = left.min(chunk).max(1);
            match self.stream.alloc_zeros::<i32>(n) {
                Ok(buf) => {
                    left = left.saturating_sub(n);
                    fillers.push(buf);
                }
                Err(_) => break,
            }
        }

        let (free1, _) = self.ctx.mem_get_info()?;
        let used = total.saturating_sub(free1);
        let mut result = self.run_simple(GpuVariant::Coalesced, b, reps)?;
        result.variant = GpuVariant::Vram;
        result.note = format!(
            "VRAM ~{:.1}% used ({:.0}/{:.0} MiB), fillerbufs={}",
            100.0 * used as f64 / total.max(1) as f64,
            used as f64 / (1024.0 * 1024.0),
            total as f64 / (1024.0 * 1024.0),
            fillers.len()
        );
        drop(fillers);
        Ok(result)
    }

    fn run_oom(
        &self,
        b: &MatrixI32,
        reps: usize,
    ) -> Result<GpuTimedResult, Box<dyn std::error::Error>> {
        assert_eq!(b.rows % 2, 0);
        let k = b.rows / 2;
        let m = b.cols;
        let (free, _) = self.ctx.mem_get_info()?;
        let i32b = std::mem::size_of::<i32>();
        let budget = (free / 8).max(16 * 1024 * 1024);
        let bytes_per_k = 4 * m * i32b;
        let mut tile_k = (budget / bytes_per_k.max(1)).max(1);
        if k > 1 {
            tile_k = tile_k.min(k.div_ceil(4).max(1));
        }
        tile_k = tile_k.min(k.max(1));

        let func = &self.coalesced;
        let mut a1 = vec![0i32; k * m];
        let mut a2 = vec![0i32; k * m];

        let max_b = 2 * tile_k * m;
        let max_out = tile_k * m;
        let mut b_dev = self.stream.alloc_zeros::<i32>(max_b)?;
        let mut a1_dev = self.stream.alloc_zeros::<i32>(max_out)?;
        let mut a2_dev = self.stream.alloc_zeros::<i32>(max_out)?;

        let ev0 = self.ctx.new_event(timing_flag())?;
        let ev1 = self.ctx.new_event(timing_flag())?;
        ev0.record(&self.stream)?;

        for _ in 0..reps {
            let mut k0 = 0usize;
            while k0 < k {
                let tk = (k - k0).min(tile_k);
                let b_off = 2 * k0 * m;
                let host_b = &b.data[b_off..b_off + 2 * tk * m];
                self.stream.memcpy_htod(host_b, &mut b_dev)?;

                let tk_i = tk as i32;
                let m_i = m as i32;
                let cfg = cfg_coalesced(tk, m);
                {
                    let mut launch = self.stream.launch_builder(func);
                    launch.arg(&b_dev);
                    launch.arg(&mut a1_dev);
                    launch.arg(&mut a2_dev);
                    launch.arg(&tk_i);
                    launch.arg(&m_i);
                    unsafe { launch.launch(cfg) }?;
                }

                let view1 = a1_dev.slice(..tk * m);
                let view2 = a2_dev.slice(..tk * m);
                let mut host_a1 = vec![0i32; tk * m];
                let mut host_a2 = vec![0i32; tk * m];
                self.stream.memcpy_dtoh(&view1, &mut host_a1)?;
                self.stream.memcpy_dtoh(&view2, &mut host_a2)?;

                let out_off = k0 * m;
                a1[out_off..out_off + tk * m].copy_from_slice(&host_a1);
                a2[out_off..out_off + tk * m].copy_from_slice(&host_a2);
                k0 += tk;
            }
            self.stream.synchronize()?;
        }

        ev1.record(&self.stream)?;
        ev1.synchronize()?;
        let total_ms = ev0.elapsed_ms(&ev1)? / reps as f32;

        Ok(GpuTimedResult {
            a1: MatrixI32 {
                rows: k,
                cols: m,
                data: a1,
            },
            a2: MatrixI32 {
                rows: k,
                cols: m,
                data: a2,
            },
            variant: GpuVariant::Oom,
            kernel_ms: total_ms,
            h2d_ms: 0.0,
            d2h_ms: 0.0,
            total_ms,
            note: format!(
                "out-of-core tiles K={tile_k} (GPU budget ≈ {:.1} MiB)",
                budget as f64 / (1024.0 * 1024.0)
            ),
        })
    }
}
