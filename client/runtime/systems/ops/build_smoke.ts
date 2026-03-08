#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..');

const REQUIRED_DIST_FILES = [
  'lib/directive_resolver.js',
  'systems/security/directive_gate.js',
  'systems/sensory/focus_controller.js',
  'systems/autonomy/self_documentation_closeout.js',
  'systems/budget/system_budget.js'
];

const DIST_LAYOUT_ROOTS = [
  path.join(ROOT, 'dist'),
  path.resolve(ROOT, '..', 'dist', 'client')
];

function resolveDistPath(rel) {
  for (const layoutRoot of DIST_LAYOUT_ROOTS) {
    const abs = path.join(layoutRoot, rel);
    if (fs.existsSync(abs)) {
      return { ok: true, abs, rel: path.relative(ROOT, abs).replace(/\\/g, '/') };
    }
  }
  return {
    ok: false,
    rel,
    attempted: DIST_LAYOUT_ROOTS.map((layoutRoot) => path.relative(ROOT, path.join(layoutRoot, rel)).replace(/\\/g, '/'))
  };
}

function main() {
  for (const rel of REQUIRED_DIST_FILES) {
    const resolved = resolveDistPath(rel);
    if (!resolved.ok) {
      throw new Error(`missing_dist_file:${rel}:attempted=${(resolved.attempted || []).join(',')}`);
    }
    const abs = resolved.abs;
    const check = spawnSync(process.execPath, ['--check', abs], {
      cwd: ROOT,
      encoding: 'utf8'
    });
    if (check.status !== 0) {
      const detail = String(check.stderr || check.stdout || '').trim();
      throw new Error(`syntax_check_failed:${resolved.rel}:${detail}`);
    }
  }

  process.stdout.write(JSON.stringify({
    ok: true,
    type: 'build_smoke',
    checked: REQUIRED_DIST_FILES.length,
    mode: 'emit_and_syntax'
  }) + '\n');
}

try {
  if (require.main === module) {
    main();
  }
} catch (err) {
  process.stderr.write(`build_smoke.js: FAIL: ${err.message}\n`);
  process.exit(1);
}
