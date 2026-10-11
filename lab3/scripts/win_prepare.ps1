# Перед парой на Windows: RDP + Tailscale + сборка release.
# Запуск (лучше админ PowerShell):
#   powershell -ExecutionPolicy Bypass -File scripts\win_prepare.ps1

$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

Write-Host "==> Remote Desktop (RDP)"
try {
    Set-ItemProperty -Path 'HKLM:\System\CurrentControlSet\Control\Terminal Server' -Name fDenyTSConnections -Value 0 -ErrorAction Stop
    Enable-NetFirewallRule -DisplayGroup "Remote Desktop" -ErrorAction SilentlyContinue
    Write-Host "RDP: включён (нужны права администратора)"
} catch {
    Write-Host "Включи вручную: Параметры → Система → Удалённый рабочий стол → Вкл."
}

Write-Host "==> Tailscale"
if (Get-Command tailscale -ErrorAction SilentlyContinue) {
    tailscale status
    Write-Host "RDP с Mac на адрес:" (tailscale ip -4)
} else {
    Write-Host "Поставь Tailscale: https://tailscale.com/download"
}

Write-Host "==> cargo build --release"
cargo build --release

Write-Host ""
Write-Host "Готово. На Mac: Windows App (App Store) → + → Add PC → Tailscale IP →"
Write-Host "  cd D:\CUDA\lab3"
Write-Host "  .\target\release\lab3.exe --k 2048 --m 2048 --mode all --gpu-variant all --reps 1 --save-image"
