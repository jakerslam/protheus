#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { parseStrictOutArgs, readFlag, cleanText } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT = 'core/local/artifacts/nexus_module_file_checklist_current.json';

const REQUIRED_FILES = [
  'core/layer2/nexus/src/lib.rs',
  'core/layer2/nexus/src/policy.rs',
  'core/layer2/nexus/src/registry.rs',
  'core/layer2/nexus/src/main_nexus.rs',
  'core/layer2/nexus/src/sub_nexus.rs',
  'core/layer2/nexus/src/conduit_manager.rs',
  'core/layer2/nexus/src/route_lease.rs',
  'core/layer2/nexus/src/template.rs',
];

function main() {
  const common = parseStrictOutArgs(process.argv.slice(2), {});
  const out = cleanText(readFlag(process.argv.slice(2), 'out') || DEFAULT_OUT, 400);
  const missing = REQUIRED_FILES.filter((row) => !fs.existsSync(path.resolve(ROOT, row)));

  const payload = {
    type: 'nexus_module_file_checklist',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    summary: {
      required_count: REQUIRED_FILES.length,
      missing_count: missing.length,
      pass: missing.length === 0,
    },
    required_files: REQUIRED_FILES,
    missing_files: missing,
  };

  process.exit(
    emitStructuredResult(payload, {
      outPath: out,
      strict: common.strict,
      ok: payload.summary.pass,
    }),
  );
}

main();
