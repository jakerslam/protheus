#!/usr/bin/env node
'use strict';

const SCOPE_ID_FALLBACK_PREFIX = 'scope';
const SCOPE_ID_PATTERN = /^[a-z0-9][a-z0-9._:-]{1,95}$/;

function parseArgs(argv = []) {
  const positional = [];
  const flags = {};
  for (const raw of Array.isArray(argv) ? argv : []) {
    const token = String(raw || '').trim();
    if (!token) continue;
    if (token.startsWith('--')) {
      const body = token.slice(2);
      const eq = body.indexOf('=');
      if (eq >= 0) flags[body.slice(0, eq)] = body.slice(eq + 1);
      else flags[body] = '1';
      continue;
    }
    positional.push(token);
  }
  return { positional, flags };
}

function parseJson(raw, fallback, errorCode) {
  if (raw == null || String(raw).trim() === '') return { ok: true, value: fallback };
  try {
    return { ok: true, value: JSON.parse(String(raw)) };
  } catch {
    return { ok: false, reason_code: errorCode };
  }
}

function normalizeList(input, { upper = false } = {}) {
  const source = Array.isArray(input)
    ? input
    : typeof input === 'string'
      ? input.split(',')
      : [];
  const out = [];
  const seen = new Set();
  for (const row of source) {
    const normalized = String(row || '').trim();
    if (!normalized) continue;
    const value = upper ? normalized.toUpperCase() : normalized.replace(/\\/g, '/');
    if (seen.has(value)) continue;
    seen.add(value);
    out.push(value);
  }
  return out;
}

function normalizePathPattern(raw) {
  return String(raw || '').trim().replace(/\\/g, '/').replace(/^\.\//, '');
}

function pathPatternOverlaps(leftRaw, rightRaw) {
  const left = normalizePathPattern(leftRaw);
  const right = normalizePathPattern(rightRaw);
  if (!left || !right) return false;
  if (left === right) return true;

  const leftPrefix = left.endsWith('*') ? left.slice(0, -1) : '';
  const rightPrefix = right.endsWith('*') ? right.slice(0, -1) : '';

  if (leftPrefix && right.startsWith(leftPrefix)) return true;
  if (rightPrefix && left.startsWith(rightPrefix)) return true;
  if (!leftPrefix && rightPrefix && left.startsWith(rightPrefix)) return true;
  if (!rightPrefix && leftPrefix && right.startsWith(leftPrefix)) return true;
  return false;
}

function findingMatchesPathScope(finding, pathScopes = []) {
  if (!Array.isArray(pathScopes) || pathScopes.length === 0) return true;
  const location = normalizePathPattern(finding && finding.location ? finding.location : '');
  if (!location) return false;

  for (const rawPattern of pathScopes) {
    const pattern = normalizePathPattern(rawPattern);
    if (!pattern) continue;
    if (pattern.endsWith('*')) {
      const prefix = pattern.slice(0, -1);
      if (location.startsWith(prefix)) return true;
      continue;
    }
    if (location === pattern || location.startsWith(`${pattern}:`) || location.startsWith(`${pattern}#`)) {
      return true;
    }
  }
  return false;
}

function findingMatchesSeriesScope(finding, seriesScopes = []) {
  if (!Array.isArray(seriesScopes) || seriesScopes.length === 0) return true;
  const itemId = String(finding && finding.item_id ? finding.item_id : '').trim().toUpperCase();
  if (!itemId) return false;
  return seriesScopes.some((series) => itemId.startsWith(String(series || '').toUpperCase()));
}

function normalizeScope(rawScope, index = 0) {
  const scope = rawScope && typeof rawScope === 'object' && !Array.isArray(rawScope) ? rawScope : {};
  const scopeIdRaw = String(scope.scope_id || scope.scopeId || `${SCOPE_ID_FALLBACK_PREFIX}-${index + 1}`)
    .trim()
    .toLowerCase();

  const scopeId = SCOPE_ID_PATTERN.test(scopeIdRaw)
    ? scopeIdRaw
    : `${SCOPE_ID_FALLBACK_PREFIX}-${index + 1}`;

  const series = normalizeList(scope.series, { upper: true });
  const paths = normalizeList(scope.paths, { upper: false }).map(normalizePathPattern).filter(Boolean);

  if (series.length === 0 && paths.length === 0) {
    return {
      ok: false,
      reason_code: 'scope_missing_series_and_paths',
      scope_id: scopeId
    };
  }

  return {
    ok: true,
    scope: {
      scope_id: scopeId,
      series,
      paths
    }
  };
}

function detectScopeOverlaps(scopes = []) {
  const overlaps = [];
  const normalized = [];

  for (let index = 0; index < scopes.length; index += 1) {
    const normalizedScope = normalizeScope(scopes[index], index);
    if (!normalizedScope.ok) {
      return {
        ok: false,
        reason_code: normalizedScope.reason_code,
        scope_id: normalizedScope.scope_id,
        overlaps: []
      };
    }
    normalized.push(normalizedScope.scope);
  }

  for (let leftIndex = 0; leftIndex < normalized.length; leftIndex += 1) {
    for (let rightIndex = leftIndex + 1; rightIndex < normalized.length; rightIndex += 1) {
      const left = normalized[leftIndex];
      const right = normalized[rightIndex];

      const overlappingSeries = left.series.filter((token) => right.series.includes(token));
      const overlappingPaths = [];

      for (const leftPath of left.paths) {
        for (const rightPath of right.paths) {
          if (pathPatternOverlaps(leftPath, rightPath)) {
            overlappingPaths.push({ left: leftPath, right: rightPath });
          }
        }
      }

      if (overlappingSeries.length > 0 || overlappingPaths.length > 0) {
        overlaps.push({
          left_scope_id: left.scope_id,
          right_scope_id: right.scope_id,
          overlapping_series: overlappingSeries,
          overlapping_paths: overlappingPaths
        });
      }
    }
  }

  return {
    ok: overlaps.length === 0,
    reason_code: overlaps.length === 0 ? 'scope_non_overlap_ok' : 'scope_overlap_detected',
    normalized_scopes: normalized,
    overlaps
  };
}

function findingInScope(finding, scope) {
  const scopeNormalized = normalizeScope(scope, 0);
  if (!scopeNormalized.ok) {
    return {
      ok: false,
      reason_code: scopeNormalized.reason_code,
      in_scope: false,
      scope_id: scopeNormalized.scope_id
    };
  }

  const normalizedScope = scopeNormalized.scope;
  const matchesSeries = findingMatchesSeriesScope(finding, normalizedScope.series);
  const matchesPaths = findingMatchesPathScope(finding, normalizedScope.paths);
  const inScope = matchesSeries && matchesPaths;

  return {
    ok: true,
    reason_code: inScope ? 'finding_in_scope' : 'finding_out_of_scope',
    in_scope: inScope,
    scope_id: normalizedScope.scope_id,
    matches_series: matchesSeries,
    matches_paths: matchesPaths
  };
}

function classifyFindingsByScope(findings = [], scope, agentId = '') {
  const inScope = [];
  const outOfScope = [];
  const violations = [];

  const normalizedAgentId = String(agentId || '').trim() || undefined;

  for (const finding of Array.isArray(findings) ? findings : []) {
    const verdict = findingInScope(finding, scope);
    if (!verdict.ok) {
      outOfScope.push(finding);
      violations.push({
        reason_code: verdict.reason_code,
        item_id: finding && finding.item_id ? finding.item_id : null,
        location: finding && finding.location ? finding.location : null,
        agent_id: normalizedAgentId,
        scope_id: verdict.scope_id || null
      });
      continue;
    }

    if (verdict.in_scope) {
      inScope.push(finding);
      continue;
    }

    outOfScope.push(finding);
    violations.push({
      reason_code: 'out_of_scope_finding',
      item_id: finding && finding.item_id ? finding.item_id : null,
      location: finding && finding.location ? finding.location : null,
      agent_id: normalizedAgentId,
      scope_id: verdict.scope_id,
      matches_series: verdict.matches_series,
      matches_paths: verdict.matches_paths
    });
  }

  return {
    ok: true,
    type: 'orchestration_scope_classification',
    in_scope: inScope,
    out_of_scope: outOfScope,
    violations
  };
}

function run(argv = process.argv.slice(2)) {
  const parsed = parseArgs(argv);
  const command = String(parsed.positional[0] || 'validate').trim().toLowerCase();

  if (command === 'validate' || command === 'overlap') {
    const scopePayload = parseJson(
      parsed.flags['scopes-json'] || parsed.flags.scopes_json,
      [],
      'invalid_scopes_json'
    );
    if (!scopePayload.ok) {
      return {
        ok: false,
        type: 'orchestration_scope_validate',
        reason_code: scopePayload.reason_code
      };
    }

    const result = detectScopeOverlaps(scopePayload.value);
    return Object.assign({ type: 'orchestration_scope_validate' }, result);
  }

  if (command === 'classify') {
    const scopePayload = parseJson(
      parsed.flags['scope-json'] || parsed.flags.scope_json,
      {},
      'invalid_scope_json'
    );
    if (!scopePayload.ok) {
      return {
        ok: false,
        type: 'orchestration_scope_classification',
        reason_code: scopePayload.reason_code
      };
    }

    const findingsPayload = parseJson(
      parsed.flags['findings-json'] || parsed.flags.findings_json,
      [],
      'invalid_findings_json'
    );
    if (!findingsPayload.ok) {
      return {
        ok: false,
        type: 'orchestration_scope_classification',
        reason_code: findingsPayload.reason_code
      };
    }

    return classifyFindingsByScope(
      findingsPayload.value,
      scopePayload.value,
      parsed.flags['agent-id'] || parsed.flags.agent_id || ''
    );
  }

  return {
    ok: false,
    type: 'orchestration_scope_command',
    reason_code: `unsupported_command:${command}`,
    commands: ['validate', 'overlap', 'classify']
  };
}

if (require.main === module) {
  const out = run(process.argv.slice(2));
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
  process.exit(out.ok ? 0 : 1);
}

module.exports = {
  SCOPE_ID_PATTERN,
  parseArgs,
  normalizeScope,
  detectScopeOverlaps,
  findingInScope,
  classifyFindingsByScope,
  run
};
