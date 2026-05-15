# code-ratchet

[![ci](https://github.com/h4444433333/code-ratchet/actions/workflows/ci.yml/badge.svg)](https://github.com/h4444433333/code-ratchet/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/code-ratchet.svg)](https://crates.io/crates/code-ratchet)
[![license: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![platforms](https://img.shields.io/badge/platforms-macOS%20%7C%20Linux%20%7C%20Windows-success)](#platform-support)

> A complexity ratchet for AI-assisted code. **Quality only goes up, never down.**

Works with **any LLM, any IDE** by keeping repo rules in `AGENTS.md` and
enforcing regressions at `git commit` time.

## Quick Start

```bash
cargo install code-ratchet
cd your-project
code-ratchet setup
```

After `setup`, just develop normally.

- `git commit` automatically runs the ratchet gate
- `code-ratchet check` manually verifies the repo right now
- `code-ratchet setup` now auto-starts background `watch`
- `code-ratchet watch` is still available for manual foreground use

## What it does

1. **Persists "best-ever" quality metrics** in `.ratchet/baseline.json`:
   lint warnings, type errors, test count, tests passing, coverage %.
2. **Rejects any change that worsens any metric** at `git commit` time via
   a pre-commit hook. You cannot bypass the gate by lowering the gate
   (changing `.ratchet.yml` or deleting tests is exactly what it catches).
3. **Writes `AGENTS.md` to the repo root** — a single repo rules file that
   many agents can use for guidance, while the hook and CI remain the hard
   enforcement layer. Karpathy's 12 rules + a four-phase loop + the hard
   rules of the ratchet, all in one place.
4. **Background `watch`** re-runs L0 (lint) on every save, keeping
   `.ratchet/feedback.md` fresh so the LLM gets real-time signal without
   waiting for commit-time.

## Install

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

## Two entry paths

`code-ratchet` now has two maintained entry paths:

- **GitHub version**: the Rust CLI distributed from GitHub Releases and crates.io
- **Skill version**: the Claude Skill under `skills/code-ratchet/`, powered by the bundled Python runtime

The old plugin packaging is gone. The repo is organized around these two
surfaces only.

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

Planned actions (5):
  • Write .ratchet.yml          (config: which lint/typecheck/test commands)
  • Write AGENTS.md             (universal LLM rules)
  • Seed baseline               (runs L0/L1/L2 once)
  • Install git pre-commit hook
  • Start background watch      (auto-refresh feedback)
```

After setup completes, `code-ratchet` also starts background `watch` and
writes its output to `.ratchet/watch.log`.

Many AI tools can use `AGENTS.md` from the repo root for guidance; if your
tool does not, point it there once and rely on the git hook / CI as the hard
gate.

## Enable flow

### GitHub version

1. Install the Rust CLI from GitHub Releases, `cargo install`, or `install.sh`.
2. Run `code-ratchet setup`.
3. `setup` writes `.ratchet.yml` and `AGENTS.md`, runs the first full `check`, installs the git hook, and auto-starts background `watch`.
4. During development, re-run `code-ratchet check` after code changes.
5. At commit time, the git pre-commit hook remains the hard gate.

### Skill version

1. Install the Skill from `skills/code-ratchet/`.
2. In Claude Code, run `/ratchet-build`.
3. The bundled Python runtime writes `.ratchet.yml` and `AGENTS.md`, runs the first full `check`, installs the git hook, and auto-starts background `watch`.
4. During development in that session, re-run the ratchet `check` after code changes.
5. `/ratchet-close` stops session-level auto behavior and stops background `watch`; `/ratchet-uninstall` removes repo-owned files.

## Runtime flow

| When                    | What runs                       | What blocks |
| ----------------------- | ------------------------------- | --- |
| As LLM edits            | background `watch` (auto-started by enablement) | nothing — informational, writes `feedback.md` |
| LLM tries `git commit`  | `.git/hooks/pre-commit`         | regression → commit rejected + feedback.md written |
| In CI                   | `code-ratchet check` job        | same as above, blocks PR |

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

After every successful `code-ratchet check`, the baseline is updated to the
better value of each field:

| Field             | Direction        |
| ----------------- | ---------------- |
| `lint_warnings`   | must not rise    |
| `type_errors`     | must not rise    |
| `test_count`      | must not fall    |
| `tests_passing`   | must not fall    |
| `coverage_percent`| must not fall (0.1pp jitter tolerated) |

A small 0.1pp tolerance on coverage handles floating-point + test
discovery noise. Everything else is exact.

## Commands

```bash
code-ratchet setup
code-ratchet check
code-ratchet watch
code-ratchet status
code-ratchet uninstall
code-ratchet init
code-ratchet install-hook
```

## Uninstall

```bash
code-ratchet uninstall
```

Removes `.ratchet/`, `.ratchet.yml`, `AGENTS.md` (only if it bears our
ownership marker), and the git pre-commit hook (only if it's ours).
Hand-edited `AGENTS.md` and third-party hooks are left alone.

## Python entry

If your team mostly works in Python, use [examples/python](examples/python/README.md) as the code entrypoint.

- `examples/python/code_ratchet.py` wraps the Rust CLI
- `examples/python/main.py` now supports a `prepare` phase before coding and a `check` phase after coding
- `examples/python/main.py` keeps bounded local history; default retention is `200`
- this is the recommended way to integrate `code-ratchet` into a Python agent loop without re-implementing the ratchet in Python

## Release

```bash
cargo login <your-crates.io-token>
git tag v0.1.1
git push origin v0.1.1
```

Before release, set `CARGO_REGISTRY_TOKEN` in repo secrets. Tag push triggers
`.github/workflows/release.yml`, which uploads binaries to GitHub Releases and
publishes to crates.io.

## License

MIT.
