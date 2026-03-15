#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

function resolveRoot(startDir = __dirname) {
  let dir = path.resolve(startDir);
  while (true) {
    const marker = path.join(dir, 'core', 'layer0', 'ops', 'Cargo.toml');
    if (fs.existsSync(marker)) return dir;
    const parent = path.dirname(dir);
    if (parent === dir) return path.resolve(__dirname, '../../../../');
    dir = parent;
  }
}

const ROOT = resolveRoot();
const DEFAULT_POLICY_PATH = path.join(ROOT, 'client', 'runtime', 'config', 'mcp_gateway_policy.json');

function nowIso() {
  return new Date().toISOString();
}

function cleanText(v, maxLen = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function normalizeToken(v, maxLen = 120) {
  return cleanText(v, maxLen).toLowerCase().replace(/[^a-z0-9_.:-]+/g, '_').replace(/^_+|_+$/g, '');
}

function stableHash(value) {
  return crypto.createHash('sha256').update(String(value || ''), 'utf8').digest('hex');
}

function parseArgs(argv = []) {
  const out = { _: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const raw = String(argv[i] || '');
    if (!raw.startsWith('--')) {
      out._.push(raw);
      continue;
    }
    if (raw.includes('=')) {
      const idx = raw.indexOf('=');
      const key = raw.slice(2, idx);
      out[key] = raw.slice(idx + 1);
      continue;
    }
    const key = raw.slice(2);
    const next = i + 1 < argv.length ? String(argv[i + 1] || '') : '';
    if (next && !next.startsWith('--')) {
      out[key] = next;
      i += 1;
    } else {
      out[key] = true;
    }
  }
  return out;
}

function boolFlag(value, fallback = false) {
  if (value == null) return fallback;
  const s = String(value).trim().toLowerCase();
  if (!s) return fallback;
  return s === '1' || s === 'true' || s === 'yes' || s === 'on';
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function readJson(filePath, fallback) {
  try {
    if (!fs.existsSync(filePath)) return fallback;
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function writeJson(filePath, value) {
  ensureDir(filePath);
  fs.writeFileSync(filePath, JSON.stringify(value, null, 2));
}

function appendJsonl(filePath, row) {
  ensureDir(filePath);
  fs.appendFileSync(filePath, `${JSON.stringify(row)}\n`);
}

function rel(filePath) {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function resolvePolicyPath(rawPath) {
  if (!rawPath) return DEFAULT_POLICY_PATH;
  const abs = path.resolve(String(rawPath));
  return abs;
}

function normalizePolicy(rawPolicy, policyPath) {
  const paths = rawPolicy && rawPolicy.paths && typeof rawPolicy.paths === 'object' ? rawPolicy.paths : {};
  return {
    version: cleanText(rawPolicy && rawPolicy.version || '1.0', 24) || '1.0',
    enabled: rawPolicy && rawPolicy.enabled !== false,
    strict_default: boolFlag(rawPolicy && rawPolicy.strict_default, true),
    event_stream: {
      enabled: boolFlag(rawPolicy && rawPolicy.event_stream && rawPolicy.event_stream.enabled, true),
      publish: boolFlag(rawPolicy && rawPolicy.event_stream && rawPolicy.event_stream.publish, true),
      stream: cleanText(rawPolicy && rawPolicy.event_stream && rawPolicy.event_stream.stream || 'skills.mcp_gateway', 160)
    },
    risk: {
      default_tier: Number(rawPolicy && rawPolicy.risk && rawPolicy.risk.default_tier) || 2,
      require_explicit_approval_tier: Number(rawPolicy && rawPolicy.risk && rawPolicy.risk.require_explicit_approval_tier) || 3
    },
    paths: {
      registry_path: path.resolve(ROOT, String(paths.registry_path || 'adapters/cognition/skills/mcp/registry.json')),
      installs_path: path.resolve(ROOT, String(paths.installs_path || 'local/state/adapters/cognition/skills/mcp_gateway/installs.json')),
      latest_path: path.resolve(ROOT, String(paths.latest_path || 'local/state/adapters/cognition/skills/mcp_gateway/latest.json')),
      events_path: path.resolve(ROOT, String(paths.events_path || 'local/state/adapters/cognition/skills/mcp_gateway/events.jsonl')),
      receipts_path: path.resolve(ROOT, String(paths.receipts_path || 'local/state/adapters/cognition/skills/mcp_gateway/receipts.jsonl')),
      memory_dir: path.resolve(ROOT, String(paths.memory_dir || 'local/state/adapters/cognition/skills/mcp')),
      adaptive_index_path: path.resolve(ROOT, String(paths.adaptive_index_path || 'local/state/adapters/cognition/skills/mcp/index.json'))
    },
    policy_path: path.resolve(policyPath)
  };
}

function loadGateway(policyPathArg) {
  const policyPath = resolvePolicyPath(policyPathArg);
  const rawPolicy = readJson(policyPath, {});
  const policy = normalizePolicy(rawPolicy, policyPath);
  const registry = readJson(policy.paths.registry_path, { skills: [] });
  const installs = readJson(policy.paths.installs_path, { schema_id: 'mcp_gateway_installs_v1', installed: [] });
  return { policy, registry, installs };
}

function capabilityMatrixForSkills(skills) {
  const sourceCaps = {
    filesystem: ['read', 'write', 'watch'],
    calendar: ['read', 'schedule'],
    issues: ['read', 'comment', 'assign'],
    github: ['read', 'comment', 'review'],
    notion: ['read', 'write', 'search'],
    linear: ['read', 'write', 'search'],
    gmail: ['read', 'send', 'label'],
    reddit: ['read', 'post', 'moderate']
  };
  const out = [];
  for (const skill of Array.isArray(skills) ? skills : []) {
    const source = cleanText(skill && skill.source, 180);
    const sourceKey = source.startsWith('mcp://') ? source.slice(6).split('/')[0] : source;
    const trustTier = cleanText(skill && skill.trust_tier, 40) || 'standard';
    const caps = (sourceCaps[sourceKey] || ['invoke']).slice();
    out.push({
      id: cleanText(skill && skill.id, 120),
      source,
      trust_tier: trustTier,
      capabilities: caps,
      requires_approval: trustTier !== 'verified'
    });
  }
  return out;
}

function installedIds(installs) {
  const rows = installs && Array.isArray(installs.installed) ? installs.installed : [];
  return new Set(rows.map((row) => cleanText(row && row.id, 120)).filter(Boolean));
}

function persistGatewayRow(policy, row) {
  writeJson(policy.paths.latest_path, row);
  appendJsonl(policy.paths.events_path, row);
  appendJsonl(policy.paths.receipts_path, row);

  const adaptive = readJson(policy.paths.adaptive_index_path, { schema_id: 'mcp_gateway_adaptive_index_v1', versions: [] });
  adaptive.updated_at = row.ts;
  adaptive.latest = {
    action: row.action,
    ok: row.ok === true,
    receipt_hash: row.receipt_hash,
    strict: row.strict === true
  };
  writeJson(policy.paths.adaptive_index_path, adaptive);
}

function emitResult(payload, code) {
  process.stdout.write(`${JSON.stringify(payload)}\n`);
  process.exit(code);
}

function requiresConduitGuard(args) {
  if (boolFlag(args.bypass, false)) {
    return {
      ok: false,
      error: 'bypass_forbidden',
      reason: 'mcp_gateway_is_conduit_only'
    };
  }
  return { ok: true };
}

function recordRow(base, policy, args) {
  const strict = boolFlag(args.strict, policy.strict_default);
  const ts = nowIso();
  const payload = {
    ...base,
    ts,
    strict,
    lane: 'adapters/cognition/skills/mcp/mcp_gateway',
    boundary: 'conduit_only',
    policy_path: rel(policy.policy_path),
    stream: policy.event_stream.stream || 'skills.mcp_gateway'
  };
  payload.receipt_hash = stableHash(JSON.stringify({
    action: payload.action,
    ts: payload.ts,
    result: payload.ok,
    details: payload.details || null
  }));
  if (boolFlag(args.apply, true)) {
    persistGatewayRow(policy, payload);
  }
  return payload;
}

function cmdStatus(ctx, args) {
  const installed = installedIds(ctx.installs);
  const skills = Array.isArray(ctx.registry.skills) ? ctx.registry.skills : [];
  return recordRow({
    ok: true,
    type: 'mcp_gateway_status',
    action: 'status',
    details: {
      policy_version: ctx.policy.version,
      registry_count: skills.length,
      installed_count: installed.size,
      strict_default: ctx.policy.strict_default,
      artifacts: {
        registry_path: rel(ctx.policy.paths.registry_path),
        installs_path: rel(ctx.policy.paths.installs_path),
        latest_path: rel(ctx.policy.paths.latest_path),
        events_path: rel(ctx.policy.paths.events_path),
        receipts_path: rel(ctx.policy.paths.receipts_path)
      }
    }
  }, ctx.policy, args);
}

function cmdDiscover(ctx, args) {
  const skills = Array.isArray(ctx.registry.skills) ? ctx.registry.skills : [];
  const installed = installedIds(ctx.installs);
  const matrix = capabilityMatrixForSkills(skills);
  return recordRow({
    ok: true,
    type: 'mcp_gateway_discover',
    action: 'discover',
    details: {
      skills: skills.map((row) => ({
        ...row,
        installed: installed.has(cleanText(row && row.id, 120))
      })),
      capability_matrix: matrix,
      export: {
        schema_id: 'mcp_gateway_discover_export_v1',
        generated_at: nowIso(),
        count: skills.length
      }
    }
  }, ctx.policy, args);
}

function cmdInstall(ctx, args) {
  const id = cleanText(args.id || args.skill || '', 120);
  if (!id) {
    return recordRow({
      ok: false,
      type: 'mcp_gateway_error',
      action: 'install',
      error: 'missing_id'
    }, ctx.policy, args);
  }
  const skills = Array.isArray(ctx.registry.skills) ? ctx.registry.skills : [];
  const target = skills.find((row) => cleanText(row && row.id, 120) === id);
  if (!target) {
    return recordRow({
      ok: false,
      type: 'mcp_gateway_error',
      action: 'install',
      error: 'unknown_skill',
      details: { id }
    }, ctx.policy, args);
  }

  const installs = ctx.installs && typeof ctx.installs === 'object' ? { ...ctx.installs } : { schema_id: 'mcp_gateway_installs_v1', installed: [] };
  installs.installed = Array.isArray(installs.installed) ? installs.installed : [];
  const exists = installs.installed.some((row) => cleanText(row && row.id, 120) === id);
  if (!exists) {
    installs.installed.push({
      id,
      source: cleanText(target.source, 200),
      installed_at: nowIso(),
      trust_tier: cleanText(target.trust_tier, 40) || 'standard'
    });
  }
  installs.updated_at = nowIso();
  if (boolFlag(args.apply, true)) {
    writeJson(ctx.policy.paths.installs_path, installs);
  }

  return recordRow({
    ok: true,
    type: 'mcp_gateway_install',
    action: 'install',
    details: {
      id,
      source: cleanText(target.source, 200),
      already_installed: exists,
      installed_count: installs.installed.length
    }
  }, ctx.policy, args);
}

function cmdUninstall(ctx, args) {
  const id = cleanText(args.id || args.skill || '', 120);
  if (!id) {
    return recordRow({
      ok: false,
      type: 'mcp_gateway_error',
      action: 'uninstall',
      error: 'missing_id'
    }, ctx.policy, args);
  }
  const installs = ctx.installs && typeof ctx.installs === 'object' ? { ...ctx.installs } : { schema_id: 'mcp_gateway_installs_v1', installed: [] };
  installs.installed = Array.isArray(installs.installed) ? installs.installed : [];
  const before = installs.installed.length;
  installs.installed = installs.installed.filter((row) => cleanText(row && row.id, 120) !== id);
  installs.updated_at = nowIso();
  if (boolFlag(args.apply, true)) {
    writeJson(ctx.policy.paths.installs_path, installs);
  }

  return recordRow({
    ok: true,
    type: 'mcp_gateway_uninstall',
    action: 'uninstall',
    details: {
      id,
      removed: before !== installs.installed.length,
      installed_count: installs.installed.length
    }
  }, ctx.policy, args);
}

function cmdExport(ctx, args) {
  const skills = Array.isArray(ctx.registry.skills) ? ctx.registry.skills : [];
  const installs = Array.isArray(ctx.installs && ctx.installs.installed) ? ctx.installs.installed : [];
  const payload = {
    schema_id: 'mcp_gateway_server_export_v1',
    generated_at: nowIso(),
    registry_count: skills.length,
    installed_count: installs.length,
    strict_default: ctx.policy.strict_default,
    capability_matrix: capabilityMatrixForSkills(skills),
    skills,
    installed: installs
  };
  return recordRow({
    ok: true,
    type: 'mcp_gateway_export',
    action: 'export',
    details: payload
  }, ctx.policy, args);
}

function usage() {
  return {
    ok: true,
    type: 'mcp_gateway_help',
    usage: [
      'node adapters/cognition/skills/mcp/mcp_gateway.ts status [--strict=1] [--apply=1]',
      'node adapters/cognition/skills/mcp/mcp_gateway.ts discover [--strict=1] [--apply=1]',
      'node adapters/cognition/skills/mcp/mcp_gateway.ts install --id=<skill_id> [--strict=1] [--apply=1]',
      'node adapters/cognition/skills/mcp/mcp_gateway.ts uninstall --id=<skill_id> [--strict=1] [--apply=1]',
      'node adapters/cognition/skills/mcp/mcp_gateway.ts export [--strict=1] [--apply=1]'
    ]
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const command = normalizeToken(args._[0] || 'status', 80) || 'status';
  if (args.help || command === 'help') {
    emitResult(usage(), 0);
  }

  const guard = requiresConduitGuard(args);
  if (!guard.ok) {
    emitResult({
      ok: false,
      type: 'mcp_gateway_error',
      action: command,
      error: guard.error,
      reason: guard.reason,
      ts: nowIso()
    }, 2);
  }

  const ctx = loadGateway(args.policy);
  if (!ctx.policy.enabled) {
    emitResult({
      ok: false,
      type: 'mcp_gateway_error',
      action: command,
      error: 'gateway_disabled',
      ts: nowIso()
    }, 2);
  }

  let out;
  if (command === 'status') out = cmdStatus(ctx, args);
  else if (command === 'discover') out = cmdDiscover(ctx, args);
  else if (command === 'install') out = cmdInstall(ctx, args);
  else if (command === 'uninstall') out = cmdUninstall(ctx, args);
  else if (command === 'export') out = cmdExport(ctx, args);
  else {
    out = {
      ok: false,
      type: 'mcp_gateway_error',
      action: command,
      error: 'unsupported_command',
      ts: nowIso()
    };
  }

  const strict = boolFlag(args.strict, ctx.policy.strict_default);
  emitResult(out, out.ok === false && strict ? 1 : 0);
}

if (require.main === module) {
  main();
}

module.exports = {
  main,
  parseArgs,
  capabilityMatrixForSkills,
  loadGateway
};
