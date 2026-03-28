'use strict';

const { runJsonCollector } = require('./collector_runtime.ts');

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
  run
};
