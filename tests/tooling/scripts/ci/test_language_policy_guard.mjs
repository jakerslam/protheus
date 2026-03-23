#!/usr/bin/env node
/* eslint-disable no-console */
import { execSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const LEGACY_JS_TEST_ROOT = 'tests/client-memory-tools/';
const MAX_LEGACY_JS_TEST_FILES = 1102;
const REPORT_PATH = resolve(
  'core/local/artifacts/test_language_policy_guard_current.json',
);

function listJsTestFiles() {
  try {
    const raw = execSync("rg --files tests -g '*.test.js'", { encoding: 'utf8' });
    return raw
      .split('\n')
      .map((line) => line.trim())
      .filter(Boolean);
  } catch {
    return [];
  }
}

function main() {
  const files = listJsTestFiles();
  const legacy = files.filter((entry) => entry.startsWith(LEGACY_JS_TEST_ROOT));
  const nonLegacy = files.filter((entry) => !entry.startsWith(LEGACY_JS_TEST_ROOT));

  const checks = [];
  if (nonLegacy.length > 0) {
    checks.push({
      check: 'no_non_legacy_js_tests',
      ok: false,
      detail:
        'JS tests outside tests/client-memory-tools are blocked. Use .ts for UI tests and Rust for core/integration tests.',
      offenders: nonLegacy.slice(0, 50),
    });
  } else {
    checks.push({ check: 'no_non_legacy_js_tests', ok: true });
  }

  if (legacy.length > MAX_LEGACY_JS_TEST_FILES) {
    checks.push({
      check: 'legacy_js_test_budget',
      ok: false,
      detail: `legacy JS test budget exceeded (${legacy.length} > ${MAX_LEGACY_JS_TEST_FILES})`,
      offenders: legacy.slice(0, 50),
    });
  } else {
    checks.push({ check: 'legacy_js_test_budget', ok: true });
  }

  const failed = checks.filter((row) => !row.ok);
  const payload = {
    ok: failed.length === 0,
    type: 'test_language_policy_guard',
    totals: {
      js_tests: files.length,
      legacy_js_tests: legacy.length,
      non_legacy_js_tests: nonLegacy.length,
      legacy_budget: MAX_LEGACY_JS_TEST_FILES,
    },
    checks,
  };

  writeFileSync(REPORT_PATH, JSON.stringify(payload, null, 2));
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
  process.exit(payload.ok ? 0 : 1);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}

