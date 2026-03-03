#!/usr/bin/env node
'use strict';
export {};

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const MANIFEST = path.join(ROOT, 'crates', 'singularity_seed', 'Cargo.toml');

type AnyObj = Record<string, any>;

function cleanText(v: unknown, maxLen = 260) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function parseJsonPayload(raw: unknown) {
  const text = String(raw == null ? '' : raw).trim();
  if (!text) return null;
  try { return JSON.parse(text); } catch {}
  const lines = text.split('\n').map((line) => line.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try { return JSON.parse(lines[i]); } catch {}
  }
  return null;
}

function binaryCandidates() {
  const explicit = cleanText(process.env.PROTHEUS_SINGULARITY_SEED_BIN || '', 500);
  const out = [
    explicit,
    path.join(ROOT, 'target', 'release', 'singularity_seed_core'),
    path.join(ROOT, 'target', 'debug', 'singularity_seed_core'),
    path.join(ROOT, 'crates', 'singularity_seed', 'target', 'release', 'singularity_seed_core'),
    path.join(ROOT, 'crates', 'singularity_seed', 'target', 'debug', 'singularity_seed_core')
  ].filter(Boolean);
  return Array.from(new Set(out));
}

function runViaRustBinary(command: string, extraArgs: string[] = []) {
  for (const candidate of binaryCandidates()) {
    try {
      if (!fs.existsSync(candidate)) continue;
      const out = spawnSync(candidate, [command, ...extraArgs], {
        cwd: ROOT,
        encoding: 'utf8',
        maxBuffer: 10 * 1024 * 1024
      });
      const payload = parseJsonPayload(out.stdout);
      if (out.status === 0 && payload && typeof payload === 'object') {
        return { ok: true, engine: 'rust_bin', binary_path: candidate, payload };
      }
    } catch {
      // continue
    }
  }
  return { ok: false, error: 'rust_binary_unavailable' };
}

function runViaCargo(command: string, extraArgs: string[] = []) {
  const args = [
    'run',
    '--quiet',
    '--manifest-path',
    MANIFEST,
    '--bin',
    'singularity_seed_core',
    '--',
    command,
    ...extraArgs
  ];
  const out = spawnSync('cargo', args, {
    cwd: ROOT,
    encoding: 'utf8',
    maxBuffer: 10 * 1024 * 1024
  });
  const payload = parseJsonPayload(out.stdout);
  if (Number(out.status) === 0 && payload && typeof payload === 'object') {
    return { ok: true, engine: 'rust_cargo', payload };
  }
  return {
    ok: false,
    error: `cargo_run_failed:${cleanText(out.stderr || out.stdout || '', 260)}`
  };
}

function runCommand(command: string, opts: AnyObj = {}) {
  const request = opts.request && typeof opts.request === 'object' ? opts.request : null;
  const allowCliFallback = opts.allow_cli_fallback !== false;

  const extraArgs: string[] = [];
  if (request) {
    const base64 = Buffer.from(JSON.stringify(request), 'utf8').toString('base64');
    extraArgs.push(`--request-base64=${base64}`);
  }

  const binResult = runViaRustBinary(command, extraArgs);
  if (binResult.ok) return binResult;

  if (!allowCliFallback) return binResult;
  return runViaCargo(command, extraArgs);
}

function freezeSeedLoops(opts: AnyObj = {}) {
  return runCommand('freeze', opts);
}

function runSingularitySeedCycle(opts: AnyObj = {}) {
  const request = opts.request && typeof opts.request === 'object' ? opts.request : {};
  return runCommand('cycle', { ...opts, request });
}

function showSingularitySeedState(opts: AnyObj = {}) {
  return runCommand('show', opts);
}

module.exports = {
  freezeSeedLoops,
  runSingularitySeedCycle,
  showSingularitySeedState,
  runViaRustBinary,
  runViaCargo
};
