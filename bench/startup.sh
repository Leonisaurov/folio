#!/usr/bin/env bash
# Genera bench/baseline.json con el arranque en frío medido bajo Xvfb.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"
export PATH="$HOME/.cargo/bin:$PATH"
command -v cargo >/dev/null || { echo "Falta cargo" >&2; exit 1; }

cargo build --release -q -p folio-app
BIN="$ROOT/target/release/folio"
TMP="$(mktemp "${TMPDIR:-/tmp}/folio-startup.XXXXXX")"
trap 'rm -f "$TMP"' EXIT

if command -v hyperfine >/dev/null; then
    hyperfine --warmup 2 --runs 10 --export-json "$TMP.hyperfine.json" \
        "xvfb-run -a -s '-screen 0 1440x900x24 -nolisten tcp' dbus-run-session -- $BIN --quit-after-init" \
        || true
fi

if [[ -f "$TMP.hyperfine.json" ]]; then
    python3 - "$TMP.hyperfine.json" "$ROOT/bench/baseline.json" <<'PY'
import json, sys, datetime
data = json.load(open(sys.argv[1]))["results"][0]
baseline = {
    "date": datetime.date.today().isoformat(),
    "environment": "Xvfb 1440x900x24, release, hyperfine",
    "runs": len(data["times"]),
    "startup_ms_median": round(data["median"] * 1000, 1),
    "startup_ms_p95": round(sorted(data["times"])[int(len(data["times"]) * 0.95)] * 1000, 1),
    "startup_ms_min": round(data["min"] * 1000, 1),
}
json.dump(baseline, open(sys.argv[2], "w"), indent=2, ensure_ascii=False)
open(sys.argv[2], "a").write("\n")
print(json.dumps(baseline, indent=2, ensure_ascii=False))
PY
    exit 0
fi

"$ROOT/scripts/automation/with-x11.sh" -- \
    bash "$ROOT/bench/measure-startup.sh" "$BIN" 7 >"$TMP" 2>&1 || {
        cat "$TMP" >&2
        echo "No se pudo medir el arranque (¿falta Xvfb/dbus?)" >&2
        exit 1
    }
python3 - "$TMP" "$ROOT/bench/baseline.json" <<'PY'
import json, re, statistics, sys, datetime
text = open(sys.argv[1]).read()
runs = [(float(t), int(rss)) for t, rss in re.findall(r"TIME ([\d.]+) RSS (\d+)", text)]
if not runs:
    print(text, file=sys.stderr)
    raise SystemExit("sin muestras de arranque")
times = [t for t, _ in runs]
cold = times[0]
warm = sorted(times[1:]) or times
median = statistics.median(warm)
p95 = warm[min(len(warm) - 1, int(len(warm) * 0.95))]
baseline = {
    "date": datetime.date.today().isoformat(),
    "environment": "Xvfb 1440x900x24, release, python time + rusage",
    "notes": "RSS en Xvfb con renderizado software; el presupuesto de 150 MB se mide en sesión real.",
    "runs": len(runs),
    "cold_start_ms": round(cold * 1000, 1),
    "startup_ms_median": round(median * 1000, 1),
    "startup_ms_p95": round(p95 * 1000, 1),
    "startup_ms_min": round(min(warm) * 1000, 1),
    "max_rss_kb": max(rss for _, rss in runs),
}
with open(sys.argv[2], "w", encoding="utf-8") as handle:
    json.dump(baseline, handle, indent=2, ensure_ascii=False)
    handle.write("\n")
print(json.dumps(baseline, indent=2, ensure_ascii=False))
PY
