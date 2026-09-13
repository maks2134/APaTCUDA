# Лаба 1 — Векторизация (Rust)

Блочное умножение матриц для курса «Архитектура процессоров».

- **Тип:** `float` (`f32`), блок **12×12**
- **C1:** `matmul_auto` — обычные циклы (автовекторизация вкл/выкл через Task)
- **C2:** `matmul_sse2` — ручной SSE2
- **Замер:** `rdtsc` на x86_64; только **Release**
- Без `clock()` / `time()`, без транспонирования, без печати матриц

## Документация

- Windows Intel: [docs/windows-intel-setup.md](docs/windows-intel-setup.md)
- Доказательство векторизации: [docs/prove-vectorization.md](docs/prove-vectorization.md)
- Intel VTune: [docs/vtune-vectorization.md](docs/vtune-vectorization.md)
- Разбор кода (для новичка в Rust): [docs/code-explained.md](docs/code-explained.md)

## Запуск

```bash
# Windows / Mac (нативная арх.)
task test
task run-vec L=64 M=64 N=64
task run-novec L=64 M=64 N=64

# Mac: SSE2 и rdtsc через Rosetta
task test-x86
task run-vec-x86 L=48 M=48 N=48
task run-novec-x86 L=48 M=48 N=48
```

`L M N` — число блоков 12×12. На защите увеличьте, пока C1 не займёт несколько секунд.

| Задача | Что делает |
|--------|------------|
| `task test` | тесты |
| `task run-vec` | с автовекторизацией |
| `task run-novec` | без автовекторизации |
| `task run-vec-x86` | то же на x86_64 (Mac) |
| `task run-novec-x86` | без векторизации на x86_64 (Mac) |
| `task test-x86` | тесты x86_64 (Mac) |
| `task asm` | дизасм ядра (нужен `cargo-show-asm`) |

## Windows + CLion (кратко)

1. Build Tools C++ + rustup (`x86_64-pc-windows-msvc`) + `winget install Task.Task`
2. Открыть папку с `Cargo.toml` в CLion
3. В Terminal: `task run-vec L=64 M=64 N=64`
4. Дизасм: `cargo install cargo-show-asm` → `task asm`  
   vec → `mulps`, novec → `mulss` (см. docs)

## Вывод программы

Пример:

```text
block 12x12 float | outer A=16x16 B=16x16 | x86_64
C1 auto:          123456 cycles  0.123 s
C2 sse2:          100000 cycles  0.100 s
match: yes  (max err 0.00e0)
```

## Структура

```text
src/main.rs      запуск и вывод
src/matrix.rs    блоки 12×12
src/mul_auto.rs  C1
src/mul_sse2.rs  C2
src/timing.rs    rdtsc
src/verify.rs    сравнение C1/C2
Taskfile.yml
docs/
```

## Чеклист к защите

- [ ] Release: `run-vec` и `run-novec`
- [ ] Две функции: auto + SSE2
- [ ] `match: yes`
- [ ] SSE2 не медленнее auto
- [ ] На Intel видны cycles (rdtsc)
- [ ] Дизасм и/или VTune
