#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

require.extensions['.ts'] = function compileTsAsJs(module, filename) {
  const source = fs.readFileSync(filename, 'utf8');
  module._compile(source, filename);
};

const mod = require(path.resolve(__dirname, '../../client/runtime/systems/ops/local_runtime_partitioner.ts'));

function writeFile(filePath, body) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, body);
}

function makeWorkspace() {
  return fs.mkdtempSync(path.join(os.tmpdir(), 'local-runtime-partitioner-'));
}

function templateSeed(root) {
  const templateDir = path.join(root, 'docs', 'workspace', 'templates', 'assistant');
  for (const name of ['SOUL.md', 'USER.md', 'HEARTBEAT.md', 'IDENTITY.md', 'TOOLS.md', 'MEMORY.md']) {
    writeFile(path.join(templateDir, name), `template:${name}\n`);
  }
}

function testInitMigratesRootAndGeneratesMissing() {
  const workspace = makeWorkspace();
  templateSeed(workspace);
  writeFile(path.join(workspace, 'SOUL.md'), 'root soul\n');
  writeFile(path.join(workspace, 'local', 'workspace', 'assistant', 'MEMORY.md'), 'existing memory\n');

  const out = mod.run(['init'], { workspaceRoot: workspace });

  assert.equal(out.ok, true);
  assert.deepEqual(out.migrated_root_files, ['SOUL.md']);
  assert.ok(out.generated_files.includes('USER.md'));
  assert.equal(
    fs.readFileSync(path.join(workspace, 'local', 'workspace', 'assistant', 'SOUL.md'), 'utf8'),
    'root soul\n'
  );
  assert.equal(
    fs.readFileSync(path.join(workspace, 'local', 'workspace', 'assistant', 'USER.md'), 'utf8'),
    'template:USER.md\n'
  );
  assert.equal(fs.existsSync(path.join(workspace, 'SOUL.md')), false);
  assert.equal(fs.existsSync(path.join(workspace, 'local', 'workspace', 'reports')), true);
}

function testResetArchivesExistingAssistantAndRestoresTemplates() {
  const workspace = makeWorkspace();
  templateSeed(workspace);
  writeFile(path.join(workspace, 'local', 'workspace', 'assistant', 'TOOLS.md'), 'custom tools\n');
  writeFile(path.join(workspace, 'TOOLS.md'), 'root tools\n');

  const out = mod.run(['reset', '--confirm=RESET_LOCAL'], { workspaceRoot: workspace });

  assert.equal(out.ok, true);
  assert.ok(out.assistant_archive_dir);
  assert.deepEqual(out.archived_root_files, ['TOOLS.md']);
  assert.equal(
    fs.readFileSync(path.join(workspace, 'local', 'workspace', 'assistant', 'TOOLS.md'), 'utf8'),
    'template:TOOLS.md\n'
  );
  assert.equal(
    fs.readFileSync(path.join(out.assistant_archive_dir, 'TOOLS.md'), 'utf8'),
    'custom tools\n'
  );
}

function main() {
  testInitMigratesRootAndGeneratesMissing();
  testResetArchivesExistingAssistantAndRestoresTemplates();
  console.log(JSON.stringify({ ok: true, type: 'local_runtime_partitioner_test' }));
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
