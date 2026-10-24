#!/usr/bin/env bash
# Запуск с Mac: ssh → lab4.exe на домашнем ПК с CUDA.
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
DEMO_DIR="${DEMO_DIR:-/d/CUDA/lab4}"
DEMO_ARGS="${DEMO_ARGS:---n 1000000 --mode all --gpu-variant all --reps 1}"
REMOTE="${DEMO_USER}@${DEMO_HOST}"

echo "==> ssh $REMOTE"
echo "==> cd $DEMO_DIR && lab4.exe $DEMO_ARGS"
echo

ssh -t "$REMOTE" "cd '$DEMO_DIR' && ./target/release/lab4.exe $DEMO_ARGS"
