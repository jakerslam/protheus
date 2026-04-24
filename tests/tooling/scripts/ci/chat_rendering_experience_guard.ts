import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { dirname } from 'path';

const SRS_ID = 'V12-CHAT-UX-CLOSURE-001';
const LEGACY_SRS_ID = 'V11-CHAT-UX-004';
const MANIFEST = 'tests/tooling/config/release_proof_pack_manifest.json';
const REGISTRY = 'tests/tooling/config/tooling_gate_registry.json';
const OUT_JSON = 'core/local/artifacts/chat_rendering_experience_guard_current.json';
const OUT_MARKDOWN = 'local/workspace/reports/CHAT_RENDERING_EXPERIENCE_GUARD_CURRENT.md';
const GATE_ID = 'ops:chat-rendering:experience:guard';

type Check = { id: string; ok: boolean; detail?: string };

type SourceContract = {
  path: string;
  tokens: string[];
};

const CONTRACTS: SourceContract[] = [
  {
    path: 'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/215-rendering-and-metadata-upgrades.ts',
    tokens: [
      'messageSourceChips',
      'messageHasSourceChips',
      'messageToolTraceSummary',
      'workspacePanelPayload',
      '_messageArtifactsForWorkspace',
      'assistantTurnMetadataFromPayload',
      'resolveMessageToolRows',
    ],
  },
  {
    path: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0003-body-part.html',
    tokens: [
      'message-source-chips',
      'messageToolTraceSummary(msg).visible',
      'messageCanRetryFromMeta',
      'messageCanReplyFromMeta',
      'messageCanForkFromMeta',
      'infring-workspace-panel-shell',
    ],
  },
  {
    path: 'client/runtime/systems/ui/infring_static/index_body.html.parts/0005-body-part.html',
    tokens: [
      'message-source-chips',
      'message-tool-trace-summary',
      'messageCanRetryFromMeta',
      'messageCanReplyFromMeta',
      'messageCanForkFromMeta',
      'chat-workspace-panel-section',
      'workspacePanelPayload().sources',
      'workspacePanelPayload().trace',
      'workspacePanelPayload().artifacts',
    ],
  },
  {
    path: 'client/runtime/systems/ui/infring_static/css/components.css.parts/0007-components-part.part02.css',
    tokens: [
      '.message-source-chips',
      '.message-source-chip',
      '.message-tool-trace-summary',
      '.thinking-inline-subtext',
      '.chat-workspace-panel',
      '.chat-workspace-source',
      '.chat-workspace-trace-row',
      '.chat-workspace-artifact-row',
    ],
  },
  {
    path: 'client/runtime/systems/ui/infring_static/css/components.css.parts/0007-components-part.css',
    tokens: [
      '.message-bubble.markdown-body .chat-codeblock',
      '.chat-codeblock-toolbar',
      '.chat-codeblock-copy',
      '.message-bubble.markdown-body .chat-table-wrap',
    ],
  },
  {
    path: 'client/runtime/systems/ui/infring_static/js/app.ts.parts/005-core-rendering-helpers.part01.ts',
    tokens: [
      'function dashboardWrapMarkdownCodeBlocks',
      'function dashboardWrapMarkdownTables',
      'function copyCode',
      'function toggleCodeFold',
      'chat-codeblock-copy',
    ],
  },
  {
    path: 'client/runtime/systems/ui/infring_static/js/app.ts.parts/010-core-state.part01.ts',
    tokens: [
      'function renderMarkdown',
      'dashboardWrapMarkdownCodeBlocks(html)',
      'dashboardWrapMarkdownTables(html)',
      'marked.setOptions',
    ],
  },
];

function arg(name: string, fallback: string): string {
  const prefix = `--${name}=`;
  return process.argv.find((item) => item.startsWith(prefix))?.slice(prefix.length) ?? fallback;
}

function flag(name: string, fallback: boolean): boolean {
  const value = arg(name, fallback ? '1' : '0').toLowerCase();
  return value === '1' || value === 'true' || value === 'yes';
}

function list(value: any): string[] {
  return Array.isArray(value) ? value.filter((item) => typeof item === 'string') : [];
}

function readJson(path: string): any {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function readText(path: string): string {
  return readFileSync(path, 'utf8');
}

function check(id: string, ok: boolean, detail?: string): Check {
  return detail ? { id, ok, detail } : { id, ok };
}

function ensureParent(path: string): void {
  mkdirSync(dirname(path), { recursive: true });
}

function requiredArtifacts(manifest: any): string[] {
  return list(manifest?.required_artifacts);
}

function workloadAndQualityArtifacts(manifest: any): string[] {
  return list(manifest?.artifact_groups?.workload_and_quality);
}

function registryArtifacts(registry: any, gateId: string): string[] {
  return list(registry?.gates?.[gateId]?.artifact_paths);
}

function registryRunnable(registry: any, gateId: string): boolean {
  const entry = registry?.gates?.[gateId];
  return Boolean(entry && (Array.isArray(entry.command) || typeof entry.script === 'string'));
}

function writeMarkdown(path: string, checks: Check[], pass: boolean): void {
  ensureParent(path);
  const lines = [
    '# Chat Rendering Experience Guard',
    '',
    `- pass: ${pass}`,
    `- srs_id: ${SRS_ID}`,
    `- legacy_srs_id: ${LEGACY_SRS_ID}`,
    '',
    '| Check | Status | Detail |',
    '| --- | --- | --- |',
    ...checks.map((row) => `| ${row.id} | ${row.ok ? 'pass' : 'fail'} | ${row.detail ?? ''} |`),
    '',
  ];
  writeFileSync(path, lines.join('\n'));
}

function main(): void {
  const manifestPath = arg('manifest', MANIFEST);
  const registryPath = arg('registry', REGISTRY);
  const outJson = arg('out-json', OUT_JSON);
  const outMarkdown = arg('out-markdown', OUT_MARKDOWN);
  const strict = flag('strict', true);
  const manifest = readJson(manifestPath);
  const registry = readJson(registryPath);
  const required = requiredArtifacts(manifest);
  const workload = workloadAndQualityArtifacts(manifest);
  const checks: Check[] = [
    check('chat_rendering_guard_required_in_proof_pack', required.includes(OUT_JSON), OUT_JSON),
    check('chat_rendering_guard_grouped_as_workload_quality', workload.includes(OUT_JSON), OUT_JSON),
    check('chat_rendering_guard_markdown_registry_exported', registryArtifacts(registry, GATE_ID).includes(OUT_MARKDOWN), OUT_MARKDOWN),
    check('chat_rendering_guard_registry_entry_runnable', registryRunnable(registry, GATE_ID)),
  ];
  for (const contract of CONTRACTS) {
    checks.push(check(`source_file_exists:${contract.path}`, existsSync(contract.path), contract.path));
    const source = existsSync(contract.path) ? readText(contract.path) : '';
    for (const token of contract.tokens) {
      checks.push(check(`source_token_present:${contract.path}:${token}`, source.includes(token), token));
    }
  }
  const pass = checks.every((row) => row.ok);
  const payload = {
    ok: pass,
    type: 'chat_rendering_experience_guard',
    srs_id: SRS_ID,
    legacy_srs_ids: [LEGACY_SRS_ID],
    generated_at: new Date().toISOString(),
    inputs: { manifest_path: manifestPath, registry_path: registryPath },
    summary: {
      pass,
      check_count: checks.length,
      source_file_count: CONTRACTS.length,
      required_feature_count: CONTRACTS.reduce((sum, row) => sum + row.tokens.length, 0),
    },
    checks,
    artifact_paths: [outJson, outMarkdown],
  };
  ensureParent(outJson);
  writeFileSync(outJson, `${JSON.stringify(payload, null, 2)}\n`);
  writeMarkdown(outMarkdown, checks, pass);
  console.log(JSON.stringify(payload, null, 2));
  if (strict && !pass) process.exit(1);
}

main();
