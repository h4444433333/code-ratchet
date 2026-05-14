# code-ratchet

[![ci](https://github.com/h4444433333/code-ratchet/actions/workflows/ci.yml/badge.svg)](https://github.com/h4444433333/code-ratchet/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/code-ratchet.svg)](https://crates.io/crates/code-ratchet)
[![license: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![platforms](https://img.shields.io/badge/platforms-macOS%20%7C%20Linux%20%7C%20Windows-success)](#platform-support)

> A complexity ratchet for AI-assisted code. **Quality only goes up, never down.**

One tool, one rules file, two commands. Works with **any LLM, any IDE** —
Claude Code, Cursor, Aider, Codex, Cline, Continue — because enforcement
happens at `git commit` time, and the LLM-facing rules live in a single
`AGENTS.md` that any modern agent reads from the repo root.

Inspired by Garry Tan's *Complexity Ratchet* and Karpathy's development rules.

## Why this exists

AI coding tools regress quality silently. They delete a test "because it
was failing," skip a typecheck "to save tokens," or refactor adjacent
code and quietly drop coverage. The next agent inherits the rotted
state. After 50 turns the codebase is unrecognizable.

`code-ratchet` is the mechanical pawl: it persists "best-ever" lint /
type / test / coverage metrics, and any commit that worsens *any* of
them is rejected. The agent **cannot make the gate pass by lowering the
gate** — that's the failure mode the design is built to catch.

It is two articles' worth of theory boiled down to two commands.

## What it does

1. **Persists "best-ever" quality metrics** in `.ratchet/baseline.json`:
   lint warnings, type errors, test count, tests passing, coverage %.
2. **Rejects any change that worsens any metric** at `git commit` time via
   a pre-commit hook. You cannot bypass the gate by lowering the gate
   (changing `.ratchet.yml` or deleting tests is exactly what it catches).
3. **Writes `AGENTS.md` to the repo root** — a single universal rules file
   any LLM in any IDE picks up. Karpathy's 12 rules + a four-phase loop +
   the hard rules of the ratchet, all in one place.
4. **`code-ratchet watch`** runs in a second terminal and re-runs L0 (lint)
   on every save, keeping `.ratchet/feedback.md` fresh so the LLM gets
   real-time signal without waiting for commit-time.

## Install + setup

Pick whichever install path suits you. After install, run `code-ratchet
setup` in any project (or the curl-bash path does it automatically).

| You're on…                | Install command                                                                                                           |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Rust toolchain (any OS)   | `cargo install code-ratchet`                                                                                              |
| macOS / Linux, no Rust    | `curl -sSf https://raw.githubusercontent.com/h4444433333/code-ratchet/main/install.sh \| bash`                             |
| macOS via Homebrew        | Coming soon                                                                                                                 |
| Windows                   | `cargo install code-ratchet` (recommended) — or download the `code-ratchet-windows-x86_64.exe` from GitHub Releases        |

`pip install` doesn't apply: code-ratchet is a Rust binary, not a Python
package. `cargo install` is the Rust-native equivalent — one command, no
build dependencies after the binary lands.

Already installed? In any project:

```bash
code-ratchet setup
```

This downloads nothing; it just writes 4 files to your repo (config,
AGENTS.md, baseline, pre-commit hook) and seeds the baseline.

## Platform support

| Platform                  | Build | Runtime |
| ------------------------- | ----- | ------- |
| macOS arm64               | ✅    | ✅      |
| macOS x86_64              | ✅    | ✅      |
| Linux x86_64              | ✅    | ✅      |
| Linux arm64               | ✅    | ✅      |
| Windows x86_64            | ✅    | ✅ *    |

\* Windows runtime note: the git pre-commit hook is a bash script; Git
for Windows ships its own `bash`, so the hook runs out of the box on a
standard Windows + Git installation. Inside the binary, every `Command`
call routes through the platform's native shell (`cmd /C` on Windows,
`sh -c` elsewhere).

CI builds + tests on Linux / macOS / Windows on every push (see
`.github/workflows/ci.yml`). Tagged releases produce binaries for all
five targets above (see `.github/workflows/release.yml`).

## What setup actually does

```
Detected language : Python

Planned actions (4):
  • Write .ratchet.yml          (config: which lint/typecheck/test commands)
  • Write AGENTS.md             (universal LLM rules)
  • Seed baseline               (runs L0/L1/L2 once)
  • Install git pre-commit hook
```

That's the whole product. No per-IDE adapters. No clawhub. No skill packages.
Any AI tool that reads the repo (Cursor, Claude Code, Aider, Codex, Cline,
Continue, …) will pick up `AGENTS.md` automatically or you can point it
there once.

## Daily use

The LLM writes code. Three things keep quality moving up:

| When                    | What runs                       | What blocks |
| ----------------------- | ------------------------------- | --- |
| As LLM edits            | `code-ratchet watch` (optional) | nothing — informational, writes `feedback.md` |
| LLM tries `git commit`  | `.git/hooks/pre-commit`         | regression → commit rejected + feedback.md written |
| In CI                   | `code-ratchet check` job        | same as above, blocks PR |

When blocked, `.ratchet/feedback.md` contains:
- The specific metrics that regressed (baseline → current → delta)
- Targeted suggestions ("Coverage dropped from 91% to 88%. Add tests for…")
- The exit code the hook returned

The LLM reads `feedback.md`, fixes the regressions, re-commits.

## Configuration

`.ratchet.yml` at the repo root. Edit to point at your real tools:

```yaml
language: python
l0:
  command: ruff check .
  required: true
l1:
  command: mypy .
  required: false
l2:
  command: pytest --cov=. --cov-report=term-missing -q
  required: true
```

Empty `command` disables a layer. `required: false` makes a layer advisory.

Defaults are emitted for Python, JavaScript, TypeScript. Other languages:
just edit the commands.

## How quality goes up

After every successful `code-ratchet check`, the baseline is updated to the
**better** value of each field:

| Field             | Direction        |
| ----------------- | ---------------- |
| `lint_warnings`   | must not rise    |
| `type_errors`     | must not rise    |
| `test_count`      | must not fall    |
| `tests_passing`   | must not fall    |
| `coverage_percent`| must not fall (0.1pp jitter tolerated) |

A small 0.1pp tolerance on coverage handles floating-point + test
discovery noise. Everything else is exact.

## Uninstall

```bash
code-ratchet uninstall
```

Removes `.ratchet/`, `.ratchet.yml`, `AGENTS.md` (only if it bears our
ownership marker), and the git pre-commit hook (only if it's ours).
Hand-edited `AGENTS.md` and third-party hooks are left alone.

## Commands

```
code-ratchet setup           # the recommended entry point
code-ratchet check           # run all layers, compare baseline, write feedback
code-ratchet watch           # real-time L0 (lint) feedback while editing
code-ratchet status          # print current baseline
code-ratchet uninstall       # reverse setup
code-ratchet init            # write defaults without seeding (manual flow)
code-ratchet install-hook    # install pre-commit hook only
```

## What you see when the ratchet fires

After a bad change, the LLM (and the human, via pre-commit) reads
`.ratchet/feedback.md`:

```markdown
# code-ratchet feedback

**Verdict:** Blocked — quality regression detected.

## Regressions (these block the commit)
| Metric             | Direction          | Baseline | Current | Delta  |
|--------------------|--------------------|----------|---------|--------|
| coverage_percent   | must not decrease  | 91.20    | 87.40   | -3.80  |
| test_count         | must not decrease  | 180      | 174     | -6     |

## Suggestions for the next agent turn
- Coverage dropped from 91.20% to 87.40%. Add tests covering the changed
  code paths — Capers Jones research shows steep defect-escape cliff
  below ~85%.
- Test count dropped from 180 to 174. If you removed a test, replace it
  with an equivalent that covers the same intent.
```

The agent reads this and self-corrects in the next turn. No human
needs to intervene.

## FAQ

**Q: How is this different from `pre-commit` (the Python framework)?**
> `pre-commit` runs your linters and tests. It doesn't remember how many
> warnings you had yesterday. `code-ratchet` does: it stores a baseline
> and blocks any *regression*. You can run both together — they
> compose.

**Q: Will it work with my LLM / IDE?**
> Yes. The git pre-commit hook is universal — it fires for any tool that
> eventually runs `git commit`. The `AGENTS.md` rules file is picked up
> by any modern agent (Cursor, Claude Code, Aider, Codex, Cline,
> Continue all read top-level repo rules). No per-IDE adapter needed.

**Q: My LLM keeps trying to delete tests to pass the ratchet. What do
I do?**
> `AGENTS.md` already tells it not to. If it persists, the ratchet still
> catches it — `test_count` is one of the gated metrics, so a removed
> test exits 1 at commit time and the LLM is forced to re-add equivalent
> coverage.

**Q: Does it work for languages other than Python / JS / TS?**
> Yes — those are the auto-detected defaults, but `.ratchet.yml` is just
> three commands (lint, typecheck, test+coverage). Point them at your
> stack's tools (golangci-lint, cargo clippy, rspec, whatever) and the
> ratchet works the same.

**Q: Can I temporarily bypass it?**
> `git commit --no-verify` works but the `AGENTS.md` rules forbid the
> LLM from doing it. For a human override during exceptional work,
> that's the escape valve.

**Q: How big is the binary?**
> ~1.2 MB. Single static binary. No runtime dependencies.

**Q: Is the baseline file checked into git?**
> Yes — `.ratchet/baseline.json` should be committed. That's how the
> ratchet's state travels with the repo. Feedback files
> (`.ratchet/feedback.json`, `.ratchet/feedback.md`) are gitignored.

**Q: What if multiple branches have different baselines?**
> The baseline file merges like any other JSON. On conflict, take the
> *better* value of each field — that's the ratchet's whole point.

## Publishing (for maintainers)

```bash
# One-time:
cargo login <your-crates.io-token>

# Cut a release:
git tag v0.1.1
git push origin v0.1.1   # triggers .github/workflows/release.yml,
                         # which builds 5 binaries + publishes to crates.io
```

The release workflow uploads platform binaries to GitHub Releases and runs
`cargo publish` automatically. Set `CARGO_REGISTRY_TOKEN` in the repo
secrets first.

## License

MIT.
