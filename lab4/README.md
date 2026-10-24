# Lab 4 — перцентили 25/50/75 (вариант 28)

```
P_q = A[⌊q · N⌋]  после сортировки A  (q ∈ {0.25, 0.50, 0.75})
```

CPU + GPU (CUDA events на sort+extract). GPU: своя bitonic-сортировка float.

| вариант | смысл |
|---------|--------|
| `basic` | все шаги bitonic в global memory |
| `shared` | мелкие стадии + хвост крупных — в shared memory |
| `streams` | 4 чанка сортируются **параллельно** на 4 CUDA stream, merge на host |
| `vram` | `shared` при занятой ~90% VRAM |
| `oom` | маленькие чанки подряд (out-of-core) |

`N ∈ [100, 10⁷]`, тип `float`.

## Установка (Windows + RTX 3050)

См. **[SETUP.md](SETUP.md)**.

## Запуск на ПК

```powershell
cd D:\CUDA\lab4
cargo run --release -- --n 1000000 --mode all --gpu-variant all --reps 1
```

| Флаг | Значение |
|------|----------|
| `--n` | размер массива |
| `--mode` | `all` \| `cpu` \| `gpu` |
| `--gpu-variant` | `basic` \| `shared` \| `streams` \| `vram` \| `oom` \| `all` |
| `--reps` | повторы kernel |
| `--fill` | `random` \| `pattern` |

В таблице **время** GPU = **sort + extract** (без H2D/D2H в отчёте).

Smoke:

```powershell
cargo run --release -- --n 128 --mode all --gpu-variant basic,shared --reps 1
```

## Демо с Mac (защита)

Пошагово: **[docs/DEMO_MAC.md](docs/DEMO_MAC.md)**.

**Рекомендуется: Tailscale + [Windows App](https://apps.apple.com/app/windows-app/id1295203466)** (клиент RDP от Microsoft на Mac):

1. Windows: Tailscale + «Удалённый рабочий стол» Вкл. + `cargo build --release`
2. Mac: Tailscale + Windows App → **+ → Add PC** → PC name = `100.x.y.z`
3. Connect → запуск `lab4.exe`, показ таблицы времён и `P25/P50/P75`

План B без сети: заранее сохранённый лог.
