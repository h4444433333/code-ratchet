#!/usr/bin/env python3
from __future__ import annotations

import json
import os
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

import main


class GithubEntryTests(unittest.TestCase):
    def test_build_history_dir_defaults_to_ratchet_subdir(self) -> None:
        repo = Path("/tmp/example")

        history_dir = main.build_history_dir(repo, "")

        self.assertEqual(history_dir, repo / ".ratchet" / "llm-history")

    def test_prune_history_files_removes_oldest_entries(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            history_dir = Path(tmp)
            oldest = history_dir / "oldest.json"
            middle = history_dir / "middle.json"
            newest = history_dir / "newest.json"
            for path in (oldest, middle, newest):
                path.write_text("{}", encoding="utf-8")

            os.utime(oldest, (10, 10))
            os.utime(middle, (20, 20))
            os.utime(newest, (30, 30))

            main.prune_history_files(history_dir, max_history=2)

            remaining = sorted(path.name for path in history_dir.iterdir())
            self.assertEqual(remaining, ["middle.json", "newest.json"])

    def test_prepare_mode_writes_bounded_history_record(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            (repo / ".ratchet").mkdir()
            (repo / "AGENTS.md").write_text("follow the rules\n", encoding="utf-8")
            (repo / ".ratchet.yml").write_text("language: python\n", encoding="utf-8")
            (repo / ".ratchet" / "baseline.json").write_text('{"ratchet_count": 1}\n', encoding="utf-8")
            history_dir = repo / "history"
            stale_file = history_dir / "stale.json"
            history_dir.mkdir()
            stale_file.write_text("{}", encoding="utf-8")
            os.utime(stale_file, (10, 10))

            with mock.patch(
                "sys.argv",
                [
                    "main.py",
                    str(repo),
                    "--mode",
                    "prepare",
                    "--task",
                    "Implement bounded history",
                    "--history-dir",
                    str(history_dir),
                    "--max-history",
                    "1",
                ],
            ):
                exit_code = main.main()

            self.assertEqual(exit_code, 0)
            history_files = list(history_dir.glob("*.json"))
            self.assertEqual(len(history_files), 1)
            self.assertNotEqual(history_files[0].name, "stale.json")
            payload = json.loads(history_files[0].read_text(encoding="utf-8"))
            self.assertEqual(payload["mode"], "prepare")
            self.assertEqual(payload["task"], "Implement bounded history")
            self.assertEqual(payload["output_kind"], "prompt")
            self.assertIn("bounded history", payload["prompt"])

    def test_collect_repo_context_reads_ratchet_files(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            (repo / ".ratchet").mkdir()
            (repo / "AGENTS.md").write_text("follow the rules\n", encoding="utf-8")
            (repo / ".ratchet.yml").write_text("language: python\n", encoding="utf-8")
            (repo / ".ratchet" / "baseline.json").write_text('{"ratchet_count": 1}\n', encoding="utf-8")
            (repo / ".ratchet" / "feedback.md").write_text("blocked\n", encoding="utf-8")

            context = main.collect_repo_context(repo)

            self.assertEqual(context["agents_md"], "follow the rules")
            self.assertEqual(context["ratchet_yml"], "language: python")
            self.assertEqual(context["baseline"], {"ratchet_count": 1})
            self.assertEqual(context["feedback_md"], "blocked")

    def test_build_prepare_prompt_includes_guardrails_and_task(self) -> None:
        repo = Path("/tmp/example")
        context = {
            "agents_md": "never bypass hooks",
            "ratchet_yml": "language: python",
            "baseline": {"tests_passing": 8},
            "feedback_md": "",
        }

        prompt = main.build_prepare_prompt(repo, "Fix failing GitHub issue", context)

        self.assertIn("Fix failing GitHub issue", prompt)
        self.assertIn("Do not bypass hooks, delete tests, or weaken ratchet config.", prompt)
        self.assertIn("which tests or coverage additions will protect the change", prompt)
        self.assertIn('"tests_passing": 8', prompt)

    def test_build_repair_prompt_includes_feedback_and_payload(self) -> None:
        repo = Path("/tmp/example")
        payload = {"verdict": "regression_blocked", "regressions": [{"metric": "coverage_percent"}]}
        context = {
            "agents_md": "",
            "ratchet_yml": "",
            "baseline": None,
            "feedback_md": "coverage dropped",
        }

        prompt = main.build_repair_prompt(repo, "Finish the feature", payload, context)

        self.assertIn("Finish the feature", prompt)
        self.assertIn("coverage dropped", prompt)
        self.assertIn('"verdict": "regression_blocked"', prompt)
        self.assertIn("coverage additions are required", prompt)

    def test_check_mode_uses_fresh_context_after_running_check(self) -> None:
        repo = Path("/tmp/example")
        contexts = [
            {"agents_md": "", "ratchet_yml": "", "baseline": None, "feedback_md": "stale feedback"},
            {"agents_md": "", "ratchet_yml": "", "baseline": None, "feedback_md": "fresh feedback"},
        ]
        captured = {}

        class FakeClient:
            def __init__(self, binary: str) -> None:
                self.binary = binary

            def check(self, _repo_path: str | Path, no_ratchet: bool = False):
                _ = no_ratchet
                return SimpleNamespace(stderr="", exit_code=1), {"verdict": "regression_blocked"}

        def fake_collect_repo_context(_repo_path: Path) -> dict:
            return contexts.pop(0)

        def fake_build_repair_prompt(_repo_path: Path, _task: str, _payload: dict, context: dict) -> str:
            captured["feedback_md"] = context["feedback_md"]
            return "prompt"

        with mock.patch("main.CodeRatchet", FakeClient), \
             mock.patch("main.collect_repo_context", side_effect=fake_collect_repo_context), \
             mock.patch("main.build_repair_prompt", side_effect=fake_build_repair_prompt), \
             mock.patch("main.emit_prompt_or_completion", return_value=("prompt", "prompt")), \
             mock.patch("sys.argv", ["main.py", str(repo), "--mode", "check"]):
            exit_code = main.main()

        self.assertEqual(exit_code, 1)
        self.assertEqual(captured["feedback_md"], "fresh feedback")


if __name__ == "__main__":
    unittest.main()
