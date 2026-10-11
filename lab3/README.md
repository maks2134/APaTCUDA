# Lab 3 — деинтерливинг строк (вариант 28)

```
A1[k, j] = B[2k, j]
A2[k, j] = B[2k + 1, j]
```

CPU + GPU (CUDA events на ядро). Оптимизации: basic / naive / coalesced / streams / vram / oom.

## Установка (Windows + RTX 3050)

См. **[SETUP.md](SETUP.md)**.

## Запуск на ПК

```powershell
cd D:\CUDA\lab3
cargo run --release -- --k 2048 --m 2048 --mode all --gpu-variant all --reps 1 --save-image
```

| Флаг | Значение |
|------|----------|
| `--k`, `--m` | размеры |
| `--mode` | `all` \| `cpu` \| `gpu` |
| `--gpu-variant` | `basic` \| `naive` \| `coalesced` \| `streams` \| `vram` \| `oom` \| `all` |
| `--reps` | повторы kernel |
| `--save-image` | PNG → `output/` (в т.ч. `panel_*.png`) |

В таблице **время** GPU = **kernel** (без H2D/D2H в отчёте).

## Демо с Mac (защита)

Пошагово: **[docs/DEMO_MAC.md](docs/DEMO_MAC.md)**.

**Рекомендуется: Tailscale + [Windows App](https://apps.apple.com/app/windows-app/id1295203466)** (клиент RDP от Microsoft на Mac):

1. Windows: Tailscale + «Удалённый рабочий стол» Вкл. + `cargo build --release`
2. Mac: Tailscale + Windows App → **+ → Add PC** → PC name = `100.x.y.z`
3. Connect → запуск `lab3.exe`, показ `output\panel_*.png`

Подробно: [docs/DEMO_MAC.md](docs/DEMO_MAC.md). SSH — запасной вариант.

План B без сети: лог + `panel_*.png` заранее.
