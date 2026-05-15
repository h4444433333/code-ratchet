#!/usr/bin/env python3
from __future__ import annotations

import json
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass
class CommandResult:
    exit_code: int
    stdout: str
    stderr: str

    @property
    def ok(self) -> bool:
        return self.exit_code == 0


class CodeRatchet:
    """Tiny Python wrapper around the Rust `code-ratchet` CLI."""

    def __init__(self, binary: str = "code-ratchet") -> None:
        self.binary = binary

    def setup(self, repo: str | Path, yes: bool = True) -> CommandResult:
        args = [self.binary, "--repo", str(Path(repo).resolve()), "setup"]
        if yes:
            args.append("-y")
        return self._run(args)

    def check(self, repo: str | Path, no_ratchet: bool = False) -> tuple[CommandResult, dict[str, Any] | None]:
        args = [self.binary, "--repo", str(Path(repo).resolve()), "check", "--json"]
        if no_ratchet:
            args.append("--no-ratchet")
        result = self._run(args)
        payload = None
        if result.stdout.strip():
            payload = json.loads(result.stdout)
        return result, payload

    def status(self, repo: str | Path) -> CommandResult:
        return self._run([self.binary, "--repo", str(Path(repo).resolve()), "status"])

    def watch(self, repo: str | Path) -> CommandResult:
        return self._run([self.binary, "--repo", str(Path(repo).resolve()), "watch"])

    def uninstall(self, repo: str | Path, yes: bool = True) -> CommandResult:
        args = [self.binary, "--repo", str(Path(repo).resolve()), "uninstall"]
        if yes:
            args.append("-y")
        return self._run(args)

    def _run(self, args: list[str]) -> CommandResult:
        completed = subprocess.run(args, text=True, capture_output=True)
        return CommandResult(completed.returncode, completed.stdout, completed.stderr)
