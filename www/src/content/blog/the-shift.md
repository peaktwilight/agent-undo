---
title: "The AI coding agent is a new kind of contributor. Your engineering stack isn't ready for it."
description: "Source control assumes a human writer. AI broke that assumption. This is what to build instead."
date: 2026-04-08
author: "agent-undo"
tags: ["ai", "developer-tools", "source-control", "philosophy"]
---

On May 27, 2025, a developer posting under the handle Jonneal3 opened a thread on the Cursor forum with a title that was half panic and half confession: *Help needed asap! - Cursor deleted my whole project.* The body was shorter than the title. "cursor agent went off the hinges and started deleting my entire app," he wrote. "90% of my app is gone… I hadnt gotten a chance to push to github yet." Seventeen replies followed. None of them recovered the code.

Seven months later, on January 16, 2026, a Cursor staff member posting as deanrie replied to a separate thread about agent-initiated file deletion with the kind of sentence that ends up pinned to an office wall somewhere. The bug, he explained, was "a known issue, a bug caused by a conflict between the Agent Review Tab and file editing." The official workaround: "Close the Agent Review Tab before the agent makes edits."

This is a system design failure, not a bug. The whole stack assumed something that stopped being true sometime in 2024.

## The silent assumption

Go pull the history of any repository you have been working on for more than a year. Look at the commit cadence. One commit every fifteen minutes on a good day, one every few hours more honestly, one per feature if you are disciplined. That cadence is not incidental. It is the heartbeat that every other tool in your stack was built around.

`git add` and `git commit` are a ceremony. The user decides what constitutes a meaningful unit of work, stages the files, writes a message explaining intent, and signs a contract with the future that says *this is what I meant to save*. Everything between commits is scratch. Everything not staged is assumed to be noise.

All of it encodes one silent assumption: the writer of the code is a human you can hold accountable. A human who will not produce four hundred lines of diff in the time it takes to reach for coffee. A human whose mistakes come in shapes you have seen before. A human whose pace gives the commit cadence time to breathe.

None of this was designed for a contributor that writes 400 lines in 8 seconds while you stare at the screen trying to read fast enough.

## The category claim

The fix is not "better checkpoints" inside a single editor. The fix is a new primitive at the filesystem layer.

- **git versions human intent.**
- **`au` records agent action.**

You need both.

Git remains the system for deliberate commits, review, merge, and deployment. `agent-undo` is the always-on local log of the code your agent actually wrote between those moments of intent.

That means four missing primitives become table stakes:

1. **Observability** — every write must be captured automatically.
2. **Attribution** — every change must be tied to an agent or session.
3. **Reversibility** — any session must be undoable as a unit.
4. **Review surface** — the session needs to be inspectable after the fact.

## Why editor checkpoints fail

Editor-bound checkpoints live at the wrong layer.

They only see their own edits. They disappear with the editor process. They race with writes from outside the editor. They cannot answer cross-editor questions like "did Claude or Cursor delete this file?" because each tool only sees its own world.

The only layer that can answer those questions is the filesystem watcher plus timeline that sits below the editor.

## What `agent-undo` does

`agent-undo` is a local-first Rust binary that snapshots file writes into a content-addressable store, records them in SQLite, and lets you restore a file or session later.

The common case is one word:

```sh
au oops
```

That command rolls back the last explicit agent session when there is one, and otherwise falls back to a recent-burst heuristic. The restore is itself recorded, so undoing the undo remains possible.

You can also inspect the timeline:

```sh
au log
au log --json
au sessions
au diff --session <id>
au blame src/auth.rs
```

That last command is the moat feature. Where `git blame` tells you which human wrote a line, `au blame` tells you which agent or session wrote it.

## What this is not

This is not a git replacement. It does not replace commits, branches, pull requests, or review. It is the second system beside git, not a rewrite of git.

It is also not a cloud product. There is no account, no sync requirement, no telemetry, and no dashboard dependency for the core workflow.

## The call

If AI agents are going to write half the code in a repository, then the industry needs to hold them to half the standards it applies to humans: observability, attribution, reversibility, review.

The tools to do that should not be optional add-ons. They should be default infrastructure.
