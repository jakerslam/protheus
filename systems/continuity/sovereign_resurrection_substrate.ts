#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-037
 * Long-term archival + sovereign resurrection substrate.
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { spawnSync } = require('child_process');
const {
  ROOT,
  nowIso,
  parseArgs,
  cleanText,
  normalizeToken,
  toBool,
  clampInt,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  emit
} = require('../../lib/queued_backlog_runtime');

const POLICY_PATH = process.env.SOVEREIGN_RESURRECTION_SUBSTRATE_POLICY_PATH
  ? path.resolve(process.env.SOVEREIGN_RESURRECTION_SUBSTRATE_POLICY_PATH)
  : path.join(ROOT, 'config', 'sovereign_resurrection_substrate_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/continuity/sovereign_resurrection_substrate.js package [--bundle-id=<id>] [--apply=0|1] [--policy=<path>]');
  console.log('  node systems/continuity/sovereign_resurrection_substrate.js drill [--bundle-id=<id>] [--target-host=<id>] [--apply=0|1] [--policy=<path>]');
  console.log('  node systems/continuity/sovereign_resurrection_substrate.js status [--policy=<path>]');
}

function parseJsonFromStdout(stdout: string) {
  const text = String(stdout || '').trim();
  if (!text) return null;
  try { return JSON.parse(text); } catch {}
  const lines = text.split('\n').map((line) => line.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try { return JSON.parse(lines[i]); } catch {}
  }
  return null;
}

function normalizeList(v: unknown) {
  if (Array.isArray(v)) return v.map((row) => cleanText(row, 320)).filter(Boolean);
  const raw = cleanText(v || '', 4000);
  if (!raw) return [];
  return raw.split(',').map((row) => cleanText(row, 320)).filter(Boolean);
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    shadow_only: true,
    continuity_identity_sources: [
      'config/soul_token_guard_policy.json',
      'config/session_continuity_vault_policy.json',
      'config/helix_policy.json'
    ],
    drill: {
      default_target_host: 'resurrection_drill_host',
      default_bundle_prefix: 'srs'
    },
    commands: {
      cold_archive: ['node', 'systems/memory/cryonics_tier.js', 'run'],
      quantum_attest: ['node', 'systems/security/post_quantum_migration_lane.js', 'verify', '--strict=1'],
      resurrection_bundle: ['node', 'systems/continuity/resurrection_protocol.js', 'bundle'],
      resurrection_verify: ['node', 'systems/continuity/resurrection_protocol.js', 'verify'],
      resurrection_restore_preview: ['node', 'systems/continuity/resurrection_protocol.js', 'restore']
    },
    paths: {
      state_path: 'state/continuity/sovereign_resurrection_substrate/state.json',
      latest_path: 'state/continuity/sovereign_resurrection_substrate/latest.json',
      receipts_path: 'state/continuity/sovereign_resurrection_substrate/receipts.jsonl',
      drills_path: 'state/continuity/sovereign_resurrection_substrate/drills.jsonl'
    }
  };
}

function loadPolicy(policyPath = POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const drill = raw.drill && typeof raw.drill === 'object' ? raw.drill : {};
  const commands = raw.commands && typeof raw.commands === 'object' ? raw.commands : {};
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  return {
    version: cleanText(raw.version || base.version, 32) || base.version,
    enabled: toBool(raw.enabled, true),
    shadow_only: toBool(raw.shadow_only, true),
    continuity_identity_sources: normalizeList(raw.continuity_identity_sources || base.continuity_identity_sources),
    drill: {
      default_target_host: normalizeToken(drill.default_target_host || base.drill.default_target_host, 120) || base.drill.default_target_host,
      default_bundle_prefix: normalizeToken(drill.default_bundle_prefix || base.drill.default_bundle_prefix, 40) || base.drill.default_bundle_prefix
    },
    commands: {
      cold_archive: normalizeList(commands.cold_archive || base.commands.cold_archive),
      quantum_attest: normalizeList(commands.quantum_attest || base.commands.quantum_attest),
      resurrection_bundle: normalizeList(commands.resurrection_bundle || base.commands.resurrection_bundle),
      resurrection_verify: normalizeList(commands.resurrection_verify || base.commands.resurrection_verify),
      resurrection_restore_preview: normalizeList(commands.resurrection_restore_preview || base.commands.resurrection_restore_preview)
    },
    paths: {
      state_path: resolvePath(paths.state_path || base.paths.state_path, base.paths.state_path),
      latest_path: resolvePath(paths.latest_path || base.paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path || base.paths.receipts_path, base.paths.receipts_path),
      drills_path: resolvePath(paths.drills_path || base.paths.drills_path, base.paths.drills_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function runCommandJson(command: string[], extraArgs: string[] = []) {
  const cmd = Array.isArray(command) ? command.slice(0) : [];
  if (cmd.length < 1) return { ok: false, status: 127, error: 'command_missing', payload: null };
  const proc = spawnSync(cmd[0], cmd.slice(1).concat(extraArgs), {
    cwd: ROOT,
    encoding: 'utf8',
    timeout: 120_000
  });
  const payload = parseJsonFromStdout(proc.stdout);
  const ok = Number(proc.status || 0) === 0 && (!payload || payload.ok !== false);
  return {
    ok,
    status: Number.isFinite(proc.status) ? Number(proc.status) : 1,
    payload: payload && typeof payload === 'object' ? payload : null,
    stderr: cleanText(proc.stderr || '', 600)
  };
}

function sha256Text(v: string) {
  return crypto.createHash('sha256').update(String(v || ''), 'utf8').digest('hex');
}

function continuityIdentityHash(policy: Record<string, any>) {
  const rows = [];
  for (const relPath of policy.continuity_identity_sources || []) {
    const abs = path.isAbsolute(relPath) ? relPath : path.join(ROOT, relPath);
    if (!fs.existsSync(abs)) {
      rows.push({ path: relPath, sha256: null, exists: false });
      continue;
    }
    const body = fs.readFileSync(abs, 'utf8');
    rows.push({
      path: relPath,
      exists: true,
      sha256: sha256Text(body)
    });
  }
  const digest = sha256Text(JSON.stringify(rows));
  return {
    identity_sources: rows,
    continuity_hash: digest
  };
}

function loadState(policy: Record<string, any>) {
  const src = readJson(policy.paths.state_path, null);
  if (!src || typeof src !== 'object') {
    return {
      schema_id: 'sovereign_resurrection_substrate_state',
      schema_version: '1.0',
      updated_at: nowIso(),
      package_runs: 0,
      drill_runs: 0,
      last_package: null,
      last_drill: null
    };
  }
  return {
    schema_id: 'sovereign_resurrection_substrate_state',
    schema_version: '1.0',
    updated_at: src.updated_at || nowIso(),
    package_runs: Math.max(0, Number(src.package_runs || 0)),
    drill_runs: Math.max(0, Number(src.drill_runs || 0)),
    last_package: src.last_package || null,
    last_drill: src.last_drill || null
  };
}

function saveState(policy: Record<string, any>, state: Record<string, any>) {
  writeJsonAtomic(policy.paths.state_path, {
    schema_id: 'sovereign_resurrection_substrate_state',
    schema_version: '1.0',
    updated_at: nowIso(),
    package_runs: Math.max(0, Number(state.package_runs || 0)),
    drill_runs: Math.max(0, Number(state.drill_runs || 0)),
    last_package: state.last_package || null,
    last_drill: state.last_drill || null
  });
}

function appendReceipt(policy: Record<string, any>, row: Record<string, any>) {
  writeJsonAtomic(policy.paths.latest_path, row);
  appendJsonl(policy.paths.receipts_path, row);
}

function buildBundleId(policy: Record<string, any>) {
  const prefix = cleanText(policy.drill.default_bundle_prefix || 'srs', 40) || 'srs';
  return `${prefix}_${Date.now().toString(36)}`;
}

function runPackage(policy: Record<string, any>, args: Record<string, any>) {
  const apply = toBool(args.apply, false);
  const bundleId = normalizeToken(args.bundle_id || args['bundle-id'] || '', 120) || buildBundleId(policy);
  const coldArchive = runCommandJson(policy.commands.cold_archive);
  const quantum = runCommandJson(policy.commands.quantum_attest);
  const bundle = runCommandJson(policy.commands.resurrection_bundle, [`--bundle-id=${bundleId}`]);
  const verify = runCommandJson(policy.commands.resurrection_verify, [`--bundle-id=${bundleId}`]);
  const continuity = continuityIdentityHash(policy);

  const out = {
    ok: coldArchive.ok && quantum.ok && bundle.ok && verify.ok,
    type: 'sovereign_resurrection_substrate_package',
    ts: nowIso(),
    apply,
    shadow_only: policy.shadow_only === true,
    bundle_id: bundleId,
    cold_archive_ok: coldArchive.ok,
    quantum_attestation_ok: quantum.ok,
    resurrection_bundle_ok: bundle.ok,
    resurrection_verify_ok: verify.ok,
    continuity_hash: continuity.continuity_hash,
    continuity_identity_sources: continuity.identity_sources,
    outputs: {
      cold_archive: coldArchive.payload,
      quantum_attestation: quantum.payload,
      resurrection_bundle: bundle.payload,
      resurrection_verify: verify.payload
    }
  };

  if (apply) {
    const state = loadState(policy);
    state.package_runs += 1;
    state.last_package = {
      ts: out.ts,
      ok: out.ok,
      bundle_id: bundleId,
      continuity_hash: out.continuity_hash
    };
    saveState(policy, state);
  }

  appendReceipt(policy, out);
  emit(out, out.ok ? 0 : 2);
}

function runDrill(policy: Record<string, any>, args: Record<string, any>) {
  const apply = toBool(args.apply, false);
  const bundleId = normalizeToken(args.bundle_id || args['bundle-id'] || '', 120) || buildBundleId(policy);
  const targetHost = normalizeToken(args.target_host || args['target-host'] || policy.drill.default_target_host, 120)
    || policy.drill.default_target_host;
  const packageResult = runCommandJson(policy.commands.resurrection_bundle, [`--bundle-id=${bundleId}`]);
  const verifyResult = runCommandJson(policy.commands.resurrection_verify, [`--bundle-id=${bundleId}`, '--strict=1']);
  const restoreToken = cleanText(process.env.RESURRECTION_DRILL_TOKEN || 'drill_token_preview', 160) || 'drill_token_preview';
  const restorePreview = runCommandJson(
    policy.commands.resurrection_restore_preview,
    [`--bundle-id=${bundleId}`, `--target-host=${targetHost}`, `--attestation-token=${restoreToken}`, '--apply=0']
  );
  const continuity = continuityIdentityHash(policy);
  const continuityMatch = packageResult.ok && verifyResult.ok && restorePreview.ok;

  const out = {
    ok: continuityMatch,
    type: 'sovereign_resurrection_substrate_drill',
    ts: nowIso(),
    apply,
    bundle_id: bundleId,
    target_host: targetHost,
    resurrection_bundle_ok: packageResult.ok,
    resurrection_verify_ok: verifyResult.ok,
    restore_preview_ok: restorePreview.ok,
    continuity_hash: continuity.continuity_hash,
    continuity_match: continuityMatch,
    outputs: {
      resurrection_bundle: packageResult.payload,
      resurrection_verify: verifyResult.payload,
      restore_preview: restorePreview.payload
    }
  };

  if (apply) {
    const state = loadState(policy);
    state.drill_runs += 1;
    state.last_drill = {
      ts: out.ts,
      ok: out.ok,
      bundle_id: bundleId,
      target_host: targetHost,
      continuity_hash: out.continuity_hash
    };
    saveState(policy, state);
    appendJsonl(policy.paths.drills_path, out);
  }

  appendReceipt(policy, out);
  emit(out, out.ok ? 0 : 2);
}

function runStatus(policy: Record<string, any>) {
  const state = loadState(policy);
  const drills = readJson(policy.paths.drills_path, []);
  emit({
    ok: true,
    type: 'sovereign_resurrection_substrate_status',
    state,
    drill_rows: Array.isArray(drills) ? drills.length : 0
  }, 0);
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = normalizeToken(args._[0] || 'status', 40) || 'status';
  if (cmd === 'help' || cmd === '--help' || cmd === '-h') {
    usage();
    process.exit(0);
  }
  const policy = loadPolicy(args.policy ? String(args.policy) : undefined);
  if (policy.enabled !== true) emit({ ok: false, error: 'policy_disabled' }, 2);
  if (cmd === 'package') return runPackage(policy, args);
  if (cmd === 'drill') return runDrill(policy, args);
  if (cmd === 'status') return runStatus(policy);
  emit({ ok: false, error: 'unknown_command', command: cmd }, 2);
}

main();
