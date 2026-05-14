# code-ratchet

> A complexity ratchet for AI-assisted code. **Quality only goes up, never down.**

One tool, one rules file, two commands. Works with any LLM, any IDE — because
enforcement happens at `git commit` time, and the LLM-facing rules live in a
single `AGENTS.md` that any modern agent reads from the repo root.

Inspired by Garry Tan's *Complexity Ratchet* and Karpathy's development rules.

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
| macOS / Linux, no Rust    | `curl -sSf https://raw.githubusercontent.com/<author>/code-ratchet/main/install.sh \| bash`                                |
| macOS via Homebrew        | `brew install <author>/tap/code-ratchet`   *(Tap published alongside v0.1)*                                                |
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

## Publishing (for maintainers)

```bash
# One-time:
cargo login <your-crates.io-token>

# Cut a release:
git tag v0.1.0
git push origin v0.1.0   # triggers .github/workflows/release.yml,
                         # which builds 5 binaries + publishes to crates.io
```

The release workflow uploads platform binaries to GitHub Releases and runs
`cargo publish` automatically. Set `CARGO_REGISTRY_TOKEN` in the repo
secrets first.

## License

MIT.
