#!/usr/bin/env node
'use strict';

const { runCoordinatorCli } = require('./coordinator_cli.ts');
const { invokeOrchestration } = require('./core_bridge.ts');

function partitionWork(items, agentCount = 1, scopes = []) {
  return invokeOrchestration('coordinator.partition', {
    items: Array.isArray(items) ? items : [],
    agent_count: Number.isFinite(Number(agentCount)) ? Number(agentCount) : 1,
    scopes: Array.isArray(scopes) ? scopes : [],
  });
}

function mergeFindings(findings) {
  return invokeOrchestration('coordinator.merge_findings', {
    findings: Array.isArray(findings) ? findings : [],
  });
}

function assignScopesToPartitions(partitions) {
  return Array.isArray(partitions) ? partitions : [];
}

function runCoordinator(input = {}) {
  return invokeOrchestration('coordinator.run', {
    ...(input && typeof input === 'object' ? input : {}),
  });
}

function run(argv = process.argv.slice(2)) {
  return runCoordinatorCli(argv, {
    runCoordinator,
    partitionWork,
    loadScratchpad(taskId, options = {}) {
      const out = invokeOrchestration('scratchpad.status', {
        task_id: String(taskId || '').trim(),
        root_dir: options.rootDir || options.root_dir || undefined,
      });
      return {
        exists: Boolean(out && out.exists),
        filePath: out && out.file_path ? out.file_path : null,
        scratchpad: out && out.scratchpad && typeof out.scratchpad === 'object'
          ? out.scratchpad
          : { progress: { processed: 0, total: 0 }, findings: [], checkpoints: [] },
      };
    },
    handleTimeout(taskId, metrics = {}, options = {}) {
      return invokeOrchestration('checkpoint.timeout', {
        task_id: String(taskId || '').trim(),
        metrics: metrics && typeof metrics === 'object' ? metrics : {},
        root_dir: options.rootDir || options.root_dir || undefined,
      });
    },
  });
}

if (require.main === module) {
  const out = run(process.argv.slice(2));
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
  process.exit(out.ok ? 0 : 1);
}

module.exports = {
  partitionWork,
  mergeFindings,
  assignScopesToPartitions,
  runCoordinator,
  run,
};
