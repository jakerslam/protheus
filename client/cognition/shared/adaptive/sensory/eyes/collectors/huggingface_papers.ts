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

function extractPapers(payload) {
  const rows = Array.isArray(payload) ? payload : [];
  return rows.map((paper) => {
    const id = cleanText(paper && paper.id, 80);
    const title = cleanText(paper && paper.title, 220);
    const url = id ? `https://huggingface.co/papers/${encodeURIComponent(id)}` : '';
    const summary = cleanText((paper && (paper.summary || paper.ai_summary)) || '', 420);
    const upvotes = Number((paper && paper.upvotes) || 0);
    return {
      title,
      url,
      description: summary,
      signal: upvotes >= 20,
      signal_type: upvotes >= 20 ? 'high_upvote_paper' : 'paper',
      topics: ['research', 'ai', 'papers', 'huggingface'],
      tags: ['huggingface', `upvotes:${Number.isFinite(upvotes) ? upvotes : 0}`],
      published_at: cleanText(paper && paper.publishedAt, 120),
      bytes: Math.max(96, summary.length + title.length + 24)
    };
  }).filter((row) => row.title && row.url);
}

async function run(options = {}) {
  const opts = options && typeof options === 'object' ? options : {};
  return runJsonCollector({
    collectorId: 'huggingface_papers',
    scope: 'sensory.collector.dynamic',
    caller: 'adaptive/sensory/eyes/collectors/huggingface_papers',
    url: 'https://huggingface.co/api/papers',
    maxItems: Number(opts.maxItems || opts.max_items || 15),
    minHours: Number(opts.minHours || opts.min_hours || 8),
    force: opts.force === true,
    extractor: extractPapers,
    topics: Array.isArray(opts.topics) ? opts.topics : ['research', 'ai', 'papers', 'huggingface'],
    attempts: Number(opts.attempts || 3)
  });
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
  extractPapers
};
