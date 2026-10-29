#!/usr/bin/env bash
# Caos de guardado: mata procesos a mitad de operación y comprueba integridad.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=automation/common.sh
source "$SCRIPT_DIR/automation/common.sh"

require_cmd cargo
require_cmd python3
cd "$ROOT"
cargo build -q -p folio-app
BIN="$ROOT/target/debug/folio"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/folio-chaos-save.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT
VAULT="$TMP/vault"

log "Creando bóveda de caos en $VAULT"
"$BIN" vault init "$VAULT" --name "Caos" >/dev/null
BOOK="$("$BIN" --vault "$VAULT" book create --title "Caos" | tail -1)"

log "20 guardados interrumpidos con kill -9"
python3 - "$BIN" "$VAULT" "$BOOK" <<'PY'
import json, os, random, signal, subprocess, sys, time
from pathlib import Path
binary, vault, book = sys.argv[1:4]
random.seed(7)
for index in range(20):
    process = subprocess.Popen(
        [binary, "--vault", vault, "note", "create", "--book", book,
         "--title", f"caos {index}", "--text", "contenido de prueba " * 40],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    time.sleep(random.random() * 0.03)
    try:
        process.send_signal(signal.SIGKILL)
    except ProcessLookupError:
        pass
    process.wait()
# Un guardado completo después del caos.
subprocess.run([binary, "--vault", vault, "note", "create", "--book", book,
                "--title", "después", "--text", "ok"], check=True,
               stdout=subprocess.DEVNULL)
# Todos los JSON deben ser legibles.
files = list(Path(vault, "books").glob("*/notes/*.json"))
assert files, "no hay notas"
for path in files:
    with path.open(encoding="utf-8") as handle:
        data = json.load(handle)
    assert data.get("id"), path
print(f"json_ok={len(files)}")
PY

log "Reconstruyendo índice y validando con doctor"
"$BIN" --vault "$VAULT" doctor --rebuild-index --json >"$TMP/doctor.json"
python3 - "$TMP/doctor.json" <<'PY'
import json, sys
report = json.load(open(sys.argv[1]))
assert not report["corrupt"], report
assert report["quarantined"] == [], report
assert report["notes"] >= 1, report
print(f"doctor_ok notes={report['notes']} reindexed={report['reindexed']}")
PY
log "Caos de guardado OK"
