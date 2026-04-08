#!/usr/bin/env bash
set -euo pipefail

ROOT="/Users/peak/coding/tools/agent-undo"
BIN="$ROOT/target/release/au"
OUTDIR="$ROOT/paper/eval"
RESULTS_JSON="$OUTDIR/results_repo_snapshot.json"
RESULTS_MD="$OUTDIR/results_repo_snapshot.md"
TMP_ROOT="$(mktemp -d /tmp/agent-undo-repoeval-XXXXXX)"
PROJECT="$TMP_ROOT/agent-undo-snapshot"

cleanup() {
  if [ -f "$PROJECT/.agent-undo/daemon.pid" ]; then
    (cd "$PROJECT" && "$BIN" stop >/dev/null 2>&1) || true
  fi
  rm -rf "$TMP_ROOT"
}
trap cleanup EXIT

mkdir -p "$PROJECT"
rsync -a \
  --exclude '.git' \
  --exclude 'target' \
  --exclude 'www/node_modules' \
  --exclude 'www/dist' \
  --exclude 'www/.astro' \
  --exclude '.omx' \
  --exclude '.agent-undo' \
  "$ROOT/" "$PROJECT/"

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
import subprocess, sys, time
binp, proj_s = sys.argv[1], sys.argv[2]
proj = Path(proj_s)
target = proj / "README.md"
def event_count():
    out = subprocess.run([binp, "status"], cwd=proj_s, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=True)
    for line in out.stdout.splitlines():
        if line.strip().startswith("events:"):
            return int(line.split(":", 1)[1].strip())
    raise RuntimeError(f"could not parse event count from status: {out.stdout!r}")
before = event_count()
t0 = time.perf_counter()
target.write_text(target.read_text() + "\n<!-- repo-snapshot-probe -->\n")
deadline = time.time() + 5.0
while time.time() < deadline:
    if event_count() > before:
        t1 = time.perf_counter()
        print((t1 - t0) * 1000)
        raise SystemExit(0)
    time.sleep(0.01)
raise SystemExit("watcher did not record edit within timeout")
PY
)

SESSION_ID=$(cd "$PROJECT" && "$BIN" session start --agent codex --metadata '{"prompt":"repo snapshot case study","tool_name":"Write","file_path":"README.md"}' 2>/dev/null)

python3 - <<'PY' "$PROJECT"
from pathlib import Path
import sys, time
proj = Path(sys.argv[1])
targets = [
    proj / "README.md",
    proj / "ARCHITECTURE.md",
    proj / "PHILOSOPHY.md",
]
for i, p in enumerate(targets):
    p.write_text(p.read_text() + f"\n<!-- repo-burst-{i} -->\n")
    time.sleep(0.15)
PY
sleep 1
(cd "$PROJECT" && "$BIN" session end "$SESSION_ID" >/dev/null 2>&1) || true

OOPS_MS=$(python3 - <<'PY' "$BIN" "$PROJECT"
import subprocess, sys, time
binp, proj = sys.argv[1], sys.argv[2]
t0 = time.perf_counter()
subprocess.run([binp, "oops", "--confirm"], cwd=proj, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
t1 = time.perf_counter()
print((t1 - t0) * 1000)
PY
)

RESTORE_OK=$(python3 - <<'PY' "$PROJECT"
from pathlib import Path
import sys
proj = Path(sys.argv[1])
targets = [
    proj / "README.md",
    proj / "ARCHITECTURE.md",
    proj / "PHILOSOPHY.md",
]
for i, p in enumerate(targets):
    if f"<!-- repo-burst-{i} -->" in p.read_text():
        print("false")
        raise SystemExit(0)
print("true")
PY
)

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
import subprocess, sys
binp, proj = sys.argv[1], sys.argv[2]
out = subprocess.run([binp, "status"], cwd=proj, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=True)
for line in out.stdout.splitlines():
    if line.strip().startswith("events:"):
        print(line.split(":", 1)[1].strip())
        raise SystemExit(0)
raise SystemExit(f"could not parse event count from status: {out.stdout!r}")
PY
)

FILES_COUNT=$(find "$PROJECT" -type f ! -path '*/.agent-undo/*' | wc -l | tr -d ' ')

python3 - <<'PY' "$RESULTS_JSON" "$RESULTS_MD" "$BIN_SIZE_BYTES" "$INIT_MS" "$WRITE_LATENCY_MS" "$OOPS_MS" "$OBJECT_BYTES" "$DB_BYTES" "$EVENTS_COUNT" "$FILES_COUNT" "$RESTORE_OK"
import json, sys
out_json, out_md = sys.argv[1], sys.argv[2]
bin_size, init_ms, write_ms, oops_ms, obj_b, db_b, events, files_count, restore_ok = sys.argv[3:]
data = {
    "binary_size_bytes": int(bin_size),
    "init_ms": float(init_ms),
    "write_detection_ms": float(write_ms),
    "oops_ms": float(oops_ms),
    "object_store_bytes": int(obj_b),
    "timeline_db_bytes": int(db_b),
    "events_count": int(events),
    "files_count": int(files_count),
    "restore_ok": restore_ok == "true",
    "touched_files": ["README.md", "ARCHITECTURE.md", "PHILOSOPHY.md"],
}
with open(out_json, "w") as f:
    json.dump(data, f, indent=2)
with open(out_md, "w") as f:
    f.write("# agent-undo repo-snapshot evaluation results\n\n")
    for k, v in data.items():
        f.write(f"- **{k}**: `{v}`\n")
PY

cat "$RESULTS_MD"
