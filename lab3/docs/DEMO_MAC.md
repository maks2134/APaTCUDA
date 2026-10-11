# Защита с Mac → домашний RTX 3050

Клиент на Mac: **Windows App** (бывший Microsoft Remote Desktop).  
Туннель домой: **Tailscale**.

```
Mac [Windows App] ──Tailscale──► Windows RDP
                                   └── lab3 / RTX 3050
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

Админ PowerShell (если удобнее):

```powershell
Set-ItemProperty -Path 'HKLM:\System\CurrentControlSet\Control\Terminal Server' -Name fDenyTSConnections -Value 0
Enable-NetFirewallRule -DisplayGroup "Remote Desktop"
```

> Home-редакция Windows иногда без полноценного RDP-хоста. Если пункта «Удалённый рабочий стол» нет — пиши, разберём обход (Parsec / RustDesk) или SSH.

### 3. Сборка lab3

```powershell
cd D:\CUDA\lab3
cargo build --release
```

### 4. Не усыплять ПК на время пары

---

## B. Mac — Windows App

### 1. Установка
App Store → поиск **Windows App** → разработчик **Microsoft Corporation** → Get.

(Старое имя «Microsoft Remote Desktop» больше не ищи — его заменили.)

### 2. Tailscale на Mac
Тот же аккаунт. Оба устройства Online.

### 3. Добавить ПК
1. Открой **Windows App**
2. **+** → **Add PC** (или Devices → +)
3. **PC name:** `100.x.y.z` из `tailscale ip -4` на Windows  
   (или MagicDNS-имя из `tailscale status`)
4. **Credentials:** Ask when required (или сохрани `User` + пароль)
5. **Add**

### 4. Подключение
Двойной клик / Connect → логин Windows → видишь рабочий стол домашнего ПК.

Проверь дома до пары (Mac и ПК в одной Wi‑Fi тоже ок через Tailscale).

---

## C. На защите

1. Mac: Tailscale Online → **Windows App** → Connect  
2. На удалённом столе:

```powershell
cd D:\CUDA\lab3
.\target\release\lab3.exe --k 2048 --m 2048 --mode all --gpu-variant all --reps 1 --save-image
```

3. Таблица (`время` = kernel, `vsCPU`) + `output\panel_*.png`

---

## План B

Нет сети: заранее лог + `panel_*.png` на Mac.  
RDP недоступен: SSH — `scripts/demo.sh` (см. `demo.env.example`).
