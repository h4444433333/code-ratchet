# Python entry

This example is the Python-side GitHub/agent entry for `code-ratchet`. It
lets Python orchestrate the development loop while the Rust CLI remains the
canonical ratchet engine.

## Quick use

```bash
cd examples/python
python3 main.py /path/to/your-repo --binary code-ratchet --setup --mode prepare --task "Implement GitHub issue #123"
```

What it does:

- `--mode prepare`: gathers `AGENTS.md`, `.ratchet.yml`, and baseline state,
  then generates the pre-coding prompt that constrains the LLM before edits
- `--mode check`: runs `code-ratchet check --json`, reads feedback, and
  generates the post-coding repair prompt for the next agent turn
- `--setup`: optionally bootstraps the repo before either phase
- history is stored under `.ratchet/llm-history` by default
- default history retention is `200`
- if `CODE_RATCHET_LLM_API_KEY` is set, can call an OpenAI-compatible LLM
- otherwise prints the prompt or gathered JSON context directly
- use `--max-history 0` to disable history persistence

## LLM environment

The entry `main.py` reads these environment variables:

```bash
export CODE_RATCHET_LLM_API_KEY=...
export CODE_RATCHET_LLM_MODEL=gpt-4o-mini
export CODE_RATCHET_LLM_BASE_URL=https://api.openai.com/v1
export CODE_RATCHET_MAX_HISTORY=200
```

## Commands

Prepare:

```bash
python3 main.py /path/to/your-repo --binary code-ratchet --mode prepare --task "Fix flaky pagination test"
```

Check:

```bash
python3 main.py /path/to/your-repo --binary code-ratchet --mode check --task "Fix flaky pagination test"
```

Print prompt only:

```bash
python3 main.py /path/to/your-repo --binary code-ratchet --mode prepare --task "Fix flaky pagination test" --print-prompt
```

Print context only:

```bash
python3 main.py /path/to/your-repo --binary code-ratchet --mode prepare --print-context
```

Smaller history:

```bash
python3 main.py /path/to/your-repo --binary code-ratchet --mode check --max-history 10
```

Disable history:

```bash
python3 main.py /path/to/your-repo --binary code-ratchet --mode prepare --max-history 0
```
