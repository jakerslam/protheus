#!/usr/bin/env node
'use strict';
export {};

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const OBS_MANIFEST = path.join(ROOT, 'crates', 'observability', 'Cargo.toml');

type AnyObj = Record<string, any>;

function cleanText(v: unknown, maxLen = 240) {
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

function normalizeChaosPayloadLegacyCompat(payload: AnyObj) {
  if (!payload || typeof payload !== 'object') return payload;
  const sovereignty = payload.sovereignty && typeof payload.sovereignty === 'object'
    ? payload.sovereignty
    : {};
  const telemetry = Number(payload.telemetry_overhead_ms || 0);
  const battery = Number(payload.chaos_battery_pct_24h || 0);
  const failClosed = Boolean((sovereignty as AnyObj).fail_closed);
  // Preserve legacy compatibility contract during Rust authority cutover.
  const resilient = failClosed !== true && telemetry <= 1.0 && battery <= 3.0;
  return {
    ...payload,
    resilient
  };
}

function binaryCandidates() {
  const explicit = cleanText(process.env.PROTHEUS_OBSERVABILITY_RUST_BIN || '', 500);
  const out = [
    explicit,
    path.join(ROOT, 'target', 'release', 'observability_core'),
    path.join(ROOT, 'target', 'debug', 'observability_core'),
    path.join(ROOT, 'crates', 'observability', 'target', 'release', 'observability_core'),
    path.join(ROOT, 'crates', 'observability', 'target', 'debug', 'observability_core')
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
      if (Number(out.status) === 0 && payload && typeof payload === 'object') {
        return { ok: true, engine: 'rust_bin', binary_path: candidate, payload };
      }
    } catch {
      // continue fallback scan
    }
  }
  return { ok: false, error: 'rust_binary_unavailable' };
}

function runViaCargo(command: string, extraArgs: string[] = []) {
  const args = [
    'run',
    '--quiet',
    '--manifest-path',
    OBS_MANIFEST,
    '--bin',
    'observability_core',
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
    error: `cargo_run_failed:${cleanText(out.stderr || out.stdout || '', 240)}`
  };
}

function loadWasmBindgenBridge() {
  return {
    ok: false,
    error: 'observability_wasm_bridge_disabled_use_rust_core'
  };
}

function loadEmbeddedObservabilityProfile(opts: AnyObj = {}) {
  const allowCliFallback = opts.allow_cli_fallback !== false;
  const binResult = runViaRustBinary('load-profile');
  if (binResult.ok) return binResult;
  if (!allowCliFallback) return binResult;
  return runViaCargo('load-profile');
}

function runChaosObservability(request: unknown, opts: AnyObj = {}) {
  const requestJson = typeof request === 'string'
    ? request
    : JSON.stringify(request && typeof request === 'object' ? request : {});
  const requestBase64 = Buffer.from(String(requestJson || '{}'), 'utf8').toString('base64');
  const allowCliFallback = opts.allow_cli_fallback !== false;

  const binResult = runViaRustBinary('run-chaos', [`--request-base64=${requestBase64}`]);
  if (binResult.ok) {
    return {
      ...binResult,
      payload: normalizeChaosPayloadLegacyCompat(binResult.payload as AnyObj)
    };
  }
  if (!allowCliFallback) return binResult;
  const cargoResult = runViaCargo('run-chaos', [`--request-base64=${requestBase64}`]);
  if (!cargoResult.ok) return cargoResult;
  return {
    ...cargoResult,
    payload: normalizeChaosPayloadLegacyCompat(cargoResult.payload as AnyObj)
  };
}

module.exports = {
  loadEmbeddedObservabilityProfile,
  runChaosObservability,
  loadWasmBindgenBridge,
  runViaRustBinary,
  runViaCargo
};
