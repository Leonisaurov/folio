#!/usr/bin/env bash
# Mide arranque dentro del entorno X11 privado; salida parseable.
set -euo pipefail
BIN="${1:?binario}"
RUNS="${2:-7}"
exec python3 - "$BIN" "$RUNS" <<'PY'
import resource
import subprocess
import sys
import time

binary, runs = sys.argv[1], int(sys.argv[2])
for _ in range(runs):
    start = time.perf_counter()
    result = subprocess.run([binary, "--quit-after-init"], stdout=subprocess.DEVNULL,
                            stderr=subprocess.DEVNULL)
    elapsed = time.perf_counter() - start
    if result.returncode != 0:
        raise SystemExit(f"folio salió con {result.returncode}")
    rss = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
    print(f"TIME {elapsed:.3f} RSS {rss}")
PY
