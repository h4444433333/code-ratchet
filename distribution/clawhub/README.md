# Publishing this Skill to clawhub

This directory contains the **discovery + onboarding Skill** for Claude Code
users. It is not part of the core `code-ratchet` binary; it is a thin
Markdown wrapper whose job is to:

1. Be findable in clawhub search.
2. Get a Claude Code user from "I heard about it" to "it's running" in two
   commands.
3. Drive traffic to the GitHub repo (where stars, issues, releases live).

## What this Skill actually does

It is **agentic, not passive**. When a user installs it from clawhub and
asks Claude for any code-change task in a repo, the Skill instructs
Claude to:

1. Detect if `code-ratchet` is on PATH; if not, install it
   (`cargo install code-ratchet` preferred; falls back to the
   curl-bash one-liner on macOS/Linux).
2. Check if the current repo has `.ratchet.yml`; if not, run
   `code-ratchet setup -y` to write the 4 artifact files and seed
   the baseline.
3. Read the `AGENTS.md` the setup just wrote and apply its four-phase
   loop + hard rules to the user's request.
4. Run `code-ratchet check` before declaring the task complete.

The user experience is: install the Skill once → ask normal questions
forever after → ratchet just works. They never type a `cargo install`
or `code-ratchet setup` command themselves.

The Skill's `allowed-tools` frontmatter pre-authorizes the specific
shell commands needed for bootstrap (`cargo install code-ratchet`, the
official curl-bash installer URL, and all `code-ratchet *` subcommands),
so Claude doesn't have to prompt the user for permission on each step.

## Uploading

Follow clawhub's contribution flow (their README is the canonical reference):

1. Fork the clawhub index repo.
2. Add an entry pointing at this `SKILL.md` — typically a YAML file with
   the skill's metadata plus a link to a raw `SKILL.md` URL.
3. Open a PR.

Once merged, the Skill appears in clawhub search.

## Keeping it in sync

When you update the `code-ratchet` CLI behavior in a way that changes the
onboarding flow (e.g., new subcommand, new platform), update this
`SKILL.md` too. Version the Skill alongside the CLI by bumping the
description in clawhub's metadata file.

The Skill should never duplicate the full `AGENTS.md` (which the binary
writes into the user's repo on setup). That would create a maintenance
fork. The Skill's job is purely onboarding; once installed, the binary's
own `AGENTS.md` takes over.

## Why this lives in `distribution/` not `adapters/`

`adapters/` historically meant "per-IDE integration logic." We deleted
that approach in v0.1.0 to keep the core surface minimal. `distribution/`
is a different concept: per-channel onboarding assets. Future siblings
might be `distribution/homebrew/` (Tap formula),
`distribution/dockerhub/` (a Docker image), etc.
