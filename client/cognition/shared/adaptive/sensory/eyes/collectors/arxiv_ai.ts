'use strict';

const { runFeedCollector } = require('./collector_runtime.ts');

function parseArgs(argv = []) {
  const out = { force: false };
  for (const token of Array.isArray(argv) ? argv : []) {
    const s = String(token || '');
    if (s === '--force') out.force = true;
    if (s.startsWith('--max=')) out.maxItems = Number(s.split('=')[1]);
    if (s.startsWith('--min-hours=')) out.minHours = Number(s.split('=')[1]);
    if (s.startsWith('--category=')) out.category = String(s.split('=')[1] || 'cs.AI');
  }
  return out;
}

async function run(options = {}) {
  const opts = options && typeof options === 'object' ? options : {};
  const category = String(opts.category || 'cs.AI').trim() || 'cs.AI';
  const base = `https://rss.arxiv.org/rss/${encodeURIComponent(category)}`;
  return runFeedCollector({
    collectorId: 'arxiv_ai',
    scope: 'sensory.collector.dynamic',
    caller: 'adaptive/sensory/eyes/collectors/arxiv_ai',
    feedCandidates: [base, 'https://rss.arxiv.org/rss/cs.AI'],
    maxItems: Number(opts.maxItems || opts.max_items || 15),
    minHours: Number(opts.minHours || opts.min_hours || 12),
    force: opts.force === true,
    topics: Array.isArray(opts.topics)
      ? opts.topics
      : ['research', 'ai', 'papers', 'arxiv'],
    signalRegex: /(llm|agent|alignment|benchmark|safety|reasoning|multimodal|retrieval)/i,
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

module.exports = { run };
