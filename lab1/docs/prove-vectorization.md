# Доказательство векторизации (Windows Intel)

Отдельный блок из инструкции по установке. На защите открывайте этот файл.

Полная установка окружения: [windows-intel-setup.md](windows-intel-setup.md).  
Через **Intel VTune** (метрики + Assembly): [vtune-vectorization.md](vtune-vectorization.md).

---

## Зачем это нужно

Преподаватель спрашивает не только «быстрее ли», а **есть ли SIMD в машинном коде**.

| Что показать | Где смотреть | Что увидеть |
|--------------|--------------|-------------|
| Автовекторизация **есть** | `mul_add_block_auto` после `run-vec` | `mulps` / `addps` (или `vmulps` / `vaddps`) |
| Автовекторизации **нет** | та же функция после `run-novec` | `mulss` / `addss`, без пачек `mulps` |
| Ручная векторизация **есть** | `mul_add_block_sse2` | всегда `mulps` / `addps` |

Один и тот же исходник C1 (`src/mul_auto.rs`). Разница только в **флагах компилятора** (`RUSTFLAGS`).

Время (rdtsc) — **дополнение**. Без дизасма доказательство слабое.  
**Intel VTune** — сильный бонус (Hotspots + Microarchitecture Exploration); пошагово: [vtune-vectorization.md](vtune-vectorization.md).

---

## Что означают инструкции

| Инструкция | Смысл |
|------------|--------|
| `mulps` / `addps` | packed: **4 float** за раз (SSE) → векторизация |
| `vmulps` / `vaddps` | то же по смыслу, AVX |
| `mulss` / `addss` | scalar: **1 float** → векторизации нет |

На ARM Mac в дампе будут `fmul.4s` / `fadd.4s` (NEON). Для **этой лабы** на защите нужны **x86** (`mulps`/`mulss`) на Intel-ноутбуке.

---

## Подготовка (один раз)

В корне проекта, в **новом** cmd/PowerShell:

```bat
cd путь\к\lab1
cargo install cargo-show-asm
```

Проверка архитектуры:

```bat
rustc -vV
```

Должно быть: `host: x86_64-pc-windows-msvc`.

---

## Шаг 1. Автовекторизация ЕСТЬ (C1)

```bat
set RUSTFLAGS=-C opt-level=3 -C target-cpu=native
cargo asm --release --bin lab1 "lab1::mul_auto::mul_add_block_auto"
```

Короткий вариант (тот же смысл):

```bat
task asm
```

### Что искать в выводе

1. Заголовок функции:

```text
lab1::mul_auto::mul_add_block_auto:
```

2. Внутри тела:

```text
mulps
addps
```

или

```text
vmulps
vaddps
```

**Вердикт:** компилятор сам превратил цикл `for j in 0..12` в SIMD → автовекторизация **есть**.

---

## Шаг 2. Автовекторизации НЕТ (C1)

В **том же** терминале смените флаги и пересоберите дизасм:

```bat
set RUSTFLAGS=-C opt-level=3 -C llvm-args=-vectorize-loops=false -C llvm-args=-vectorize-slp=false
cargo asm --release --bin lab1 "lab1::mul_auto::mul_add_block_auto"
```

### Что искать

```text
mulss
addss
```

**Не** должно быть рядов `mulps` / `addps` в этой функции.

**Вердикт:** тот же код, но LLVM не векторизует циклы → автовекторизации **нет**.

Фраза устно:  
«Выключаю автовекторизацию флагами LLVM `-vectorize-loops=false` и `-vectorize-slp=false`, оставляя `-O3`. В дизасме скалярные `mulss`/`addss`.»

---

## Шаг 3. Ручной SSE2 (C2) — всегда есть

Это не автовекторизация, а **intrinsics** в `src/mul_sse2.rs`:

```bat
set RUSTFLAGS=-C opt-level=3 -C target-cpu=native
cargo asm --release --bin lab1 "lab1::mul_sse2::mul_add_block_sse2"
```

Ожидается снова:

```text
mulps
addps
```

Даже после novec-флагов у C2 packed-инструкции остаются: вы сами вызвали `_mm_mul_ps` / `_mm_add_ps`.

---

## Шаг 4. Время как второе доказательство

На одинаковых `L M N` (лучше больших):

```bat
task run-novec L=64 M=64 N=64
task run-vec L=64 M=64 N=64
```

Ожидаемо:

1. C1 в **novec** заметно медленнее C1 в **vec**.
2. C2 SSE2 **не медленнее** C1 в **vec**.
3. `match: yes` (все элементы C1 и C2 совпали в допуске).

Для float в методичке ориентир ускорения авто ~3–3.5× относительно скаляра (на живом Intel ближе к слайду).

---

## Если cargo-show-asm нет / не принимает

### dumpbin (Visual Studio)

1. Откройте **x64 Native Tools Command Prompt for VS**.
2. Соберите:

```bat
cd путь\к\lab1
task run-vec L=8 M=8 N=8
```

3. Дизасм:

```bat
dumpbin /DISASM target\release\lab1.exe > disasm_vec.txt
```

4. Откройте файл, найдите `mul_add_block_auto` и `mulps`.

5. Повторите после `task run-novec ...` → ищите `mulss`.

### CLion

1. Соберите Release (`task run-vec` / `run-novec`).
2. Поставьте breakpoint в `mul_add_block_auto` / `mul_add_block_sse2`.
3. Debug **Release**-сборки → View → **Disassembly**.

Для сравнения vec/novec нужны **две разные** Release-сборки с разными `RUSTFLAGS` (как в Task).

---

## Частые ошибки

| Ошибка | Почему плохо |
|--------|----------------|
| Смотреть дизасм на Mac ARM без `--target x86_64-...` | Увидите NEON (`fmul.4s`), не SSE2 лабы |
| Сравнивать Debug и Release | Debug не «выключенная векторизация» |
| Смотреть только `from_seed` или `main` | Векторизация нужна в **ядре** `mul_add_block_auto` |
| Забыть `set RUSTFLAGS=...` перед `cargo asm` | Получите «чужую» сборку |
| Доказывать только временем | Преподаватель может потребовать ассемблер |

---

## Шпаргалка команд (скопировать на защиту)

```bat
cd путь\к\lab1

REM --- C1: векторизация ЕСТЬ ---
set RUSTFLAGS=-C opt-level=3 -C target-cpu=native
cargo asm --release --bin lab1 "lab1::mul_auto::mul_add_block_auto"
REM ждать: mulps / addps

REM --- C1: векторизации НЕТ ---
set RUSTFLAGS=-C opt-level=3 -C llvm-args=-vectorize-loops=false -C llvm-args=-vectorize-slp=false
cargo asm --release --bin lab1 "lab1::mul_auto::mul_add_block_auto"
REM ждать: mulss / addss

REM --- C2: ручной SSE2 ---
set RUSTFLAGS=-C opt-level=3 -C target-cpu=native
cargo asm --release --bin lab1 "lab1::mul_sse2::mul_add_block_sse2"
REM ждать: mulps / addps

REM --- замеры ---
task run-novec L=64 M=64 N=64
task run-vec L=64 M=64 N=64
```

---

## Карта «есть / нет»

```text
src/mul_auto.rs  →  mul_add_block_auto
        │
        ├─ RUSTFLAGS vec     →  mulps / addps   = авто ЕСТЬ
        └─ RUSTFLAGS novec   →  mulss / addss   = авто НЕТ

src/mul_sse2.rs  →  mul_add_block_sse2
        └─ intrinsics        →  mulps / addps   = ручная ЕСТЬ
```
