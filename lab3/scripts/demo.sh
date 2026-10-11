#!/usr/bin/env bash
# Запуск с Mac: ssh → lab3.exe на домашнем ПК с CUDA.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ENV_FILE="${DEMO_ENV:-$ROOT/scripts/demo.env}"

if [[ -f "$ENV_FILE" ]]; then
  # shellcheck disable=SC1090
  set -a
  # shellcheck disable=SC1091
  source "$ENV_FILE"
  set +a
fi

: "${DEMO_HOST:?Задай DEMO_HOST (см. scripts/demo.env.example)}"
: "${DEMO_USER:?Задай DEMO_USER}"
DEMO_DIR="${DEMO_DIR:-/d/CUDA/lab3}"
DEMO_ARGS="${DEMO_ARGS:---k 2048 --m 2048 --mode all --gpu-variant all --reps 1 --save-image}"
REMOTE="${DEMO_USER}@${DEMO_HOST}"

echo "==> ssh $REMOTE"
echo "==> cd $DEMO_DIR && lab3.exe $DEMO_ARGS"
echo

ssh -t "$REMOTE" "cd '$DEMO_DIR' && ./target/release/lab3.exe $DEMO_ARGS"

if [[ -n "${DEMO_PNG_OUT:-}" ]]; then
  echo
  echo "==> scp panel PNG → $DEMO_PNG_OUT"
  # Берём самый свежий panel_*.png
  REMOTE_PNG=$(ssh "$REMOTE" "ls -t '$DEMO_DIR'/output/panel_*.png 2>/dev/null | head -1")
  if [[ -z "$REMOTE_PNG" ]]; then
    echo "panel PNG на ПК не найден (запускали с --save-image?)"
    exit 1
  fi
  scp "$REMOTE:$REMOTE_PNG" "$DEMO_PNG_OUT"
  echo "готово: $DEMO_PNG_OUT"
  if command -v open >/dev/null 2>&1; then
    open "$DEMO_PNG_OUT"
  fi
fi
