          decisions.push({
            id: match[1],
            date: match[2],
            domain: match[3],
            context: match[4].trim(),
            action: match[5].trim(),
            expected: match[6].trim(),
            metric: match[7].trim(),
            check_on: match[8],
            status: match[9],
            next: match[10] ? match[10].trim() : '',
            file,
            line: i + 1,
            tokens: tokenCount
          });
        }
      }
    }
  }

  return { decisions, decisionWarnings };
}

// Scan for decisions
const { decisions, decisionWarnings } = scanForDecisions();

// Build DECISIONS_INDEX.md (open decisions only, sorted by check_on)
const openDecisions = decisions
  .filter(d => d.status === 'open')
  .sort((a, b) => a.check_on.localeCompare(b.check_on));

let decisionsIndex = `# DECISIONS_INDEX.md
# Last regenerated: ${new Date().toISOString().split('T')[0]}
# Only open decisions, sorted by check_on

| id | date | domain | check_on | metric | file | preview |
|----|------|--------|----------|--------|------|---------|
${openDecisions.map(d => `| ${d.id} | ${d.date} | ${d.domain} | ${d.check_on} | ${d.metric.substring(0, 30)} | ${d.file} | ${d.context.substring(0, 60)} |`).join('\n') || '| | | | | | | |'}

## Stats
- Total decisions: ${decisions.length}
- Open decisions: ${openDecisions.length}
- Due today/overdue: ${openDecisions.filter(d => d.check_on <= new Date().toISOString().split('T')[0]).length}
- Oversized (>120 tokens): ${decisionWarnings.length}
`;

fs.writeFileSync(path.join(memoryDir, 'DECISIONS_INDEX.md'), decisionsIndex);
console.log('DECISIONS_INDEX.md rebuilt');

// Generate SNIPPET_INDEX.md
const { snippets, tagCounts } = scanForSnippets();

let snippetIndex = `# SNIPPET_INDEX.md
# Last regenerated: ${new Date().toISOString().split('T')[0]}
# Snippet markers found via: <!-- TAGS: #tag1 #tag2 --> | # TAGS: #tag1 #tag2 | [TAGS: #tag1 #tag2]
# Window: ±${SNIPPET_WINDOW} lines around marker
# Source paths: workspace (excluding node_modules, dist, build, .git, binaries)

## Stats
- Total markers: ${snippets.length}
- Unique tags: ${tagCounts.size}

## Snippets by Tag
| tag | file | line_start | line_end | preview |
|-----|------|------------|----------|---------|
${snippets.map(s => `| ${s.tag} | ${s.file} | ${s.line_start} | ${s.line_end} | ${s.preview} |`).join('\n')}

## Top Tags by Frequency
${Array.from(tagCounts.entries())
  .sort((a, b) => b[1] - a[1])
  .slice(0, 10)
  .map(([tag, count], i) => `${i + 1}. ${tag}: ${count} occurrences`)
  .join('\n')}
`;

fs.writeFileSync(path.join(memoryDir, 'SNIPPET_INDEX.md'), snippetIndex);
console.log('SNIPPET_INDEX.md rebuilt');

console.log('\n=== FORMAT VIOLATIONS ===');
console.log(`Count: ${formatViolations.length}`);
if (formatViolations.length > 0) {
  console.log('| file | node_id | reasons |');
  console.log('|------|---------|---------|');
  formatViolations.slice(0, 10).forEach(v => {
    console.log(`| ${v.file} | ${v.node_id} | ${v.reasons} |`);
  });
  if (formatViolations.length > 10) {
    console.log(`... and ${formatViolations.length - 10} more`);
  }
}

console.log('\n=== REGISTRY-FORCED CATEGORY CHANGES ===');
console.log(`Count: ${registryForcedChanges.length}`);
if (registryForcedChanges.length > 0) {
  registryForcedChanges.forEach(c => {
    console.log(`- ${c.node_id}: ${c.baseline} → ${c.final} (${c.reason})`);
  });
}

// NEW: Registry warnings
console.log('\n=== REGISTRY WARNINGS ===');
console.log(`Count: ${registryWarnings.length}`);
if (registryWarnings.length > 0) {
  registryWarnings.forEach(w => {
    console.log(`- ${w.node_id}: ${w.warning} - ${w.details}`);
  });
} else {
  console.log('No registry/tag mismatches detected.');
}

console.log('\n=== BLOAT VIOLATIONS ===');
console.log(`Count: ${bloatWarnings.length}`);
if (bloatWarnings.length > 0) {
  bloatWarnings.forEach(b => {
    console.log(`- ${b.node_id}: ${b.tokens}/${b.cap} tokens (+${b.excess})`);
  });
}

const totalTokens = allNodes.reduce((sum, n) => sum + n.token_estimate, 0);
const avgTokens = allNodes.length > 0 ? Math.round(totalTokens / allNodes.length) : 0;

console.log('\n=== SNIPPET INDEX ===');
console.log(`Markers found: ${snippets.length}`);
console.log(`Unique tags: ${tagCounts.size}`);
if (tagCounts.size > 0) {
  const topTags = Array.from(tagCounts.entries())
    .sort((a, b) => b[1] - a[1])
    .slice(0, 5)
    .map(([tag, count]) => `${tag}(${count})`)
    .join(', ');
  console.log(`Top tags: ${topTags}`);
}

// Generate tagged blocks summary
const taggedBlocks = scanForTaggedBlocks();
console.log('\n=== TAGGED BLOCKS ===');
console.log(`Total @tag blocks: ${taggedBlocks.total}`);
if (taggedBlocks.total > 0) {
  console.log(`By scope: ${Object.entries(taggedBlocks.byScope).map(([k, v]) => `${k}=${v}`).join(', ')}`);
  console.log(`Topic tags minted: ${taggedBlocks.topicTagCount} (${taggedBlocks.topicTags.join(', ') || 'none'})`);
}

console.log('\n=== DECISION JOURNAL ===');
console.log(`Total decisions: ${decisions.length}`);
console.log(`Open decisions: ${openDecisions.length}`);
const dueTodayOrOverdue = openDecisions.filter(d => d.check_on <= new Date().toISOString().split('T')[0]).length;
console.log(`Due today/overdue: ${dueTodayOrOverdue}`);
console.log(`Oversized warnings: ${decisionWarnings.length}`);
if (decisionWarnings.length > 0) {
  decisionWarnings.slice(0, 3).forEach(w => {
    console.log(`- ${w.id}: ${w.tokens} tokens (>120 cap)`);
  });
}

console.log('\n=== SMOKE TEST SUMMARY ===');
console.log(`Files: ${dailyFiles.length} | Valid nodes: ${allNodes.length} | Skipped: ${formatViolations.length}`);
console.log(`Snippet markers: ${snippets.length} | Tagged blocks: ${taggedBlocks.total}`);
console.log(`Projects: ${projects.length} | Ops/Logs: ${ops.length}`);
console.log(`Tokens: ${totalTokens} (${avgTokens}/node avg)`);
console.log(`forced_changes: ${registryForcedChanges.length} | bloat: ${bloatWarnings.length} | registry_warnings: ${registryWarnings.length}`);
console.log(`decisions: ${decisions.length} | open: ${openDecisions.length} | due: ${dueTodayOrOverdue}`);
console.log(`Snapshots: ${backedUpFiles.length} created`);
console.log('\nEnforcement: ACTIVE (separator-parsed, validated, snippet-indexed, tag-contract-active, snapshot-backup-active, decision-journal-active)');
console.log('=== COMPLETE ===');
