#!/usr/bin/env bash
# Presupuestos de rendimiento medibles y comparables con bench/baseline.json.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=automation/common.sh
source "$SCRIPT_DIR/automation/common.sh"
cd "$ROOT"
require_cmd cargo
require_cmd python3

log "Compilando release"
cargo build --release -q -p folio-app -p folio-tools

TMP="$(mktemp -d "${TMPDIR:-/tmp}/folio-perf.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

log "Generando bóveda de 10k notas e índice"
python3 - "$TMP" <<'PY'
import subprocess, sys, time
from pathlib import Path

tmp = Path(sys.argv[1])
vault = tmp / "vault-10k"

start = time.perf_counter()
gen = subprocess.run(
    ["./target/release/folio-tools", "gen-vault", "--notes", "10000",
     "--books", "20", "--out", str(vault), "--index"],
    capture_output=True, text=True,
)
elapsed = time.perf_counter() - start
if gen.returncode != 0:
    print(gen.stdout, gen.stderr, file=sys.stderr)
    raise SystemExit(gen.returncode)
line = gen.stdout.strip().splitlines()[-1]
(tmp / "gen.txt").write_text(f"gen_vault_seconds={elapsed:.2f}\n{line}\n")

start = time.perf_counter()
rebuild = subprocess.run(
    ["./target/release/folio", "--vault", str(vault), "index", "rebuild"],
    capture_output=True, text=True,
)
rebuild_elapsed = time.perf_counter() - start
if rebuild.returncode != 0:
    print(rebuild.stdout, rebuild.stderr, file=sys.stderr)
    raise SystemExit(rebuild.returncode)

start = time.perf_counter()
search = subprocess.run(
    ["./target/release/folio", "--vault", str(vault), "search", "palabra42", "--json"],
    capture_output=True, text=True,
)
search_elapsed = time.perf_counter() - start
hits = search.stdout.count('"note_id"')
(tmp / "search.txt").write_text(
    f"index_rebuild_seconds={rebuild_elapsed:.2f} (10000 notas)\n"
    f"search_seconds={search_elapsed:.3f} (hits={hits})\n"
)
print((tmp / "gen.txt").read_text().strip())
print((tmp / "search.txt").read_text().strip())
PY

log "Layout de 500 bloques"
./target/release/folio-tools bench-layout --blocks 500 | tee "$TMP/layout.txt"

log "Arranque (Xvfb privado)"
./bench/startup.sh

log "Resumen"
cat "$TMP/gen.txt" "$TMP/search.txt" "$TMP/layout.txt"
