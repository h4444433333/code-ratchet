#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
from datetime import datetime, timezone
from urllib import request
from urllib.error import HTTPError, URLError

from code_ratchet import CodeRatchet

DEFAULT_MAX_HISTORY = 200


def read_text(path: Path) -> str:
    if not path.exists():
        return ""
    return path.read_text(encoding="utf-8", errors="ignore").strip()


def read_json(path: Path) -> dict | None:
    if not path.exists():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def resolve_task_text(task: str, task_file: str) -> str:
    if task and task_file:
        raise ValueError("Pass either --task or --task-file, not both.")
    if task_file:
        return Path(task_file).read_text(encoding="utf-8").strip()
    return task.strip()


def collect_repo_context(repo: Path) -> dict:
    return {
        "repo": str(repo),
        "agents_md": read_text(repo / "AGENTS.md"),
        "ratchet_yml": read_text(repo / ".ratchet.yml"),
        "baseline": read_json(repo / ".ratchet" / "baseline.json"),
        "feedback_md": read_text(repo / ".ratchet" / "feedback.md"),
    }


def build_history_dir(repo: Path, history_dir: str) -> Path:
    if history_dir:
        return Path(history_dir).expanduser().resolve()
    return repo / ".ratchet" / "llm-history"


def prune_history_files(history_dir: Path, max_history: int) -> None:
    if max_history < 1 or not history_dir.exists():
        return
    files = sorted(
        (path for path in history_dir.iterdir() if path.is_file()),
        key=lambda path: (path.stat().st_mtime, path.name),
    )
    overflow = len(files) - max_history
    for path in files[:max(0, overflow)]:
        path.unlink()


def write_history_record(
    history_dir: Path,
    max_history: int,
    mode: str,
    task: str,
    prompt: str,
    context: dict | None,
    output_kind: str,
    output_content: str,
) -> Path | None:
    if max_history < 1:
        return None
    history_dir.mkdir(parents=True, exist_ok=True)
    timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    record_path = history_dir / f"{timestamp}-{mode}.json"
    record = {
        "timestamp": timestamp,
        "mode": mode,
        "task": task,
        "prompt": prompt,
        "context": context,
        "output_kind": output_kind,
        "output_content": output_content,
    }
    record_path.write_text(json.dumps(record, indent=2), encoding="utf-8")
    prune_history_files(history_dir, max_history=max_history)
    return record_path


def build_prepare_prompt(repo: Path, task: str, context: dict) -> str:
    agents_md = context["agents_md"] or "AGENTS.md is missing. Run setup before asking the LLM to code."
    ratchet_yml = context["ratchet_yml"] or ".ratchet.yml is missing. Run setup before asking the LLM to code."
    baseline_json = json.dumps(context["baseline"], indent=2) if context["baseline"] is not None else "baseline.json is missing."
    requested_task = task or "No GitHub issue text was provided. Infer the smallest safe next step from the repo state."
    return f"""You are the coding agent for a repository protected by code-ratchet.

Repository:
{repo}

GitHub task:
{requested_task}

Development contract:
1. Read and obey AGENTS.md before writing code.
2. Make the smallest safe change that satisfies the task.
3. Preserve or improve lint, typecheck, test count, passing tests, and coverage.
4. Do not bypass hooks, delete tests, or weaken ratchet config.
5. Before coding, state which tests or coverage additions will protect the change.
6. After coding, run the ratchet check and use its result to decide the next turn.

AGENTS.md:
{agents_md}

.ratchet.yml:
{ratchet_yml}

Current baseline.json:
{baseline_json}

Respond with:
1. the exact constraints you will follow,
2. the minimal implementation plan,
3. the tests you will add or preserve,
4. the first coding step.
"""


def build_repair_prompt(repo: Path, task: str, payload: dict, context: dict) -> str:
    requested_task = task or "Continue the current GitHub task without violating the repo ratchet."
    feedback_md = context["feedback_md"] or "feedback.md is missing; rely on the JSON result below."
    return f"""You are a coding assistant helping repair a repository after code-ratchet analysis.

Repository:
{repo}

GitHub task:
{requested_task}

Your task:
1. Read the ratchet result JSON and feedback below.
2. Identify the exact regressions or missing layers that block the repo.
3. Propose the smallest safe fix plan that keeps the task moving.
4. Explain which tests or coverage additions are required before the next commit.
5. Do not suggest bypassing git hooks, deleting tests, or weakening checks.

feedback.md:
{feedback_md}

Ratchet JSON:
{json.dumps(payload, indent=2)}
"""


def call_openai_compatible_llm(prompt: str, model: str, api_key: str, base_url: str) -> str:
    url = base_url.rstrip("/") + "/chat/completions"
    body = {
        "model": model,
        "messages": [
            {"role": "system", "content": "You are a precise software engineering assistant."},
            {"role": "user", "content": prompt},
        ],
        "temperature": 0.2,
    }
    data = json.dumps(body).encode("utf-8")
    req = request.Request(
        url,
        data=data,
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {api_key}",
        },
        method="POST",
    )
    try:
        with request.urlopen(req, timeout=60) as resp:
            payload = json.loads(resp.read().decode("utf-8"))
    except HTTPError as exc:
        detail = exc.read().decode("utf-8", errors="ignore")
        raise RuntimeError(f"LLM HTTP error {exc.code}: {detail}") from exc
    except URLError as exc:
        raise RuntimeError(f"LLM network error: {exc}") from exc

    try:
        return payload["choices"][0]["message"]["content"].strip()
    except (KeyError, IndexError, TypeError) as exc:
        raise RuntimeError(f"Unexpected LLM response: {payload}") from exc


def maybe_run_setup(client: CodeRatchet, repo: Path, enabled: bool) -> int:
    if not enabled:
        return 0
    setup_result = client.setup(repo)
    if setup_result.stdout:
        print(setup_result.stdout.strip())
    if setup_result.stderr:
        print(setup_result.stderr.strip())
    return setup_result.exit_code


def emit_prompt_or_completion(prompt: str, args: argparse.Namespace, fallback: dict | None = None) -> tuple[str, str]:
    if args.print_context and fallback is not None:
        rendered = json.dumps(fallback, indent=2)
        print(rendered)
        return "context", rendered
    if args.print_prompt:
        print(prompt)
        return "prompt", prompt
    if args.api_key:
        completion = call_openai_compatible_llm(prompt, args.model, args.api_key, args.base_url)
        print(completion)
        return "completion", completion
    if fallback is not None:
        rendered = json.dumps(fallback, indent=2)
        print(rendered)
        print()
    else:
        rendered = ""
    print(prompt)
    combined = f"{rendered}\n\n{prompt}".strip() if fallback is not None else prompt
    return "prompt", combined


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Python GitHub/agent entry for the Rust code-ratchet CLI")
    parser.add_argument("repo", nargs="?", default=".", help="target repository")
    parser.add_argument("--binary", default="code-ratchet", help="path to the code-ratchet binary")
    parser.add_argument("--mode", choices=["prepare", "check"], default="check", help="prepare the LLM before coding, or check the repo after coding")
    parser.add_argument("--setup", action="store_true", help="run setup before the selected mode")
    parser.add_argument("--task", default="", help="GitHub issue/task text for the LLM")
    parser.add_argument("--task-file", default="", help="path to a file containing the GitHub issue/task text")
    parser.add_argument("--advance-ratchet", action="store_true", help="allow a passing check to advance the baseline")
    parser.add_argument("--print-prompt", action="store_true", help="print the generated prompt instead of calling an LLM")
    parser.add_argument("--print-context", action="store_true", help="print gathered repo context JSON instead of the prompt")
    parser.add_argument("--model", default=os.environ.get("CODE_RATCHET_LLM_MODEL", "gpt-4o-mini"))
    parser.add_argument("--base-url", default=os.environ.get("CODE_RATCHET_LLM_BASE_URL", "https://api.openai.com/v1"))
    parser.add_argument("--api-key", default=os.environ.get("CODE_RATCHET_LLM_API_KEY", ""))
    parser.add_argument("--history-dir", default="", help="directory for prompt/completion history; defaults to <repo>/.ratchet/llm-history")
    parser.add_argument("--max-history", type=int, default=int(os.environ.get("CODE_RATCHET_MAX_HISTORY", str(DEFAULT_MAX_HISTORY))), help="max history files to retain; 0 disables history")
    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()

    try:
        task_text = resolve_task_text(args.task, args.task_file)
    except ValueError as exc:
        parser.error(str(exc))

    repo = Path(args.repo).resolve()
    client = CodeRatchet(binary=args.binary)
    history_dir = build_history_dir(repo, args.history_dir)

    setup_exit_code = maybe_run_setup(client, repo, args.setup)
    if setup_exit_code != 0:
        return setup_exit_code

    context = collect_repo_context(repo)
    if args.mode == "prepare":
        prompt = build_prepare_prompt(repo, task_text, context)
        output_kind, output_content = emit_prompt_or_completion(prompt, args, fallback=context)
        write_history_record(history_dir, args.max_history, args.mode, task_text, prompt, context, output_kind, output_content)
        return 0

    check_result, payload = client.check(repo, no_ratchet=not args.advance_ratchet)
    if payload is not None:
        context = collect_repo_context(repo)
        prompt = build_repair_prompt(repo, task_text, payload, context)
        output_kind, output_content = emit_prompt_or_completion(prompt, args, fallback=payload)
        write_history_record(history_dir, args.max_history, args.mode, task_text, prompt, context, output_kind, output_content)
    if check_result.stderr:
        print(check_result.stderr.strip())
    return check_result.exit_code


if __name__ == "__main__":
    raise SystemExit(main())
