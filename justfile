# Atajos de Folio. Requiere `just` (opcional).
# Los scripts funcionan sin just.

check:
    ./scripts/automation/all.sh

check-fast:
    ./scripts/automation/all.sh --fast

plan:
    ./scripts/automation/plan.sh

test-unit:
    ./scripts/automation/unit.sh

test-ui:
    ./scripts/automation/ui.sh

test-watch:
    ./scripts/automation/watch.sh

x11:
    ./scripts/automation/with-x11.sh -- sh -c 'echo "DISPLAY=$DISPLAY"; echo "HOME=$HOME"'
