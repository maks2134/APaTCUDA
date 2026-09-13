# Лаба 1 — Векторизация (Rust, x86_64)

Блочное умножение матриц: float, блоки 12×12.

- **C1:** `matmul_auto` — обычные циклы (автовекторизация вкл/выкл)
- **C2:** `matmul_sse2` — ручной SSE2
- **Замер:** `_rdtsc`, только **Release**
- Только **x86_64** (Windows, Intel или AMD)

## Документация

- Установка: [docs/windows-intel-setup.md](docs/windows-intel-setup.md)
- Дизасм (`mulps` / `mulss`): [docs/prove-vectorization.md](docs/prove-vectorization.md)
- **AMD uProf** (аналог VTune на AMD): [docs/uprof-vectorization.md](docs/uprof-vectorization.md)
- Разбор кода: [docs/code-explained.md](docs/code-explained.md)

## Запуск

```bat
task run-vec L=64 M=64 N=64
task run-novec L=64 M=64 N=64
task build-both-exe
task asm
task asm-sse2
```

| Задача | Что делает |
|--------|------------|
| `run-vec` | с автовекторизацией |
| `run-novec` | без автовекторизации |
| `build-both-exe` | сохранить `lab1_vec.exe` и `lab1_novec.exe` для uProf |
| `asm` | дизасм C1 |
| `asm-sse2` | дизасм C2 |

## Структура

```text
src/main.rs      запуск
src/matrix.rs    блоки 12×12
src/mul_auto.rs  C1
src/mul_sse2.rs  C2
src/timing.rs    rdtsc
src/verify.rs    сравнение C1/C2
```
