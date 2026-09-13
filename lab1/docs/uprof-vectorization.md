# Доказательство векторизации через AMD uProf

Инструкция для **Windows + AMD CPU** (лаба: float, блоки 12×12, Rust).

Связанные документы:

- установка окружения: [windows-intel-setup.md](windows-intel-setup.md) (там же Rust/Task; название файла историческое)
- дизассемблер (`mulps` / `mulss`): [prove-vectorization.md](prove-vectorization.md)

В методичке указан Intel VTune. На **AMD** его заменяет родной **AMD μProf (uProf)** — тот же смысл: hotspots, сравнение с/без автовекторизации, узкие места.  
Дизасм всё равно держите под рукой: **uProf + `mulps`/`mulss`** = сильная защита.

Скачать: [AMD uProf](https://www.amd.com/en/developer/uprof.html)  
Документация: [Hotspots в GUI](https://docs.amd.com/r/en-US/68658-uProf-getting-started-guide/Hotspots-Analysis-Using-AMD-uProf-GUI)

---

## Что доказываем

Нужны **два прогона** (лучше сохранить два `.exe`):

| Прогон | Как собрать | Ожидание в uProf |
|--------|-------------|------------------|
| C1 **с** автовекторизацией | `task run-vec L=…` | hotspot в `mul_add_block_auto`; **меньше** CPU Time |
| C1 **без** автовекторизации | `task run-novec L=…` | тот же hotspot **дольше** |
| C2 ручной SSE2 | в том же запуске | время C2 не хуже C1-vec; в asm ядра — packed SIMD |

Устно:  
«Процессор AMD, поэтому вместо VTune — AMD uProf. Один исходник `mul_add_block_auto`: с автовекторизацией функция быстрее в Hotspots, без неё — медленнее. SIMD подтверждаю дизасмом.»

---

## 1. Установка AMD uProf

1. Скачайте установщик с [amd.com/developer/uprof](https://www.amd.com/en/developer/uprof.html).
2. Установите на Windows (**AMD** CPU, `PROCESSOR_ARCHITECTURE=AMD64`).
3. Запустите **AMD uProf** (GUI). При необходимости согласитесь на установку драйверов sampling.
4. Проверка CLI (путь может отличаться):

```bat
where AMDuProfCLI
AMDuProfCLI --help
```

Часто лежит вроде:

```text
C:\Program Files\AMD\AMDuProf\bin\AMDuProfCLI.exe
```

---

## 2. Как сохранить два `.exe` (vec и novec)

`cargo build` / `task run-*` каждый раз **перезаписывают** один файл `target\release\lab1.exe`.  
Для uProf нужны **две разные** копии, иначе сравните один и тот же бинарник дважды.

### Способ A — через Task (проще)

В корне проекта:

```bat
task build-both-exe
```

Или по отдельности:

```bat
task build-vec-exe
task build-novec-exe
```

Появятся:

```text
target\release\lab1_vec.exe     ← с автовекторизацией
target\release\lab1_novec.exe   ← без автовекторизации
```

Проверка:

```bat
dir target\release\lab1_*.exe
```

Запуск вручную (подобрать размер «несколько секунд»):

```bat
target\release\lab1_vec.exe --l 64 --m 64 --n 64
target\release\lab1_novec.exe --l 64 --m 64 --n 64
```

### Способ B — вручную (cmd)

```bat
cd путь\к\lab1

REM 1) сборка С автовекторизацией → сразу сохранить копию
task run-vec L=64 M=64 N=64
copy /Y target\release\lab1.exe target\release\lab1_vec.exe

REM 2) сборка БЕЗ автовекторизации (lab1.exe перезапишется — vec уже сохранён)
task run-novec L=64 M=64 N=64
copy /Y target\release\lab1.exe target\release\lab1_novec.exe
```

### Способ C — вручную (PowerShell)

```powershell
cd путь\к\lab1

$env:RUSTFLAGS="-C opt-level=3 -C target-cpu=native"
cargo build --release
Copy-Item -Force target\release\lab1.exe target\release\lab1_vec.exe

$env:RUSTFLAGS="-C opt-level=3 -C llvm-args=-vectorize-loops=false -C llvm-args=-vectorize-slp=false"
cargo build --release
Copy-Item -Force target\release\lab1.exe target\release\lab1_novec.exe
```

### Важно

1. Сначала копируете **vec**, потом собираете **novec** — порядок важен.  
2. В uProf указываете `lab1_vec.exe` и `lab1_novec.exe`, **не** общий `lab1.exe`.  
3. При профиле аргументы: `--l 64 --m 64 --n 64` (или ваши размеры).  
4. `L M N` подберите так, чтобы C1 шла **несколько секунд**.

Для лучших имён функций в uProf (по желанию) добавьте в `RUSTFLAGS`: `-C debuginfo=line-tables-only`.

---

## 3. Профиль в GUI (основной путь)

### 3.1. Hotspots для vec

1. Welcome → **Profile an Application**.
2. **Select Profile Target**:
   - Application: `…\lab1\target\release\lab1_vec.exe`
   - Application arguments: `--l 64 --m 64 --n 64` (ваши размеры)
   - Working directory: корень `lab1`
3. **Next** → Predefined Configs → **Hotspots**.
4. Timer Interval: по умолчанию 10 ms; для коротких прогонов можно уменьшить, для длинных — оставить.
5. Advanced Options (по желанию):
   - **Enable CSS** (call stack sampling) — чтобы видеть цепочку вызовов.
6. **Start Profile**.

После сбора откроется результат.

### 3.2. Что смотреть в отчёте

1. Список функций / hotspots — сортировка по CPU Time / Samples.
2. Найдите (по подстроке):
   - `mul_add_block_auto`
   - `matmul_auto`
   - `mul_add_block_sse2`
   - `lab1` (если имя mangled)
3. Откройте функцию → **Source** / **Disassembly** (если есть в вашей версии uProf).
4. В Assembly у **vec**-сборки для C1 желательно видеть packed (`mulps`/`vmulps`); у **novec** — `mulss`/`addss`.  
   Если в uProf asm бедный — тот же вывод через `cargo asm` / `task asm`.

### 3.3. Повторить для novec

Тот же сценарий, но:

- Application: `lab1_novec.exe`
- те же `--l --m --n`

Сравните CPU Time у `mul_add_block_auto`: **novec > vec**.

---

## 4. Профиль через CLI (удобно для скриншотов)

Из «x64 Native Tools» / PowerShell (подставьте свой путь к CLI):

```bat
cd путь\к\lab1

AMDuProfCLI profile --config hotspots -g --detail -o uprof_vec ^
  target\release\lab1_vec.exe --l 64 --m 64 --n 64

AMDuProfCLI profile --config hotspots -g --detail -o uprof_novec ^
  target\release\lab1_novec.exe --l 64 --m 64 --n 64
```

Флаги:

| Флаг | Зачем |
|------|--------|
| `--config hotspots` | анализ горячих точек |
| `-g` | call stacks |
| `--detail` | подробный отчёт (в т.ч. `report.csv`) |
| `-o …` | каталог результатов |

Отчёт можно открыть в GUI (**Import session** / путь к `.uprof`) или смотреть `report.csv`.

Дизасм в отчёте CLI (если нужно):

```bat
AMDuProfCLI report -i uprof_vec --detail --disasm -o uprof_vec_disasm
```

(точные флаги смотрите в `AMDuProfCLI report --help` вашей версии.)

---

## 5. Сравнение vec vs novec на защите

| | lab1_vec | lab1_novec |
|---|----------|------------|
| CPU Time `mul_add_block_auto` | меньше | больше |
| Assembly C1 | `mulps` / `vmulps` | `mulss` / `addss` |
| `mul_add_block_sse2` | packed SIMD | packed SIMD |
| Вывод программы | меньше тактов C1 | больше тактов C1 |
| `совпадение` | да | да |

Дополнительно покажите консоль:

```bat
target\release\lab1_vec.exe --l 64 --m 64 --n 64
target\release\lab1_novec.exe --l 64 --m 64 --n 64
```

**rdtsc + совпадение + uProf Hotspots + дизасм** = полный комплект.

---

## 6. Текст для преподавателя

1. «CPU AMD → профилировщик **AMD uProf**, аналог VTune из слайдов.»  
2. «Hotspots: время в `mul_add_block_auto` и `mul_add_block_sse2`.»  
3. «Сборка с автовекторизацией быстрее по CPU Time; без (`-vectorize-loops=false`) — медленнее.»  
4. «SIMD подтверждаю дизасмом: vec — packed, novec — scalar; C2 всегда packed.»  
5. «C1 и C2 совпадают; SSE2 не медленнее auto.»

---

## 7. Типичные проблемы

| Проблема | Что делать |
|----------|------------|
| Прогон слишком короткий | Увеличить `L M N` |
| Не видно имён Rust | `-C debuginfo=line-tables-only`; искать `mul_add` |
| Vec и novec одинаковые | Забыли `copy` двух exe / профилируете один файл |
| Драйвер / права | Переустановить uProf, запуск от админа |
| Нет `mulps` в C1-vec | См. [prove-vectorization.md](prove-vectorization.md); правильный `$env:RUSTFLAGS` в PowerShell |

---

## 8. Чеклист

- [ ] AMD uProf установлен
- [ ] Есть `lab1_vec.exe` и `lab1_novec.exe` (`task build-both-exe`)
- [ ] Прогон несколько секунд
- [ ] Hotspots: найдены ядра C1 и C2
- [ ] novec медленнее vec по CPU Time
- [ ] Дизасм: packed vs scalar (uProf и/или `cargo asm`)
- [ ] Готова фраза «uProf вместо VTune, потому что AMD»

---

## 9. Связка с дизассемблером

uProf показывает **где** тратится время.  
Дизасм показывает **какие** инструкции:

```powershell
$env:RUSTFLAGS="-C opt-level=3 -C target-cpu=native"
cargo asm --release --bin lab1 "lab1::mul_auto::mul_add_block_auto"

$env:RUSTFLAGS="-C opt-level=3 -C llvm-args=-vectorize-loops=false -C llvm-args=-vectorize-slp=false"
cargo asm --release --bin lab1 "lab1::mul_auto::mul_add_block_auto"
```

Подробно: [prove-vectorization.md](prove-vectorization.md).
