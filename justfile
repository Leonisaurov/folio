# Atajos de Folio. Requiere `just` (opcional).
# Los scripts funcionan sin just.

check:
    ./scripts/automation/all.sh

check-fast:
    ./scripts/automation/all.sh --fast

plan:
    ./scripts/automation/plan.sh

fmt:
    cargo fmt --all

lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
    cargo test --workspace --all-features

test-unit:
    ./scripts/automation/unit.sh

test-ui:
    ./scripts/automation/ui.sh

test-watch:
    ./scripts/automation/watch.sh

run:
    cargo run -p folio-app --

bench:
    ./bench/startup.sh

chaos:
    ./scripts/chaos.sh

security:
    ./scripts/security.sh

a11y:
    ./scripts/a11y.sh

deb:
    ./scripts/build-deb.sh

perf:
    ./scripts/perf.sh

soak:
    ./scripts/soak.sh --quick 30

dist:
    ./scripts/release.sh --dry-run

x11:
    ./scripts/automation/with-x11.sh -- sh -c 'echo "DISPLAY=$DISPLAY"; echo "HOME=$HOME"'
