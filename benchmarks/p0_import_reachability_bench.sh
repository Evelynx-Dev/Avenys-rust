#!/usr/bin/env bash
set -euo pipefail

# Reproducible micro-benchmark for compiler import reachability mode.
# Measures compile wall time and peak RSS for legacy vs reachable.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="${TMPDIR:-/tmp}/mire_p0_import_bench"
APP_DIR="$WORK_DIR/app"

rm -rf "$WORK_DIR"
mkdir -p "$APP_DIR/code"

cat > "$APP_DIR/owl.toml" <<'EOF'
[project]
name = "mire-p0-import-bench"
version = "0.1.0"
entry = "main.mire"
EOF

cat > "$APP_DIR/code/heavy.mire" <<'EOF'
pub fn hot: () :i64 { return 7 }
EOF

# Generate many cold exports to stress loader/analyzer.
for i in $(seq 1 1200); do
  echo "pub fn cold_${i}: () :i64 { return ${i} }" >> "$APP_DIR/code/heavy.mire"
done

cat > "$APP_DIR/main.mire" <<'EOF'
load ./code/heavy
pub fn main: () {
  use dasu(hot())
}
EOF

MIRE_BIN="${MIRE_BIN:-$ROOT_DIR/target/debug/mire}"
if [[ ! -x "$MIRE_BIN" ]]; then
  echo "Building compiler binary first..."
  (cd "$ROOT_DIR" && cargo build --bin mire >/dev/null)
fi

run_case() {
  local mode="$1"
  local i
  echo "== import-mode: $mode =="
  for i in 1 2 3 4 5; do
    local start_ns end_ns wall_ms peak_kb pid
    start_ns="$(date +%s%N)"
    "$MIRE_BIN" build "$APP_DIR/main.mire" --import-mode "$mode" >/dev/null &
    pid=$!
    peak_kb=0
    while kill -0 "$pid" 2>/dev/null; do
      local rss
      rss="$(awk '/VmRSS:/ {print $2}' "/proc/$pid/status" 2>/dev/null || echo 0)"
      if [[ "${rss:-0}" -gt "$peak_kb" ]]; then
        peak_kb="$rss"
      fi
      sleep 0.01
    done
    wait "$pid"
    end_ns="$(date +%s%N)"
    wall_ms="$(( (end_ns - start_ns) / 1000000 ))"
    echo "run=$i wall_ms=$wall_ms rss_kb_peak=$peak_kb"
  done
}

run_case legacy
run_case reachable

echo "Benchmark done at: $WORK_DIR"
