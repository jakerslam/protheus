#!/usr/bin/env node
'use strict';
export {};

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const OBS_MANIFEST = path.join(ROOT, 'crates', 'observability', 'Cargo.toml');

let cachedWasmBinding: any = null;
let cachedWasmPath = '';
let cachedWasmErr = '';

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

function wasmCandidates() {
  const explicit = cleanText(process.env.PROTHEUS_OBSERVABILITY_WASM_BINDING_PATH || '', 500);
  const out = [
    explicit,
    path.join(ROOT, 'crates', 'observability', 'pkg', 'protheus_observability_core_v1.js'),
    path.join(ROOT, 'crates', 'observability', 'pkg-node', 'protheus_observability_core_v1.js')
  ].filter(Boolean);
  return Array.from(new Set(out));
}

function loadWasmBindgenBridge() {
  if (cachedWasmBinding) {
    return { ok: true, binding: cachedWasmBinding, module_path: cachedWasmPath };
  }
  if (cachedWasmErr) {
    return { ok: false, error: cachedWasmErr };
  }

  const candidates = wasmCandidates();
  const errs: string[] = [];
  for (const candidate of candidates) {
    try {
      if (!fs.existsSync(candidate)) {
        errs.push(`missing:${candidate}`);
        continue;
      }
      // eslint-disable-next-line import/no-dynamic-require, global-require
      const mod = require(candidate);
      const runChaos = mod && (mod.run_chaos_resilience_wasm || mod.runChaosResilienceWasm);
      const loadProfile = mod && (mod.load_embedded_observability_profile_wasm || mod.loadEmbeddedObservabilityProfileWasm);
      if (typeof runChaos !== 'function' || typeof loadProfile !== 'function') {
        errs.push(`invalid_exports:${candidate}`);
        continue;
      }
      cachedWasmBinding = { runChaos, loadProfile };
      cachedWasmPath = candidate;
      cachedWasmErr = '';
      return { ok: true, binding: cachedWasmBinding, module_path: candidate };
    } catch (err) {
      errs.push(`load_failed:${candidate}:${cleanText(err && (err as any).message, 120)}`);
    }
  }

  cachedWasmErr = errs.length ? errs[0] : 'wasm_bindgen_bridge_unavailable';
  return { ok: false, error: cachedWasmErr };
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

function runLoadViaWasm() {
  const bridge = loadWasmBindgenBridge();
  if (!bridge.ok || !bridge.binding || typeof bridge.binding.loadProfile !== 'function') {
    return { ok: false, error: bridge.error || 'wasm_bindgen_bridge_unavailable' };
  }
  try {
    const raw = bridge.binding.loadProfile();
    const payload = parseJsonPayload(raw);
    if (!payload || typeof payload !== 'object') {
      return { ok: false, error: 'wasm_bindgen_invalid_payload' };
    }
    return { ok: true, engine: 'rust_wasm_bindgen', module_path: bridge.module_path, payload };
  } catch (err) {
    return { ok: false, error: `wasm_bindgen_call_failed:${cleanText(err && (err as any).message, 160)}` };
  }
}

function runChaosViaWasm(requestJson: string) {
  const bridge = loadWasmBindgenBridge();
  if (!bridge.ok || !bridge.binding || typeof bridge.binding.runChaos !== 'function') {
    return { ok: false, error: bridge.error || 'wasm_bindgen_bridge_unavailable' };
  }
  try {
    const raw = bridge.binding.runChaos(String(requestJson || '{}'));
    const payload = parseJsonPayload(raw);
    if (!payload || typeof payload !== 'object') {
      return { ok: false, error: 'wasm_bindgen_invalid_payload' };
    }
    return { ok: true, engine: 'rust_wasm_bindgen', module_path: bridge.module_path, payload };
  } catch (err) {
    return { ok: false, error: `wasm_bindgen_call_failed:${cleanText(err && (err as any).message, 160)}` };
  }
}

function loadEmbeddedObservabilityProfile(opts: AnyObj = {}) {
  const preferWasm = opts.prefer_wasm !== false;
  const allowCliFallback = opts.allow_cli_fallback !== false;

  if (preferWasm) {
    const wasmResult = runLoadViaWasm();
    if (wasmResult.ok) return wasmResult;
    if (!allowCliFallback) return wasmResult;
  }

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

  const preferWasm = opts.prefer_wasm !== false;
  const allowCliFallback = opts.allow_cli_fallback !== false;

  if (preferWasm) {
    const wasmResult = runChaosViaWasm(requestJson);
    if (wasmResult.ok) return wasmResult;
    if (!allowCliFallback) return wasmResult;
  }

  const binResult = runViaRustBinary('run-chaos', [`--request-base64=${requestBase64}`]);
  if (binResult.ok) return binResult;

  if (!allowCliFallback) return binResult;
  return runViaCargo('run-chaos', [`--request-base64=${requestBase64}`]);
}

module.exports = {
  loadEmbeddedObservabilityProfile,
  runChaosObservability,
  loadWasmBindgenBridge,
  runViaRustBinary,
  runViaCargo
};
