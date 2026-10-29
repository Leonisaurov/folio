#!/usr/bin/env bash
# Orquesta la suite de Folio.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "$SCRIPT_DIR/common.sh"

MODE="all"
case "${1:-}" in
    --fast|--no-ui) MODE="fast" ;;
    --plan-only)    MODE="plan" ;;
    --help|-h)
        cat <<'EOF'
Uso: all.sh [--fast|--no-ui|--plan-only]

  (sin flags)  plan + calidad + unidad + UI aislada
  --fast       plan + calidad + unidad (sin UI)
  --no-ui      igual que --fast
  --plan-only  solo valida este plan
EOF
        exit 0
        ;;
    "") ;;
    *) die "Opción desconocida: $1 (usa --help)" ;;
esac

run_step() {
    local name="$1"; shift
    log "Paso: $name"
    "$@"
}

run_step "plan" "$SCRIPT_DIR/plan.sh"
if [[ "$MODE" == "plan" ]]; then
    log "Suite de plan OK"
    exit 0
fi

run_step "calidad" "$SCRIPT_DIR/quality.sh"
run_step "unidad" "$SCRIPT_DIR/unit.sh"

if [[ "$MODE" == "all" ]]; then
    run_step "UI aislada" "$SCRIPT_DIR/ui.sh"
else
    log "UI omitida (modo $MODE)"
fi

log "Suite OK"
