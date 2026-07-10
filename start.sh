#!/usr/bin/env bash
set -e

cd "$(dirname "$0")"

if [ ! -d ".venv" ]; then
    python3 -m venv --system-site-packages .venv
    ./.venv/bin/pip install -r requirements.txt
fi

source .venv/bin/activate
python3 main.py
