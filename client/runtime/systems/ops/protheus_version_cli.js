#!/usr/bin/env node
'use strict';

// Compatibility shim for route maps that still point at protheus_version_cli.js.
// Delegates to the TS lane when available.

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const TS_ENTRY = path.join(ROOT, 'client', 'runtime', 'lib', 'ts_entrypoint.ts');
const TS_TARGET = path.join(__dirname, 'protheus_version_cli.ts');
const PACKAGE_JSON_PATH = path.join(ROOT, 'package.json');
const INSTALL_COMMAND =
  'curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --full';

function cleanText(raw, maxLen = 160) {
  return String(raw || '')
    .trim()
    .replace(/\s+/g, ' ')
    .slice(0, maxLen);
}

function readVersion() {
  try {
    const parsed = JSON.parse(fs.readFileSync(PACKAGE_JSON_PATH, 'utf8'));
    const version = cleanText(parsed && parsed.version ? parsed.version : '', 80).replace(/^v/i, '');
    return version || '0.0.0-unknown';
  } catch (_) {
    return '0.0.0-unknown';
  }
}

function main() {
  if (fs.existsSync(TS_ENTRY) && fs.existsSync(TS_TARGET)) {
    const proc = spawnSync(process.execPath, [TS_ENTRY, TS_TARGET, ...process.argv.slice(2)], {
      stdio: 'inherit',
      cwd: ROOT
    });
    process.exit(Number.isFinite(proc.status) ? proc.status : 1);
  }

  const args = process.argv.slice(2).map((token) => cleanText(token, 160)).filter(Boolean);
  const command =
    args.length > 0 && !args[0].startsWith('--') ? cleanText(args[0], 40).toLowerCase() : 'version';
  const jsonMode = args.includes('--json') || args.includes('--json=1');
  const currentVersion = readVersion();
  const payload = {
    ok: true,
    type: 'protheus_version_cli_fallback',
    command,
    current_version: currentVersion,
    latest_version: currentVersion,
    update_available: false,
    source: 'js_compat_fallback'
  };

  if (jsonMode) {
    process.stdout.write(`${JSON.stringify(payload)}\n`);
    return;
  }

  if (command === 'check-quiet') {
    return;
  }
  if (command === 'update') {
    process.stdout.write(`[infring update] release check unavailable in JS fallback lane\n`);
    process.stdout.write(`[infring update] current version: ${currentVersion}\n`);
    process.stdout.write(`[infring update] install: ${INSTALL_COMMAND}\n`);
    return;
  }
  process.stdout.write(`infring ${currentVersion}\n`);
}

if (require.main === module) {
  main();
}

module.exports = { main };
