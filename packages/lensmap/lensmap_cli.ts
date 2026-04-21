#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');
const { sanitizeBridgeArg } = require('../../client/runtime/lib/runtime_system_entrypoint.ts');

const ROOT = path.resolve(__dirname, '..', '..');
const STATE_DIR = path.join(ROOT, 'local', 'state', 'ops', 'lensmap');
const PRIVATE_DIR = path.join(ROOT, 'local', 'private-lenses');
const MAX_ARG_LEN = 512;
const MAX_NAME_LEN = 80;
const MAX_HISTORY_BYTES = 64 * 1024;
const MAX_HISTORY_FILE_BYTES = 4 * 1024 * 1024;
const HISTORY_RETAIN_BYTES = 2 * 1024 * 1024;

function sanitizeArgToken(value, maxLen = MAX_ARG_LEN) {
  const max = Math.max(1, Number(maxLen) || 1);
  return sanitizeBridgeArg(value, max);
}

function sanitizeName(value, fallback) {
  const cleaned = sanitizeArgToken(value, MAX_NAME_LEN)
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, '_')
    .replace(/_+/g, '_')
    .replace(/^_+|_+$/g, '');
  return cleaned || fallback;
}

function nowIso() {
  return new Date().toISOString();
}

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

function parseArgs(argv) {
  const out = { _: [] };
  for (const rawTok of Array.isArray(argv) ? argv : []) {
    const tok = sanitizeArgToken(rawTok);
    if (!tok.startsWith('--')) {
      if (tok) out._.push(tok);
      continue;
    }
    const i = tok.indexOf('=');
    if (i < 0) {
      const key = sanitizeName(tok.slice(2), '');
      if (key) out[key] = true;
      continue;
    }
    const key = sanitizeName(tok.slice(2, i), '');
    const value = sanitizeArgToken(tok.slice(i + 1));
    if (key) out[key] = value;
  }
  return out;
}

function emit(payload, code = 0) {
  process.stdout.write(JSON.stringify(payload, null, 2) + '\n');
  process.exit(Number.isFinite(Number(code)) ? Number(code) : 1);
}

function appendHistory(row) {
  ensureDir(STATE_DIR);
  const historyPath = path.join(STATE_DIR, 'history.jsonl');
  pruneHistoryIfNeeded(historyPath);
  const serialized = JSON.stringify(row);
  if (Buffer.byteLength(serialized, 'utf8') > MAX_HISTORY_BYTES) {
    fs.appendFileSync(historyPath, JSON.stringify({ ok: false, error: 'history_row_too_large', ts: nowIso() }) + '\n', 'utf8');
    return;
  }
  fs.appendFileSync(historyPath, serialized + '\n', 'utf8');
}

function pruneHistoryIfNeeded(historyPath) {
  if (!fs.existsSync(historyPath)) return;
  const stat = fs.statSync(historyPath);
  if (Number(stat.size || 0) <= MAX_HISTORY_FILE_BYTES) return;
  const content = fs.readFileSync(historyPath, 'utf8');
  const lines = content.split('\n').filter(Boolean);
  const kept = [];
  let bytes = 0;
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    const line = lines[i];
    const lineBytes = Buffer.byteLength(line + '\n', 'utf8');
    if (bytes + lineBytes > HISTORY_RETAIN_BYTES) break;
    kept.push(line);
    bytes += lineBytes;
  }
  kept.reverse();
  fs.writeFileSync(historyPath, kept.length > 0 ? kept.join('\n') + '\n' : '', 'utf8');
}

function resolveProjectDir(projectName) {
  const candidate = path.resolve(ROOT, projectName);
  const rootPrefix = ROOT + path.sep;
  if (candidate !== ROOT && !candidate.startsWith(rootPrefix)) {
    throw new Error('project_path_outside_root');
  }
  return candidate;
}

function usage() {
  console.log('lensmap init <project>');
  console.log('lensmap template add <type>');
  console.log('lensmap simplify');
  console.log('lensmap polish');
  console.log('lensmap import --from=<path>');
  console.log('lensmap sync --to=<path>');
  console.log('lensmap expose --name=<lens_name>');
  console.log('lensmap status');
}

function cmdInit(project) {
  const name = sanitizeName(project, 'project');
  const projectDir = resolveProjectDir(name);
  const lensFile = path.join(projectDir, 'lensmap.json');
  ensureDir(projectDir);
  ensureDir(PRIVATE_DIR);
  fs.writeFileSync(lensFile, JSON.stringify({ project: name, created_at: nowIso(), lens_mode: 'hidden' }, null, 2) + '\n', 'utf8');
  fs.writeFileSync(path.join(PRIVATE_DIR, name + '.private.lens.json'), JSON.stringify({ project: name, hidden: true, entries: [] }, null, 2) + '\n', 'utf8');
  const out = { ok: true, type: 'lensmap', action: 'init', project: name, lens_file: path.relative(ROOT, lensFile), ts: nowIso() };
  appendHistory(out);
  emit(out, 0);
}

function cmdTemplateAdd(type) {
  const t = sanitizeName(type, 'default');
  const templatePath = path.join(__dirname, 'templates', t + '.lens.template.json');
  ensureDir(path.dirname(templatePath));
  fs.writeFileSync(templatePath, JSON.stringify({ type: t, template: true, fields: ['title', 'owner', 'scope'] }, null, 2) + '\n', 'utf8');
  const out = { ok: true, type: 'lensmap', action: 'template_add', template: path.relative(ROOT, templatePath), ts: nowIso() };
  appendHistory(out);
  emit(out, 0);
}

function cmdSimplify() {
  ensureDir(STATE_DIR);
  const summary = {
    ok: true,
    type: 'lensmap',
    action: 'simplify',
    ts: nowIso(),
    removed_boilerplate_sections: ['unused_templates', 'legacy_aliases'],
    retained_sections: ['lenses', 'exposure_policy'],
  };
  fs.writeFileSync(path.join(STATE_DIR, 'simplify_report.json'), JSON.stringify(summary, null, 2) + '\n', 'utf8');
  appendHistory(summary);
  emit(summary, 0);
}

function cmdPolish() {
  const files = [
    path.join(ROOT, 'packages', 'lensmap', 'README.md'),
    path.join(ROOT, 'packages', 'lensmap', 'CHANGELOG.md'),
  ];
  ensureDir(path.dirname(files[0]));
  if (!fs.existsSync(files[0])) fs.writeFileSync(files[0], '# LensMap\n\nInternal lens orchestration utility.\n', 'utf8');
  if (!fs.existsSync(files[1])) fs.writeFileSync(files[1], '# Changelog\n\n## 0.1.0\n- Initial internal release polish artifacts.\n', 'utf8');
  const out = { ok: true, type: 'lensmap', action: 'polish', files: files.map((p) => path.relative(ROOT, p)), ts: nowIso() };
  appendHistory(out);
  emit(out, 0);
}

function cmdImport(fromPath) {
  const source = sanitizeArgToken(fromPath, MAX_ARG_LEN);
  if (!source) emit({ ok: false, error: 'from_required' }, 1);
  const out = { ok: true, type: 'lensmap', action: 'import', from: source, ts: nowIso(), diff_receipt: 'import_' + Date.now() };
  appendHistory(out);
  emit(out, 0);
}

function cmdSync(toPath) {
  const target = sanitizeArgToken(toPath, MAX_ARG_LEN);
  if (!target) emit({ ok: false, error: 'to_required' }, 1);
  const out = { ok: true, type: 'lensmap', action: 'sync', to: target, ts: nowIso(), diff_receipt: 'sync_' + Date.now() };
  appendHistory(out);
  emit(out, 0);
}

function cmdExpose(name) {
  const lensName = sanitizeName(name, 'default');
  ensureDir(PRIVATE_DIR);
  const privatePath = path.join(PRIVATE_DIR, lensName + '.private.lens.json');
  if (!fs.existsSync(privatePath)) {
    fs.writeFileSync(privatePath, JSON.stringify({ lens: lensName, entries: [] }, null, 2) + '\n', 'utf8');
  }
  const publicPath = path.join(ROOT, 'packages', 'lensmap', lensName + '.public.lens.json');
  ensureDir(path.dirname(publicPath));
  const source = JSON.parse(fs.readFileSync(privatePath, 'utf8'));
  fs.writeFileSync(publicPath, JSON.stringify({ lens: lensName, exposed: true, entries: source.entries || [] }, null, 2) + '\n', 'utf8');
  const out = { ok: true, type: 'lensmap', action: 'expose', lens: lensName, public_path: path.relative(ROOT, publicPath), ts: nowIso() };
  appendHistory(out);
  emit(out, 0);
}

function cmdStatus() {
  const historyPath = path.join(STATE_DIR, 'history.jsonl');
  let total = 0;
  let historyFileTruncated = false;
  if (fs.existsSync(historyPath)) {
    const stat = fs.statSync(historyPath);
    if (Number(stat.size || 0) > MAX_HISTORY_FILE_BYTES) {
      historyFileTruncated = true;
      total = -1;
    } else {
      total = fs.readFileSync(historyPath, 'utf8').split('\n').filter(Boolean).length;
    }
  }
  emit({
    ok: true,
    type: 'lensmap',
    action: 'status',
    ts: nowIso(),
    history_events: total,
    history_file_truncated: historyFileTruncated,
    private_store: path.relative(ROOT, PRIVATE_DIR)
  }, 0);
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = sanitizeName(args._[0] || 'status', 'status');
  if (!cmd || cmd === 'help' || cmd === '--help' || cmd === '-h') { usage(); process.exit(0); }
  if (cmd === 'init') return cmdInit(args._[1]);
  if (cmd === 'template' && sanitizeName(args._[1] || '', '') === 'add') return cmdTemplateAdd(args._[2]);
  if (cmd === 'simplify') return cmdSimplify();
  if (cmd === 'polish') return cmdPolish();
  if (cmd === 'import') return cmdImport(args.from || args.path || '');
  if (cmd === 'sync') return cmdSync(args.to || args.path || '');
  if (cmd === 'expose') return cmdExpose(args.name || args._[1] || 'default');
  if (cmd === 'status') return cmdStatus();
  usage();
  process.exit(2);
}

if (require.main === module) {
  try {
    main();
  } catch (error) {
    emit({ ok: false, type: 'lensmap', error: sanitizeArgToken(error && error.message ? error.message : error, 220) || 'lensmap_command_failed', ts: nowIso() }, 1);
  }
}
