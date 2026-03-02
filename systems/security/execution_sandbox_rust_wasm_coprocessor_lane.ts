#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-186
 * Optional Rust/WASM sandbox coprocessor lane with policy-gated parity checks.
 */

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
const {
  ROOT,
  nowIso,
  cleanText,
  normalizeToken,
  toBool,
  readJson,
  writeJsonAtomic
} = require('../../lib/queued_backlog_runtime');
const { runStandardLane } = require('../../lib/upgrade_lane_runtime');

const POLICY_PATH = process.env.EXECUTION_SANDBOX_RUST_WASM_COPROCESSOR_LANE_POLICY_PATH
  ? path.resolve(process.env.EXECUTION_SANDBOX_RUST_WASM_COPROCESSOR_LANE_POLICY_PATH)
  : path.join(ROOT, 'config', 'execution_sandbox_rust_wasm_coprocessor_lane_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/security/execution_sandbox_rust_wasm_coprocessor_lane.js configure --owner=<owner_id>');
  console.log('  node systems/security/execution_sandbox_rust_wasm_coprocessor_lane.js verify --owner=<owner_id> [--mock=1] [--strict=1] [--apply=1]');
  console.log('  node systems/security/execution_sandbox_rust_wasm_coprocessor_lane.js status [--owner=<owner_id>]');
}

function parseJson(stdout) {
  const txt = String(stdout || '').trim();
  if (!txt) return null;
  try { return JSON.parse(txt); } catch {}
  const lines = txt.split('\n').map((line) => line.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try { return JSON.parse(lines[i]); } catch {}
  }
  return null;
}

function runNode(scriptPath, args, timeoutMs, mock, label) {
  if (mock) {
    return {
      ok: true,
      status: 0,
      payload: { ok: true, type: `${normalizeToken(label || 'mock', 80) || 'mock'}_mock` },
      stderr: ''
    };
  }
  const run = spawnSync(process.execPath, [scriptPath, ...args], {
    cwd: ROOT,
    encoding: 'utf8',
    timeout: timeoutMs
  });
  return {
    ok: Number(run.status || 0) === 0,
    status: Number.isFinite(run.status) ? Number(run.status) : 1,
    payload: parseJson(run.stdout || ''),
    stderr: cleanText(run.stderr || '', 400)
  };
}

function resolveMaybe(rawPath, fallbackRel) {
  const txt = cleanText(rawPath || '', 420);
  if (!txt) return path.join(ROOT, fallbackRel);
  return path.isAbsolute(txt) ? path.resolve(txt) : path.join(ROOT, txt);
}

function rel(absPath) {
  return path.relative(ROOT, absPath).replace(/\\/g, '/');
}

function readState(policy) {
  return readJson(policy.paths.coprocessor_state_path, {
    schema_id: 'execution_sandbox_rust_wasm_coprocessor_lane_state',
    schema_version: '1.0',
    runs: 0,
    updated_at: null,
    last_result: null
  });
}

function writeState(policy, state) {
  fs.mkdirSync(path.dirname(policy.paths.coprocessor_state_path), { recursive: true });
  writeJsonAtomic(policy.paths.coprocessor_state_path, {
    schema_id: 'execution_sandbox_rust_wasm_coprocessor_lane_state',
    schema_version: '1.0',
    runs: Number(state.runs || 0),
    updated_at: state.updated_at || nowIso(),
    last_result: state.last_result || null
  });
}

runStandardLane({
  lane_id: 'V3-RACE-186',
  script_rel: 'systems/security/execution_sandbox_rust_wasm_coprocessor_lane.js',
  policy_path: POLICY_PATH,
  stream: 'security.execution_sandbox_rust_wasm_coprocessor',
  paths: {
    memory_dir: 'memory/security/execution_sandbox_rust_wasm_coprocessor_lane',
    adaptive_index_path: 'adaptive/security/execution_sandbox_rust_wasm_coprocessor_lane/index.json',
    events_path: 'state/security/execution_sandbox_rust_wasm_coprocessor_lane/events.jsonl',
    latest_path: 'state/security/execution_sandbox_rust_wasm_coprocessor_lane/latest.json',
    receipts_path: 'state/security/execution_sandbox_rust_wasm_coprocessor_lane/receipts.jsonl',
    coprocessor_state_path: 'state/security/execution_sandbox_rust_wasm_coprocessor_lane/state.json'
  },
  usage,
  handlers: {
    verify(policy, args, ctx) {
      const ownerId = normalizeToken(args.owner || args.owner_id, 120);
      if (!ownerId) return { ok: false, error: 'missing_owner' };

      const strict = toBool(args.strict, true);
      const apply = toBool(args.apply, true);
      const mock = toBool(args.mock, false);
      const coprocessorEnabled = toBool(policy.enable_coprocessor, true);

      const sandboxScript = resolveMaybe(policy.sandbox_script, 'systems/security/execution_sandbox_envelope.js');
      const wasmScript = resolveMaybe(policy.wasm_runtime_script, 'systems/wasm/component_runtime.js');

      const jsWorkflow = runNode(
        sandboxScript,
        ['evaluate-workflow', '--step-id=rsi_probe', '--step-type=command', '--command=node systems/ops/protheusctl.js status'],
        120000,
        mock,
        'execution_sandbox_workflow_eval'
      );
      const jsActuation = runNode(
        sandboxScript,
        ['evaluate-actuation', '--kind=browser_automation', '--context={"risk_class":"shell"}'],
        120000,
        mock,
        'execution_sandbox_actuation_eval'
      );

      const wasmStatus = coprocessorEnabled
        ? runNode(wasmScript, ['status', `--owner=${ownerId}`], 120000, mock, 'wasm_runtime_status')
        : { ok: true, status: 0, payload: { ok: true, disabled: true }, stderr: '' };

      const jsOk = jsWorkflow.ok && jsActuation.ok;
      const wasmOk = wasmStatus.ok;
      const parityOk = jsOk && wasmOk;
      const allOk = parityOk && (coprocessorEnabled ? wasmOk : true);

      if (apply) {
        const state = readState(policy);
        writeState(policy, {
          runs: Number(state.runs || 0) + 1,
          updated_at: nowIso(),
          last_result: {
            owner_id: ownerId,
            ts: nowIso(),
            coprocessor_enabled: coprocessorEnabled,
            js_ok: jsOk,
            wasm_ok: wasmOk,
            parity_ok: parityOk
          }
        });
      }

      const receipt = ctx.cmdRecord(policy, {
        ...args,
        event: 'execution_sandbox_rust_wasm_coprocessor_verify',
        apply,
        payload_json: JSON.stringify({
          owner_id: ownerId,
          strict,
          coprocessor_enabled: coprocessorEnabled,
          js_workflow_ok: jsWorkflow.ok,
          js_actuation_ok: jsActuation.ok,
          wasm_status_ok: wasmOk,
          parity_ok: parityOk,
          sandbox_script: rel(sandboxScript),
          wasm_runtime_script: rel(wasmScript)
        })
      });

      if (strict && !allOk) {
        return {
          ...receipt,
          ok: false,
          error: 'execution_sandbox_coprocessor_failed',
          parity_ok: parityOk
        };
      }

      return {
        ...receipt,
        execution_sandbox_coprocessor_ok: allOk,
        parity_ok: parityOk
      };
    },

    status(policy, args, ctx) {
      const base = ctx.cmdStatus(policy, args);
      const state = readState(policy);
      return {
        ...base,
        coprocessor_state: state,
        artifacts: {
          ...base.artifacts,
          coprocessor_state_path: rel(policy.paths.coprocessor_state_path)
        }
      };
    }
  }
});
