# Publishing this Skill to clawhub

This directory contains the **discovery + onboarding Skill** for Claude Code
users. It is not part of the core `code-ratchet` binary; it is a thin
Markdown wrapper whose job is to:

1. Be findable in clawhub search.
2. Get a Claude Code user from "I heard about it" to "it's running" in two
   commands.
3. Drive traffic to the GitHub repo (where stars, issues, releases live).

## What gets uploaded

A single file: `SKILL.md`.

The frontmatter at the top is what clawhub indexes for search:
- `name` — must match `code-ratchet`; this is the slug.
- `description` — first 2 lines are what users see in the listing.
  Front-load the keywords that matter (LLM, quality, ratchet, AGENTS.md).
- `keywords` — clawhub uses these for tag-based discovery.

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
