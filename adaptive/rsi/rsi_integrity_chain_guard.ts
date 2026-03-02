#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-181
 * Verify RSI hash-chain continuity, Merkle root integrity, and rollback/resurrection linkage.
 */

const crypto = require('crypto');
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
  readJsonl,
  writeJsonAtomic
} = require('../../lib/queued_backlog_runtime');
const { runStandardLane } = require('../../lib/upgrade_lane_runtime');

const POLICY_PATH = process.env.RSI_INTEGRITY_CHAIN_GUARD_POLICY_PATH
  ? path.resolve(process.env.RSI_INTEGRITY_CHAIN_GUARD_POLICY_PATH)
  : path.join(ROOT, 'config', 'rsi_integrity_chain_guard_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node adaptive/rsi/rsi_integrity_chain_guard.js configure --owner=<owner_id>');
  console.log('  node adaptive/rsi/rsi_integrity_chain_guard.js verify --owner=<owner_id> [--strict=1] [--mock=1] [--apply=1]');
  console.log('  node adaptive/rsi/rsi_integrity_chain_guard.js rollback-drill --owner=<owner_id> [--proposal-id=<id>] [--strict=1] [--mock=1] [--apply=1]');
  console.log('  node adaptive/rsi/rsi_integrity_chain_guard.js status [--owner=<owner_id>]');
}

function sha256Hex(raw) {
  return crypto.createHash('sha256').update(String(raw || ''), 'utf8').digest('hex');
}

function buildMerkleRoot(inputHashes) {
  let level = (inputHashes || []).map((row) => String(row || '').trim()).filter(Boolean);
  if (level.length < 1) return null;
  while (level.length > 1) {
    const next = [];
    for (let i = 0; i < level.length; i += 2) {
      const left = level[i];
      const right = level[i + 1] || left;
      next.push(sha256Hex(`${left}${right}`));
    }
    level = next;
  }
  return level[0] || null;
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

function rel(absPath) {
  return path.relative(ROOT, absPath).replace(/\\/g, '/');
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
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

function resolvePathMaybe(rawPath, fallbackRel) {
  const txt = cleanText(rawPath || '', 420);
  if (!txt) return path.join(ROOT, fallbackRel);
  return path.isAbsolute(txt) ? path.resolve(txt) : path.join(ROOT, txt);
}

function readState(policy) {
  return readJson(policy.paths.integrity_state_path, {
    schema_id: 'rsi_integrity_chain_guard_state',
    schema_version: '1.0',
    runs: 0,
    updated_at: null,
    last_verification: null,
    rollback_drills: 0,
    last_rollback_drill: null
  });
}

function writeState(policy, state) {
  ensureDir(policy.paths.integrity_state_path);
  writeJsonAtomic(policy.paths.integrity_state_path, {
    schema_id: 'rsi_integrity_chain_guard_state',
    schema_version: '1.0',
    runs: Number(state.runs || 0),
    updated_at: state.updated_at || nowIso(),
    last_verification: state.last_verification || null,
    rollback_drills: Number(state.rollback_drills || 0),
    last_rollback_drill: state.last_rollback_drill || null
  });
}

function verifyChain(rows) {
  if (!Array.isArray(rows) || rows.length < 1) {
    return {
      ok: false,
      reason: 'chain_empty',
      linkage_ok: false,
      step_hash_count: 0,
      hashes: []
    };
  }
  let linkageOk = true;
  const hashes = [];
  let prev = null;
  for (const row of rows) {
    const stepHash = cleanText(row && row.step_hash || '', 200);
    const prevHash = cleanText(row && row.prev_hash || '', 200) || null;
    if (!stepHash) {
      linkageOk = false;
      continue;
    }
    if (prev !== null && prevHash !== prev) linkageOk = false;
    hashes.push(stepHash);
    prev = stepHash;
  }
  return {
    ok: linkageOk,
    reason: linkageOk ? null : 'linkage_mismatch',
    linkage_ok: linkageOk,
    step_hash_count: hashes.length,
    hashes
  };
}

runStandardLane({
  lane_id: 'V3-RACE-181',
  script_rel: 'adaptive/rsi/rsi_integrity_chain_guard.js',
  policy_path: POLICY_PATH,
  stream: 'adaptive.rsi.integrity_chain_guard',
  paths: {
    memory_dir: 'memory/adaptive/rsi_integrity_chain_guard',
    adaptive_index_path: 'adaptive/rsi/integrity_chain_guard/index.json',
    events_path: 'state/adaptive/rsi_integrity_chain_guard/events.jsonl',
    latest_path: 'state/adaptive/rsi_integrity_chain_guard/latest.json',
    receipts_path: 'state/adaptive/rsi_integrity_chain_guard/receipts.jsonl',
    integrity_state_path: 'state/adaptive/rsi_integrity_chain_guard/state.json'
  },
  usage,
  handlers: {
    verify(policy, args, ctx) {
      const ownerId = normalizeToken(args.owner || args.owner_id, 120);
      if (!ownerId) return { ok: false, error: 'missing_owner' };

      const strict = toBool(args.strict, true);
      const apply = toBool(args.apply, true);
      const mock = toBool(args.mock, false);

      const chainPath = resolvePathMaybe(policy.rsi_chain_path, 'state/adaptive/rsi/chain.jsonl');
      const merklePath = resolvePathMaybe(policy.rsi_merkle_path, 'state/adaptive/rsi/merkle.json');
      const reversionScript = resolvePathMaybe(policy.reversion_script, 'systems/autonomy/self_mod_reversion_drill.js');
      const continuityScript = resolvePathMaybe(policy.continuity_script, 'systems/continuity/resurrection_protocol.js');

      const chainRows = readJsonl(chainPath, []);
      const chain = verifyChain(chainRows);
      const expectedMerkle = buildMerkleRoot(chain.hashes);
      const merkleRow = readJson(merklePath, {});
      const storedMerkle = cleanText(merkleRow && merkleRow.merkle_root || '', 200) || null;
      const merkleOk = expectedMerkle != null && storedMerkle === expectedMerkle;

      const reversionStatus = runNode(reversionScript, ['status'], 120000, mock, 'reversion_status');
      const continuityStatus = runNode(continuityScript, ['status'], 120000, mock, 'continuity_status');
      const allOk = chain.ok === true
        && merkleOk
        && reversionStatus.ok === true
        && continuityStatus.ok === true;

      if (apply) {
        const state = readState(policy);
        writeState(policy, {
          ...state,
          runs: Number(state.runs || 0) + 1,
          updated_at: nowIso(),
          last_verification: {
            owner_id: ownerId,
            ts: nowIso(),
            ok: allOk,
            chain_count: chain.step_hash_count,
            stored_merkle_root: storedMerkle,
            computed_merkle_root: expectedMerkle
          }
        });
      }

      const receipt = ctx.cmdRecord(policy, {
        ...args,
        event: 'rsi_integrity_chain_verify',
        apply,
        payload_json: JSON.stringify({
          owner_id: ownerId,
          strict,
          chain_ok: chain.ok,
          chain_reason: chain.reason,
          chain_count: chain.step_hash_count,
          merkle_ok: merkleOk,
          stored_merkle_root: storedMerkle,
          computed_merkle_root: expectedMerkle,
          reversion_status_ok: reversionStatus.ok,
          continuity_status_ok: continuityStatus.ok,
          chain_path: rel(chainPath),
          merkle_path: rel(merklePath)
        })
      });

      if (strict && !allOk) {
        return {
          ...receipt,
          ok: false,
          error: 'rsi_integrity_chain_failed',
          chain_ok: chain.ok,
          merkle_ok: merkleOk
        };
      }

      return {
        ...receipt,
        integrity_chain_ok: allOk,
        chain_ok: chain.ok,
        merkle_ok: merkleOk
      };
    },

    'rollback-drill': function rollbackDrill(policy, args, ctx) {
      const ownerId = normalizeToken(args.owner || args.owner_id, 120);
      if (!ownerId) return { ok: false, error: 'missing_owner' };
      const strict = toBool(args.strict, true);
      const apply = toBool(args.apply, true);
      const mock = toBool(args.mock, false);

      const chainPath = resolvePathMaybe(policy.rsi_chain_path, 'state/adaptive/rsi/chain.jsonl');
      const chainRows = readJsonl(chainPath, []);
      const latest = Array.isArray(chainRows) && chainRows.length > 0 ? chainRows[chainRows.length - 1] : null;
      const proposalId = normalizeToken(args['proposal-id'] || args.proposal_id || (latest && latest.proposal_id) || 'latest', 120) || 'latest';

      const reversionScript = resolvePathMaybe(policy.reversion_script, 'systems/autonomy/self_mod_reversion_drill.js');
      const continuityScript = resolvePathMaybe(policy.continuity_script, 'systems/continuity/resurrection_protocol.js');

      const reversionRun = runNode(
        reversionScript,
        ['run', `--proposal-id=${proposalId}`, '--apply=0', '--reason=rsi_integrity_chain_guard_drill'],
        240000,
        mock,
        'reversion_drill_run'
      );
      const continuityRun = runNode(continuityScript, ['status'], 120000, mock, 'continuity_status');
      const allOk = reversionRun.ok === true && continuityRun.ok === true;

      if (apply) {
        const state = readState(policy);
        writeState(policy, {
          ...state,
          rollback_drills: Number(state.rollback_drills || 0) + 1,
          updated_at: nowIso(),
          last_rollback_drill: {
            owner_id: ownerId,
            ts: nowIso(),
            proposal_id: proposalId,
            ok: allOk
          }
        });
      }

      const receipt = ctx.cmdRecord(policy, {
        ...args,
        event: 'rsi_integrity_chain_rollback_drill',
        apply,
        payload_json: JSON.stringify({
          owner_id: ownerId,
          strict,
          proposal_id: proposalId,
          reversion_ok: reversionRun.ok,
          continuity_ok: continuityRun.ok
        })
      });

      if (strict && !allOk) {
        return {
          ...receipt,
          ok: false,
          error: 'rollback_drill_failed',
          proposal_id: proposalId
        };
      }

      return {
        ...receipt,
        rollback_drill_ok: allOk,
        proposal_id: proposalId
      };
    },

    status(policy, args, ctx) {
      const base = ctx.cmdStatus(policy, args);
      const state = readState(policy);
      return {
        ...base,
        integrity_state: state,
        artifacts: {
          ...base.artifacts,
          integrity_state_path: rel(policy.paths.integrity_state_path)
        }
      };
    }
  }
});
