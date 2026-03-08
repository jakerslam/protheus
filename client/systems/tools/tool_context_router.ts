#!/usr/bin/env node
'use strict';
export {};

const path = require('path');
const {
  ROOT,
  nowIso,
  parseArgs,
  cleanText,
  normalizeToken,
  toBool,
  clampNumber,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  stableHash,
  emit
} = require('../../lib/queued_backlog_runtime');

type AnyObj = Record<string, any>;

const DEFAULT_POLICY_PATH = process.env.TOOL_CONTEXT_ROUTER_POLICY_PATH
  ? path.resolve(String(process.env.TOOL_CONTEXT_ROUTER_POLICY_PATH))
  : path.join(ROOT, 'client', 'config', 'tool_context_router_policy.json');

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    allow_unknown_tool: false,
    score_weights: {
      scope: 0.45,
      tag_overlap: 0.35,
      base_priority: 0.2
    },
    tool_profiles: [
      {
        tool: 'research',
        scopes: ['analysis', 'intelligence', 'market'],
        tags: ['research', 'intel', 'market', 'competitor'],
        base_priority: 0.82
      },
      {
        tool: 'assimilate',
        scopes: ['memory', 'knowledge', 'synthesis'],
        tags: ['memory', 'insight', 'decision', 'node'],
        base_priority: 0.8
      },
      {
        tool: 'cli_suggestion_engine',
        scopes: ['ops', 'cli', 'runtime'],
        tags: ['terminal', 'command', 'ops', 'status'],
        base_priority: 0.74
      }
    ],
    paths: {
      latest_path: 'state/tools/tool_context_router/latest.json',
      history_path: 'state/tools/tool_context_router/history.jsonl'
    }
  };
}

function normalizeTags(raw: unknown) {
  const list = Array.isArray(raw) ? raw : String(raw || '').split(/[,\s]+/);
  const out: string[] = [];
  for (const token of list) {
    const t = normalizeToken(token, 64).replace(/^#/, '');
    if (!t) continue;
    if (!out.includes(t)) out.push(t);
  }
  return out.slice(0, 48);
}

function parseContext(raw: unknown) {
  if (typeof raw === 'object' && raw != null) {
    const scope = normalizeToken((raw as AnyObj).scope || '', 64) || 'general';
    return {
      scope,
      tags: normalizeTags((raw as AnyObj).tags || []),
      objective: cleanText((raw as AnyObj).objective || '', 180) || null,
      task_id: cleanText((raw as AnyObj).task_id || '', 120) || null
    };
  }
  const text = cleanText(raw || '', 20000);
  if (!text) return { scope: 'general', tags: [], objective: null, task_id: null };
  try {
    return parseContext(JSON.parse(text));
  } catch {
    return { scope: 'general', tags: normalizeTags(text), objective: null, task_id: null };
  }
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const base = defaultPolicy();
  const raw = readJson(policyPath, {});
  const weights = raw && raw.score_weights && typeof raw.score_weights === 'object'
    ? raw.score_weights
    : {};
  const toolProfiles = Array.isArray(raw && raw.tool_profiles) ? raw.tool_profiles : base.tool_profiles;
  const normalizedProfiles = toolProfiles
    .map((row: AnyObj) => {
      const tool = normalizeToken(row.tool || '', 80);
      if (!tool) return null;
      return {
        tool,
        scopes: normalizeTags(row.scopes || []),
        tags: normalizeTags(row.tags || []),
        base_priority: clampNumber(row.base_priority, 0, 1, 0.5)
      };
    })
    .filter(Boolean);
  const paths = raw && raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  return {
    version: cleanText(raw.version || base.version, 40) || '1.0',
    enabled: toBool(raw.enabled, true),
    allow_unknown_tool: toBool(raw.allow_unknown_tool, base.allow_unknown_tool),
    score_weights: {
      scope: clampNumber(weights.scope, 0, 1, base.score_weights.scope),
      tag_overlap: clampNumber(weights.tag_overlap, 0, 1, base.score_weights.tag_overlap),
      base_priority: clampNumber(weights.base_priority, 0, 1, base.score_weights.base_priority)
    },
    tool_profiles: normalizedProfiles.length > 0 ? normalizedProfiles : base.tool_profiles,
    paths: {
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      history_path: resolvePath(paths.history_path, base.paths.history_path)
    }
  };
}

function scoreCandidate(profile: AnyObj, context: AnyObj, policy: AnyObj) {
  const scopeMatch = profile.scopes.includes(context.scope) ? 1 : 0;
  const overlap = context.tags.filter((tag: string) => profile.tags.includes(tag));
  const tagOverlap = context.tags.length > 0 ? overlap.length / context.tags.length : 0;
  const scoreRaw = (
    (policy.score_weights.scope * scopeMatch)
    + (policy.score_weights.tag_overlap * tagOverlap)
    + (policy.score_weights.base_priority * profile.base_priority)
  );
  return {
    tool: profile.tool,
    score: Math.round(scoreRaw * 10000) / 10000,
    scope_match: scopeMatch,
    overlap_tags: overlap,
    profile_base_priority: profile.base_priority
  };
}

function routeTool(args: AnyObj = {}) {
  const policyPath = args.policy
    ? path.resolve(String(args.policy))
    : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  if (!policy.enabled) {
    return {
      ok: false,
      type: 'tool_context_route',
      error: 'tool_context_router_disabled',
      policy_path: policyPath
    };
  }
  const context = parseContext(args['context-json'] || args.context_json || args.context || '{}');
  const candidates = policy.tool_profiles.map((row: AnyObj) => scoreCandidate(row, context, policy));
  candidates.sort((a: AnyObj, b: AnyObj) => {
    const delta = Number(b.score || 0) - Number(a.score || 0);
    if (Math.abs(delta) > 1e-9) return delta;
    return String(a.tool).localeCompare(String(b.tool));
  });

  const winner = candidates[0] || null;
  const apply = toBool(args.apply, true);
  const threshold = clampNumber(args.threshold, 0, 1, 0.25);
  const selected = winner && Number(winner.score) >= threshold ? winner : null;
  const rejected = !selected && policy.allow_unknown_tool !== true;
  const receipt = {
    ok: rejected ? false : true,
    type: 'tool_context_route',
    ts: nowIso(),
    receipt_id: stableHash([
      'tool_context_route',
      context.scope || '',
      (context.tags || []).join(','),
      selected ? selected.tool : 'none'
    ].join('|'), 24),
    policy_version: policy.version,
    policy_path: policyPath,
    context,
    threshold,
    selected_tool: selected ? selected.tool : null,
    selected_score: selected ? selected.score : null,
    candidates,
    blocked_reason: rejected ? 'no_candidate_above_threshold' : null,
    apply
  };

  if (apply && receipt.ok === true) {
    writeJsonAtomic(policy.paths.latest_path, receipt);
    appendJsonl(policy.paths.history_path, receipt);
  }
  return receipt;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'route', 40).toLowerCase();
  if (!['route', 'status'].includes(cmd)) {
    emit({ ok: false, error: `unknown_command:${cmd}` }, 1);
  }
  const policyPath = args.policy
    ? path.resolve(String(args.policy))
    : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  if (cmd === 'status') {
    const latest = readJson(policy.paths.latest_path, null);
    emit({
      ok: true,
      type: 'tool_context_router_status',
      policy_version: policy.version,
      policy_path: policyPath,
      latest_path: path.relative(ROOT, policy.paths.latest_path).replace(/\\/g, '/'),
      history_path: path.relative(ROOT, policy.paths.history_path).replace(/\\/g, '/'),
      latest
    });
  }
  const receipt = routeTool(args);
  emit(receipt, receipt.ok === true ? 0 : 1);
}

if (require.main === module) {
  main();
}

module.exports = {
  routeTool,
  loadPolicy,
  parseContext
};
