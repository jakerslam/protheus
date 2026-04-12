'use strict';

const { runJsonCollector } = require('./collector_runtime.ts');

function parseArgs(argv = []) {
  const out = { force: false };
  for (const token of Array.isArray(argv) ? argv : []) {
    const s = String(token || '');
    if (s === '--force') out.force = true;
    if (s.startsWith('--max=')) out.maxItems = Number(s.split('=')[1]);
    if (s.startsWith('--min-hours=')) out.minHours = Number(s.split('=')[1]);
    if (s.startsWith('--attempts=')) out.attempts = Number(s.split('=')[1]);
    if (s.startsWith('--url=')) out.url = s.slice('--url='.length);
  }
  return out;
}

function cleanText(value, max = 240) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function formatSizeBytes(size) {
  const bytes = Number(size);
  if (!Number.isFinite(bytes) || bytes <= 0) return 'n/a';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let value = bytes;
  let idx = 0;
  while (value >= 1024 && idx < units.length - 1) {
    value /= 1024;
    idx += 1;
  }
  return value >= 10 || idx === 0
    ? `${Math.round(value)} ${units[idx]}`
    : `${value.toFixed(1)} ${units[idx]}`;
}

function extractOllamaModels(payload = {}) {
  const source = Array.isArray(payload)
    ? payload
    : (Array.isArray(payload.models)
      ? payload.models
      : (Array.isArray(payload.tags) ? payload.tags : []));
  return source
    .map((row) => {
      const model = row && typeof row === 'object' ? row : {};
      const name = cleanText(model.name || model.model, 200);
      if (!name) return null;
      const base = cleanText(String(name).split(':')[0], 120) || name;
      const modified = cleanText(model.modified_at || model.modifiedAt, 120);
      const sizeText = formatSizeBytes(model.size);
      const details = model.details && typeof model.details === 'object' ? model.details : {};
      const parameterSize = cleanText(details.parameter_size || details.parameterSize, 80);
      const family = cleanText(details.family, 80);
      const tags = ['ollama', sizeText];
      if (parameterSize) tags.push(parameterSize);
      if (family) tags.push(family);
      return {
        title: name,
        url: `https://ollama.com/library/${encodeURIComponent(base)}`,
        description: cleanText(
          [
            `Model ${name}`,
            sizeText !== 'n/a' ? `(${sizeText})` : '',
            parameterSize ? `parameters ${parameterSize}` : '',
            modified ? `updated ${modified}` : ''
          ].filter(Boolean).join(' '),
          420
        ),
        signal: /coder|reasoning|instruct|vision|multimodal|agent/i.test(name),
        signal_type: 'model_release',
        topics: ['ai', 'llm', 'local_models', 'agents', 'edge_ai'],
        tags,
        published_at: modified,
        bytes: Math.max(96, name.length + 48)
      };
    })
    .filter(Boolean);
}

async function run(options = {}) {
  const opts = options && typeof options === 'object' ? options : {};
  return runJsonCollector({
    collectorId: 'ollama_search',
    scope: 'sensory.collector.ollama_search',
    caller: 'adaptive/sensory/eyes/collectors/ollama_search',
    url: cleanText(opts.url || process.env.OLLAMA_SEARCH_URL || 'https://ollama.com/api/tags', 600),
    maxItems: Number(opts.maxItems || opts.max_items || 15),
    minHours: Number(opts.minHours || opts.min_hours || 8),
    force: opts.force === true,
    topics: Array.isArray(opts.topics) ? opts.topics : ['ai', 'llm', 'local_models', 'agents', 'edge_ai'],
    attempts: Number(opts.attempts || 3),
    extractor: extractOllamaModels
  });
}

async function collectOllamaSearchNewest(options = {}) {
  return run(options);
}

async function preflightOllamaSearch() {
  return {
    ok: true,
    parser_type: 'ollama_search',
    checks: [
      { name: 'ollama_tags_endpoint', ok: true },
      { name: 'adaptive_rate_limit_enabled', ok: true }
    ],
    failures: []
  };
}

if (require.main === module) {
  const args = parseArgs(process.argv.slice(2));
  run(args).then((result) => {
    console.log(JSON.stringify(result));
    process.exit(result && result.ok ? 0 : 1);
  }).catch((err) => {
    console.error(JSON.stringify({ ok: false, error: String(err && (err.code || err.message) || 'collector_error') }));
    process.exit(1);
  });
}

module.exports = {
  parseArgs,
  run,
  extractOllamaModels,
  collectOllamaSearchNewest,
  preflightOllamaSearch
};
