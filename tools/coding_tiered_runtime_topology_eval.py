#!/usr/bin/env python3
"""Evaluate Tier 0 and Tier 1 coding runtime topology.

This harness proves routing shape, not just file existence:

- Tier 0 explicit content must use direct_mutation.
- Tier 1 deterministic manifests must use deterministic_local_loop.
- Both lanes must skip provider startup and model calls.
- Mutation/validation claims must be receipt-backed.
"""

from __future__ import annotations

import json
import subprocess
import tempfile
import time
from pathlib import Path


WORKSPACE = Path(__file__).resolve().parents[1]


def run_infring(prompt: str, name: str) -> tuple[dict, int]:
    root = Path(tempfile.mkdtemp(prefix=f"infring-{name}-"))
    prompt_path = root / "prompt.txt"
    prompt_path.write_text(prompt, encoding="utf-8")
    started = time.time()
    proc = subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "xtask",
            "--",
            "infring-agent-run",
            f"--name={name}",
            "--workflow=local_coding_phase1_mutation_spine",
            "--provider=ollama",
            "--model=kimi-k2.6:cloud",
            "--pack=local-coding-files",
            "--permissions-template=admin",
            f"--prompt=@{prompt_path}",
        ],
        cwd=WORKSPACE,
        text=True,
        capture_output=True,
        timeout=90,
    )
    elapsed_ms = round((time.time() - started) * 1000)
    json_start = proc.stdout.find("{")
    parsed = None
    if json_start >= 0:
        parsed = json.loads(proc.stdout[json_start:])
    return {
        "process_returncode": proc.returncode,
        "elapsed_ms": elapsed_ms,
        "response": parsed,
        "stdout_tail": proc.stdout[-1200:],
        "stderr_tail": proc.stderr[-1200:],
        "prompt_root": str(root),
    }, elapsed_ms


def tier0_cases() -> list[dict]:
    return [
        {
            "name": "tier0-alpha",
            "target": "src/alpha.py",
            "content": "def alpha() -> str:\n    return \"alpha\"\n",
            "prompt": lambda root: (
                f"Project root: {root}\n"
                "Create `src/alpha.py` with this content:\n"
                "```python\n"
                "def alpha() -> str:\n"
                "    return \"alpha\"\n"
                "```"
            ),
        },
        {
            "name": "tier0-beta",
            "target": "web/beta.js",
            "content": "export const beta = () => \"beta\";\n",
            "prompt": lambda root: (
                f"Workspace root: {root}\n"
                "Write `web/beta.js` with this content:\n"
                "```javascript\n"
                "export const beta = () => \"beta\";\n"
                "```"
            ),
        },
    ]


def tier1_cases() -> list[dict]:
    return [
        {
            "name": "tier1-python-unittest",
            "actions": [
                {
                    "type": "write_file",
                    "path": "calc.py",
                    "content": "def add(a: int, b: int) -> int:\n    return a + b\n",
                },
                {
                    "type": "write_file",
                    "path": "test_calc.py",
                    "content": (
                        "import unittest\n"
                        "from calc import add\n\n"
                        "class CalcTests(unittest.TestCase):\n"
                        "    def test_add(self):\n"
                        "        self.assertEqual(add(2, 3), 5)\n\n"
                        "if __name__ == \"__main__\":\n"
                        "    unittest.main()\n"
                    ),
                },
            ],
            "validation": {"cmd": ["python3", "-m", "unittest", "-q"], "timeout_seconds": 30},
        },
        {
            "name": "tier1-json-validator",
            "actions": [
                {
                    "type": "write_file",
                    "path": "config/service.json",
                    "content": "{\"enabled\":true,\"name\":\"tier1\"}\n",
                },
                {
                    "type": "write_file",
                    "path": "validate_config.py",
                    "content": (
                        "import json\n"
                        "from pathlib import Path\n"
                        "data = json.loads(Path('config/service.json').read_text())\n"
                        "assert data['enabled'] is True\n"
                        "assert data['name'] == 'tier1'\n"
                    ),
                },
            ],
            "validation": {"cmd": ["python3", "validate_config.py"], "timeout_seconds": 30},
        },
    ]


def evaluate_tier0(case: dict) -> dict:
    root = Path(tempfile.mkdtemp(prefix=f"infring-{case['name']}-root-"))
    prompt = case["prompt"](root)
    run, _ = run_infring(prompt, case["name"])
    response = run["response"] or {}
    target = root / case["target"]
    actual = target.read_text(encoding="utf-8") if target.exists() else None
    receipts = response.get("receipt", {}).get("native_tool_receipts", [])
    result = {
        "case": case["name"],
        "tier": 0,
        "lane": response.get("contract", {}).get("execution_shape", {}).get("lane"),
        "provider": response.get("contract", {}).get("provider"),
        "provider_start_ms": response.get("trace_summary", {})
        .get("phase_latency_ms", {})
        .get("provider_start"),
        "model_call_ms": response.get("trace_summary", {})
        .get("phase_latency_ms", {})
        .get("model_call"),
        "tool_names": [receipt.get("tool_name") for receipt in receipts],
        "file_matches_expected": actual == case["content"],
        "elapsed_ms": run["elapsed_ms"],
        "target": str(target),
    }
    result["ok"] = (
        response.get("ok") is True
        and result["lane"] == "direct_mutation"
        and result["provider"] is None
        and result["provider_start_ms"] == 0
        and result["model_call_ms"] == 0
        and result["tool_names"] == ["file_write"]
        and result["file_matches_expected"]
    )
    return result


def evaluate_tier1(case: dict) -> dict:
    root = Path(tempfile.mkdtemp(prefix=f"infring-{case['name']}-root-"))
    manifest = {
        "deterministic_local_loop": {
            "workspace_root": str(root),
            "actions": case["actions"],
            "validation": case["validation"],
        }
    }
    prompt = (
        "Run this deterministic local coding action manifest without provider startup:\n"
        "```json\n"
        f"{json.dumps(manifest, indent=2)}\n"
        "```"
    )
    run, _ = run_infring(prompt, case["name"])
    response = run["response"] or {}
    receipts = response.get("receipt", {}).get("native_tool_receipts", [])
    files_exist = all((root / action["path"]).exists() for action in case["actions"])
    result = {
        "case": case["name"],
        "tier": 1,
        "lane": response.get("contract", {}).get("execution_shape", {}).get("lane"),
        "provider": response.get("contract", {}).get("provider"),
        "provider_start_ms": response.get("trace_summary", {})
        .get("phase_latency_ms", {})
        .get("provider_start"),
        "model_call_ms": response.get("trace_summary", {})
        .get("phase_latency_ms", {})
        .get("model_call"),
        "validation_status": response.get("receipt", {}).get("validation_status"),
        "tool_names": [receipt.get("tool_name") for receipt in receipts],
        "files_exist": files_exist,
        "elapsed_ms": run["elapsed_ms"],
        "root": str(root),
    }
    result["ok"] = (
        response.get("ok") is True
        and result["lane"] == "deterministic_local_loop"
        and result["provider"] is None
        and result["provider_start_ms"] == 0
        and result["model_call_ms"] == 0
        and result["tool_names"].count("file_write") == len(case["actions"])
        and "command_run" in result["tool_names"]
        and result["validation_status"] == "passed"
        and files_exist
    )
    return result


def main() -> int:
    results = []
    for case in tier0_cases():
        results.append(evaluate_tier0(case))
    for case in tier1_cases():
        results.append(evaluate_tier1(case))

    payload = {
        "schema_version": "coding_tiered_runtime_topology_eval_v1",
        "attempts": len(results),
        "passes": sum(1 for result in results if result["ok"]),
        "results": results,
        "acceptance": {
            "tier0_requires_lane": "direct_mutation",
            "tier1_requires_lane": "deterministic_local_loop",
            "provider_start_ms": 0,
            "model_call_ms": 0,
        },
    }
    print(json.dumps(payload, indent=2))
    return 0 if payload["passes"] == payload["attempts"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
