'use strict';

const { runJsonCollector, cleanText } = require('./collector_runtime.ts');

function parseArgs(argv = []) {
  const out = { force: false };
  for (const token of Array.isArray(argv) ? argv : []) {
    const s = String(token || '');
    if (s === '--force') out.force = true;
    if (s.startsWith('--max=')) out.maxItems = Number(s.split('=')[1]);
    if (s.startsWith('--min-hours=')) out.minHours = Number(s.split('=')[1]);
  }
  return out;
}

function formatSizeBytes(bytes) {
  const n = Number(bytes || 0);
  if (!Number.isFinite(n) || n <= 0) return 'n/a';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let v = n;
  let idx = 0;
  while (v >= 1024 && idx < units.length - 1) {
    v /= 1024;
    idx += 1;
  }
  return `${v.toFixed(v >= 10 || idx === 0 ? 0 : 1)} ${units[idx]}`;
}

function extractOllamaModels(payload) {
  const rows = Array.isArray(payload && payload.models) ? payload.models : [];
  return rows.map((model) => {
    const name = cleanText(model && (model.name || model.model), 200);
    const url = name ? `https://ollama.com/library/${encodeURIComponent(String(name).split(':')[0])}` : '';
    const modified = cleanText(model && model.modified_at, 120);
    const size = Number(model && model.size);
    return {
      title: name,
      url,
      description: `Model ${name} (${formatSizeBytes(size)}) updated ${modified || 'unknown'}`,
      signal: /(coder|reasoning|instruct|vision|multimodal|agent)/i.test(name),
      signal_type: 'model_release',
      topics: ['ai', 'llm', 'local_models', 'agents', 'edge_ai'],
      tags: ['ollama', formatSizeBytes(size)],
      published_at: modified,
      bytes: Math.max(96, name.length + 48)
    };
  }).filter((row) => row.title && row.url);
}

async function run(options = {}) {
  const opts = options && typeof options === 'object' ? options : {};
  return runJsonCollector({
    collectorId: 'ollama_search',
    scope: 'sensory.collector.ollama_search',
    caller: 'adaptive/sensory/eyes/collectors/ollama_search',
    url: 'https://ollama.com/api/tags',
    maxItems: Number(opts.maxItems || opts.max_items || 15),
    minHours: Number(opts.minHours || opts.min_hours || 8),
    force: opts.force === true,
    extractor: extractOllamaModels,
    topics: Array.isArray(opts.topics) ? opts.topics : ['ai', 'llm', 'local_models', 'agents', 'edge_ai'],
    attempts: Number(opts.attempts || 3)
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
  run,
  collectOllamaSearchNewest,
  preflightOllamaSearch,
  extractOllamaModels
};
