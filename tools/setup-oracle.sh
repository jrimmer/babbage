#!/bin/bash
# Creates .oracle-venv with pint, the independent source of exact unit factors.
set -euo pipefail
cd "$(dirname "$0")/.."
[ -x .oracle-venv/bin/python ] || python3 -m venv .oracle-venv
.oracle-venv/bin/pip install -q pint
.oracle-venv/bin/python -c 'import pint; print("pint", pint.__version__)'
