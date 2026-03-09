#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: core/layer2/ops::spawn_broker (authoritative)
// Thin compatibility shim kept for older imports; all authority executes in Rust via spawn_broker.js.
const path = require('path');
const { spawnSync } = require('child_process');

const SCRIPT = path.join(__dirname, 'spawn_broker.js');

function parseJson(stdout: unknown) {
  const text = String(stdout || '').trim();
  if (!text) return null;
  try { return JSON.parse(text); } catch {}
  const lines = text.split('\n').map((line) => String(line || '').trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try { return JSON.parse(lines[i]); } catch {}
  }
  return null;
}

function run(args: string[] = []) {
  const proc = spawnSync(process.execPath, [SCRIPT, ...(Array.isArray(args) ? args : [])], {
    cwd: path.resolve(__dirname, '..', '..'),
    encoding: 'utf8',
    env: process.env
  });
  return {
    status: Number.isFinite(proc.status) ? Number(proc.status) : 1,
    stdout: String(proc.stdout || ''),
    stderr: String(proc.stderr || ''),
    payload: parseJson(proc.stdout)
  };
}

function loadPolicy() {
  const out = run(['status']);
  return out.payload && out.payload.policy ? out.payload.policy : {};
}

function loadState() {
  const out = run(['status']);
  return out.payload && out.payload.state ? out.payload.state : {};
}

function computeLimits() {
  const out = run(['status']);
  return out.payload && out.payload.limits ? out.payload.limits : {};
}

function hardwareBounds() {
  const out = run(['status']);
  return out.payload && out.payload.hardware_bounds ? out.payload.hardware_bounds : {};
}

function main() {
  const out = run(process.argv.slice(2));
  if (out.stdout) process.stdout.write(out.stdout);
  if (out.stderr) process.stderr.write(out.stderr);
  process.exit(out.status);
}

if (require.main === module) {
  main();
}

module.exports = {
  run,
  loadPolicy,
  loadState,
  computeLimits,
  hardwareBounds
};

