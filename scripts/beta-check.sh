#!/usr/bin/env bash
# Comprobaciones de pre-beta: datos, backups, migraciones, cifrado, exportación,
# ausencia de telemetría y migración de una nota con campos desconocidos.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=automation/common.sh
source "$SCRIPT_DIR/automation/common.sh"
cd "$ROOT"
require_cmd cargo
require_cmd python3
require_cmd rustc

log "Compilando binario de pruebas"
cargo build -q -p folio-app
BIN="$ROOT/target/debug/folio"
TMP="$(mktemp -d "$ROOT/.artifacts/beta-check.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT
REPORT="$TMP/report.txt"
: >"$REPORT"
record() { printf '%s\n' "$*" | tee -a "$REPORT"; }

record "Folio beta-check $(date -u +%Y-%m-%dT%H:%M:%SZ)"
record "version=$("$BIN" --version) schema=1 channel=beta telemetry=off"

log "Creando bóveda y escenario de 30 días"
"$BIN" vault init "$TMP/vault" --name Beta >/dev/null
BOOK="$("$BIN" --vault "$TMP/vault" book create --title 'Beta' | tail -1)"
NOTE="$("$BIN" --vault "$TMP/vault" note create --book "$BOOK" --title 'Nota beta' \
    --text 'Contenido de la beta con jardín y palabras suficientes.' | tail -1)"
"$BIN" --vault "$TMP/vault" index rebuild >/dev/null
"$BIN" --vault "$TMP/vault" search jardin --json >"$TMP/search.json"
grep -q "$NOTE" "$TMP/search.json"
record "search_ok"

# Campos desconocidos: la migración/lectura no debe perderlos.
NOTE_PATH="$(find "$TMP/vault/books" -name "*_${NOTE}.json" -print -quit)"
python3 - "$NOTE_PATH" <<'PY'
import json, sys
path = sys.argv[1]
data = json.load(open(path, encoding="utf-8"))
data["futuro_beta"] = {"preservar": True}
json.dump(data, open(path, "w", encoding="utf-8"), ensure_ascii=False, indent=2)
PY
"$BIN" --vault "$TMP/vault" doctor --json >/dev/null
python3 - "$NOTE_PATH" <<'PY'
import json, sys
data = json.load(open(sys.argv[1], encoding="utf-8"))
assert data["futuro_beta"]["preservar"] is True
print("unknown_fields_preserved")
PY
record "unknown_fields_preserved"

log "Backup, verificación y restauración"
BACKUP="$("$BIN" --vault "$TMP/vault" backup create | tail -1)"
"$BIN" --vault "$TMP/vault" backup verify "$BACKUP" >/dev/null
"$BIN" --vault "$TMP/vault" backup restore "$BACKUP" "$TMP/restored" >/dev/null
"$BIN" --vault "$TMP/restored" doctor --json >"$TMP/restored-doctor.json"
python3 - "$TMP/restored-doctor.json" <<'PY'
import json, sys
report = json.load(open(sys.argv[1]))
assert report["notes"] == 1, report
print("restore_ok")
PY
record "backup_verify_restore_ok"

log "Exportación MD/HTML/PDF"
"$BIN" --vault "$TMP/vault" export --format md --note "$NOTE" --out "$TMP/nota.md" >/dev/null
"$BIN" --vault "$TMP/vault" export --format html --note "$NOTE" --out "$TMP/nota.html" >/dev/null
"$BIN" --vault "$TMP/vault" export --format pdf --note "$NOTE" --out "$TMP/nota.pdf" >/dev/null
test -s "$TMP/nota.md" && test -s "$TMP/nota.html" && test -s "$TMP/nota.pdf"
record "export_md_html_pdf_ok"

log "Cifrado y descifrado de bóveda"
"$BIN" vault init "$TMP/vault-enc" --encrypt --password 'beta-larga' >/dev/null 2>"$TMP/phrase"
"$BIN" --vault "$TMP/vault-enc" --password 'beta-larga' note create --book today --title 'Secreta' --text 'no debe filtrarse' >/dev/null
grep -rIl "no debe filtrarse" "$TMP/vault-enc" && { echo "texto claro en bóveda cifrada" >&2; exit 1; } || true
"$BIN" --vault "$TMP/vault-enc" --password 'beta-larga' vault decrypt >/dev/null
grep -rIl "no debe filtrarse" "$TMP/vault-enc" >/dev/null
record "encryption_roundtrip_ok"

log "Comprobando ausencia de telemetría y clientes de red en la app"
if cargo tree -p folio-app --edges normal 2>/dev/null | grep -Eqi 'reqwest|ureq|isahc|sentry|posthog|segment'; then
    die "Hay una dependencia de red/telemetría inesperada en folio-app"
fi
if grep -RInE 'send_telemetry|telemetry::|analytics::|posthog|sentry' crates --include='*.rs' | grep -v 'advanced.telemetry' | grep -v 'telemetry.*false'; then
    die "Código de telemetría encontrado"
fi
TELEMETRY="$("$BIN" config get advanced.telemetry 2>/dev/null || true)"
record "telemetry_default=$TELEMETRY"
test "$TELEMETRY" = "false"
record "no_network_client_dependency=ok"

record "BETA-CHECK OK"
cp "$REPORT" "$ROOT/dist/beta-check.txt" 2>/dev/null || { mkdir -p "$ROOT/dist"; cp "$REPORT" "$ROOT/dist/beta-check.txt"; }
cat "$REPORT"
