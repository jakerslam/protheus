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

function extractFromHfPapers(payload) {
  const rows = Array.isArray(payload) ? payload : [];
  return rows.map((paper) => {
    const id = cleanText(paper && paper.id, 80);
    const title = cleanText(paper && paper.title, 220);
    const url = id ? `https://huggingface.co/papers/${encodeURIComponent(id)}` : '';
    const summary = cleanText((paper && (paper.summary || paper.ai_summary)) || '', 420);
    const githubRepo = cleanText(paper && paper.githubRepo, 500);
    const tags = ['papers_with_code_mirror'];
    if (githubRepo) tags.push('code_available');
    return {
      title,
      url,
      description: summary,
      signal: Boolean(githubRepo),
      signal_type: githubRepo ? 'paper_with_repo' : 'paper',
      topics: ['research', 'ai', 'papers', 'code'],
      tags,
      published_at: cleanText(paper && paper.publishedAt, 120),
      bytes: Math.max(96, summary.length + title.length + githubRepo.length)
    };
  }).filter((row) => row.title && row.url);
}

async function run(options = {}) {
  const opts = options && typeof options === 'object' ? options : {};
  return runJsonCollector({
    collectorId: 'papers_with_code',
    scope: 'sensory.collector.dynamic',
    caller: 'adaptive/sensory/eyes/collectors/papers_with_code',
    // PWC now redirects to HuggingFace papers; this collector preserves parser contract.
    url: 'https://huggingface.co/api/papers',
    maxItems: Number(opts.maxItems || opts.max_items || 15),
    minHours: Number(opts.minHours || opts.min_hours || 8),
    force: opts.force === true,
    extractor: extractFromHfPapers,
    topics: Array.isArray(opts.topics) ? opts.topics : ['research', 'ai', 'papers', 'code'],
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
  extractFromHfPapers
};
