# Подготовка лабы на Windows (Intel / AMD x86_64)

Пошаговая инструкция: установка окружения, запуск в CLion, замеры и **доказательство**, что автовекторизация есть / её нет.

Вариант лабы: **float**, блоки **12×12**. Язык: **Rust**. Запуск только **Release**. Таймер: **`rdtsc` / `_rdtsc()`** (не `clock()` / `time()`).

---

## 0. Что должно получиться в итоге

На ноутбуке с **настоящим x86_64** (не ARM):

1. Собирается и запускается `lab1.exe` в Release.
2. Две сборки одной программы:
   - **с** автовекторизацией (`task run-vec`);
   - **без** автовекторизации (`task run-novec`).
3. Считаются **C1** (`matmul_auto`) и **C2** (`matmul_sse2`), вывод: такты rdtsc, секунды, `match: yes`.
4. В дизассемблере:
   - C1 + vec → `mulps` / `addps` (или `vmulps`);
   - C1 + novec → `mulss` / `addss`;
   - C2 SSE2 → всегда `mulps` / `addps`.

---

## 1. Проверить процессор

Откройте **PowerShell** или **cmd**:

```bat
echo %PROCESSOR_ARCHITECTURE%
```

Ожидается: `AMD64`.

Если `ARM64` — этот ноутбук для защиты по SSE2/`rdtsc` **не подходит** (нужен Intel/AMD x86_64).

---

## 2. Установить Visual Studio Build Tools

Rust на Windows линкуется через MSVC.

1. Скачайте [Build Tools for Visual Studio 2022](https://visualstudio.microsoft.com/visual-cpp-build-tools/).
2. В установщике отметьте workload:

   **Desktop development with C++**  
   (Разработка классических приложений на C++)

3. Установите и **перезапустите** терминал (лучше перезагрузить ПК).

Можно поставить полный **Visual Studio 2022 Community** с тем же workload — тоже нормально.  
Для лабы достаточно Build Tools.

---

## 3. Установить Rust (MSVC, x86_64)

1. Откройте [https://rustup.rs](https://rustup.rs) и скачайте `rustup-init.exe`.
2. Запустите установщик.
3. Выберите toolchain по умолчанию:

   **`stable-x86_64-pc-windows-msvc`**

4. После установки откройте **новый** терминал и проверьте:

```bat
rustc -vV
cargo --version
```

В выводе `rustc -vV` обязательно:

```text
host: x86_64-pc-windows-msvc
```

Не должно быть `aarch64`.

Дополнительно (удобно для проверки и форматирования):

```bat
rustup component add clippy rustfmt
```

---

## 4. Установить Task (go-task)

Единая точка запуска сборок — файл `Taskfile.yml` в корне проекта.

**Вариант A (рекомендуется):**

```bat
winget install Task.Task
```

**Вариант B:** Chocolatey — `choco install go-task`  
**Вариант C:** Scoop — `scoop install task`

Закройте и снова откройте терминал:

```bat
task --version
task --list
```

(команду `task --list` запускайте уже из папки проекта).

---

## 5. Скопировать проект на Windows-ноутбук

Скопируйте всю папку `lab1` (с `Cargo.toml`, `src/`, `Taskfile.yml`).

В терминале:

```bat
cd путь\к\lab1
dir
```

Должны быть видны как минимум:

- `Cargo.toml`
- `Taskfile.yml`
- `src\`
- `docs\`

---

## 6. CLion (опционально, но удобно)

1. Установите **CLion** (или **RustRover**).
2. Плагин **Rust** (в CLion: Settings → Plugins).
3. **File → Open** → папка с `Cargo.toml`.
4. **Settings → Languages & Frameworks → Rust**:
   - toolchain: `x86_64-pc-windows-msvc`
   - путь к `rustc` / `cargo` из `%USERPROFILE%\.cargo\bin`

### Запуск из Terminal в CLion

Нижняя панель **Terminal** (это обычный cmd/PowerShell с PATH):

```bat
task test
task run-vec L=16 M=16 N=16
task run-novec L=16 M=16 N=16
```

### Run Configuration (по желанию)

Две конфигурации типа **Shell Script** / **Application**:

| Поле | Config 1 | Config 2 |
|------|----------|----------|
| Executable | `task` | `task` |
| Program arguments | `run-vec L=64 M=64 N=64` | `run-novec L=64 M=64 N=64` |
| Working directory | корень `lab1` | корень `lab1` |

**Не** используйте Debug для замеров времени — только Release (Task уже гоняет `--release`).

---

## 7. Первый запуск и тесты

Из корня проекта:

```bat
task test
```

Ожидается: все тесты зелёные (на Windows x86_64 будут и тесты SSE2).

Маленький прогон:

```bat
task run-vec L=8 M=8 N=8
```

В выводе должно быть примерно:

```text
arch: x86_64
timer: rdtsc cycles
C1 matmul_auto:  ... rdtsc cycles | ... s
C2 matmul_sse2:  ... rdtsc cycles | ... s
match: yes
```

Если `arch: aarch64` или timer про ARM — вы не на нужном toolchain / не на Intel-ноутбуке.

---

## 8. Размеры для защиты (несколько секунд на C1)

Внешние `L`, `M`, `N` — это **число блоков 12×12**, не размер в float.

Поднимайте, пока **C1** не станет занимать **несколько секунд**:

```bat
task run-vec L=64 M=64 N=64
task run-novec L=64 M=64 N=64
```

Если всё ещё быстро — `80`, `96`, `128` и т.д.

Сравнение на защите:

1. `run-novec` — C1 медленнее (скаляр).
2. `run-vec` — C1 быстрее (авто-SIMD).
3. C2 SSE2 **не медленнее**, чем C1 в `run-vec`.
4. `match: yes`.

Переменные Task задаются **без** `--` перед ними:

```bat
task run-vec L=48 M=48 N=48
```

Доп. флаги программы — после `--`:

```bat
task run-vec L=32 M=32 N=32
```

---

## 9. Доказать векторизацию (главное на защите)

**Отдельный короткий документ (откройте на защите):**  
→ **[prove-vectorization.md](prove-vectorization.md)**

Ниже — сжатая копия того же блока.

Время — только намёк. Доказательство — **дизассемблер**.

### 9.1. Установить cargo-show-asm

```bat
cargo install cargo-show-asm
```

### 9.2. Автовекторизация ЕСТЬ (C1, vec)

```bat
set RUSTFLAGS=-C opt-level=3 -C target-cpu=native
cargo asm --release --bin lab1 "lab1::mul_auto::mul_add_block_auto"
```

Или:

```bat
task asm
```

**Ищите в дампе:**

```text
mulps
addps
```

или

```text
vmulps
vaddps
```

Это packed SIMD (несколько float за раз) → **автовекторизация есть**.

### 9.3. Автовекторизации НЕТ (C1, novec)

В **том же** терминале:

```bat
set RUSTFLAGS=-C opt-level=3 -C llvm-args=-vectorize-loops=false -C llvm-args=-vectorize-slp=false
cargo asm --release --bin lab1 "lab1::mul_auto::mul_add_block_auto"
```

**Ищите:**

```text
mulss
addss
```

Не должно быть пачек `mulps`/`addps` в этой функции.

Фраза на защите:  
«Один исходник `mul_add_block_auto`. С автовекторизацией LLVM пишет `mulps`, с выключенной — `mulss`.»

### 9.4. Ручной SSE2 (C2) — всегда «есть»

```bat
set RUSTFLAGS=-C opt-level=3 -C target-cpu=native
cargo asm --release --bin lab1 "lab1::mul_sse2::mul_add_block_sse2"
```

Снова `mulps` / `addps` — это intrinsics из `src/mul_sse2.rs`, не автовекторизация.

### 9.5. Альтернатива без cargo-show-asm

Соберите Release:

```bat
task run-vec L=8 M=8 N=8
```

Дизасм:

```bat
dumpbin /DISASM target\release\lab1.exe > disasm_vec.txt
```

(`dumpbin` из «x64 Native Tools Command Prompt for VS»).

Откройте `disasm_vec.txt`, найдите `mul_add_block_auto` / `mulps`.

То же для novec-сборки после `task run-novec ...`.

В CLion: можно Debug по **Release**-сборке и открыть окно **Disassembly** на функции ядра.

Подробности, таблица инструкций, типичные ошибки и шпаргалка команд — в **[prove-vectorization.md](prove-vectorization.md)**.  
Доказательство через **AMD uProf** (Hotspots, сравнение vec/novec; аналог VTune на AMD) — в **[uprof-vectorization.md](uprof-vectorization.md)**.

---

## 10. Что где в коде (чтобы показать на защите)

| Что | Файл | Функция |
|-----|------|---------|
| C1 авто | `src/mul_auto.rs` | `matmul_auto`, `mul_add_block_auto` |
| C2 SSE2 (intrinsics) | `src/mul_sse2.rs` | `matmul_sse2`, `mul_add_block_sse2` |
| Блок 12×12 | `src/matrix.rs` | `Block`, `BlockMatrix` |
| rdtsc | `src/timing.rs` | `read_cycles` → `_rdtsc()` |
| Сравнение C1/C2 | `src/verify.rs` | `matrices_close` |
| Запуск vec/novec | `Taskfile.yml` | `run-vec`, `run-novec` |

Отдельного `.asm` файла нет: в задании **intrinsics = альтернатива ассемблеру**.

---

## 11. Типичные проблемы

| Проблема | Что делать |
|----------|------------|
| `linker link.exe not found` | Не установлен workload C++ в Build Tools; переустановить, переоткрыть терминал |
| `host: aarch64-...` | Не тот rustup; нужен `x86_64-pc-windows-msvc` |
| `task` не находится | `winget install Task.Task`, новый терминал, PATH |
| `match: NO` | На нормальных размерах при допуске `1e-4` обычно проходит; проверьте, что сравниваете C1 и C2 одной сборки |
| Debug «быстрее править» | Для замеров и дизасма — только Release |
| В asm нет `mulps` на vec | Проверьте, что `RUSTFLAGS` выставлены **в той же** сессии, что и `cargo asm`; смотрите именно `mul_add_block_auto` |
| C2 медленнее C1 vec | Увеличить размер / убедиться, что Release; на защите SSE2 не должен проигрывать авто |

---

## 12. Чеклист перед защитой

- [ ] Ноутбук Intel/AMD, `PROCESSOR_ARCHITECTURE=AMD64`
- [ ] `rustc -vV` → `x86_64-pc-windows-msvc`
- [ ] Build Tools C++ установлены
- [ ] `task --version` работает
- [ ] `task test` — OK
- [ ] `task run-vec` и `task run-novec` с большим `L M N` (C1 несколько секунд)
- [ ] В выводе `rdtsc cycles`, `match: yes`
- [ ] Дизасм C1 vec: `mulps`/`addps`
- [ ] Дизасм C1 novec: `mulss`/`addss`
- [ ] Дизасм C2: `mulps`/`addps`
- [ ] Знаете, где intrinsics (`mul_sse2.rs`) и где обычные циклы (`mul_auto.rs`)

---

## 13. Краткие команды (шпаргалка)

```bat
cd путь\к\lab1

task test
task run-vec L=64 M=64 N=64
task run-novec L=64 M=64 N=64

cargo install cargo-show-asm

set RUSTFLAGS=-C opt-level=3 -C target-cpu=native
cargo asm --release --bin lab1 "lab1::mul_auto::mul_add_block_auto"

set RUSTFLAGS=-C opt-level=3 -C llvm-args=-vectorize-loops=false -C llvm-args=-vectorize-slp=false
cargo asm --release --bin lab1 "lab1::mul_auto::mul_add_block_auto"

set RUSTFLAGS=-C opt-level=3 -C target-cpu=native
cargo asm --release --bin lab1 "lab1::mul_sse2::mul_add_block_sse2"
```
