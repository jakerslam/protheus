  if (node.tags.includes('workflow') || node.tags.includes('comms') || node.tags.includes('calibration')) return 'Rules';
  if (node.tags.includes('architecture') || node.tags.includes('income') || node.tags.includes('platform') || node.tags.includes('concept')) return 'Concepts';
  if (node.tags.includes('system') || node.tags.includes('protocol') || node.tags.includes('capabilities') || node.tags.includes('registry')) return 'System';
  return 'Ops/Logs';
}

function validateNode(chunk, file, expectedDate) {
  const violations = [];
  let nodeId = '(undetectable)';
  let uid = null;

  const frontmatterMatch = chunk.match(/---\s*\n([\s\S]*?)\n---/);
  if (!frontmatterMatch) {
    violations.push('missing frontmatter block');
    return { valid: false, nodeId, violations };
  }

  const fm = frontmatterMatch[1];

  const dateMatch = fm.match(/date:\s*(\d{4}-\d{2}-\d{2})/);
  const nodeDate = dateMatch ? dateMatch[1] : expectedDate;
  if (!dateMatch) {
    violations.push('missing date in frontmatter');
  } else if (dateMatch[1] !== expectedDate) {
    violations.push(`date mismatch: ${dateMatch[1]} != ${expectedDate}`);
  }

  const nodeIdMatch = fm.match(/node_id:\s*(\S+)/);
  if (!nodeIdMatch) {
    violations.push('missing node_id in frontmatter');
  } else {
    nodeId = nodeIdMatch[1];
  }

  const tagsMatch = fm.match(/tags:\s*\[([^\]]*)\]/);
  if (!tagsMatch) {
    violations.push('missing tags array in frontmatter');
  }

  const uidMatch = fm.match(/uid:\s*(\S+)/);
  if (!uidMatch) {
    if (requiresUid(nodeDate)) {
      violations.push(`missing uid in frontmatter (required since ${UID_ENFORCE_SINCE})`);
    }
  } else {
    uid = String(uidMatch[1] || '').trim();
    if (!UID_PATTERN.test(uid)) {
      violations.push(`uid not alphanumeric: ${uid}`);
    }
  }

  const h1Match = chunk.match(/\n#\s*(\S+)/);
  if (!h1Match) {
    violations.push('missing H1 title line');
  } else if (nodeIdMatch && h1Match[1] !== nodeIdMatch[1]) {
    violations.push(`H1 mismatch: "${h1Match[1]}" != "${nodeIdMatch[1]}"`);
  }

  return {
    valid: violations.length === 0,
    nodeId,
    uid,
    violations,
    date: dateMatch ? dateMatch[1] : null,
    tags: tagsMatch ? tagsMatch[1].split(',').map(t => t.trim()).filter(t => t) : [],
    body: chunk.replace(/---\s*\n[\s\S]*?\n---\s*\n#[^\n]+\n/, '').replace(/\s*<!--\s*NODE\s*-->\s*$/, '')
  };
}

function parseDailyFileRecords(file, content, expectedDate) {
  const records = [];
  const formatViolationsLocal = [];
  const chunks = String(content || '').split(/\s*<!--\s*NODE\s*-->\s*/).filter(c => c.trim());

  for (const chunk of chunks) {
    const trimmed = chunk.trim();
    if (!trimmed) continue;

    const validation = validateNode(trimmed, file, expectedDate);
    if (!validation.valid) {
      formatViolationsLocal.push({
        file,
        node_id: validation.nodeId,
        reasons: validation.violations.join(', ')
      });
      continue;
    }

    const { nodeId, uid, tags, body } = validation;
    const tokenEstimate = estimateTokens(trimmed);
    const firstBullet = body.match(/[-*]\s*(.+?)(?=\n|$)/);
    const firstLine = body.split('\n')[0];
    const summary = firstBullet ? firstBullet[1].trim().substring(0, 60)
      : firstLine ? firstLine.substring(0, 60)
        : 'Node content';

    records.push({
      node_id: nodeId,
      uid: uid || null,
      tags,
      file,
      date: expectedDate,
      token_estimate: tokenEstimate,
      summary
    });
  }

  return {
    records,
    format_violations: formatViolationsLocal
  };
}

// Parse files with delta cache (changed files only).
const priorDeltaCache = loadDeltaCache();
const nextDeltaCache = { version: 1, files: {} };
let deltaHits = 0;
let deltaMisses = 0;

for (const file of dailyFiles) {
  const filePath = path.join(memoryDir, file);
  const content = fs.readFileSync(filePath, 'utf8');
  const expectedDate = file.replace('.md', '');
  const fileHash = sha1Text(content);

  const cached = priorDeltaCache.files && priorDeltaCache.files[file] ? priorDeltaCache.files[file] : null;
  let parsed = null;
  if (cached && cached.sha1 === fileHash && cached.parsed && Array.isArray(cached.parsed.records) && Array.isArray(cached.parsed.format_violations)) {
    parsed = cached.parsed;
    deltaHits++;
  } else {
    parsed = parseDailyFileRecords(file, content, expectedDate);
    deltaMisses++;
  }

  const safeParsed = {
    records: Array.isArray(parsed && parsed.records) ? parsed.records : [],
    format_violations: Array.isArray(parsed && parsed.format_violations) ? parsed.format_violations : []
  };
  nextDeltaCache.files[file] = {
    sha1: fileHash,
    parsed: safeParsed
  };

  formatViolations.push(...safeParsed.format_violations);

  for (const rec of safeParsed.records) {
    const nodeId = String(rec && rec.node_id || '').trim();
    if (!nodeId) continue;
    if (seenNodes.has(nodeId)) continue;
    seenNodes.add(nodeId);
    const uid = String(rec && rec.uid || '').trim();
    if (uid) {
      if (seenUids.has(uid)) {
        formatViolations.push({
          file: String(rec && rec.file || file),
          node_id: nodeId,
          reasons: `duplicate uid: ${uid}`
        });
        continue;
      }
      seenUids.add(uid);
    }

    const tags = Array.isArray(rec.tags) ? rec.tags.map(t => String(t || '').trim()).filter(Boolean) : [];
    const inActiveCore = projectRegistry['active-core'].includes(nodeId);
    const hasProjectTag = tags.includes('project');

    if (!inActiveCore && hasProjectTag) {
      registryWarnings.push({
        node_id: nodeId,
        warning: 'PROJECT_TAG_NOT_IN_CORE',
        details: 'Node has #project tag but is not in project-registry.active-core'
      });
    }
    if (inActiveCore && !hasProjectTag) {
      registryWarnings.push({
        node_id: nodeId,
        warning: 'CORE_MISSING_PROJECT_TAG',
        details: 'Node is in project-registry.active-core but missing #project tag'
      });
    }

    const tokenEstimate = Number.isFinite(Number(rec.token_estimate)) ? Number(rec.token_estimate) : 0;
    const cap = getNodeCap(nodeId);
    if (tokenEstimate > cap) {
      bloatWarnings.push({
        node_id: nodeId,
        file: rec.file || file,
        tokens: tokenEstimate,
        cap,
        excess: tokenEstimate - cap
      });
    }

    allNodes.push({
      node_id: nodeId,
      uid: uid || null,
      tags,
      file: rec.file || file,
      summary: String(rec.summary || 'Node content'),
      date: rec.date || expectedDate,
      token_estimate: tokenEstimate,
      in_active_core: inActiveCore
    });

    for (const tag of tags) {
      if (!allTags.has(tag)) {
        allTags.set(tag, new Set());
      }
      allTags.get(tag).add(nodeId);
    }
  }
}

saveDeltaCache(nextDeltaCache);
console.log(`Delta cache: hits=${deltaHits} misses=${deltaMisses}`);

console.log(`Valid nodes indexed: ${allNodes.length}`);
if (formatViolations.length > 0) {
  console.log(`Skipped nodes: ${formatViolations.length} (FORMAT_VIOLATIONS)`);
}

const projects = [];
const rules = [];
const concepts = [];
const systems = [];
const ops = [];
const assigned = new Set();

for (const node of allNodes) {
  if (assigned.has(node.node_id)) continue;

  const baselineCategory = getBaselineCategory(node);
  let finalCategory = baselineCategory;
  let overrideReason = null;

  if (node.node_id === 'project-registry') {
    systems.push(node);
    assigned.add(node.node_id);
    continue;
  }

  // STRICT REGISTRY CLASSIFICATION: registry takes precedence over ALL tags
  if (projectRegistry['active-core'].includes(node.node_id)) {
    finalCategory = 'Projects';
    overrideReason = 'active-core';
    projects.push(node);
    assigned.add(node.node_id);
    projectsFromRegistry++;
  } else if (projectRegistry['active-test'].includes(node.node_id) ||
             projectRegistry.paused.includes(node.node_id) ||
             projectRegistry.retired.includes(node.node_id)) {
    // All non-active-core registry entries go to Ops/Logs, regardless of tags
    finalCategory = 'Ops/Logs';
    if (projectRegistry['active-test'].includes(node.node_id)) overrideReason = 'active-test';
    else if (projectRegistry.paused.includes(node.node_id)) overrideReason = 'paused';
    else if (projectRegistry.retired.includes(node.node_id)) overrideReason = 'retired';
    ops.push(node);
    assigned.add(node.node_id);
  } else {
    // Not in registry - use tag-based classification
    if (node.tags.includes('project')) {
      // This is now a WARNING case since we have a registry
      projectsFromTag++;
      projects.push(node);
      assigned.add(node.node_id);
    } else if (node.tags.includes('workflow') || node.tags.includes('comms') || node.tags.includes('calibration')) {
      rules.push(node);
      assigned.add(node.node_id);
    } else if (node.tags.includes('architecture') || node.tags.includes('income') || node.tags.includes('platform') || node.tags.includes('concept')) {
      concepts.push(node);
      assigned.add(node.node_id);
    } else if (node.tags.includes('system') || node.tags.includes('protocol') || node.tags.includes('capabilities') || node.tags.includes('registry')) {
      systems.push(node);
      assigned.add(node.node_id);
    } else {
      ops.push(node);
      assigned.add(node.node_id);
    }
  }

  if (overrideReason && baselineCategory !== finalCategory) {
    registryForcedChanges.push({
      node_id: node.node_id,
      baseline: baselineCategory,
      final: finalCategory,
      reason: overrideReason
    });
  }
}

console.log(`\nCategory counts:`);
console.log(`- Projects: ${projects.length}`);
console.log(`- Rules: ${rules.length}`);
console.log(`- Concepts: ${concepts.length}`);
console.log(`- System: ${systems.length}`);
console.log(`- Ops/Logs: ${ops.length}`);

// Build indices
let memIndex = `# MEMORY_INDEX.md
# Last regenerated: ${new Date().toISOString().split('T')[0]}
# Whitelist: YYYY-MM-DD.md top-level; parsed by "<!-- NODE -->" separators

## Projects
| node_id | uid | tags | file | summary |
|---------|-----|------|------|---------|
${projects.map(n => `| ${n.node_id} | ${n.uid || ''} | ${n.tags.map(t => '#'+t).join(' ')} | ${n.file} | ${n.summary} |`).join('\n') || '| | | | | |'}

## Rules
| node_id | uid | tags | file | summary |
|---------|-----|------|------|---------|
${rules.map(n => `| ${n.node_id} | ${n.uid || ''} | ${n.tags.map(t => '#'+t).join(' ')} | ${n.file} | ${n.summary} |`).join('\n') || '| | | | | |'}

## Concepts
| node_id | uid | tags | file | summary |
|---------|-----|------|------|---------|
${concepts.map(n => `| ${n.node_id} | ${n.uid || ''} | ${n.tags.map(t => '#'+t).join(' ')} | ${n.file} | ${n.summary} |`).join('\n') || '| | | | | |'}

## System
| node_id | uid | tags | file | summary |
|---------|-----|------|------|---------|
${systems.map(n => `| ${n.node_id} | ${n.uid || ''} | ${n.tags.map(t => '#'+t).join(' ')} | ${n.file} | ${n.summary} |`).join('\n') || '| | | | | |'}

## Ops/Logs
| node_id | uid | tags | file | summary |
|---------|-----|------|------|---------|
${ops.map(n => `| ${n.node_id} | ${n.uid || ''} | ${n.tags.map(t => '#'+t).join(' ')} | ${n.file} | ${n.summary} |`).join('\n') || '| | | | | |'}
`;

fs.writeFileSync(path.join(memoryDir, 'MEMORY_INDEX.md'), memIndex);
console.log('\nMEMORY_INDEX.md rebuilt');

let tagIndex = `# TAGS_INDEX.md
# Last regenerated: ${new Date().toISOString().split('T')[0]}

${Array.from(allTags.entries())
  .sort((a, b) => a[0].localeCompare(b[0]))
  .map(([tag, nodeIds]) => `#${tag} → ${Array.from(nodeIds).join(', ')}`)
  .join('\n')}
`;

fs.writeFileSync(path.join(memoryDir, 'TAGS_INDEX.md'), tagIndex);
console.log('TAGS_INDEX.md rebuilt');

function syncRootMemoryIndexCompat() {
  if (process.env.MEMORY_ROOT_COMPAT_SYNC !== '1') {
    console.log('Root MEMORY/TAGS compatibility indices skipped (set MEMORY_ROOT_COMPAT_SYNC=1 to enable)');
    return;
  }
  const mappings = [
    { from: path.join(memoryDir, 'MEMORY_INDEX.md'), to: path.join(WORKSPACE_ROOT, 'MEMORY_INDEX.md') },
    { from: path.join(memoryDir, 'TAGS_INDEX.md'), to: path.join(WORKSPACE_ROOT, 'TAGS_INDEX.md') }
  ];
  for (const mapping of mappings) {
    try {
      fs.copyFileSync(mapping.from, mapping.to);
    } catch (err) {
      console.log(`Root index sync warning (${path.basename(mapping.to)}): ${String(err && err.message ? err.message : err).slice(0, 200)}`);
    }
  }
}

syncRootMemoryIndexCompat();

const matrixScript = path.join(CLIENT_ROOT, 'runtime', 'systems', 'memory', 'memory_matrix.js');
if (fs.existsSync(matrixScript)) {
  const matrixRun = spawnSync('node', [matrixScript, 'run', '--apply=1', '--reason=rebuild_exclusive'], {
    cwd: WORKSPACE_ROOT,
    encoding: 'utf8',
    env: { ...process.env }
  });
  if (Number(matrixRun.status) === 0) {
    console.log('TAG_MEMORY_MATRIX rebuilt');
  } else {
    console.log(`TAG_MEMORY_MATRIX rebuild warning: ${(matrixRun.stderr || matrixRun.stdout || '').toString().trim().slice(0, 220) || 'unknown_error'}`);
  }
}

const sequencerScript = path.join(CLIENT_ROOT, 'runtime', 'systems', 'memory', 'dream_sequencer.js');
if (fs.existsSync(sequencerScript)) {
  const sequencerRun = spawnSync('node', [sequencerScript, 'run', '--apply=1', '--reason=rebuild_exclusive'], {
    cwd: WORKSPACE_ROOT,
    encoding: 'utf8',
    env: { ...process.env }
  });
  if (Number(sequencerRun.status) === 0) {
    console.log('Dream sequencer reordered matrix');
  } else {
    console.log(`Dream sequencer warning: ${(sequencerRun.stderr || sequencerRun.stdout || '').toString().trim().slice(0, 220) || 'unknown_error'}`);
  }
}

// Scan for @decision entries in daily files
function scanForDecisions() {
  const decisions = [];
  const decisionWarnings = [];
  const decisionPattern = /^@decision\s+id:(\S+)\s+date:(\d{4}-\d{2}-\d{2})\s+domain:(\S+)\s+context:(.+?)\s+action:(.+?)\s+expected:(.+?)\s+metric:(.+?)\s+check_on:(\d{4}-\d{2}-\d{2})\s+status:(\S+)(?:\s+next:(.*))?$/;

  for (const file of dailyFiles) {
    const filePath = path.join(memoryDir, file);
    const content = fs.readFileSync(filePath, 'utf8');
    const lines = content.split('\n');

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i].trim();
      if (line.startsWith('@decision ')) {
        const match = line.match(decisionPattern);
        if (match) {
          const tokenCount = Math.round(line.split(/\s+/).length / 0.75);
          if (tokenCount > 120) {
            decisionWarnings.push({
              file,
              line: i + 1,
              id: match[1],
              tokens: tokenCount,
              warning: 'OVERSIZE_DECISION'
            });
          }
