#!/usr/bin/env node
'use strict';
export {};

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const EXECUTION_MANIFEST = path.join(ROOT, 'crates', 'execution', 'Cargo.toml');

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
  const explicit = cleanText(process.env.PROTHEUS_EXECUTION_WASM_BINDING_PATH || '', 500);
  const out = [
    explicit,
    path.join(ROOT, 'crates', 'execution', 'pkg', 'execution_core.js'),
    path.join(ROOT, 'crates', 'execution', 'pkg-node', 'execution_core.js')
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
      const fn = mod && (mod.run_workflow_wasm || mod.runWorkflowWasm || mod.run_workflow_json || mod.default);
      if (typeof fn !== 'function') {
        errs.push(`invalid_exports:${candidate}`);
        continue;
      }
      cachedWasmBinding = fn;
      cachedWasmPath = candidate;
      cachedWasmErr = '';
      return { ok: true, binding: fn, module_path: candidate };
    } catch (err) {
      errs.push(`load_failed:${candidate}:${cleanText(err && (err as any).message, 100)}`);
    }
  }

  cachedWasmErr = errs.length ? errs[0] : 'wasm_bindgen_bridge_unavailable';
  return { ok: false, error: cachedWasmErr };
}

function binaryCandidates() {
  const explicit = cleanText(process.env.PROTHEUS_EXECUTION_RUST_BIN || '', 500);
  const out = [
    explicit,
    path.join(ROOT, 'target', 'release', 'execution_core'),
    path.join(ROOT, 'target', 'debug', 'execution_core'),
    path.join(ROOT, 'crates', 'execution', 'target', 'release', 'execution_core'),
    path.join(ROOT, 'crates', 'execution', 'target', 'debug', 'execution_core')
  ].filter(Boolean);
  return Array.from(new Set(out));
}

function runViaRustBinary(yaml: string) {
  const encoded = Buffer.from(String(yaml || ''), 'utf8').toString('base64');
  for (const candidate of binaryCandidates()) {
    try {
      if (!fs.existsSync(candidate)) continue;
      const out = spawnSync(candidate, ['run', `--yaml-base64=${encoded}`], {
        cwd: ROOT,
        encoding: 'utf8',
        maxBuffer: 10 * 1024 * 1024
      });
      const payload = parseJsonPayload(out.stdout);
      if (out.status === 0 && payload && typeof payload === 'object') {
        return { ok: true, engine: 'rust_bin', binary_path: candidate, payload };
      }
    } catch {
      // keep trying next candidate
    }
  }
  return { ok: false, error: 'rust_binary_unavailable' };
}

function runViaCargo(yaml: string) {
  const encoded = Buffer.from(String(yaml || ''), 'utf8').toString('base64');
  const args = [
    'run',
    '--quiet',
    '--manifest-path',
    EXECUTION_MANIFEST,
    '--bin',
    'execution_core',
    '--',
    'run',
    `--yaml-base64=${encoded}`
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
    error: `cargo_run_failed:${cleanText(out.stderr || out.stdout || '', 200)}`
  };
}

function runViaWasm(yaml: string) {
  const bridge = loadWasmBindgenBridge();
  if (!bridge.ok || typeof bridge.binding !== 'function') {
    return { ok: false, error: bridge.error || 'wasm_bindgen_bridge_unavailable' };
  }
  try {
    const raw = bridge.binding(String(yaml || ''));
    const payload = parseJsonPayload(raw);
    if (!payload || typeof payload !== 'object') {
      return { ok: false, error: 'wasm_bindgen_invalid_payload' };
    }
    return {
      ok: true,
      engine: 'rust_wasm_bindgen',
      module_path: bridge.module_path,
      payload
    };
  } catch (err) {
    return {
      ok: false,
      error: `wasm_bindgen_call_failed:${cleanText(err && (err as any).message, 160)}`
    };
  }
}

function runWorkflow(yamlOrSpec: unknown, opts: AnyObj = {}) {
  const yaml = typeof yamlOrSpec === 'string'
    ? yamlOrSpec
    : JSON.stringify(yamlOrSpec && typeof yamlOrSpec === 'object' ? yamlOrSpec : {});

  const preferWasm = opts.prefer_wasm !== false;
  const allowCliFallback = opts.allow_cli_fallback !== false;

  if (preferWasm) {
    const wasmResult = runViaWasm(yaml);
    if (wasmResult.ok) return wasmResult;
    if (!allowCliFallback) return wasmResult;
  }

  const binResult = runViaRustBinary(yaml);
  if (binResult.ok) return binResult;

  if (!allowCliFallback) return binResult;
  return runViaCargo(yaml);
}

module.exports = {
  runWorkflow,
  loadWasmBindgenBridge,
  runViaWasm,
  runViaRustBinary,
  runViaCargo
};
