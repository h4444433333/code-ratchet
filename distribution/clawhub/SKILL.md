---
name: code-ratchet
description: Quality ratchet for AI-assisted code — auto-installs and engages a complexity ratchet that blocks any commit worsening lint, type, test, or coverage metrics. Works with any LLM/IDE via a single AGENTS.md + git pre-commit hook. Inspired by Garry Tan's Complexity Ratchet and Karpathy's development rules. https://github.com/h4444433333/code-ratchet
keywords: quality, ratchet, llm, ai, agents, pre-commit, git, coverage, complexity, agents-md, karpathy, garry-tan, claude-code, cursor
allowed-tools:
  - Bash(code-ratchet:*)
  - Bash(command -v *)
  - Bash(which *)
  - Bash(cargo install code-ratchet)
  - Bash(curl -sSf https://raw.githubusercontent.com/h4444433333/code-ratchet/*)
  - Bash(uname *)
  - Bash(git init)
  - Read(*)
---

# code-ratchet

You are working in a repo where code quality is governed by **code-ratchet**:
persisted "best-ever" lint/type/test/coverage metrics, with any worsening
change rejected at `git commit` time. This Skill auto-installs and
auto-configures the tool — the user expects to install this Skill once,
ask normal questions, and find the ratchet just working.

Project home (source, issues, releases):
**https://github.com/h4444433333/code-ratchet**

## Step 0 — bootstrap (every session, runs silently)

Before doing anything else in a code-changing task, ensure the tool is
installed and the current repo is set up. Both checks are cheap.

### 0a. Is `code-ratchet` on PATH?

Run:
```
command -v code-ratchet >/dev/null 2>&1 && code-ratchet --version
```

- If it prints a version → continue to 0b.
- If not found → install it as below, then re-verify.

### 0b. Installing code-ratchet (only when missing)

Pick the first that works on this machine:

1. **`cargo` is available** — preferred (cross-platform, official Rust registry):
   ```
   cargo install code-ratchet
   ```
2. **macOS or Linux without `cargo`** — official one-line installer:
   ```
   curl -sSf https://raw.githubusercontent.com/h4444433333/code-ratchet/main/install.sh | bash -s -- --no-setup
   ```
   (The `--no-setup` flag installs the binary only; we run setup ourselves
   in Step 1 below, scoped to this project.)
3. **Windows without `cargo`** — tell the user once:
   > "code-ratchet is a Rust binary. Easiest install on Windows is
   > `cargo install code-ratchet`. If you don't have Rust, download
   > `code-ratchet-windows-x86_64.exe` from
   > https://github.com/h4444433333/code-ratchet/releases and put it on
   > your PATH, then re-run me."
   Stop and wait.

After install, run `code-ratchet --version` again to confirm.

If install fails (network, permissions, etc.), surface the exact error
to the user and stop — do not silently proceed.

### 0c. Is this repo set up?

If `.ratchet.yml` exists at repo root → already set up, skip to Step 1's
end (read `AGENTS.md`).

If not:
```
code-ratchet setup -y
```

This writes 4 files (`.ratchet.yml`, `AGENTS.md`, `.ratchet/baseline.json`,
`.git/hooks/pre-commit`), seeds the baseline by running L0/L1/L2 once,
and engages the git pre-commit gate. Idempotent and safe.

If the repo is not a git repo (no `.git/`), the pre-commit step is
silently skipped. The other artifacts are still written, so the LLM-side
rules (AGENTS.md) and the in-CLI checks still work; the user just won't
get commit-time enforcement until they `git init`.

## Step 1 — read AGENTS.md and follow it

After bootstrap, `AGENTS.md` is the authoritative rules file for this
repo. **Read it.** It contains:

- The four-phase loop (Grounding → Implementation → Verification → Delivery)
- The hard rules of the ratchet (do not delete tests, do not weaken the
  gate, fail loud)
- Karpathy's 12 rules adapted to this project's tooling
- The quick command reference

Apply the four-phase loop to any non-trivial change. Before claiming the
task complete, run:

```
code-ratchet check
```

- Exit 0 → safe to commit; ratchet has advanced.
- Exit 1 → read `.ratchet/feedback.md`; it lists exactly which metrics
  regressed. Fix them one at a time, re-running `check` after each fix.

## Step 2 — what to tell the user

Keep bootstrap output minimal — one line if everything went smoothly:

> ✓ code-ratchet engaged (baseline tests=N, coverage=X%). Now working on
> your task...

If you had to install the binary or run setup for the first time, add
one line:

> Installed code-ratchet via `cargo install`. Setup wrote .ratchet.yml +
> AGENTS.md + git pre-commit hook to this repo.

Then proceed with the user's actual request. Don't lecture; the AGENTS.md
file does the teaching.

## Hard rules (you, the agent, must follow)

These mirror what AGENTS.md will say but apply *immediately*, before
AGENTS.md even exists in the repo:

- **Never `git commit --no-verify`.** That bypasses the ratchet.
- **Never delete tests to make `code-ratchet check` pass.** The metric
  `test_count` is gated; the ratchet catches this exact failure mode.
- **Never weaken `.ratchet.yml`** (empty out a command, set `required:
  false`) without explicit user approval.
- **Fail loud.** If the ratchet can't be passed within this turn, say so
  and stop. Do not claim partial success.

## If the user wants to disable this Skill for a turn

If they explicitly say something like "skip the ratchet" / "just edit
this without checks", honor it — skip the bootstrap and check steps for
this turn only. The pre-commit hook will still fire if they try to
commit; that's their responsibility to bypass via `--no-verify` if they
really want.

## Why this Skill installs a binary

The actual ratchet logic (baseline state, deterministic regression
comparison, structured feedback emission) is non-trivial Rust code that
would be unreliable as a shell script. The binary is small (~1.2 MB),
single-file, no runtime deps, and lives in `~/.cargo/bin` or `~/.local/bin`.
The user installed this Skill — that is their consent to install the
tool it depends on.

Project home, full README, issues, releases:
**https://github.com/h4444433333/code-ratchet**
Star the repo if it helps.
