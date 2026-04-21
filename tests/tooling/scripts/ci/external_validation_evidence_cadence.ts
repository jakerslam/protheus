#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type CadenceLane = {
  id: string;
  owner: string;
  cadence_days: number;
  command_id: string;
  evidence_artifact: string;
  publication_surface: string;
};

type CadencePlan = {
  schema_id: string;
  schema_version: string;
  cadence_days_default: number;
  required_workflow_path: string;
  required_workflow_commands: string[];
  lanes: CadenceLane[];
};

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/external_validation_evidence_cadence_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 500),
    outMarkdown: cleanText(
      readFlag(argv, 'out-markdown') ||
        'local/workspace/reports/EXTERNAL_VALIDATION_EVIDENCE_CADENCE_CURRENT.md',
      500,
    ),
    planPath: cleanText(
      readFlag(argv, 'plan') || 'client/runtime/config/external_validation_evidence_cadence.json',
      500,
    ),
    packagePath: cleanText(readFlag(argv, 'package') || 'package.json', 500),
  };
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# External Validation Evidence Cadence');
  lines.push('');
  lines.push(`- generated_at: ${payload.generated_at}`);
  lines.push(`- revision: ${payload.revision}`);
  lines.push(`- pass: ${payload.ok}`);
  lines.push(`- strict: ${payload.strict}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- lanes: ${payload.summary.lanes}`);
  lines.push(`- workflow_commands_checked: ${payload.summary.workflow_commands_checked}`);
  lines.push(`- failures: ${payload.summary.failures}`);
  lines.push('');
  lines.push('| lane | owner | cadence_days | command | artifact | publication_surface | pass |');
  lines.push('| --- | --- | ---: | --- | --- | --- | --- |');
  for (const lane of payload.lanes || []) {
    lines.push(
      `| ${cleanText(lane.id || '', 100)} | ${cleanText(lane.owner || '', 100)} | ${lane.cadence_days} | ${cleanText(
        lane.command_id || '',
        120,
      )} | ${cleanText(lane.evidence_artifact || '', 140)} | ${cleanText(lane.publication_surface || '', 120)} | ${lane.ok ? 'true' : 'false'} |`,
    );
  }
  lines.push('');
  lines.push('## Failures');
  if (!Array.isArray(payload.failures) || payload.failures.length === 0) {
    lines.push('- none');
  } else {
    for (const failure of payload.failures) {
      lines.push(`- ${cleanText(failure.id || '', 120)}: ${cleanText(failure.detail || '', 240)}`);
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  const planAbs = path.resolve(root, args.planPath);
  const packageAbs = path.resolve(root, args.packagePath);

  let plan: CadencePlan;
  try {
    plan = JSON.parse(fs.readFileSync(planAbs, 'utf8')) as CadencePlan;
  } catch (error) {
    return emitStructuredResult(
      {
        ok: false,
        type: 'external_validation_evidence_cadence',
        error: 'cadence_plan_unavailable',
        detail: cleanText((error as Error)?.message || 'cadence_plan_unavailable', 240),
        plan_path: args.planPath,
      },
      { outPath: args.outPath, strict: args.strict, ok: false },
    );
  }

  let scripts: Record<string, string> = {};
  try {
    const packageJson = JSON.parse(fs.readFileSync(packageAbs, 'utf8')) as { scripts?: Record<string, string> };
    scripts = packageJson.scripts || {};
  } catch (error) {
    return emitStructuredResult(
      {
        ok: false,
        type: 'external_validation_evidence_cadence',
        error: 'package_json_unavailable',
        detail: cleanText((error as Error)?.message || 'package_json_unavailable', 240),
        package_path: args.packagePath,
      },
      { outPath: args.outPath, strict: args.strict, ok: false },
    );
  }

  const failures: Array<{ id: string; detail: string }> = [];
  const lanes = Array.isArray(plan.lanes) ? plan.lanes : [];
  const laneIdSet = new Set<string>();
  const laneRows = lanes.map((lane) => {
    const id = cleanText(lane.id || '', 120);
    const owner = cleanText(lane.owner || '', 120);
    const commandId = cleanText(lane.command_id || '', 160);
    const evidenceArtifact = cleanText(lane.evidence_artifact || '', 300);
    const publicationSurface = cleanText(lane.publication_surface || '', 240);
    const cadenceDays = Number(lane.cadence_days || plan.cadence_days_default || 0);
    let ok = true;

    if (!id) {
      failures.push({ id: 'cadence_lane_missing_id', detail: 'missing_id' });
      ok = false;
    } else if (laneIdSet.has(id)) {
      failures.push({ id: 'cadence_lane_duplicate_id', detail: id });
      ok = false;
    } else {
      laneIdSet.add(id);
    }

    if (!owner) {
      failures.push({ id: 'cadence_lane_missing_owner', detail: id || 'unknown' });
      ok = false;
    }
    if (!commandId) {
      failures.push({ id: 'cadence_lane_missing_command', detail: id || 'unknown' });
      ok = false;
    } else if (!scripts[commandId]) {
      failures.push({ id: 'cadence_lane_command_not_in_package', detail: `${id}:${commandId}` });
      ok = false;
    }
    if (!Number.isFinite(cadenceDays) || cadenceDays < 1 || cadenceDays > 30) {
      failures.push({ id: 'cadence_lane_invalid_days', detail: `${id}:${cadenceDays}` });
      ok = false;
    }
    if (!evidenceArtifact) {
      failures.push({ id: 'cadence_lane_missing_evidence_artifact', detail: id || 'unknown' });
      ok = false;
    } else if (
      !(
        evidenceArtifact.startsWith('core/local/artifacts/') ||
        evidenceArtifact.startsWith('local/workspace/reports/') ||
        evidenceArtifact.startsWith('releases/proof-packs/')
      )
    ) {
      failures.push({
        id: 'cadence_lane_artifact_path_outside_allowed_roots',
        detail: `${id}:${evidenceArtifact}`,
      });
      ok = false;
    }
    if (!publicationSurface) {
      failures.push({ id: 'cadence_lane_missing_publication_surface', detail: id || 'unknown' });
      ok = false;
    }

    return {
      id,
      owner,
      cadence_days: cadenceDays,
      command_id: commandId,
      evidence_artifact: evidenceArtifact,
      publication_surface: publicationSurface,
      ok,
    };
  });

  const workflowPath = cleanText(plan.required_workflow_path || '', 500);
  const workflowAbs = path.resolve(root, workflowPath || '.github/workflows/external-validation-evidence-cadence.yml');
  let workflowSource = '';
  if (!workflowPath || !fs.existsSync(workflowAbs)) {
    failures.push({
      id: 'cadence_required_workflow_missing',
      detail: workflowPath || '.github/workflows/external-validation-evidence-cadence.yml',
    });
  } else {
    workflowSource = fs.readFileSync(workflowAbs, 'utf8');
  }

  const workflowCommands = Array.isArray(plan.required_workflow_commands)
    ? plan.required_workflow_commands.map((row) => cleanText(String(row || ''), 160)).filter(Boolean)
    : [];
  for (const command of workflowCommands) {
    if (!workflowSource.includes(command)) {
      failures.push({
        id: 'cadence_required_workflow_command_missing',
        detail: command,
      });
    }
  }

  const payload = {
    ok: failures.length === 0,
    type: 'external_validation_evidence_cadence',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    strict: args.strict,
    plan_path: args.planPath,
    package_path: args.packagePath,
    summary: {
      lanes: laneRows.length,
      workflow_commands_checked: workflowCommands.length,
      failures: failures.length,
    },
    lanes: laneRows,
    failures,
  };

  writeTextArtifact(args.outMarkdown, markdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outPath,
    strict: args.strict,
    ok: payload.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
