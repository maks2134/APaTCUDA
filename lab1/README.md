# Lab 1 — Vectorization (Rust)

Blocked matrix multiply for the «Architecture of Processors» course.

- **Element type:** `float` (`f32`)
- **Tile size:** 12×12 (variant)
- **C1:** `matmul_auto` — ordinary loops; LLVM auto-vectorization on/off via `RUSTFLAGS`
- **C2:** `matmul_sse2` — manual SSE2 (`xmm`, 3×4 floats per row)
- **Timing:** `rdtsc` / `_rdtsc()` on x86_64 (ARM Instant fallback is for local debug only)
- **Forbidden:** `clock()`, `time()`; run **Release** only
- **No transpose**; do not print matrix fragments

## Requirements

- Rust stable (`x86_64-pc-windows-msvc` on the defense laptop)
- [Task](https://taskfile.dev) (`go-task`)
- On Apple Silicon Mac: Rosetta + `x86_64-apple-darwin` target for SSE2 / `rdtsc`

## Quick start (Mac)

```bash
brew install go-task          # if needed
export PATH="$HOME/.cargo/bin:$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"

task test
task run-vec L=8 M=8 N=8      # correctness on ARM (NEON ≠ lab SSE2)
```

Full lab path under Rosetta (SSE2 + RDTSC):

```bash
rustup target add x86_64-apple-darwin
task test-x86
task run-vec-x86 L=48 M=48 N=48
task run-novec-x86 L=48 M=48 N=48
```

Increase `L`/`M`/`N` until **C1** takes several seconds on the defense machine.

Variable overrides are Task vars (no `--` before them). Extra CLI flags go after `--`:

```bash
task run-vec L=32 M=32 N=32 -- --seed 7
```

## Task targets

| Task | Meaning |
|------|---------|
| `task` / `task --list` | List tasks |
| `task test` | `cargo test --release` (native) |
| `task clippy` / `task fmt` | Lint / format |
| `task run-vec` | Release + auto-vectorization |
| `task run-novec` | Release + vectorization **off** |
| `task run-vec-x86` | Darwin only: x86_64 + vec |
| `task run-novec-x86` | Darwin only: x86_64 + no vec |
| `task test-x86` | Darwin only: tests on x86_64 |
| `task asm-auto` | Disassemble `matmul_auto` (`cargo-show-asm`) |

Override sizes:

```bash
task run-vec L=64 M=64 N=64
```

`RUSTFLAGS` are set per Task (not in `.cargo/config.toml`) so `vec` and `novec` do not clash.

## Windows Intel + CLion

Полная инструкция на русском: **[docs/windows-intel-setup.md](docs/windows-intel-setup.md)**  
Доказательство векторизации (`mulps` vs `mulss`): **[docs/prove-vectorization.md](docs/prove-vectorization.md)**  
Intel VTune: **[docs/vtune-vectorization.md](docs/vtune-vectorization.md)**  
(установка Build Tools / Rust / Task, CLion, прогоны).

### 1. Install toolchain

1. [Visual Studio Build Tools 2022](https://visualstudio.microsoft.com/visual-cpp-build-tools/) — workload **Desktop development with C++**
2. [rustup](https://rustup.rs) → choose `stable-x86_64-pc-windows-msvc`  
   Check: `rustc -vV` shows `host: x86_64-pc-windows-msvc` (not `aarch64`)
3. Task: `winget install Task.Task` (or Chocolatey `choco install go-task` / Scoop `scoop install task`)  
   Reopen the terminal, then `task --version`

### 2. Open in CLion

1. Install the **Rust** plugin (or use RustRover)
2. **File → Open** this folder (the one with `Cargo.toml`)
3. **Settings → Languages & Frameworks → Rust**: toolchain `x86_64-pc-windows-msvc`

### 3. Run (preferred: Terminal)

Same commands as on defense:

```bat
task run-vec L=64 M=64 N=64
task run-novec L=64 M=64 N=64
```

Optional CLion **Shell Script** / **Application** run configs:

- Executable: `task`
- Options: `run-vec` and a second config `run-novec`
- Working directory: project root  
  Do **not** use Debug for timing.

### 4. Prove vectorization

```bat
cargo install cargo-show-asm
task asm-auto
```

Expect `mulps` / `addps` (or `vmulps`) in the `vec` build of `matmul_auto`, and scalar `mulss` / `addss` in `novec`. Compare with the SSE2 kernel (`mul_add_block_sse2`) which always uses packed ops.

## CLI

```text
lab1 [--l L] [--m M] [--n N] [--seed S] [--tol T] [--skip-sse2]
```

Defaults: `L=M=N=16`, seed `42`, abs tolerance `1e-4`.  
Output: sizes, RDTSC cycles + wall seconds for C1/C2, `match: yes/no` over **all** floats (no matrix dumps).

## Layout

```text
src/main.rs      CLI + timed C1/C2 + compare
src/matrix.rs    Block 12×12, BlockMatrix
src/mul_auto.rs  matmul_auto (C1)
src/mul_sse2.rs  matmul_sse2 (C2, x86_64)
src/timing.rs    _rdtsc / Instant fallback
src/verify.rs    full-matrix compare
Taskfile.yml     vec / novec / x86 helpers
```

## Defense checklist

- [ ] Release only (`task run-vec` / `run-novec`)
- [ ] Two functions: auto + SSE2
- [ ] C1 vec vs C1 novec (same source, different `RUSTFLAGS`)
- [ ] C1 and C2 match completely
- [ ] SSE2 not slower than auto
- [ ] RDTSC shown on Intel laptop
- [ ] Disassembly or VTune if asked
