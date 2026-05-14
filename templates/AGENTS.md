# AGENTS.md

> Code quality in this repo is enforced by **code-ratchet** — a single tool
> that ratchets quality metrics in one direction only: upward. This file
> tells you (the AI agent) how to work here without fighting the gate.
> _Managed by `code-ratchet setup`; this marker line is used by `uninstall`._

## The contract

Every code change in this repo must satisfy three conditions before it can be
committed (the git pre-commit hook enforces them mechanically):

1. **Lint warnings must not increase.**
2. **Type errors must not increase.**
3. **Test count, passing tests, and coverage must not decrease.**

The current floor is in `.ratchet/baseline.json`. After every successful
check, the floor is raised to your new values. Quality only ever goes up.

The hard rule for you: **do not try to make the gate pass by lowering the
gate.** Don't delete tests, don't disable lint rules, don't edit
`.ratchet.yml` to drop a layer. If you genuinely can't pass the gate within
this turn, say so and stop — fail loud, not silently.

## What you must produce in every code-changing turn

Three artifacts, in the same turn, atomically:

1. **The code change itself** — minimal, surgical, no unrequested refactoring.
2. **Tests covering the new behavior.** Tests should encode the *intent*
   ("why this is needed") — if a logically equivalent rewrite of the code
   would still pass the test, the test is good. If the test only passes
   when the implementation is byte-identical, the test is too brittle.
3. **A commit message** explaining the *why*, not just the *what*. The
   diff already shows the what.

If a behavior branch is new (an `if`, a new condition, a new error case),
add a test for it. Don't defer.

## Four-phase loop (apply for any non-trivial change)

### 1. Grounding — read before you write
- State the task in one sentence.
- List the files you have actually read (not guessed at).
- Flag uncertainty explicitly. If you can't say what the success criterion
  is, stop and ask the user.

### 2. Implementation — surgical only
- Touch only what the task requires.
- Match the existing conventions of the codebase (naming, file layout,
  error patterns). Convention beats novelty.
- Don't refactor adjacent code "while you're there" — coverage drops from
  unsolicited refactors will block the commit.
- Don't introduce new dependencies without explicit justification.

### 3. Verification — run the gate
```
code-ratchet check
```
- Exit 0 + `PASS — ratchet advanced` → safe to commit.
- Exit 1 + `BLOCKED` → read `.ratchet/feedback.md`; it lists exactly which
  metrics regressed and by how much. Fix the listed regressions one at a
  time; re-run after each fix.

For tighter feedback while editing, ask the user to run `code-ratchet
watch` in a second terminal — it re-runs L0 (lint) on every save and keeps
`.ratchet/feedback.md` fresh.

### 4. Delivery — honest report
When you hand back to the user, include:
- Files changed.
- The new `ratchet_count` (run `code-ratchet status` if unsure).
- Which metrics improved (lint warnings dropped, coverage rose, tests
  added) — those are your evidence of progress.

## Hard rules — non-negotiable

- **Tests verify intent, not behavior.** A test that breaks when you
  refactor without changing logic is a bad test. Rewrite it.
- **Fail loud.** If the ratchet can't be passed in this turn, say so
  explicitly. Don't claim partial success and hide the failure.
- **Don't `git commit --no-verify`.** The pre-commit hook is the
  cross-tool fallback. Bypassing it removes the ratchet's pawl.
- **Don't reduce test_count, tests_passing, or coverage to pass the
  gate.** That is the exact failure mode the gate exists to catch.
- **Don't edit `.ratchet.yml` to weaken the gate.** If a layer is wrong
  (e.g., points at the wrong test command), surface it to the user; do
  not silently rewrite it.

## Soft rules — apply judgment

- **Think before coding.** State assumptions; ask when ambiguous; prefer
  the simplest approach unless the user has signalled they want more.
- **Simplicity first.** No speculative features. No abstractions for
  single-use code. Three repeated lines is better than a premature
  abstraction.
- **Read before you write.** Check existing exports, callers, and shared
  utilities before adding new functions. Don't write duplicates.
- **Surface conflicts.** If the codebase has two conflicting patterns
  (camelCase vs snake_case, two error handling styles), pick one and
  flag the other to the user. Don't quietly blend them.
- **Hard token budgets.** Single task: ~4k tokens of context. Single
  session: ~30k. As you approach those, summarize and restart.
- **Checkpoint after each significant step** in a multi-step task — one
  short progress line — so we can stop and resume cleanly.
- **Surgical, not exhaustive.** This is a bug fix or a feature change.
  Not a code-quality cleanup unless asked.

## What the ratchet does NOT enforce

- Architectural taste, naming aesthetics, whether the diff is "clean".
- Whether the test you added actually tests the *right* thing — the
  ratchet counts tests, not their quality. You are responsible for that.
- Performance, security, accessibility — they each need their own checks.
- Whether the *commit message* is honest. You are.

Treat code-ratchet as the floor. The four-phase loop and the rules above
are the ceiling. Apply your own judgment in the space between.

## Quick reference

| Goal                       | Command                       |
| -------------------------- | ----------------------------- |
| Run all checks now         | `code-ratchet check`          |
| Real-time L0 feedback      | `code-ratchet watch`          |
| Print current baseline     | `code-ratchet status`         |
| Read structured feedback   | `cat .ratchet/feedback.md`    |
