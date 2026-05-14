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
  - Bash(git clone *)
  - Bash(gh auth status)
  - Bash(gh repo clone *)
  - Bash(pwd)
  - Bash(ls *)
  - Read(*)
---

# code-ratchet

You are working in a repo where code quality is governed by **code-ratchet**:
persisted "best-ever" lint/type/test/coverage metrics, with any worsening
change rejected at `git commit` time. This Skill must make onboarding feel
automatic: if the repo is already present, engage the ratchet; if the user
gave a GitHub repo and the code is not local yet, clone it first; if auth is
required, guide the user to use their local GitHub auth without ever asking
for a password.

Project home (source, issues, releases):
**https://github.com/h4444433333/code-ratchet**

## Step 0 — bootstrap (every session, runs silently)

Before doing anything else in a code-changing task, ensure you are in the
right repo, the tool is installed, and the repo is set up. Keep the user
experience smooth: prefer doing the work silently and only interrupt when a
real choice or auth boundary exists.

### 0a. Decide what repo you are operating on

Use this decision order:

1. If the current directory is already the user's project repo, stay there.
2. If the user provided a GitHub URL or `owner/repo` and the repo is not
   present locally, clone it first.
3. If the user is clearly asking to try or inspect **code-ratchet itself**
   and no target repo is given, clone:
   ```
   https://github.com/h4444433333/code-ratchet
   ```
4. If the target repo is ambiguous, ask one concise question instead of
   guessing.

For cloning:

- **Public repo**: clone directly with `git clone` or `gh repo clone`.
- **Private repo**: first check `gh auth status`.
  - If auth is already present, clone normally.
  - If auth is missing, tell the user to run `gh auth login` locally, then
    continue after they confirm.

Never ask the user for a GitHub username, password, PAT, or SSH private key.
Always rely on the machine's existing git/gh/SSH auth.

After clone, `cd` into the repo root and continue with the steps below.

### 0b. Is `code-ratchet` on PATH?

Run:
```
command -v code-ratchet >/dev/null 2>&1 && code-ratchet --version
```

- If it prints a version → continue to 0c.
- If not found → install it as below, then re-verify.

### 0c. Installing code-ratchet (only when missing)

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

### 0d. Is this repo set up?

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

### 0e. Default smooth-path behaviors

Apply these defaults unless the user asked for something else:

- If you can clone and set up without asking, do it.
- If auth is needed, ask only for the minimum next step:
  > "I need your local GitHub auth for this private repo. Please run
  > `gh auth login`, then tell me to continue."
- If the repo is already local, do not explain the bootstrap in detail; just
  engage the ratchet and move on.
- If the user only wants to **use** code-ratchet rather than modify its
  source, prefer installing the binary and setting up their target repo over
  cloning the `code-ratchet` source tree.

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

If you had to clone/install/setup for the first time, add one line:

> Cloned the repo, installed code-ratchet, and engaged the ratchet
> (`.ratchet.yml` + `AGENTS.md` + git pre-commit hook).

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
- **Never ask the user for GitHub credentials.** Use existing local auth or
  tell them to run `gh auth login` themselves.
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

## Quick onboarding examples

### Example A — user already opened a local repo

1. Confirm `code-ratchet` is installed.
2. Run `code-ratchet setup -y` if `.ratchet.yml` is missing.
3. Read `AGENTS.md`.
4. Work normally and finish with `code-ratchet check`.

### Example B — user gives a public GitHub repo

1. Clone the repo silently.
2. `cd` into it.
3. Install `code-ratchet` if missing.
4. Run `code-ratchet setup -y`.
5. Read `AGENTS.md` and continue the task.

### Example C — user gives a private GitHub repo

1. Check `gh auth status`.
2. If logged in, clone and continue.
3. If not logged in, ask the user to run `gh auth login`.
4. After they confirm, clone, set up, and continue.
