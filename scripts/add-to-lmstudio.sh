#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
binary="$repo_root/target/release/veles"
open_deeplink=true

if [[ "${1:-}" == "--print-only" ]]; then
    open_deeplink=false
fi

cargo build --release

deeplink="$(python3 - "$binary" <<'PY'
import base64
import json
import sys
from urllib.parse import quote

binary = sys.argv[1]
config = {
    "command": binary,
    "args": ["--stdio"],
    "env": {
        "VELES_REQUESTS_PER_SECOND": "1",
        "VELES_CACHE_TTL_SECONDS": "3600",
        "VELES_DDG_REGION": "wt-wt",
        "VELES_SAFESEARCH": "moderate",
    },
}
encoded = base64.b64encode(json.dumps(config, separators=(",", ":")).encode()).decode()
print(f"lmstudio://add_mcp?name=veles&config={quote(encoded)}")
PY
)"

printf '%s\n' "$deeplink"

if [[ "$open_deeplink" == true ]] && command -v open >/dev/null 2>&1; then
    open "$deeplink"
fi
