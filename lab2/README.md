# Lab 2 — оптимизация доступа в кэш

| Результат | Алгоритм |
|-----------|----------|
| **C0** | скаляр, порядок `i→j→k`, без NEON |
| **C1** | NEON-ядро, без L1/L2/L3-тайлинга, `i→j→k` |
| **C2** | NEON + тайлы **L3→L2→L1** + packing |

Цепочка C2: `matmul_l3` → `matmul_l2` → `matmul_l1` → `mul_add_block_neon`.

## Сравнение скорости (честно)

Сравнивать только **одинаковые** `--l --m --n --reps`. Смотреть на **секунды/тики в выводе программы**, не на длину записи samply и не на «99% в функции».

```bash
task run-novec L=128 M=128 N=128 MODE=all REPS=1
```

Ожидаемо: C0 медленнее всех; C1 и C2 быстрее C0; C2 ≥ C1 на больших размерах (256).

## Профайлинг (samply) — один и тот же размер!

```bash
task build-both

# ВСЕ ТРИ с одинаковыми L/M/N/reps:
samply record ./target/release/lab2_novec --mode scalar --l 128 --m 128 --n 128 --reps 1
samply record ./target/release/lab2_vec   --mode neon   --l 128 --m 128 --n 128 --reps 1
samply record ./target/release/lab2_vec   --mode l3     --l 128 --m 128 --n 128 --reps 1
```

Скаляр только через `lab2_novec` (иначе LLVM сам векторизует циклы).

В профайлере сравнивай **длительность записи / wall time** при одинаковых аргументах.  
«99% в `mul_add_block_*`» значит «время ушло сюда», а не «эта версия быстрее».

## Дизассемблирование

```bash
task asm-scalar
task asm-neon
task asm-l3
```

## Обоснование

- C0: скалярные `f32`, плохая locality.
- C1: SIMD внутри блока 12×12.
- C2: SIMD + панели в L3/L2/L1, ширина кратна 64 байтам.
