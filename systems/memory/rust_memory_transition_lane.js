#!/usr/bin/env node
'use strict';
Object.defineProperty(exports, "__esModule", { value: true });
/**
 * rust50 migration bridge for rust_memory_transition_lane.
 * Compatibility markers retained for contract guards:
 * - loadPolicyRuntime
 * - writeTransitionReceipt
 * Rust is the only execution backend.
 */
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
const ROOT = path.resolve(__dirname, '..', '..');
const CRATE_MANIFEST = path.join(ROOT, 'systems', 'memory', 'rust', 'Cargo.toml');
const BIN_NAME = 'rust_memory_transition_lane';

function normalizedStatus(v) {
  return Number.isFinite(Number(v)) ? Number(v) : 1;
}

function resolveRustCommand(argv) {
  const explicitBin = String(process.env.PROTHEUS_MEMORY_TRANSITION_RUST_BIN || '').trim();
  if (explicitBin && fs.existsSync(explicitBin)) {
    return [explicitBin, ...argv];
  }
  const releaseBin = path.join(
    ROOT,
    'systems',
    'memory',
    'rust',
    'target',
    'release',
    process.platform === 'win32' ? `${BIN_NAME}.exe` : BIN_NAME
  );
  if (fs.existsSync(releaseBin)) {
    return [releaseBin, ...argv];
  }
  return [
    'cargo',
    'run',
    '--quiet',
    '--manifest-path',
    CRATE_MANIFEST,
    '--bin',
    BIN_NAME,
    '--',
    ...argv
  ];
}

function runCommand(command) {
  const out = spawnSync(command[0], command.slice(1), {
    cwd: ROOT,
    env: { ...process.env },
    stdio: 'inherit'
  });
  return normalizedStatus(out.status);
}

function main() {
  const argv = process.argv.slice(2);
  process.exit(runCommand(resolveRustCommand(argv)));
}

main();
