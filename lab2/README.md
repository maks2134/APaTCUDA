# Lab 2 — оптимизация доступа в кэш

| Результат | Алгоритм |
|-----------|----------|
| **C1** | NEON-ядро, **без** L1/L2/L3-тайлинга, порядок `i→j→k` |
| **C2** | то же NEON-ядро + тайлы **L3→L2→L1** + packing панелей A/B |

Цепочка в профайлере: `matmul_l3` → `matmul_l2` → `matmul_l1` → `mul_add_block_neon`.

## Какое ускорение ждать

| Сравнение | Типично |
|-----------|---------|
| кэш vs **скаляр** | ~10–30× |
| кэш vs **ручная векторизация** (C2/C1) | ~1.7–2.3× |

```bash
task run L=256 M=256 N=256 MODE=all REPS=1
# ускорение C2/C1: ~2.4×
```

На 64×64 набор ≈6.8 MiB ≈ SLC — почти без выигрыша.

## Профайлинг

```bash
task build
samply record ./target/release/lab2 --mode neon --l 256 --m 256 --n 256 --reps 2
samply record ./target/release/lab2 --mode l3   --l 256 --m 256 --n 256 --reps 2
```

## Дизассемблирование

```bash
cargo install cargo-show-asm

task asm-neon    # mul_add_block_neon
task asm-l3      # matmul_l3 → L2 → L1
task asm         # оба
```

```bash
cargo asm --release --bin lab2 "lab2::mul_neon::mul_add_block_neon"
cargo asm --release --bin lab2 "lab2::mul_l3::matmul_l3"
```

## Обоснование

- C1 (`i→j→k`): векторизация есть, но плохая locality.
- C2: панель A пакуется и держится, пока не обработаны все j-панели B; размеры ≤ `⌊cache/3·0.9⌋`.
- Ширина A/B кратна 64 байтам.
