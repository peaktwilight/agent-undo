# agent-undo paper

This directory contains a first real paper draft for `agent-undo`.

## Files

- `main.tex` — LaTeX draft
- `bibliography.bib` — references used by the draft

## Build

```bash
cd paper
pdflatex -interaction=nonstopmode main.tex
bibtex main
pdflatex -interaction=nonstopmode main.tex
pdflatex -interaction=nonstopmode main.tex
```

## Scope

This is intentionally written as a systems / tool paper draft grounded in the
current open-source artifact. It does **not** pretend we already have a full
performance evaluation or user study. The paper claims:

- a concrete problem shift from human-paced commits to agent-paced writes
- a local-first provenance-and-rollback system design
- a working prototype artifact with supported features
- a clear limitations section and next-step evaluation plan

The draft should be tightened only after we add:

1. overhead measurements
2. storage-growth measurements
3. multi-editor attribution experiments
4. a small user or case-study evaluation
