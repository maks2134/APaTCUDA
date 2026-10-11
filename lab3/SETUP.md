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

Рекомендуемая ветка: **CUDA 12.x или 13.x** (у драйвера уже UMD 13.3 — подойдёт и 12.8, и 13.x с сайта).

В установщике достаточно:

- CUDA Runtime / Toolkit
- NVRTC
- Visual Studio Integration (если предложит)

Проверка — **каждая команда отдельно** (не склеивать в одну строку).

В **cmd**:

```bat
nvcc --version
echo %CUDA_PATH%
```

В **PowerShell**:

```powershell
nvcc --version
echo $env:CUDA_PATH
```

`CUDA_PATH` обычно вида `C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.x`.

Если Toolkit новее драйвера (как 13.4 toolkit при драйвере с CUDA 13.3), lab3 всё равно собирается: ядро компилируется в **cubin** через `nvcc` в `build.rs`.

## 3. Rust (MSVC toolchain)

**Ссылка:** https://rustup.rs/  
→ скачать `rustup-init.exe`

В установщике выбери toolchain по умолчанию: **x86_64-pc-windows-msvc**.

Проверка (снова **по одной** команде):

```bat
rustc --version
cargo --version
```

## 4. Smoke-тест после установки

В новом терминале (чтобы подхватить PATH):

```powershell
cd D:\CUDA\lab3
cargo run --release -- --k 64 --m 64 --mode all --reps 1
```

Ожидается: имя GPU, времена CPU/GPU, `OK`, фрагмент матриц.

## 5. Для защиты с Mac (по желанию, не блокирует сборку)

| Что | Зачем | Ссылка |
|-----|--------|--------|
| **Tailscale** (на ПК и на Mac) | доступ к дому из аудитории | https://tailscale.com/download |
| **OpenSSH Server** (на Windows) | `ssh` с Mac | Параметры → Приложения → Дополнительные компоненты → OpenSSH Server |

Подробности запуска на защите — в [README.md](README.md).
