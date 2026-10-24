use std::sync::Arc;

use cudarc::driver::sys::CUevent_flags;
use cudarc::driver::{
    CudaContext, CudaFunction, CudaModule, CudaSlice, CudaStream, LaunchConfig, PushKernelArg,
};
use cudarc::nvrtc::Ptx;

use crate::array::{Percentiles, next_pow2};

const SHARED: usize = 1024;
const STREAM_CHUNKS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuVariant {
    Basic,
    Shared,
    Streams,
    Vram,
    Oom,
}

impl GpuVariant {
    pub fn label(self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Shared => "shared",
            Self::Streams => "streams",
            Self::Vram => "vram",
            Self::Oom => "oom",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "basic" => Self::Basic,
            "shared" => Self::Shared,
            "streams" => Self::Streams,
            "vram" => Self::Vram,
            "oom" => Self::Oom,
            _ => return None,
        })
    }

    pub fn all() -> &'static [GpuVariant] {
        &[
            Self::Basic,
            Self::Shared,
            Self::Streams,
            Self::Vram,
            Self::Oom,
        ]
    }
}

pub struct GpuTimedResult {
    pub percentiles: Percentiles,
    pub variant: GpuVariant,
    pub kernel_ms: f32,
}

pub struct GpuEngine {
    ctx: Arc<CudaContext>,
    stream: Arc<CudaStream>,
    #[allow(dead_code)]
    module: Arc<CudaModule>,
    bitonic_step: CudaFunction,
    bitonic_shared_stage: CudaFunction,
    bitonic_shared_finish: CudaFunction,
    extract: CudaFunction,
}

fn ev_flag() -> Option<CUevent_flags> {
    Some(CUevent_flags::CU_EVENT_DEFAULT)
}

fn cfg_1d(n: usize, block: u32) -> LaunchConfig {
    LaunchConfig {
        grid_dim: (n.div_ceil(block as usize) as u32, 1, 1),
        block_dim: (block, 1, 1),
        shared_mem_bytes: 0,
    }
}

fn pad_host(a: &[f32]) -> (Vec<f32>, usize) {
    let n_pad = next_pow2(a.len());
    let mut v = vec![f32::MAX; n_pad];
    v[..a.len()].copy_from_slice(a);
    (v, n_pad)
}

fn merge_two(a: &[f32], b: &[f32]) -> Vec<f32> {
    let mut out = Vec::with_capacity(a.len() + b.len());
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i].total_cmp(&b[j]).is_le() {
            out.push(a[i]);
            i += 1;
        } else {
            out.push(b[j]);
            j += 1;
        }
    }
    out.extend_from_slice(&a[i..]);
    out.extend_from_slice(&b[j..]);
    out
}

fn merge_all(chunks: &[Vec<f32>]) -> Vec<f32> {
    if chunks.is_empty() {
        return Vec::new();
    }
    let mut cur = chunks.to_vec();
    while cur.len() > 1 {
        let mut next = Vec::new();
        let mut i = 0;
        while i < cur.len() {
            if i + 1 < cur.len() {
                next.push(merge_two(&cur[i], &cur[i + 1]));
                i += 2;
            } else {
                next.push(std::mem::take(&mut cur[i]));
                i += 1;
            }
        }
        cur = next;
    }
    cur.pop().unwrap()
}

impl GpuEngine {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let ctx = CudaContext::new(0)?;
        const CUBIN: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/percentiles.cubin"));
        let module = ctx.load_module(Ptx::from_binary(CUBIN.to_vec()))?;
        Ok(Self {
            stream: ctx.default_stream(),
            bitonic_step: module.load_function("bitonic_step")?,
            bitonic_shared_stage: module.load_function("bitonic_shared_stage")?,
            bitonic_shared_finish: module.load_function("bitonic_shared_finish")?,
            extract: module.load_function("extract_percentiles")?,
            module,
            ctx,
        })
    }

    pub fn device_name(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(self.ctx.name()?)
    }

    pub fn run(
        &self,
        variant: GpuVariant,
        a: &[f32],
        reps: usize,
    ) -> Result<GpuTimedResult, Box<dyn std::error::Error>> {
        match variant {
            GpuVariant::Basic => self.run_full(variant, a, reps, false),
            GpuVariant::Shared => self.run_full(variant, a, reps, true),
            GpuVariant::Streams => self.run_streams(a, reps),
            GpuVariant::Vram => self.run_vram(a, reps),
            GpuVariant::Oom => self.run_oom(a, reps),
        }
    }

    fn launch_step(
        &self,
        s: &CudaStream,
        data: &mut CudaSlice<f32>,
        n_pad: i32,
        j: i32,
        k: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut lb = s.launch_builder(&self.bitonic_step);
        lb.arg(data).arg(&n_pad).arg(&j).arg(&k);
        unsafe { lb.launch(cfg_1d(n_pad as usize, 256)) }?;
        Ok(())
    }

    fn launch_shared_stage(
        &self,
        s: &CudaStream,
        data: &mut CudaSlice<f32>,
        n_pad: i32,
        k: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let ku = k as u32;
        let cfg = LaunchConfig {
            grid_dim: ((n_pad as u32).div_ceil(ku), 1, 1),
            block_dim: (ku, 1, 1),
            shared_mem_bytes: 0,
        };
        let mut lb = s.launch_builder(&self.bitonic_shared_stage);
        lb.arg(data).arg(&n_pad).arg(&k);
        unsafe { lb.launch(cfg) }?;
        Ok(())
    }

    fn launch_shared_finish(
        &self,
        s: &CudaStream,
        data: &mut CudaSlice<f32>,
        n_pad: i32,
        k: i32,
        j_start: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cfg = LaunchConfig {
            grid_dim: ((n_pad as u32).div_ceil(SHARED as u32), 1, 1),
            block_dim: (SHARED as u32, 1, 1),
            shared_mem_bytes: 0,
        };
        let mut lb = s.launch_builder(&self.bitonic_shared_finish);
        lb.arg(data).arg(&n_pad).arg(&k).arg(&j_start);
        unsafe { lb.launch(cfg) }?;
        Ok(())
    }

    fn launch_extract(
        &self,
        s: &CudaStream,
        data: &CudaSlice<f32>,
        out3: &mut CudaSlice<f32>,
        n: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cfg = LaunchConfig {
            grid_dim: (1, 1, 1),
            block_dim: (1, 1, 1),
            shared_mem_bytes: 0,
        };
        let mut lb = s.launch_builder(&self.extract);
        lb.arg(data).arg(out3).arg(&n);
        unsafe { lb.launch(cfg) }?;
        Ok(())
    }

    fn sort(
        &self,
        s: &CudaStream,
        data: &mut CudaSlice<f32>,
        n_pad: usize,
        shared: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let np = n_pad as i32;
        let mut k = 2usize;
        while k <= n_pad {
            if shared && k <= SHARED {
                self.launch_shared_stage(s, data, np, k as i32)?;
            } else if shared && k > SHARED {
                let mut j = k >> 1;
                while j > SHARED / 2 {
                    self.launch_step(s, data, np, j as i32, k as i32)?;
                    j >>= 1;
                }
                self.launch_shared_finish(s, data, np, k as i32, (SHARED / 2) as i32)?;
            } else {
                let mut j = k >> 1;
                while j > 0 {
                    self.launch_step(s, data, np, j as i32, k as i32)?;
                    j >>= 1;
                }
            }
            k <<= 1;
        }
        Ok(())
    }

    fn time_sort_extract(
        &self,
        host_pad: &[f32],
        n: usize,
        n_pad: usize,
        shared: bool,
        reps: usize,
    ) -> Result<(Percentiles, f32), Box<dyn std::error::Error>> {
        let s = &self.stream;
        let mut data = s.clone_htod(host_pad)?;
        let mut out3 = s.alloc_zeros::<f32>(3)?;
        let n_i = n as i32;

        self.sort(s, &mut data, n_pad, shared)?;
        self.launch_extract(s, &data, &mut out3, n_i)?;
        s.synchronize()?;

        let mut acc = 0.0f32;
        for _ in 0..reps {
            s.memcpy_htod(host_pad, &mut data)?;
            s.synchronize()?;
            let t0 = self.ctx.new_event(ev_flag())?;
            let t1 = self.ctx.new_event(ev_flag())?;
            t0.record(s)?;
            self.sort(s, &mut data, n_pad, shared)?;
            self.launch_extract(s, &data, &mut out3, n_i)?;
            t1.record(s)?;
            t1.synchronize()?;
            acc += t0.elapsed_ms(&t1)?;
        }

        let out = s.clone_dtoh(&out3)?;
        Ok((
            Percentiles {
                p25: out[0],
                p50: out[1],
                p75: out[2],
            },
            acc / reps as f32,
        ))
    }

    fn run_full(
        &self,
        variant: GpuVariant,
        a: &[f32],
        reps: usize,
        shared: bool,
    ) -> Result<GpuTimedResult, Box<dyn std::error::Error>> {
        let (host_pad, n_pad) = pad_host(a);
        let (percentiles, kernel_ms) =
            self.time_sort_extract(&host_pad, a.len(), n_pad, shared, reps)?;
        Ok(GpuTimedResult {
            percentiles,
            variant,
            kernel_ms,
        })
    }

    fn run_streams(
        &self,
        a: &[f32],
        reps: usize,
    ) -> Result<GpuTimedResult, Box<dyn std::error::Error>> {
        let n = a.len();
        let streams: Vec<Arc<CudaStream>> = (0..STREAM_CHUNKS)
            .map(|_| self.ctx.new_stream())
            .collect::<Result<_, _>>()?;

        let chunk_len = n.div_ceil(STREAM_CHUNKS);
        let ranges: Vec<(usize, usize)> = (0..STREAM_CHUNKS)
            .filter_map(|i| {
                let lo = i * chunk_len;
                if lo >= n {
                    return None;
                }
                let hi = (lo + chunk_len).min(n);
                Some((lo, hi))
            })
            .collect();

        let pads: Vec<Vec<f32>> = ranges
            .iter()
            .map(|&(lo, hi)| pad_host(&a[lo..hi]).0)
            .collect();
        let pads_len: Vec<usize> = pads.iter().map(|p| p.len()).collect();
        let real_lens: Vec<usize> = ranges.iter().map(|&(lo, hi)| hi - lo).collect();

        let mut dev: Vec<CudaSlice<f32>> = pads
            .iter()
            .zip(streams.iter())
            .map(|(p, s)| s.clone_htod(p))
            .collect::<Result<_, _>>()?;

        for i in 0..ranges.len() {
            self.sort(&streams[i], &mut dev[i], pads_len[i], true)?;
        }
        for s in &streams {
            s.synchronize()?;
        }

        let mut acc = 0.0f32;
        let mut last = Percentiles {
            p25: 0.0,
            p50: 0.0,
            p75: 0.0,
        };

        for _ in 0..reps {
            for i in 0..ranges.len() {
                streams[i].memcpy_htod(&pads[i], &mut dev[i])?;
            }
            for s in &streams {
                s.synchronize()?;
            }

            let t0 = self.ctx.new_event(ev_flag())?;
            let t1 = self.ctx.new_event(ev_flag())?;
            t0.record(&streams[0])?;
            for i in 0..ranges.len() {
                self.sort(&streams[i], &mut dev[i], pads_len[i], true)?;
            }
            for s in &streams {
                s.synchronize()?;
            }
            t1.record(&streams[0])?;
            t1.synchronize()?;
            acc += t0.elapsed_ms(&t1)?;

            let mut sorted_chunks = Vec::with_capacity(ranges.len());
            for i in 0..ranges.len() {
                let full = streams[i].clone_dtoh(&dev[i])?;
                sorted_chunks.push(full[..real_lens[i]].to_vec());
            }
            last = Percentiles::from_sorted(&merge_all(&sorted_chunks));
        }

        Ok(GpuTimedResult {
            percentiles: last,
            variant: GpuVariant::Streams,
            kernel_ms: acc / reps as f32,
        })
    }

    fn run_vram(
        &self,
        a: &[f32],
        reps: usize,
    ) -> Result<GpuTimedResult, Box<dyn std::error::Error>> {
        let (free0, _total) = self.ctx.mem_get_info()?;
        let target = (free0 as f64 * 0.90) as usize;
        let f32b = std::mem::size_of::<f32>();
        let work = (next_pow2(a.len()) + 3) * f32b;
        let mut left = target.saturating_sub(work) / f32b;
        let chunk = (64 * 1024 * 1024) / f32b;

        let mut fillers = Vec::new();
        while left > 0 {
            let n = left.min(chunk).max(1);
            match self.stream.alloc_zeros::<f32>(n) {
                Ok(buf) => {
                    left = left.saturating_sub(n);
                    fillers.push(buf);
                }
                Err(_) => break,
            }
        }

        let r = self.run_full(GpuVariant::Vram, a, reps, true)?;
        drop(fillers);
        Ok(r)
    }

    fn run_oom(
        &self,
        a: &[f32],
        reps: usize,
    ) -> Result<GpuTimedResult, Box<dyn std::error::Error>> {
        let n = a.len();
        let tile = (1 << 16).min(next_pow2(n)).max(SHARED);

        let mut data = self.stream.alloc_zeros::<f32>(tile)?;
        let mut acc = 0.0f32;
        let mut last = Percentiles {
            p25: 0.0,
            p50: 0.0,
            p75: 0.0,
        };

        for _ in 0..reps {
            let t0 = self.ctx.new_event(ev_flag())?;
            let t1 = self.ctx.new_event(ev_flag())?;
            t0.record(&self.stream)?;

            let mut chunks = Vec::new();
            let mut off = 0;
            while off < n {
                let len = (n - off).min(tile);
                let (mut host_pad, pad) = pad_host(&a[off..off + len]);
                if pad > tile {
                    host_pad.truncate(tile);
                }
                while host_pad.len() < tile {
                    host_pad.push(f32::MAX);
                }
                self.stream.memcpy_htod(&host_pad, &mut data)?;
                self.sort(&self.stream, &mut data, pad.min(tile), true)?;
                self.stream.synchronize()?;
                let sorted = self.stream.clone_dtoh(&data)?;
                chunks.push(sorted[..len].to_vec());
                off += len;
            }
            last = Percentiles::from_sorted(&merge_all(&chunks));

            t1.record(&self.stream)?;
            t1.synchronize()?;
            acc += t0.elapsed_ms(&t1)?;
        }

        Ok(GpuTimedResult {
            percentiles: last,
            variant: GpuVariant::Oom,
            kernel_ms: acc / reps as f32,
        })
    }
}
