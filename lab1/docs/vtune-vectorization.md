# Доказательство векторизации через Intel VTune

Инструкция для **Windows + Intel CPU** (лаба: float, блоки 12×12, Rust).

Связанные документы:

- установка окружения: [windows-intel-setup.md](windows-intel-setup.md)
- дизассемблер (`mulps` / `mulss`): [prove-vectorization.md](prove-vectorization.md)

VTune в методичке — способ **доказать векторизацию** и бонус за разбор узких мест.  
Дизасм всё равно держите под рукой: VTune + `mulps`/`mulss` = сильная защита.

---

## Что доказываем с помощью VTune

Нужны **два (лучше три) прогона** одного бинарника/проекта:

| Прогон | Как собрать | Ожидание в VTune |
|--------|-------------|------------------|
| C1 **с** автовекторизацией | `task run-vec L=…` | hotspot в ядре умножения; **выше** vectorization / FP SIMD |
| C1 **без** автовекторизации | `task run-novec L=…` | тот же hotspot **дольше**; vectorization **ниже** / больше скаляра |
| C2 ручной SSE2 | тот же `lab1.exe` (считается в том же запуске) | время C2 не хуже C1-vec; SIMD в `mul_add_block_sse2` |

Устно:  
«Один исходник `mul_add_block_auto`. При включённой автовекторизации VTune показывает эффективное использование SIMD; при выключенной (`-vectorize-loops=false`) — метрики векторизации падают, CPU Time растёт.»

---

## 1. Установка VTune

1. Скачайте **Intel VTune Profiler** (часто в составе [Intel oneAPI Base Toolkit](https://www.intel.com/content/www/us/en/developer/tools/oneapi/base-toolkit.html) или отдельным установщиком [VTune](https://www.intel.com/content/www/us/en/developer/tools/oneapi/vtune-profiler.html)).
2. Установите на Windows с **Intel** CPU (на AMD часть hardware-метрик беднее; Hotspots всё равно работают).
3. Запустите **Intel VTune Profiler** (GUI).
4. При первом запуске согласитесь с драйверами sampling, если установщик предложит (для Hardware Event-Based Sampling).

Проверка: ноутбук **не ARM**, `echo %PROCESSOR_ARCHITECTURE%` → `AMD64`.

---

## 2. Подготовить две Release-сборки (vec / novec)

VTune профилирует **готовый `.exe`**. Vec и novec — это **разный код** в одном пути `target\release\lab1.exe`, поэтому:

1. Сначала соберите и **сохраните** копию vec-бинарника (или профилируйте сразу после сборки).
2. Потом пересоберите novec и профилируйте снова.

Рекомендуемый размер: C1 должна идти **несколько секунд** (иначе sampling шумный).

```bat
cd путь\к\lab1

REM --- сборка С автовекторизацией ---
task run-vec L=64 M=64 N=64
copy /Y target\release\lab1.exe target\release\lab1_vec.exe

REM --- сборка БЕЗ автовекторизации ---
task run-novec L=64 M=64 N=64
copy /Y target\release\lab1.exe target\release\lab1_novec.exe
```

Поднимите `L M N`, если на вашем CPU 64 слишком быстро.

Для лучшей читаемости символов Rust (по желанию):

```bat
set RUSTFLAGS=-C opt-level=3 -C target-cpu=native -C debuginfo=line-tables-only
cargo build --release
```

(для novec добавьте в `RUSTFLAGS` флаги `-C llvm-args=-vectorize-loops=false -C llvm-args=-vectorize-slp=false`).

---

## 3. Создать проект анализа в VTune

1. **File → New → Project…** (например `lab1-vec`).
2. **Configure Analysis**:
   - **Application:**  
     `…\lab1\target\release\lab1_vec.exe`  
     (или `lab1.exe` сразу после `task run-vec`)
   - **Application parameters:**  
     `--l 64 --m 64 --n 64`  
     (те же числа, что при сборке/замерах)
   - **Working directory:** корень `lab1`
3. Аналогично второй проект `lab1-novec` → `lab1_novec.exe` с теми же аргументами.

Не используйте Debug-сборку.

---

## 4. Какие типы анализа запускать

### 4.1. Hotspots (обязательно, первый шаг)

**Analysis type → Hotspots**

Рекомендации:

- Collection: **Hardware Event-Based Sampling** (если драйвер есть); иначе User-Mode Sampling тоже покажет hot-функции.
- Включите **Show additional performance insights** (если есть галочка) — там как раз insights по vectorization.

**Start**, дождитесь конца прогона.

Что открыть:

1. **Summary** — общее CPU Time, Insights / Vectorization (если есть).
2. **Bottom-up** — функции, отсортированные по CPU Time.
3. Найдите строки вроде:
   - `mul_add_block_auto`
   - `matmul_auto`
   - `mul_add_block_sse2`
   - `lab1` / mangled-имя с этими подстроками
4. Double-click по hotspot → **Source** / **Assembly**.

На защите покажите:

- горячее место — ядро умножения блоков;
- в Assembly у **vec** видны packed-инструкции (`mulps`/`addps` или AVX-аналоги);
- у **novec** — в основном `mulss`/`addss`.

### 4.2. Microarchitecture Exploration (бонус / «узкие места»)

**Analysis type → Microarchitecture Exploration**  
(в старых версиях могло называться **General Exploration**)

Смотрите top-down метрики на hotspot-функции:

| Метрика / область | Как интерпретировать для лабы |
|-------------------|-------------------------------|
| **Retiring** высокий + хороший SIMD | ядро реально считает, SIMD «кормит» пайплайн |
| **Vector Capacity Usage** (если есть) | ближе к 100% → float считается векторами; низко → скаляр / слабая векторизация |
| **FP Arithmetic** / vector vs scalar | у vec доля vector выше, у novec — ниже |
| **Core Bound** / port pressure | без векторизации больше давление на порты (скалярные uop) |
| **Memory Bound** | если данные не в кэше — отдельно сказать «узкое место — память», это бонус методички |

Сравните **один и тот же** hotspot `mul_add_block_auto` в результатах vec vs novec.

---

## 5. Сравнение двух результатов (как на защите)

1. Откройте результат **vec** и **novec**.
2. В VTune: **Compare** (если доступно) **или** просто два окна / два скриншота Summary + Bottom-up.
3. Таблица для рассказа:

| | lab1_vec | lab1_novec |
|---|----------|------------|
| CPU Time C1-ядра | меньше | больше |
| Insights: vectorization | лучше / flagged OK | хуже / low vector |
| Assembly в `mul_add_block_auto` | `mulps` / `addps` | `mulss` / `addss` |
| `mul_add_block_sse2` | SIMD есть | SIMD есть (intrinsics) |

4. Дополнительно покажите вывод программы:

```bat
target\release\lab1_vec.exe --l 64 --m 64 --n 64
target\release\lab1_novec.exe --l 64 --m 64 --n 64
```

`rdtsc` + `match: yes` + VTune = полный комплект.

---

## 6. Что говорить преподавателю (короткий текст)

1. «Профилировал Release через VTune Hotspots.»
2. «Горячая точка — `mul_add_block_auto` (авто) и `mul_add_block_sse2` (ручной SSE2).»
3. «Сборка с автовекторизацией: в Assembly packed SSE/AVX, метрики vectorization выше, CPU Time меньше.»
4. «Сборка с `-vectorize-loops=false`: скалярные `mulss`, vectorization ниже, время больше.»
5. «Ручной SSE2 не медленнее авто; C1 и C2 совпадают.»
6. (бонус) «Microarchitecture Exploration: узкое место — … (Retiring / Memory Bound / …).»

---

## 7. Типичные проблемы

| Проблема | Что делать |
|----------|------------|
| Прогон слишком короткий, пустые/шумные метрики | Увеличить `L M N`, чтобы C1 шла несколько секунд |
| Не видно имён Rust-функций | `-C debuginfo=line-tables-only`; искать по подстроке `mul_add` |
| Hardware sampling недоступен | Поставить драйвер VTune / запуск от админа; или User-Mode Hotspots + Assembly |
| Сравниваете Debug | Не считается; только Release |
| Vec и novec «одинаковые» в VTune | Забыли пересобрать / профилируете один и тот же `.exe` без копий `lab1_vec` / `lab1_novec` |
| Mac / ARM | VTune для этой лабы — на Windows Intel |

---

## 8. Чеклист VTune

- [ ] VTune установлен на Intel Windows
- [ ] Есть `lab1_vec.exe` и `lab1_novec.exe` (или две последовательные сессии после разных `task`)
- [ ] Arguments `--l … --m … --n …` дают несколько секунд работы
- [ ] Hotspots: найдены `mul_add_block_auto` и SSE2-ядро
- [ ] Assembly: vec = `mulps`, novec = `mulss`
- [ ] (бонус) Microarchitecture Exploration / Vector Capacity Usage сравнены
- [ ] Рядом готов `cargo asm` или dumpbin на всякий случай

---

## 9. Связка с дизассемблером

VTune показывает **профиль и метрики**.  
Дизасм однозначно показывает **инструкции**:

```bat
set RUSTFLAGS=-C opt-level=3 -C target-cpu=native
cargo asm --release --bin lab1 "lab1::mul_auto::mul_add_block_auto"
```

Подробные команды: [prove-vectorization.md](prove-vectorization.md).
