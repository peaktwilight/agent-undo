# Micro-evaluation harness

This directory contains a small local benchmark harness for the current
`agent-undo` artifact.

## What it measures

- release binary size
- `au init` latency on a synthetic repository
- edit detection latency with the watcher running
- `au oops --confirm` latency after a grouped write burst
- `.agent-undo/objects/` and `timeline.db` growth after repeated edits

## Run

```bash
cd paper/eval
./run.sh
```

Outputs:

- `results.json`
- `results.md`

These results are machine-local and should be presented as artifact-level
measurements, not universal performance claims.
