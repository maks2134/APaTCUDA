# Что скачать и поставить (Windows + RTX 3050)

Драйвер NVIDIA уже есть (`nvidia-smi` показывает RTX 3050). Нужны три вещи ниже.

> Если в фоне ещё крутится установка Visual Studio Build Tools от прошлого `winget` — дождись окончания **или** сними процессы в Диспетчере задач и поставь вручную по шагам ниже.

## 1. Visual Studio Build Tools 2022 (MSVC)

**Ссылка:** https://visualstudio.microsoft.com/downloads/  
→ раздел *Tools for Visual Studio* → **Build Tools for Visual Studio 2022**

При установке отметь workload:

- **Desktop development with C++**
  - MSVC v143
  - Windows 10/11 SDK

`cl.exe` **не обязан** быть в обычном `PATH` (это нормально). Rust и `build.rs` сами находят MSVC.

Проверка, что компилятор есть на диске — в **Developer PowerShell for VS** или:

```bat
"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=amd64
where cl
```

## 2. CUDA Toolkit

**Ссылка:** https://developer.nvidia.com/cuda-downloads  
→ Windows → x86_64 → exe (local)

Рекомендуемая ветка: **CUDA 12.x или 13.x**.

В установщике достаточно:

- CUDA Runtime / Toolkit
- NVRTC
- Visual Studio Integration (если предложит)

Проверка — **каждая команда отдельно**.

В **PowerShell**:

```powershell
nvcc --version
echo $env:CUDA_PATH
```

Ядро компилируется в **cubin** через `nvcc` в `build.rs` (`LAB4_CUDA_ARCH`, по умолчанию `sm_86`).

## 3. Rust (MSVC toolchain)

**Ссылка:** https://rustup.rs/  
→ скачать `rustup-init.exe`

В установщике выбери toolchain по умолчанию: **x86_64-pc-windows-msvc**.

```bat
rustc --version
cargo --version
```

## 4. Smoke-тест после установки

```powershell
cd D:\CUDA\lab4
cargo run --release -- --n 128 --mode all --reps 1
```

Ожидается: имя GPU, времена CPU/GPU, `OK`, значения `P25/P50/P75`.

## 5. Для защиты с Mac (по желанию)

| Что | Зачем | Ссылка |
|-----|--------|--------|
| **Tailscale** (на ПК и на Mac) | доступ к дому из аудитории | https://tailscale.com/download |
| **OpenSSH Server** (на Windows) | `ssh` с Mac | Параметры → Приложения → Дополнительные компоненты → OpenSSH Server |

Подробности — в [docs/DEMO_MAC.md](docs/DEMO_MAC.md).
