#!/usr/bin/env node
/* eslint-disable no-console */

import fs from 'node:fs';
import path from 'node:path';
import { parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

type GateCheck = {
  id: string;
  ok: boolean;
  detail: string;
};

type Args = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
  policyPath: string;
  processPath?: string;
  ownerRosterPath?: string;
  waiversPath?: string;
  artifactSchemaPath?: string;
};

type GovernancePolicy = {
  hard_gates?: {
    severity_taxonomy?: {
      required_levels?: string[];
      definitions?: Record<string, string>;
    };
    ownership_map?: {
      required_roles?: string[];
      lanes?: Record<string, string>;
      roster_path?: string;
    };
    escalation_sla_minutes?: Record<string, {
      ack?: number;
      escalate?: number;
      update_cadence?: number;
    }>;
    rollback_criteria?: {
      must_define?: string[];
    };
    post_incident_artifacts?: {
      required?: string[];
      schema_path?: string;
    };
    waiver_contract?: {
      waivers_path?: string;
      required_fields?: string[];
    };
  };
  policy_contract?: {
    communication_templates?: {
      required_templates?: string[];
    };
    deployment_checklist?: {
      required_items?: string[];
    };
    reporting_format?: {
      required_sections?: string[];
    };
    script_output_conventions?: {
      required_fields?: string[];
    };
  };
  process_contract?: {
    process_doc_path?: string;
    required_headings?: string[];
  };
};

type OwnerRoster = {
  owners?: Array<{
    owner_id?: string;
    display_name?: string;
    contact?: string;
    roles?: string[];
    active?: boolean;
  }>;
};

type GovernanceWaivers = {
  waivers?: Array<{
    waiver_id?: string;
    check_ids?: string[];
    reason?: string;
    approver?: string;
    expires_at?: string;
    status?: string;
  }>;
};

type ArtifactSchema = {
  allowed_field_types?: string[];
  artifacts?: Record<string, {
    type?: string;
    required_fields?: Record<string, string>;
  }>;
};

type ActiveWaiver = {
  waiverId: string;
  checkIds: string[];
  reason: string;
  approver: string;
  expiresAt: string;
};

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/incident_operations_governance_gate_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/INCIDENT_OPERATIONS_GOVERNANCE_GATE_CURRENT.md';
const DEFAULT_POLICY_PATH = 'client/runtime/config/incident_operations_governance_policy.json';
const DEFAULT_PROCESS_PATH = 'docs/workspace/process/incident_response_workflow.md';
const DEFAULT_OWNER_ROSTER_PATH = 'client/runtime/config/incident_owner_roster.json';
const DEFAULT_WAIVERS_PATH = 'client/runtime/config/incident_operations_governance_waivers.json';
const DEFAULT_ARTIFACT_SCHEMA_PATH = 'client/runtime/config/post_incident_artifact_schema.json';

const PLACEHOLDER_TOKENS = ['tbd', 'todo', 'placeholder', 'example', 'unknown', 'owner_name_here', 'n/a'];

function cleanText(value: unknown, maxLen = 800): string {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function resolveArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, { out: DEFAULT_OUT_JSON });
  const processPath = cleanText(readFlag(argv, 'process') || '', 500);
  const ownerRosterPath = cleanText(readFlag(argv, 'owner-roster') || '', 500);
  const waiversPath = cleanText(readFlag(argv, 'waivers') || '', 500);
  const artifactSchemaPath = cleanText(readFlag(argv, 'artifact-schema') || '', 500);
  return {
    strict: common.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || readFlag(argv, 'out') || common.out || DEFAULT_OUT_JSON, 500),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MD, 500),
    policyPath: cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY_PATH, 500),
    processPath: processPath || undefined,
    ownerRosterPath: ownerRosterPath || undefined,
    waiversPath: waiversPath || undefined,
    artifactSchemaPath: artifactSchemaPath || undefined,
  };
}

function readJson<T>(filePath: string): T | null {
  try {
    return JSON.parse(fs.readFileSync(path.resolve(ROOT, filePath), 'utf8')) as T;
  } catch {
    return null;
  }
}

function uniqueNonEmpty(list: unknown): string[] {
  if (!Array.isArray(list)) return [];
  return Array.from(new Set(list.map((row) => cleanText(row, 120)).filter(Boolean)));
}

function resolveInputPath(flagPath: string | undefined, policyPath: string | undefined, fallback: string): string {
  const fromFlag = cleanText(flagPath || '', 500);
  if (fromFlag) return fromFlag;
  const fromPolicy = cleanText(policyPath || '', 500);
  if (fromPolicy) return fromPolicy;
  return fallback;
}

function parseIsoMs(value: string): number | null {
  const parsed = Date.parse(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function hasPlaceholderToken(value: string): boolean {
  const normalized = value.toLowerCase();
  return PLACEHOLDER_TOKENS.some((token) => normalized.includes(token));
}

function checkSeverityTaxonomy(policy: GovernancePolicy): GateCheck {
  const required = ['P0', 'P1', 'P2', 'P3'];
  const levels = uniqueNonEmpty(policy.hard_gates?.severity_taxonomy?.required_levels);
  const definitions = policy.hard_gates?.severity_taxonomy?.definitions || {};
  const missingLevels = required.filter((id) => !levels.includes(id));
  const missingDefinitions = required.filter((id) => cleanText(definitions[id], 240).length === 0);
  const ok = missingLevels.length === 0 && missingDefinitions.length === 0;
  return {
    id: 'severity_taxonomy',
    ok,
    detail: ok
      ? `levels=${levels.join(',')}`
      : `missing_levels=${missingLevels.join(',') || 'none'};missing_definitions=${missingDefinitions.join(',') || 'none'}`,
  };
}

function checkOwnershipMap(policy: GovernancePolicy): GateCheck {
  const roles = uniqueNonEmpty(policy.hard_gates?.ownership_map?.required_roles);
  const lanes = policy.hard_gates?.ownership_map?.lanes || {};
  const requiredRoles = [
    'incident_commander',
    'ops_oncall',
    'runtime_owner',
    'adapter_owner',
    'dashboard_owner',
    'release_owner',
  ];
  const requiredLaneKeys = [
    'runtime_authority',
    'adapter_fail_closed',
    'dashboard_freshness',
    'release_governance',
    'incident_command',
  ];
  const missingRoles = requiredRoles.filter((role) => !roles.includes(role));
  const missingLanes = requiredLaneKeys.filter((lane) => cleanText(lanes[lane], 120).length === 0);
  const laneValues = Object.values(lanes).map((row) => cleanText(row, 120)).filter(Boolean);
  const laneRoleViolations = laneValues.filter((value) => !roles.includes(value));
  const ok =
    missingRoles.length === 0
    && missingLanes.length === 0
    && laneRoleViolations.length === 0
    && Object.keys(lanes).length > 0;
  return {
    id: 'ownership_map',
    ok,
    detail: ok
      ? `roles=${roles.length};lanes=${Object.keys(lanes).length}`
      : `missing_roles=${missingRoles.join(',') || 'none'};missing_lanes=${missingLanes.join(',') || 'none'};lane_role_violations=${laneRoleViolations.join(',') || 'none'};lane_count=${Object.keys(lanes).length}`,
  };
}

function checkOwnerRoster(policy: GovernancePolicy, ownerRosterPath: string): GateCheck {
  const roster = readJson<OwnerRoster>(ownerRosterPath);
  if (!roster) {
    return {
      id: 'owner_roster_contract',
      ok: false,
      detail: `missing_or_invalid:${ownerRosterPath}`,
    };
  }

  const requiredRoles = uniqueNonEmpty(policy.hard_gates?.ownership_map?.required_roles);
  const owners = Array.isArray(roster.owners) ? roster.owners : [];
  const activeOwners = owners.filter((owner) => owner?.active !== false);

  const duplicateIds = new Set<string>();
  const seenIds = new Set<string>();
  const placeholderOwners: string[] = [];

  for (const owner of activeOwners) {
    const ownerId = cleanText(owner.owner_id, 120);
    const displayName = cleanText(owner.display_name, 160);
    const contact = cleanText(owner.contact, 160);
    if (!ownerId || !displayName || !contact) {
      placeholderOwners.push(ownerId || displayName || 'unknown_owner_missing_fields');
      continue;
    }
    if (seenIds.has(ownerId)) duplicateIds.add(ownerId);
    seenIds.add(ownerId);
    if (hasPlaceholderToken(ownerId) || hasPlaceholderToken(displayName) || hasPlaceholderToken(contact)) {
      placeholderOwners.push(ownerId);
    }
  }

  const missingRoles = requiredRoles.filter((requiredRole) => !activeOwners.some((owner) => {
    const ownerRoles = uniqueNonEmpty(owner.roles);
    return ownerRoles.includes(requiredRole);
  }));
  const criticalRoleOwnerIds = new Set<string>();
  for (const owner of activeOwners) {
    const ownerId = cleanText(owner.owner_id, 120);
    const ownerRoles = uniqueNonEmpty(owner.roles);
    if (!ownerId) continue;
    if (ownerRoles.includes('incident_commander') || ownerRoles.includes('ops_oncall')) {
      criticalRoleOwnerIds.add(ownerId);
    }
  }
  const criticalRoleSeparationViolation = criticalRoleOwnerIds.size < 2;

  const ok =
    missingRoles.length === 0
    && duplicateIds.size === 0
    && placeholderOwners.length === 0
    && !criticalRoleSeparationViolation;
  return {
    id: 'owner_roster_contract',
    ok,
    detail: ok
      ? `path=${ownerRosterPath};active_owners=${activeOwners.length}`
      : `missing_roles=${missingRoles.join(',') || 'none'};duplicate_owner_ids=${Array.from(duplicateIds).join(',') || 'none'};placeholder_or_invalid_owners=${placeholderOwners.join(',') || 'none'};critical_role_owner_separation=${criticalRoleSeparationViolation ? 'failed' : 'ok'}`,
  };
}

function checkEscalationSla(policy: GovernancePolicy): GateCheck {
  const required = uniqueNonEmpty(policy.hard_gates?.severity_taxonomy?.required_levels);
  if (required.length === 0) {
    required.push('P0', 'P1', 'P2', 'P3');
  }
  const sla = policy.hard_gates?.escalation_sla_minutes || {};
  const missing = required.filter((id) => !(id in sla));
  const orderViolations: string[] = [];
  const fieldViolations: string[] = [];
  const fields: Array<'ack' | 'escalate' | 'update_cadence'> = ['ack', 'escalate', 'update_cadence'];

  for (const severity of required) {
    const row = sla[severity] || {};
    for (const field of fields) {
      const value = Number(row[field]);
      if (!Number.isFinite(value) || value <= 0) {
        fieldViolations.push(`${severity}.${field}`);
      }
    }
    const ack = Number(row.ack);
    const escalate = Number(row.escalate);
    const cadence = Number(row.update_cadence);
    if (Number.isFinite(ack) && Number.isFinite(escalate) && Number.isFinite(cadence)) {
      if (!(ack <= escalate && escalate <= cadence)) orderViolations.push(severity);
      if (cadence > 1440) orderViolations.push(`${severity}_cadence_exceeds_24h`);
    }
  }

  for (const field of fields) {
    const vals = required.map((severity) => Number(sla[severity]?.[field]));
    if (vals.every((value) => Number.isFinite(value))) {
      for (let index = 0; index < vals.length - 1; index += 1) {
        if (!(vals[index] <= vals[index + 1])) {
          orderViolations.push(`cross_severity_${field}`);
          break;
        }
      }
    }
  }

  const ok = missing.length === 0 && fieldViolations.length === 0 && orderViolations.length === 0;
  return {
    id: 'escalation_sla_minutes',
    ok,
    detail: ok
      ? 'sla_contract=ok'
      : `missing=${missing.join(',') || 'none'};invalid_fields=${fieldViolations.join(',') || 'none'};order_violations=${orderViolations.join(',') || 'none'}`,
  };
}

function checkRollbackCriteria(policy: GovernancePolicy): GateCheck {
  const requiredRollback = [
    'integrity_violation_detected',
    'error_budget_burn_rate_exceeded',
    'fail_closed_contract_breach',
    'unbounded_resource_growth',
    'operator_unrecoverable_state',
  ];
  const rollback = uniqueNonEmpty(policy.hard_gates?.rollback_criteria?.must_define);
  const missingRollback = requiredRollback.filter((id) => !rollback.includes(id));
  return {
    id: 'rollback_criteria',
    ok: missingRollback.length === 0,
    detail:
      missingRollback.length === 0
        ? `rollback_criteria=${rollback.length}`
        : `missing_rollback=${missingRollback.join(',')}`,
  };
}

function checkPostIncidentArtifactSchema(policy: GovernancePolicy, schemaPath: string): GateCheck {
  const requiredArtifacts = uniqueNonEmpty(policy.hard_gates?.post_incident_artifacts?.required);
  const schema = readJson<ArtifactSchema>(schemaPath);
  if (!schema) {
    return {
      id: 'post_incident_artifact_schema',
      ok: false,
      detail: `missing_or_invalid:${schemaPath}`,
    };
  }

  const allowedTypes = uniqueNonEmpty(schema.allowed_field_types);
  const knownTypeSet = new Set(allowedTypes.length ? allowedTypes : ['string', 'number', 'boolean', 'array', 'object']);
  const artifacts = schema.artifacts || {};
  const missingSchemas = requiredArtifacts.filter((artifact) => !(artifact in artifacts));
  const invalidSchemas: string[] = [];

  for (const artifact of requiredArtifacts) {
    const row = artifacts[artifact];
    if (!row) continue;
    if (cleanText(row.type, 40) !== 'object') {
      invalidSchemas.push(`${artifact}.type`);
      continue;
    }
    const fields = row.required_fields || {};
    const entries = Object.entries(fields);
    if (entries.length === 0) {
      invalidSchemas.push(`${artifact}.required_fields_empty`);
      continue;
    }
    for (const [field, type] of entries) {
      const fieldName = cleanText(field, 100);
      const typeName = cleanText(type, 40);
      if (!fieldName || !knownTypeSet.has(typeName)) {
        invalidSchemas.push(`${artifact}.${fieldName || 'unknown_field'}`);
      }
    }
  }

  const ok = missingSchemas.length === 0 && invalidSchemas.length === 0;
  return {
    id: 'post_incident_artifact_schema',
    ok,
    detail: ok
      ? `path=${schemaPath};required_artifacts=${requiredArtifacts.length}`
      : `missing_artifact_schemas=${missingSchemas.join(',') || 'none'};invalid_schema_entries=${invalidSchemas.join(',') || 'none'}`,
  };
}

function checkPolicyContracts(policy: GovernancePolicy): GateCheck {
  const templates = uniqueNonEmpty(policy.policy_contract?.communication_templates?.required_templates);
  const checklist = uniqueNonEmpty(policy.policy_contract?.deployment_checklist?.required_items);
  const reporting = uniqueNonEmpty(policy.policy_contract?.reporting_format?.required_sections);
  const scriptFields = uniqueNonEmpty(policy.policy_contract?.script_output_conventions?.required_fields);
  const ok = templates.length >= 5 && checklist.length >= 5 && reporting.length >= 6 && scriptFields.length >= 6;
  return {
    id: 'policy_contracts',
    ok,
    detail: ok
      ? `templates=${templates.length};checklist=${checklist.length};reporting=${reporting.length};script_fields=${scriptFields.length}`
      : `templates=${templates.length};checklist=${checklist.length};reporting=${reporting.length};script_fields=${scriptFields.length}`,
  };
}

function checkPolicyContractRequiredTokens(policy: GovernancePolicy): GateCheck {
  const templateRequired = [
    'initial_alert',
    'status_update',
    'mitigation_started',
    'rollback_notice',
    'resolved',
    'postmortem_ready',
  ];
  const checklistRequired = [
    'scope_declared',
    'risk_level_declared',
    'rollback_plan_linked',
    'owner_acknowledged',
    'observer_assigned',
  ];
  const reportingRequired = [
    'summary',
    'severity_and_scope',
    'timeline',
    'actions_taken',
    'residual_risk',
    'followups',
  ];
  const scriptFieldRequired = [
    'status',
    'reason_code',
    'artifact_path',
    'next_action',
    'owner',
    'severity',
  ];

  const templates = uniqueNonEmpty(policy.policy_contract?.communication_templates?.required_templates);
  const checklist = uniqueNonEmpty(policy.policy_contract?.deployment_checklist?.required_items);
  const reporting = uniqueNonEmpty(policy.policy_contract?.reporting_format?.required_sections);
  const scriptFields = uniqueNonEmpty(policy.policy_contract?.script_output_conventions?.required_fields);

  const templateMissing = templateRequired.filter((id) => !templates.includes(id));
  const checklistMissing = checklistRequired.filter((id) => !checklist.includes(id));
  const reportingMissing = reportingRequired.filter((id) => !reporting.includes(id));
  const scriptFieldMissing = scriptFieldRequired.filter((id) => !scriptFields.includes(id));

  const placeholderTokens = [
    ...templates.map((id) => ({ group: 'templates', id })),
    ...checklist.map((id) => ({ group: 'checklist', id })),
    ...reporting.map((id) => ({ group: 'reporting', id })),
    ...scriptFields.map((id) => ({ group: 'script_fields', id })),
  ]
    .filter((row) => hasPlaceholderToken(row.id))
    .map((row) => `${row.group}.${row.id}`);

  const ok =
    templateMissing.length === 0
    && checklistMissing.length === 0
    && reportingMissing.length === 0
    && scriptFieldMissing.length === 0
    && placeholderTokens.length === 0;
  return {
    id: 'policy_contract_required_tokens',
    ok,
    detail: ok
      ? 'required_tokens=ok'
      : `missing_templates=${templateMissing.join(',') || 'none'};missing_checklist=${checklistMissing.join(',') || 'none'};missing_reporting=${reportingMissing.join(',') || 'none'};missing_script_fields=${scriptFieldMissing.join(',') || 'none'};placeholder_tokens=${placeholderTokens.join(',') || 'none'}`,
  };
}

function checkWaiverPolicyContract(policy: GovernancePolicy): GateCheck {
  const required = uniqueNonEmpty(policy.hard_gates?.waiver_contract?.required_fields);
  const expected = ['waiver_id', 'check_ids', 'reason', 'approver', 'expires_at', 'status'];
  const missing = expected.filter((id) => !required.includes(id));
  const unknown = required.filter((id) => !expected.includes(id));
  const placeholder = required.filter((id) => hasPlaceholderToken(id));
  const ok = missing.length === 0 && unknown.length === 0 && placeholder.length === 0;
  return {
    id: 'waiver_policy_contract',
    ok,
    detail: ok
      ? `required_fields=${required.length}`
      : `missing_fields=${missing.join(',') || 'none'};unknown_fields=${unknown.join(',') || 'none'};placeholder_fields=${placeholder.join(',') || 'none'}`,
  };
}

function checkProcessDoc(policy: GovernancePolicy, processPath: string): GateCheck {
  const requiredHeadings = uniqueNonEmpty(policy.process_contract?.required_headings);
  try {
    const raw = fs.readFileSync(path.resolve(ROOT, processPath), 'utf8');
    const missing = requiredHeadings.filter((heading) => !raw.includes(heading));
    const headingPositions = requiredHeadings.map((heading) => raw.indexOf(heading));
    const orderViolations: string[] = [];
    for (let i = 0; i < headingPositions.length - 1; i += 1) {
      const currentPos = headingPositions[i];
      const nextPos = headingPositions[i + 1];
      if (currentPos >= 0 && nextPos >= 0 && currentPos > nextPos) {
        orderViolations.push(`${requiredHeadings[i]}>${requiredHeadings[i + 1]}`);
      }
    }
    const ok = missing.length === 0 && orderViolations.length === 0;
    return {
      id: 'process_doc_contract',
      ok,
      detail: ok
        ? `path=${processPath};headings=${requiredHeadings.length}`
        : `path=${processPath};missing_headings=${missing.join('|') || 'none'};heading_order_violations=${orderViolations.join('|') || 'none'}`,
    };
  } catch {
    return {
      id: 'process_doc_contract',
      ok: false,
      detail: `missing_or_invalid:${processPath}`,
    };
  }
}

function checkWaiverContract(waiversPath: string): { check: GateCheck; activeWaivers: ActiveWaiver[] } {
  const waiversDoc = readJson<GovernanceWaivers>(waiversPath);
  if (!waiversDoc) {
    return {
      check: {
        id: 'waiver_contract',
        ok: false,
        detail: `missing_or_invalid:${waiversPath}`,
      },
      activeWaivers: [],
    };
  }

  const rows = Array.isArray(waiversDoc.waivers) ? waiversDoc.waivers : [];
  const now = Date.now();
  const violations: string[] = [];
  const activeWaivers: ActiveWaiver[] = [];
  const seenWaiverIds = new Set<string>();
  const duplicateWaiverIds = new Set<string>();
  const allowedStatuses = new Set(['active', 'expired', 'revoked']);

  for (const row of rows) {
    const waiverId = cleanText(row.waiver_id, 120);
    const reason = cleanText(row.reason, 240);
    const approver = cleanText(row.approver, 120);
    const expiresAt = cleanText(row.expires_at, 80);
    const checkIds = uniqueNonEmpty(row.check_ids);
    const status = cleanText(row.status || 'active', 40).toLowerCase();

    if (waiverId) {
      if (seenWaiverIds.has(waiverId)) duplicateWaiverIds.add(waiverId);
      seenWaiverIds.add(waiverId);
    }
    if (!waiverId || !reason || !approver || !expiresAt || checkIds.length === 0) {
      violations.push(`${waiverId || 'unknown_waiver'}:missing_required_fields`);
      continue;
    }
    if (!allowedStatuses.has(status)) {
      violations.push(`${waiverId}:invalid_status_${status || 'unknown'}`);
      continue;
    }
    if (hasPlaceholderToken(approver)) {
      violations.push(`${waiverId}:placeholder_approver`);
      continue;
    }

    const expiresMs = parseIsoMs(expiresAt);
    if (expiresMs == null) {
      violations.push(`${waiverId}:invalid_expires_at`);
      continue;
    }
    if (status === 'active' && expiresMs <= now) {
      violations.push(`${waiverId}:expired_active_waiver`);
      continue;
    }

    if (status === 'active' && expiresMs > now) {
      activeWaivers.push({
        waiverId,
        checkIds,
        reason,
        approver,
        expiresAt,
      });
    }
  }
  if (duplicateWaiverIds.size > 0) {
    violations.push(`duplicate_waiver_ids:${Array.from(duplicateWaiverIds).join(',')}`);
  }

  return {
    check: {
      id: 'waiver_contract',
      ok: violations.length === 0,
      detail:
        violations.length === 0
          ? `path=${waiversPath};active_waivers=${activeWaivers.length};total_waivers=${rows.length}`
          : `violations=${violations.join('|')}`,
    },
    activeWaivers,
  };
}

function applyWaivers(checks: GateCheck[], activeWaivers: ActiveWaiver[]): {
  checks: GateCheck[];
  waiversApplied: Array<{ waiver_id: string; check_id: string; expires_at: string; approver: string; reason: string }>;
} {
  const waiversApplied: Array<{ waiver_id: string; check_id: string; expires_at: string; approver: string; reason: string }> = [];
  if (activeWaivers.length === 0) return { checks, waiversApplied };

  const waivedChecks = checks.map((check) => {
    if (check.ok) return check;
    if (check.id === 'policy_file' || check.id === 'waiver_contract') return check;
    const match = activeWaivers.find((waiver) => waiver.checkIds.includes('*') || waiver.checkIds.includes(check.id));
    if (!match) return check;
    waiversApplied.push({
      waiver_id: match.waiverId,
      check_id: check.id,
      expires_at: match.expiresAt,
      approver: match.approver,
      reason: match.reason,
    });
    return {
      id: check.id,
      ok: true,
      detail: `${check.detail};waived_by=${match.waiverId};waiver_expires_at=${match.expiresAt}`,
    };
  });

  return { checks: waivedChecks, waiversApplied };
}

function toMarkdown(payload: {
  generated_at: string;
  revision: string;
  ok: boolean;
  inputs: Record<string, unknown>;
  checks: GateCheck[];
  failures: string[];
  waivers_applied: Array<{ waiver_id: string; check_id: string; expires_at: string; approver: string; reason: string }>;
}): string {
  const lines: string[] = [];
  lines.push('# Incident Operations Governance Gate');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Inputs');
  lines.push(`- Policy: ${payload.inputs.policy_path}`);
  lines.push(`- Process Doc: ${payload.inputs.process_path}`);
  lines.push(`- Owner Roster: ${payload.inputs.owner_roster_path}`);
  lines.push(`- Artifact Schema: ${payload.inputs.artifact_schema_path}`);
  lines.push(`- Waivers: ${payload.inputs.waivers_path}`);
  lines.push('');
  lines.push('## Checks');
  for (const check of payload.checks) {
    lines.push(`-   — `);
  }
  if (payload.waivers_applied.length > 0) {
    lines.push('');
    lines.push('## Waivers Applied');
    for (const waiver of payload.waivers_applied) {
      lines.push(`- ${waiver.waiver_id} -> ${waiver.check_id} (expires ${waiver.expires_at}; approver ${waiver.approver})`);
    }
  }
  if (payload.failures.length) {
    lines.push('');
    lines.push('## Failures');
    for (const row of payload.failures) lines.push(`- ${row}`);
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function run(argv: string[]): number {
  const args = resolveArgs(argv);
  const policy = readJson<GovernancePolicy>(args.policyPath);
  const checks: GateCheck[] = [];

  if (!policy) {
    checks.push({ id: 'policy_file', ok: false, detail: `missing_or_invalid:${args.policyPath}` });
    const payload = {
      gate: 'ops:incident-governance:gate',
      generated_at: new Date().toISOString(),
      revision: currentRevision(ROOT),
      ok: false,
      checks,
      failures: checks.map((check) => `${check.id}:${check.detail}`),
      waivers_applied: [],
      inputs: {
        policy_path: args.policyPath,
        process_path: args.processPath || DEFAULT_PROCESS_PATH,
        owner_roster_path: args.ownerRosterPath || DEFAULT_OWNER_ROSTER_PATH,
        artifact_schema_path: args.artifactSchemaPath || DEFAULT_ARTIFACT_SCHEMA_PATH,
        waivers_path: args.waiversPath || DEFAULT_WAIVERS_PATH,
      },
    };
    writeJsonArtifact(args.outJson, payload);
    writeTextArtifact(args.outMarkdown, toMarkdown(payload));
    return emitStructuredResult(payload, {
      outPath: args.outJson,
      strict: args.strict,
      ok: false,
    });
  }

  const processPath = resolveInputPath(
    args.processPath,
    policy.process_contract?.process_doc_path,
    DEFAULT_PROCESS_PATH,
  );
  const ownerRosterPath = resolveInputPath(
    args.ownerRosterPath,
    policy.hard_gates?.ownership_map?.roster_path,
    DEFAULT_OWNER_ROSTER_PATH,
  );
  const artifactSchemaPath = resolveInputPath(
    args.artifactSchemaPath,
    policy.hard_gates?.post_incident_artifacts?.schema_path,
    DEFAULT_ARTIFACT_SCHEMA_PATH,
  );
  const waiversPath = resolveInputPath(
    args.waiversPath,
    policy.hard_gates?.waiver_contract?.waivers_path,
    DEFAULT_WAIVERS_PATH,
  );

  checks.push({ id: 'policy_file', ok: true, detail: `path=${args.policyPath}` });
  checks.push(checkSeverityTaxonomy(policy));
  checks.push(checkOwnershipMap(policy));
  checks.push(checkOwnerRoster(policy, ownerRosterPath));
  checks.push(checkEscalationSla(policy));
  checks.push(checkRollbackCriteria(policy));
  checks.push(checkPostIncidentArtifactSchema(policy, artifactSchemaPath));
  checks.push(checkPolicyContracts(policy));
  checks.push(checkPolicyContractRequiredTokens(policy));
  checks.push(checkWaiverPolicyContract(policy));
  checks.push(checkProcessDoc(policy, processPath));

  const waiverEvaluation = checkWaiverContract(waiversPath);
  checks.push(waiverEvaluation.check);

  const waivedResult = applyWaivers(checks, waiverEvaluation.activeWaivers);
  const finalChecks = waivedResult.checks;
  const failures = finalChecks.filter((check) => !check.ok).map((check) => `${check.id}:${check.detail}`);
  const ok = failures.length === 0;

  const payload = {
    gate: 'ops:incident-governance:gate',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    ok,
    checks: finalChecks,
    failures,
    waivers_applied: waivedResult.waiversApplied,
    inputs: {
      policy_path: args.policyPath,
      process_path: processPath,
      owner_roster_path: ownerRosterPath,
      artifact_schema_path: artifactSchemaPath,
      waivers_path: waiversPath,
    },
  };

  writeJsonArtifact(args.outJson, payload);
  writeTextArtifact(args.outMarkdown, toMarkdown(payload));

  return emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok,
  });
}

process.exit(run(process.argv.slice(2)));
