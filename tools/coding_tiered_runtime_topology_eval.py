#!/usr/bin/env python3
"""Evaluate Tier 0 through Tier 4 coding runtime topology.

This harness proves routing shape, not just file existence:

- Tier 0 explicit content must use direct_mutation.
- Tier 1 deterministic manifests must use deterministic_local_loop.
- Tier 2 bounded natural-language local tasks must use model_manifest_planner.
- Tier 3 existing-project tasks must use native discovery/read/edit/validation tooling.
- Tier 4 repair tasks must use failed validation as input, mutate, then pass validation.
- Tier 0 and Tier 1 must skip provider startup and model calls.
- Tier 2 must skip provider startup, but may use one model planning call.
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
        timeout=180,
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


def tier2_cases() -> list[dict]:
    return [
        {
            "name": "tier2-python-utility",
            "expected_paths": ["palindrome.py", "test_palindrome.py"],
            "prompt": lambda root: (
                f"Project root: {root}\n"
                "Build a tiny Python palindrome utility using only the standard library.\n"
                "Create palindrome.py and test_palindrome.py.\n"
                "The utility should expose is_palindrome(value: str) -> bool.\n"
                "The tests should cover mixed case, spacing, punctuation, and a negative example.\n"
                "Run the local unittest validation after writing the files."
            ),
        }
    ]


def tier3_cases() -> list[dict]:
    return [
        {
            "name": "tier3-existing-project-edit",
            "initial_files": {
                "math_tools.py": "def add(a, b):\n    return a + b\n",
                "test_math_tools.py": (
                    "import unittest\n"
                    "from math_tools import add\n\n"
                    "class MathToolsTests(unittest.TestCase):\n"
                    "    def test_add(self):\n"
                    "        self.assertEqual(add(2, 3), 5)\n\n"
                    "if __name__ == \"__main__\":\n"
                    "    unittest.main()\n"
                ),
            },
            "expected_paths": ["math_tools.py", "test_math_tools.py"],
            "prompt": lambda root: (
                f"Project root: {root}\n"
                "This is an existing project. Inspect the local files and add a subtract(a, b) "
                "function to math_tools.py, then add a unittest for it in test_math_tools.py. "
                "Run the local unittest validation after editing."
            ),
            "expected_content_markers": ["def subtract", "test_subtract"],
        }
    ]


def tier4_cases() -> list[dict]:
    return [
        {
            "name": "tier4-validation-guided-repair",
            "initial_files": {
                "slug_tools.py": (
                    "def slugify(value: str) -> str:\n"
                    "    return value.lower().replace(\" \", \"-\")\n"
                ),
                "test_slug_tools.py": (
                    "import unittest\n"
                    "from slug_tools import slugify\n\n"
                    "class SlugToolsTests(unittest.TestCase):\n"
                    "    def test_removes_punctuation(self):\n"
                    "        self.assertEqual(slugify(\"Hello, World!\"), \"hello-world\")\n\n"
                    "    def test_collapses_spaces(self):\n"
                    "        self.assertEqual(slugify(\"multi   space\"), \"multi-space\")\n\n"
                    "    def test_preserves_existing_slug_shape(self):\n"
                    "        self.assertEqual(slugify(\"Already-Slug\"), \"already-slug\")\n\n"
                    "if __name__ == \"__main__\":\n"
                    "    unittest.main()\n"
                ),
            },
            "expected_paths": ["slug_tools.py", "test_slug_tools.py"],
            "prompt": lambda root: (
                f"Project root: {root}\n"
                "This is an existing Python project with failing tests. First run the local "
                "unittest validation to observe the failure, then inspect the relevant files, "
                "repair the slugify implementation, and rerun validation until the tests pass."
            ),
        }
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


def evaluate_tier2(case: dict) -> dict:
    root = Path(tempfile.mkdtemp(prefix=f"infring-{case['name']}-root-"))
    prompt = case["prompt"](root)
    run, _ = run_infring(prompt, case["name"])
    response = run["response"] or {}
    receipts = response.get("receipt", {}).get("native_tool_receipts", [])
    expected_files_exist = all((root / path).exists() for path in case["expected_paths"])
    result = {
        "case": case["name"],
        "tier": 2,
        "lane": response.get("contract", {}).get("execution_shape", {}).get("lane"),
        "execution_lane": response.get("contract", {})
        .get("execution_shape", {})
        .get("execution_lane"),
        "provider": response.get("contract", {}).get("provider"),
        "planner_provider": response.get("receipt", {}).get("planner_provider"),
        "provider_start_ms": response.get("trace_summary", {})
        .get("phase_latency_ms", {})
        .get("provider_start"),
        "model_call_ms": response.get("trace_summary", {})
        .get("phase_latency_ms", {})
        .get("model_call"),
        "validation_status": response.get("receipt", {}).get("validation_status"),
        "tool_names": [receipt.get("tool_name") for receipt in receipts],
        "expected_files_exist": expected_files_exist,
        "elapsed_ms": run["elapsed_ms"],
        "root": str(root),
    }
    result["ok"] = (
        response.get("ok") is True
        and result["lane"] == "model_manifest_planner"
        and result["execution_lane"] == "deterministic_local_loop"
        and result["provider"] is None
        and result["planner_provider"] is not None
        and result["provider_start_ms"] == 0
        and isinstance(result["model_call_ms"], int)
        and result["model_call_ms"] > 0
        and result["tool_names"].count("file_write") >= len(case["expected_paths"])
        and "command_run" in result["tool_names"]
        and result["validation_status"] == "passed"
        and expected_files_exist
    )
    return result


def evaluate_tier3(case: dict) -> dict:
    root = Path(tempfile.mkdtemp(prefix=f"infring-{case['name']}-root-"))
    for relative_path, content in case["initial_files"].items():
        target = root / relative_path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")

    prompt = case["prompt"](root)
    run, _ = run_infring(prompt, case["name"])
    response = run["response"] or {}
    receipts = response.get("receipt", {}).get("native_tool_receipts", [])
    output = response.get("output", "")
    tool_names = [receipt.get("tool_name") for receipt in receipts]
    expected_files_exist = all((root / path).exists() for path in case["expected_paths"])
    content_markers_present = all(
        marker in "\n".join(
            (root / path).read_text(encoding="utf-8")
            for path in case["expected_paths"]
            if (root / path).exists()
        )
        for marker in case["expected_content_markers"]
    )
    lower_output = output.lower()
    false_receipt_blocker = any(
        marker in lower_output
        for marker in [
            "receipt-backed evidence unavailable",
            "receipts are not present",
            "none confirmed",
            "pending tool receipts",
            "no command_run receipt surfaced",
        ]
    )
    result = {
        "case": case["name"],
        "tier": 3,
        "provider": response.get("contract", {}).get("provider"),
        "workflow_native_success_criteria": response.get("contract", {})
        .get("workflow", {})
        .get("native_success_criteria"),
        "tool_names": tool_names,
        "has_read_before_mutation": any(
            name in ["file_read", "file_read_many"] for name in tool_names
        )
        and any(name in ["file_write", "file_patch"] for name in tool_names)
        and min(
            idx
            for idx, name in enumerate(tool_names)
            if name in ["file_read", "file_read_many"]
        )
        < min(
            idx
            for idx, name in enumerate(tool_names)
            if name in ["file_write", "file_patch"]
        ),
        "has_mutation": any(name in ["file_write", "file_patch"] for name in tool_names),
        "has_validation": "command_run" in tool_names,
        "validation_status": response.get("receipt", {}).get("status"),
        "expected_files_exist": expected_files_exist,
        "content_markers_present": content_markers_present,
        "false_receipt_blocker": false_receipt_blocker,
        "elapsed_ms": run["elapsed_ms"],
        "root": str(root),
    }
    result["ok"] = (
        response.get("ok") is True
        and result["provider"] is not None
        and (result["workflow_native_success_criteria"] or {}).get(
            "synthesize_final_after_successful_validation"
        )
        is True
        and result["has_read_before_mutation"]
        and result["has_mutation"]
        and result["has_validation"]
        and result["validation_status"] == "ok"
        and expected_files_exist
        and content_markers_present
        and not false_receipt_blocker
    )
    return result


def evaluate_tier4(case: dict) -> dict:
    root = Path(tempfile.mkdtemp(prefix=f"infring-{case['name']}-root-"))
    for relative_path, content in case["initial_files"].items():
        target = root / relative_path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")

    prompt = case["prompt"](root)
    run, _ = run_infring(prompt, case["name"])
    response = run["response"] or {}
    receipts = response.get("receipt", {}).get("native_tool_receipts", [])
    output = response.get("output", "")
    tool_names = [receipt.get("tool_name") for receipt in receipts]
    mutation_indices = [
        idx
        for idx, receipt in enumerate(receipts)
        if receipt.get("status") == "ok"
        and receipt.get("tool_name") in ["file_write", "file_patch"]
    ]
    command_results = [
        (idx, receipt.get("result", {}).get("success"))
        for idx, receipt in enumerate(receipts)
        if receipt.get("tool_name") == "command_run"
    ]
    first_mutation_idx = min(mutation_indices) if mutation_indices else None
    has_failed_validation_before_mutation = (
        first_mutation_idx is not None
        and any(idx < first_mutation_idx and success is False for idx, success in command_results)
    )
    has_successful_validation_after_mutation = (
        first_mutation_idx is not None
        and any(idx > first_mutation_idx and success is True for idx, success in command_results)
    )
    expected_files_exist = all((root / path).exists() for path in case["expected_paths"])
    lower_output = output.lower()
    false_receipt_blocker = any(
        marker in lower_output
        for marker in [
            "receipt-backed evidence unavailable",
            "receipts are not present",
            "none confirmed",
            "pending tool receipts",
            "no command_run receipt surfaced",
        ]
    )
    result = {
        "case": case["name"],
        "tier": 4,
        "provider": response.get("contract", {}).get("provider"),
        "workflow_native_success_criteria": response.get("contract", {})
        .get("workflow", {})
        .get("native_success_criteria"),
        "tool_names": tool_names,
        "has_failed_validation_before_mutation": has_failed_validation_before_mutation,
        "has_mutation": bool(mutation_indices),
        "has_successful_validation_after_mutation": has_successful_validation_after_mutation,
        "expected_files_exist": expected_files_exist,
        "false_receipt_blocker": false_receipt_blocker,
        "elapsed_ms": run["elapsed_ms"],
        "root": str(root),
    }
    result["ok"] = (
        response.get("ok") is True
        and result["provider"] is not None
        and (result["workflow_native_success_criteria"] or {}).get(
            "requires_successful_mutation_receipt"
        )
        is True
        and (result["workflow_native_success_criteria"] or {}).get(
            "repair_uncovered_requirements_before_final"
        )
        is True
        and has_failed_validation_before_mutation
        and bool(mutation_indices)
        and has_successful_validation_after_mutation
        and expected_files_exist
        and not false_receipt_blocker
    )
    return result


def main() -> int:
    results = []
    for case in tier0_cases():
        results.append(evaluate_tier0(case))
    for case in tier1_cases():
        results.append(evaluate_tier1(case))
    for case in tier2_cases():
        results.append(evaluate_tier2(case))
    for case in tier3_cases():
        results.append(evaluate_tier3(case))
    for case in tier4_cases():
        results.append(evaluate_tier4(case))

    payload = {
        "schema_version": "coding_tiered_runtime_topology_eval_v4",
        "attempts": len(results),
        "passes": sum(1 for result in results if result["ok"]),
        "results": results,
        "acceptance": {
            "tier0_requires_lane": "direct_mutation",
            "tier1_requires_lane": "deterministic_local_loop",
            "tier2_requires_lane": "model_manifest_planner",
            "tier2_requires_execution_lane": "deterministic_local_loop",
            "tier3_requires_native_existing_project_loop": True,
            "tier4_requires_validation_guided_repair_loop": True,
            "tier0_tier1_provider_start_ms": 0,
            "tier0_tier1_model_call_ms": 0,
            "tier2_provider_start_ms": 0,
            "tier2_model_call_ms": "greater_than_zero",
            "tier3_requires_read_before_mutation": True,
            "tier3_forbids_false_receipt_blocker": True,
            "tier4_requires_failed_validation_before_mutation": True,
            "tier4_requires_successful_validation_after_mutation": True,
        },
    }
    print(json.dumps(payload, indent=2))
    return 0 if payload["passes"] == payload["attempts"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
