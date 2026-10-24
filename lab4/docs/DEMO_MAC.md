# Защита с Mac → домашний RTX 3050

Клиент на Mac: **Windows App** (бывший Microsoft Remote Desktop).  
Туннель домой: **Tailscale**.

```
Mac [Windows App] ──Tailscale──► Windows RDP
                                   └── lab4 / RTX 3050
```

---

## A. Windows (дома) — один раз

### 1. Tailscale
https://tailscale.com/download → тот же аккаунт, что на Mac.  
В PowerShell:

```powershell
tailscale ip -4
```

Запиши адрес вида `100.x.y.z`.

### 2. Удалённый рабочий стол
Параметры → Система → **Удалённый рабочий стол** → **Вкл.**

Нужен **пароль** у Windows-пользователя (без пароля RDP часто не пускает).

### 3. Сборка lab4

```powershell
cd D:\CUDA\lab4
cargo build --release
```

### 4. Не усыплять ПК на время пары

---

## B. Mac — Windows App

### 1. Установка
App Store → поиск **Windows App** → разработчик **Microsoft Corporation** → Get.

### 2. Tailscale на Mac
Тот же аккаунт. Оба устройства Online.

### 3. Добавить ПК
1. Открой **Windows App**
2. **+** → **Add PC**
3. **PC name:** `100.x.y.z` из `tailscale ip -4` на Windows
4. **Credentials:** Ask when required
5. **Add**

### 4. Подключение
Двойной клик / Connect → логин Windows → видишь рабочий стол домашнего ПК.

---

## C. На защите

1. Mac: Tailscale Online → **Windows App** → Connect  
2. На удалённом столе:

```powershell
cd D:\CUDA\lab4
.\target\release\lab4.exe --n 1000000 --mode all --gpu-variant all --reps 1
```

3. Таблица (`время` = sort+extract, `vsCPU`) + строка `P25/P50/P75`

---

## План B

Нет сети: заранее лог на Mac.  
RDP недоступен: SSH — `scripts/demo.sh` (см. `demo.env.example`).
