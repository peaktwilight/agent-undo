#!/usr/bin/env bash
set -euo pipefail

ROOT="/Users/peak/coding/tools/agent-undo"
BIN="$ROOT/target/release/au"
OUTDIR="$ROOT/paper/eval"
RESULTS_JSON="$OUTDIR/results.json"
RESULTS_MD="$OUTDIR/results.md"
TMP_ROOT="$(mktemp -d /tmp/agent-undo-eval-XXXXXX)"
PROJECT="$TMP_ROOT/project"
mkdir -p "$PROJECT/src" "$PROJECT/tests"

cleanup() {
  if [ -f "$PROJECT/.agent-undo/daemon.pid" ]; then
    "$BIN" stop >/dev/null 2>&1 || true
  fi
  rm -rf "$TMP_ROOT"
}
trap cleanup EXIT

python3 - <<'PY' "$PROJECT"
from pathlib import Path
import sys
root = Path(sys.argv[1])
for i in range(50):
    p = root / "src" / f"file_{i:03d}.rs"
    p.write_text("\n".join([f"pub fn f{i}_{j}() -> usize {{ {i+j} }}" for j in range(40)]) + "\n")
for i in range(10):
    p = root / "tests" / f"test_{i:02d}.txt"
    p.write_text(("baseline-line\n" * 200))
(root / "README.md").write_text("# synthetic repo\n")
PY

BIN_SIZE_BYTES=$(stat -f %z "$BIN")

INIT_MS=$(python3 - <<'PY' "$BIN" "$PROJECT"
import subprocess, sys, time
binp, proj = sys.argv[1], sys.argv[2]
t0 = time.perf_counter()
subprocess.run([binp, "init"], cwd=proj, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
t1 = time.perf_counter()
print((t1 - t0) * 1000)
PY
)

(cd "$PROJECT" && "$BIN" serve --daemon >/dev/null 2>&1) &
sleep 1

python3 - <<'PY' "$BIN" "$PROJECT"
import subprocess, sys, time
binp, proj = sys.argv[1], sys.argv[2]
deadline = time.time() + 5.0
while time.time() < deadline:
    out = subprocess.run([binp, "status"], cwd=proj, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    if out.returncode == 0 and "daemon:   running" in out.stdout:
        raise SystemExit(0)
    time.sleep(0.05)
raise SystemExit("daemon not ready")
PY

WRITE_LATENCY_MS=$(python3 - <<'PY' "$BIN" "$PROJECT"
from pathlib import Path
import subprocess, sys, time, re
binp, proj_s = sys.argv[1], sys.argv[2]
proj = Path(proj_s)
target = proj / "src" / "file_000.rs"
def event_count():
    out = subprocess.run([binp, "status"], cwd=proj_s, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=True)
    for line in out.stdout.splitlines():
        if line.strip().startswith("events:"):
            return int(line.split(":", 1)[1].strip())
    raise RuntimeError(f"could not parse event count from status: {out.stdout!r}")
before = event_count()
t0 = time.perf_counter()
target.write_text(target.read_text() + "\n// watcher-latency-probe\n")
deadline = time.time() + 5.0
seen = False
while time.time() < deadline:
    n = event_count()
    if n > before:
        seen = True
        break
    time.sleep(0.01)
t1 = time.perf_counter()
if not seen:
    raise SystemExit("watcher did not record edit within timeout")
print((t1 - t0) * 1000)
PY
)

python3 - <<'PY' "$PROJECT"
from pathlib import Path
import sys
proj = Path(sys.argv[1])
for i in range(5):
    p = proj / "src" / f"file_{i:03d}.rs"
    p.write_text(p.read_text() + f"\n// burst-{i}\n")
PY
sleep 1

OOPS_MS=$(python3 - <<'PY' "$BIN" "$PROJECT"
import subprocess, sys, time
binp, proj = sys.argv[1], sys.argv[2]
t0 = time.perf_counter()
subprocess.run([binp, "oops", "--confirm"], cwd=proj, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
t1 = time.perf_counter()
print((t1 - t0) * 1000)
PY
)

python3 - <<'PY' "$PROJECT"
from pathlib import Path
import sys
proj = Path(sys.argv[1])
for round_idx in range(20):
    p = proj / "src" / f"file_{round_idx % 10:03d}.rs"
    p.write_text(p.read_text() + f"\n// growth-{round_idx}\n")
PY
sleep 1

OBJECT_BYTES=$(python3 - <<'PY' "$PROJECT"
from pathlib import Path
import sys
proj = Path(sys.argv[1])
obj = proj / ".agent-undo" / "objects"
total = sum(p.stat().st_size for p in obj.rglob("*") if p.is_file())
print(total)
PY
)

DB_BYTES=$(stat -f %z "$PROJECT/.agent-undo/timeline.db")
EVENTS_COUNT=$(python3 - <<'PY' "$BIN" "$PROJECT"
import subprocess, sys, re
binp, proj = sys.argv[1], sys.argv[2]
out = subprocess.run([binp, "status"], cwd=proj, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=True)
for line in out.stdout.splitlines():
    if line.strip().startswith("events:"):
        print(line.split(":", 1)[1].strip())
        raise SystemExit(0)
raise SystemExit(f"could not parse event count from status: {out.stdout!r}")
PY
)

python3 - <<'PY' "$RESULTS_JSON" "$RESULTS_MD" "$BIN_SIZE_BYTES" "$INIT_MS" "$WRITE_LATENCY_MS" "$OOPS_MS" "$OBJECT_BYTES" "$DB_BYTES" "$EVENTS_COUNT"
import json, sys
out_json, out_md = sys.argv[1], sys.argv[2]
bin_size, init_ms, write_ms, oops_ms, obj_b, db_b, events = sys.argv[3:]
data = {
    "binary_size_bytes": int(bin_size),
    "init_ms": float(init_ms),
    "write_detection_ms": float(write_ms),
    "oops_ms": float(oops_ms),
    "object_store_bytes": int(obj_b),
    "timeline_db_bytes": int(db_b),
    "events_count": int(events),
}
with open(out_json, "w") as f:
    json.dump(data, f, indent=2)
with open(out_md, "w") as f:
    f.write("# agent-undo micro-evaluation results\n\n")
    for k, v in data.items():
        f.write(f"- **{k}**: `{v}`\n")
PY

cat "$RESULTS_MD"
