#!/usr/bin/env bash
# Caos de movimiento: mover notas con kill -9 y comprobar que no se duplican.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=automation/common.sh
source "$SCRIPT_DIR/automation/common.sh"

require_cmd cargo
require_cmd python3
cd "$ROOT"
cargo build -q -p folio-app
BIN="$ROOT/target/debug/folio"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/folio-chaos-move.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT
VAULT="$TMP/vault"
"$BIN" vault init "$VAULT" --name "Caos" >/dev/null
A="$("$BIN" --vault "$VAULT" book create --title "A" | tail -1)"
B="$("$BIN" --vault "$VAULT" book create --title "B" | tail -1)"

python3 - "$BIN" "$VAULT" "$A" "$B" <<'PY'
import json, random, signal, subprocess, sys, time
from pathlib import Path
binary, vault, a, b = sys.argv[1:5]
random.seed(11)
notes = []
for index in range(30):
    out = subprocess.check_output([binary, "--vault", vault, "note", "create",
                                   "--book", a, "--title", f"n{index}"], text=True)
    notes.append(out.strip().splitlines()[-1])
for index, note in enumerate(notes):
    target = b if index % 2 else a
    process = subprocess.Popen([binary, "--vault", vault, "note", "move", note,
                                "--to", target], stdout=subprocess.DEVNULL,
                               stderr=subprocess.DEVNULL)
    time.sleep(random.random() * 0.02)
    try:
        process.send_signal(signal.SIGKILL)
    except ProcessLookupError:
        pass
    process.wait()
ids = []
for path in Path(vault, "books").glob("*/notes/*.json"):
    ids.append(json.load(open(path, encoding="utf-8"))["id"])
assert len(ids) == len(set(ids)) == 30, f"duplicados/pérdidas: {len(ids)} {len(set(ids))}"
print(f"move_ok={len(ids)}")
PY

"$BIN" --vault "$VAULT" doctor --rebuild-index --json >"$TMP/doctor.json"
python3 - "$TMP/doctor.json" <<'PY'
import json, sys
report = json.load(open(sys.argv[1]))
assert not report["corrupt"], report
assert report["notes"] == 30, report
print(f"doctor_ok notes={report['notes']}")
PY
log "Caos de movimiento OK"
