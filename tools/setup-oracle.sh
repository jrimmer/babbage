#!/bin/bash
# Creates .oracle-venv with pint (exact unit factors) and dateutil (calendar arithmetic), the independent oracles.
set -euo pipefail
cd "$(dirname "$0")/.."
[ -x .oracle-venv/bin/python ] || python3 -m venv .oracle-venv
.oracle-venv/bin/pip install -q pint python-dateutil tzdata==2025.2
.oracle-venv/bin/python -c 'import pint; print("pint", pint.__version__)'
