# Villain receipts — the data-loss evidence pack

This doc collects the on-the-record quotes and incidents that justify `agent-undo`'s existence and power the launch narrative. Every quote here is sourced and dated. Use these in: the README villain section, the HN Show post, the Twitter thread, the follow-up essay.

## The primary villain quote — Cursor staff on the record

**"deanrie" (Cursor staff), Jan 16, 2026**
Thread: *"Agent code changes are automatically deleted"*
URL: https://forum.cursor.com/t/agent-code-changes-are-automatically-deleted/149024

> *"This is a known issue, a bug caused by a conflict between the Agent Review Tab and file editing."*

**Official workaround:** *"Close the Agent Review Tab before the agent makes edits."*

This is THE quote. A Cursor employee, on the official forum, telling users to disable a core feature to prevent their code from being deleted. Screenshot it. Quote it. Put it in the README.

**Second Cursor staff confirmation:**

**"deanrie", Feb 23, 2026**
Thread: *"Keep/Undo buttons missing and Discard to Checkpoint not reverting changes"*
URL: https://forum.cursor.com/t/keep-undo-buttons-missing-and-discard-to-checkpoint-not-reverting-changes-auto-applies-edits/152621

> *"Both issues are related and already known. The root cause is a diffs display bug."*

Two separate staff acknowledgments that the checkpoint/undo/diff layer is broken at the architectural level.

## The HN post opener — user stories

**Jonneal3, May 27, 2025**
Thread: *"Help needed asap! - Cursor deleted my whole project"*
URL: https://forum.cursor.com/t/help-needed-asap-cursor-deleted-my-whole-proejct/97589 (17 replies)

> *"cursor agent went off the hinges and started deleting my entire app"*
> *"90% of my app is gone… I hadnt gotten a chance to push to github yet"*
> *"I cannot believe cursor would be so blind as to not have a 'restore to recent' checkpoint feature"*

**davidktx, May 28, 2025** (same thread):
> *"It just did the same thing to me. Timeline is probably empty because the .git and probably its supporting folders were deleted... You unfortunately are not alone"*

This is the HN Show post opener. Build empathy, name the villain, present the fix. The progression writes itself.

## The sustained-pain quote — this is the Twitter opener

**nvs, Feb 20–27, 2025**
Thread: *"Cursor destroyed my code/full app, now 7th time"*
URL: https://forum.cursor.com/t/cursor-destroyed-my-code-full-app-now-7th-time/52371

> *"Every 3rd day, I was finding myself having to rewrite the code again"*
> *"What I could do in a week manually, took me 5 weeks, due to crashes"*
> *"I have spent more time, rebuilding codebase than actually building logic"*
> *"I am almost giving up on cursor"*

One user. Seven incidents. Five-week delay on one week of work. This is what the whole thing is worth.

## Checkpoint specifically broken (recent)

**MidnightOak, Feb 20, 2026**
Thread: *"[v2.5.20] Revert to Checkpoint Broken"*
URL: https://forum.cursor.com/t/v2-5-20-revert-to-checkpoint-broken/152345

> *"Reverting to checkpoint no longer reverts. Changes in code remain, even if it shows in chat that a revert was done."*

**swordsith, Feb 22, 2026** (same thread):
> *"This is happening for me right now, very frustrating."*

**whe, Jan 23, 2026** on the auto-delete bug:
> *"The agent will fix a bug (found during Agent Review), and then the code is immediately deleted."*

**bladerunner2020, Jan 31, 2026**
Thread: *"Cursor just deleted itself + github + more"*
URL: https://forum.cursor.com/t/cursor-just-deleted-itself-github-more/150422

> *"rejecting the suggested changes led to the entire file being deleted"*

**DjWarmonger, Feb 2, 2026** (same thread):
> *"with recent updates Cursor silently toggled from 'Use Allowlist' to 'Run everything'"*

## Hacker News — "Is Cursor deleting working code for you too"

Source: https://news.ycombinator.com/item?id=43298275 (Mar 8, 2025)

**neuralkoi:**
> *"I had an issue yesterday where it liberally deleted a chunk of code it must have thought was extraneous but at the same time introduced a huge privacy vulnerability."*

**mort96:**
> *"How does it feel, being a full time code reviewer responsible for reviewing the output of an idiot which removes and breaks stuff randomly?"*

**muzani** on Claude 3.7 in agents:
> *"Claude 3.7 feels overtuned... it will rewrite the mock to pass... Sometimes it's even aware of this, saying things like 'I should be careful to not delete this'"*

## Cline — multi-editor evidence (this isn't just Cursor)

**Cline issue #5124**, July 2025 — the title is itself the quote:
> *"Cline autonomously delete files without keeping track of the deleted/changed files. Very Dangerous and Critical Issue!!!"*

URL: https://github.com/cline/cline/issues/5124

**Cline issue #7600**, Nov 2025:
> *"replace_in_file tool deletes next line of code after replacement"*

URL: https://github.com/cline/cline/issues/7600

**Cline issue #2858**, April 2025 — the `remove_file` tool silently does nothing, *"never been fixed."*

URL: https://github.com/cline/cline/issues/2858

Use these to refute the inevitable "this is a Cursor-specific problem" objection. It's an editor-class problem, not a Cursor problem.

## The big-money quote

**@adxtyahq** (via vibecoding.app):
> *"The boss spent about $5,500 on Cursor credits... when Cursor couldn't refactor his 18,000 line Node API, he hit a wall."*

A Medium developer (cited in the same roundup) documented losing **approximately four months of work** to Cursor reversion bugs.

## Additional receipts (bench for follow-up posts)

- https://forum.cursor.com/t/chat-checkpoints-dont-revert-code-properly/152855
- https://forum.cursor.com/t/rollback-fails-in-cursor-checkpoint-restore-doesn-t-work-either/122069
- https://forum.cursor.com/t/restore-checkpoint-not-working/120490
- https://forum.cursor.com/t/after-updating-the-software-the-restore-checkpoint-function-stopped-working-and-some-files-were-lost/121750
- https://forum.cursor.com/t/cursor-editor-keeps-removing-changes-made-by-ai-agent/132183
- https://forum.cursor.com/t/urgent-lost-all-chat-and-agent-mode-history/136644
- https://www.getmrq.com/blog/ai-deleted-my-code — Claude Code recovery guide, confirms `/rewind` is the only escape hatch

## How to deploy the receipts

| Asset | Where to use it |
|---|---|
| deanrie "close the Agent Review Tab" | README villain paragraph, HN post subtitle, Twitter thread tweet #1 |
| Jonneal3 "90% of my app is gone" | HN post opening paragraph, blog post #1 opening |
| nvs "5 weeks vs 1 week" | Twitter thread tweet #3 (sustained pain) |
| MidnightOak "Revert to Checkpoint Broken" | Screenshot in README |
| Cline issue #5124 title | "This isn't just Cursor" rebuttal tweet |
| neuralkoi "introduced a huge privacy vulnerability" | Security blog post (leverages founder credibility) |
| muzani "rewrite the mock to pass" | "AI agents need ceremony" essay — illustrates the accountability gap |
| $5,500 / 4 months lost | "The AI-editor data-loss tax" follow-up post |

## Editorial note

Don't be petty in the README itself. The villain goes in the **launch post**, not the tool's docs. The README describes the problem neutrally ("AI coding agents can destroy uncommitted work; this fixes it") and leaves the receipts for the HN post, Twitter thread, and blog essays. This keeps the repo professional for enterprise readers while the launch uses the receipts as rocket fuel.
