#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
binary="$repo_root/target/release/veles"
config_path="${OPENCODE_CONFIG:-$HOME/.config/opencode/opencode.json}"

cargo build --release
mkdir -p "$(dirname "$config_path")"

python3 - "$config_path" "$binary" <<'PY'
import json
import sys
from pathlib import Path

config_path = Path(sys.argv[1]).expanduser()
binary = sys.argv[2]

if config_path.exists() and config_path.read_text().strip():
    config = json.loads(config_path.read_text())
else:
    config = {}

config.setdefault("$schema", "https://opencode.ai/config.json")
mcp = config.setdefault("mcp", {})
mcp["veles"] = {
    "type": "local",
    "command": [binary, "--stdio"],
    "enabled": False,
    "timeout": 120000,
    "environment": {
        "VELES_REQUESTS_PER_SECOND": "1",
        "VELES_CACHE_TTL_SECONDS": "3600",
        "VELES_DDG_REGION": "wt-wt",
        "VELES_SAFESEARCH": "moderate",
        "VELES_BROWSER_ENABLED": "false",
        "VELES_BROWSER_DRIVER": "geckodriver",
        "VELES_BROWSER_HEADLESS": "true",
        "VELES_BROWSER_PAGE_TIMEOUT_MS": "90000",
        "VELES_BROWSER_SETTLE_MS": "2000",
    },
}

config_path.write_text(json.dumps(config, indent=2) + "\n")
print(f"Updated {config_path}")
print("Veles was added with enabled=false. Enable it in OpenCode when you want to use it.")
PY
