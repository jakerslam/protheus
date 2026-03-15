'use strict';

const { runJsonCollector, cleanText } = require('./collector_runtime.ts');

function parseArgs(argv = []) {
  const out = { force: false };
  for (const token of Array.isArray(argv) ? argv : []) {
    const s = String(token || '');
    if (s === '--force') out.force = true;
    if (s.startsWith('--max=')) out.maxItems = Number(s.split('=')[1]);
    if (s.startsWith('--min-hours=')) out.minHours = Number(s.split('=')[1]);
    if (s.startsWith('--venue=')) out.venue = String(s.split('=')[1] || 'ICLR 2026 Oral');
  }
  return out;
}

function extractOpenReviewNotes(payload) {
  const notes = Array.isArray(payload && payload.notes) ? payload.notes : [];
  return notes.map((note) => {
    const id = cleanText(note && note.id, 120);
    const content = note && note.content && typeof note.content === 'object' ? note.content : {};
    const title = cleanText(content.title && content.title.value, 240);
    const abstract = cleanText(content.abstract && content.abstract.value, 420);
    const venue = cleanText(content.venue && content.venue.value, 120);
    const url = id ? `https://openreview.net/forum?id=${encodeURIComponent(id)}` : '';
    const keywords = Array.isArray(content.keywords && content.keywords.value)
      ? content.keywords.value.slice(0, 3).map((v) => cleanText(v, 60)).filter(Boolean)
      : [];
    return {
      title,
      url,
      description: abstract || venue,
      signal: /(agent|llm|reasoning|retrieval|safety|alignment|benchmark)/i.test(`${title} ${abstract}`),
      signal_type: 'peer_review_paper',
      topics: ['research', 'peer_review', 'openreview', 'ai'],
      tags: ['openreview'].concat(keywords),
      published_at: cleanText(note && (note.pdate || note.cdate || note.mdate), 120),
      bytes: Math.max(96, title.length + abstract.length + 64)
    };
  }).filter((row) => row.title && row.url);
}

async function run(options = {}) {
  const opts = options && typeof options === 'object' ? options : {};
  const venue = String(opts.venue || 'ICLR 2026 Oral').trim() || 'ICLR 2026 Oral';
  return runJsonCollector({
    collectorId: 'openreview_venues',
    scope: 'sensory.collector.dynamic',
    caller: 'adaptive/sensory/eyes/collectors/openreview_venues',
    url: `https://api2.openreview.net/notes?content.venue=${encodeURIComponent(venue)}&limit=${encodeURIComponent(String(Number(opts.maxItems || opts.max_items || 10)))}`,
    maxItems: Number(opts.maxItems || opts.max_items || 10),
    minHours: Number(opts.minHours || opts.min_hours || 12),
    force: opts.force === true,
    extractor: extractOpenReviewNotes,
    topics: Array.isArray(opts.topics) ? opts.topics : ['research', 'peer_review', 'openreview', 'ai'],
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
  extractOpenReviewNotes
};
