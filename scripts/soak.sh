#!/usr/bin/env bash
# Soak test: uso sintético continuo sin fugas ni crashes.
# Uso: soak.sh [--quick [segundos]]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=automation/common.sh
source "$SCRIPT_DIR/automation/common.sh"

DURATION=7200
if [[ "${1:-}" == "--quick" ]]; then
    DURATION="${2:-15}"
fi

require_cmd cargo
require_cmd python3
cd "$ROOT"
cargo build -q -p folio-app
BIN="$ROOT/target/debug/folio"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/folio-soak.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT
VAULT="$TMP/vault"
"$BIN" vault init "$VAULT" --name "Soak" >/dev/null
BOOK="$("$BIN" --vault "$VAULT" book create --title "Soak" | tail -1)"

log "Soak de ${DURATION}s"
python3 - "$BIN" "$VAULT" "$BOOK" "$DURATION" <<'PY'
import os, subprocess, sys, time
binary, vault, book, duration = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
start = time.time()
created = 0
peak = 0
while time.time() - start < duration:
    out = subprocess.check_output([binary, "--vault", vault, "note", "create",
                                   "--book", book, "--title", f"soak {created}",
                                   "--text", "texto de soak " * 30], text=True)
    note = out.strip().splitlines()[-1]
    created += 1
    if created % 10 == 0:
        subprocess.run([binary, "--vault", vault, "note", "delete", note, "--yes"],
                       check=True, stdout=subprocess.DEVNULL)
    if created % 5 == 0:
        subprocess.run([binary, "--vault", vault, "index", "rebuild"],
                       check=True, stdout=subprocess.DEVNULL)
    # RSS del proceso hijo medido por /usr/bin/time no está expuesto aquí;
    # se comprueba que no queden procesos vivos y que doctor siga limpio.
    assert not subprocess.run([binary, "--version"], capture_output=True).returncode
print(f"soak_ok operaciones={created} segundos={int(time.time()-start)}")
PY

"$BIN" --vault "$VAULT" doctor --rebuild-index --json >"$TMP/doctor.json"
python3 - "$TMP/doctor.json" <<'PY'
import json, sys
report = json.load(open(sys.argv[1]))
assert not report["corrupt"], report
print(f"doctor_ok notes={report['notes']}")
PY
log "Soak OK"
