#!/usr/bin/env node
'use strict';

const { parseArgs, parseJson, invokeOrchestration } = require('./core_bridge.ts');

const SCOPE_ID_FALLBACK_PREFIX = 'scope';
const SCOPE_ID_PATTERN = /^[a-z0-9][a-z0-9._:-]{1,95}$/;

function normalizeScope(scope, index = 0) {
  const out = invokeOrchestration('scope.normalize', {
    scope: scope && typeof scope === 'object' ? scope : {},
  });

  if (out && out.ok === true && out.scope && typeof out.scope === 'object') {
    return {
      ok: true,
      scope: out.scope,
    };
  }

  return {
    ok: false,
    reason_code: String(out && out.reason_code ? out.reason_code : 'scope_invalid'),
    scope_id: String(
      (out && out.scope_id) || (scope && (scope.scope_id || scope.scopeId)) || `${SCOPE_ID_FALLBACK_PREFIX}-${index + 1}`
    ).toLowerCase(),
  };
}

function detectScopeOverlaps(scopes = []) {
  return invokeOrchestration('scope.detect_overlaps', {
    scopes: Array.isArray(scopes) ? scopes : [],
  });
}

function findingInScope(finding, scope) {
  return invokeOrchestration('scope.finding_in_scope', {
    finding: finding && typeof finding === 'object' ? finding : {},
    scope: scope && typeof scope === 'object' ? scope : {},
  });
}

function classifyFindingsByScope(findings = [], scope, agentId = '') {
  return invokeOrchestration('scope.classify_findings', {
    findings: Array.isArray(findings) ? findings : [],
    scope: scope && typeof scope === 'object' ? scope : {},
    agent_id: String(agentId || '').trim(),
  });
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
        reason_code: scopePayload.reason_code,
      };
    }

    return Object.assign(
      { type: 'orchestration_scope_validate' },
      detectScopeOverlaps(scopePayload.value),
    );
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
        reason_code: scopePayload.reason_code,
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
        reason_code: findingsPayload.reason_code,
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
    commands: ['validate', 'overlap', 'classify'],
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
  run,
};
