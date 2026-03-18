#!/usr/bin/env node
'use strict';

// SRS coverage: V6-WORKFLOW-006.1, V6-WORKFLOW-006.2, V6-WORKFLOW-006.3,
// V6-WORKFLOW-006.4, V6-WORKFLOW-006.5, V6-WORKFLOW-006.6,
// V6-WORKFLOW-006.7, V6-WORKFLOW-006.8

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const ts = require('typescript');

if (!require.extensions['.ts']) {
  require.extensions['.ts'] = function compileTs(module, filename) {
    const source = fs.readFileSync(filename, 'utf8');
    const transpiled = ts.transpileModule(source, {
      compilerOptions: {
        module: ts.ModuleKind.CommonJS,
        target: ts.ScriptTarget.ES2022,
        moduleResolution: ts.ModuleResolutionKind.NodeJs,
        esModuleInterop: true,
        allowSyntheticDefaultImports: true
      },
      fileName: filename,
      reportDiagnostics: false
    }).outputText;
    module._compile(transpiled, filename);
  };
}

const bridge = require('../../client/runtime/systems/workflow/metagpt_bridge.ts');
const adapter = require('../../adapters/protocol/metagpt_config_bridge.ts');

function run() {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'metagpt-bridge-'));
  const statePath = path.join(tmpDir, 'state.json');
  const historyPath = path.join(tmpDir, 'history.jsonl');
  const approvalQueuePath = path.join(tmpDir, 'reviews.yaml');
  const tracePath = path.join(tmpDir, 'trace.jsonl');

  const company = bridge.registerCompany({
    company_name: 'launch_company',
    product_goal: 'ship launch plan',
    roles: [
      { role: 'pm', goal: 'shape scope' },
      { role: 'architect', goal: 'design system' },
      { role: 'engineer', goal: 'implement changes' }
    ],
    org_chart: ['pm', 'architect', 'engineer'],
    state_path: statePath,
    history_path: historyPath,
  });
  assert.strictEqual(Boolean(company.company.company_id), true);

  const sop = bridge.runSop({
    company_id: company.company.company_id,
    pipeline_name: 'prd_to_release',
    steps: [
      { name: 'requirements', owner: 'pm' },
      { name: 'design', owner: 'architect' },
      { name: 'build', owner: 'engineer' }
    ],
    checkpoint_labels: ['req', 'design', 'build'],
    budget: { tokens: 1800, max_stages: 3 },
    state_path: statePath,
    history_path: historyPath,
  });
  assert.strictEqual(sop.sop_run.stage_count, 3);

  const pr = adapter.simulatePr({
    task: 'add launch summary',
    changed_files: ['client/runtime/lib/metagpt_bridge.ts', 'docs/workspace/SRS.md'],
    generated_patch_summary: 'adds governed workflow surface',
    tests: ['node tests/client-memory-tools/metagpt_bridge.test.js'],
    sandbox_mode: 'readonly',
    state_path: statePath,
    history_path: historyPath,
  });
  assert.strictEqual(pr.pr_simulation.review_required, true);
  assert.strictEqual(pr.pr_simulation.bridge_path, 'adapters/protocol/metagpt_config_bridge.ts');

  const debate = bridge.runDebate({
    proposal: 'ship launch dashboard',
    participants: ['pm', 'architect', 'engineer'],
    rounds: 3,
    profile: 'tiny-max',
    context_budget: 800,
    recommendation: 'revise',
    state_path: statePath,
    history_path: historyPath,
  });
  assert.strictEqual(debate.debate.rounds, 2);
  assert.strictEqual(debate.debate.degraded, true);

  const plan = bridge.planRequirements({
    prd_title: 'Launch Assistant',
    requirements: ['capture FAQ context', 'draft launch reply'],
    stakeholders: ['ops', 'marketing'],
    auto_recall_query: 'launch assistant',
    state_path: statePath,
    history_path: historyPath,
  });
  assert.strictEqual(plan.requirements_plan.stories.length, 2);

  const oversight = bridge.recordOversight({
    operator_id: 'human-reviewer',
    action: 'approve',
    target_id: company.company.company_id,
    notes: 'company setup approved',
    approval_queue_path: approvalQueuePath,
    state_path: statePath,
    history_path: historyPath,
  });
  assert.strictEqual(oversight.oversight.action, 'approve');
  assert.strictEqual(fs.existsSync(approvalQueuePath), true);

  const trace = bridge.recordPipelineTrace({
    run_id: 'pipeline-1',
    stage: 'design',
    message: 'architect completed design',
    metrics: { latency_ms: 55 },
    trace_path: tracePath,
    state_path: statePath,
    history_path: historyPath,
  });
  assert.strictEqual(trace.pipeline_trace.stage, 'design');
  assert.strictEqual(fs.existsSync(tracePath), true);

  const config = adapter.ingestConfig({
    config_yaml: 'roles:\n  - pm\n  - engineer\nsops:\n  - requirements\n  - build\nextensions:\n  - docs\n',
    state_path: statePath,
    history_path: historyPath,
  });
  assert.strictEqual(config.config.bridge_path, 'adapters/protocol/metagpt_config_bridge.ts');
  assert.strictEqual(config.config.roles, 2);
  assert.strictEqual(config.config.sops, 2);

  const status = bridge.status({ state_path: statePath, history_path: historyPath });
  assert.strictEqual(status.companies, 1);
  assert.strictEqual(status.sop_runs, 1);
  assert.strictEqual(status.pr_simulations, 1);
  assert.strictEqual(status.debates, 1);
  assert.strictEqual(status.requirements, 1);
  assert.strictEqual(status.oversight, 1);
  assert.strictEqual(status.traces, 1);
  assert.strictEqual(status.configs, 1);

  console.log(JSON.stringify({ ok: true, type: 'metagpt_bridge_test' }));
}

run();
