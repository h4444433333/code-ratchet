---
name: code-ratchet
description: A complexity ratchet for AI-assisted code — quality only goes up, never down. Persists best-ever lint/type/test/coverage metrics; blocks any commit that worsens any of them. Works with any LLM/IDE via one AGENTS.md file and a git pre-commit gate. Inspired by Garry Tan's Complexity Ratchet and Karpathy's development rules. https://github.com/yourname/code-ratchet
keywords: quality, ratchet, llm, ai, agents, pre-commit, git, claude-code, cursor, codex, aider, coverage, complexity, karpathy, garry-tan, agents-md
---

# code-ratchet (Claude Code skill)

> Onboarding wizard for the open-source `code-ratchet` CLI.
> Project home: **https://github.com/yourname/code-ratchet**

## What is code-ratchet?

A tiny Rust CLI that persists "best-ever" quality metrics (lint warnings,
type errors, test count, tests passing, coverage %) in a baseline file. Any
change that worsens *any* metric is rejected at `git commit` time. The
ratchet only turns one way — quality goes up, never down.

It is one tool, one rules file (`AGENTS.md`), and one git pre-commit hook.
No per-IDE adapters. Works with Claude Code, Cursor, Aider, Codex, Cline,
Continue — anything that reads top-level repo files.

## When to invoke this skill

The user has asked you to:

- "Set up code-ratchet" / "Install code-ratchet" / "Add the ratchet to this repo"
- Improve quality discipline in an AI-assisted codebase
- Prevent regressions in lint / tests / coverage when an LLM is doing the editing
- "Enforce 90% coverage" or related Garry Tan / Karpathy-style discipline

## Onboarding flow

### Step 1 — check if `code-ratchet` is already installed

Run:
```
command -v code-ratchet
```

If found, skip to Step 3.

### Step 2 — install

Offer the user **one** of these install paths, in order of preference:

1. **Rust toolchain available**:
   ```
   cargo install code-ratchet
   ```
2. **macOS / Linux, no Rust**:
   ```
   curl -sSf https://raw.githubusercontent.com/yourname/code-ratchet/main/install.sh | bash
   ```
3. **Windows**: `cargo install code-ratchet` or download the
   `code-ratchet-windows-x86_64.exe` from
   https://github.com/yourname/code-ratchet/releases

After install, verify:
```
code-ratchet --version
```

### Step 3 — set up the current repo

In the repo root:
```
code-ratchet setup
```

This is a single confirmation prompt that:
- Detects language (Python / JavaScript / TypeScript)
- Writes `.ratchet.yml` with sensible defaults
- Writes `AGENTS.md` (a universal rules file the user can edit later)
- Runs an initial check to seed `.ratchet/baseline.json`
- Installs the git pre-commit hook (only if `.git/` exists)

Pass `-y` to skip the confirmation if the user has already approved.

### Step 4 — explain what just happened

Tell the user, in one paragraph:
- "Quality is now gated. Any future `git commit` that worsens lint /
  type / test / coverage metrics will be rejected."
- "Run `code-ratchet status` to see the current baseline."
- "Run `code-ratchet watch` in another terminal for real-time L0 feedback
  as you edit."
- "The `AGENTS.md` file in your repo root is the LLM-facing rules; any
  modern AI coding tool will read it. Edit it freely."
- "Full docs and source: https://github.com/yourname/code-ratchet"

## After onboarding — read AGENTS.md

Once `AGENTS.md` exists in the repo, it is the authoritative rules file —
read it on every subsequent code-change task. It contains the four-phase
loop (Grounding / Implementation / Verification / Delivery), the Karpathy
12 rules adapted for this repo, and the hard rules of the ratchet.

## Quick reference (after install)

| Goal                       | Command                       |
| -------------------------- | ----------------------------- |
| Run all checks now         | `code-ratchet check`          |
| Real-time L0 feedback      | `code-ratchet watch`          |
| Print current baseline     | `code-ratchet status`         |
| Reverse setup              | `code-ratchet uninstall`      |
| Read structured feedback   | `cat .ratchet/feedback.md`    |
| Project home / source      | https://github.com/yourname/code-ratchet |

## Why this Skill exists

`code-ratchet` is universal — it doesn't need a per-IDE adapter to work.
This Skill is just a discovery and onboarding wrapper for Claude Code
users: it gets them from "I heard about this" to "it's running in my
repo" in two commands. After that, the Skill steps aside; the binary,
the `AGENTS.md` it writes, and the git pre-commit hook do all the work.

Star the repo if it helps: https://github.com/yourname/code-ratchet
