const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { spawnSync } = require('child_process');

const WORKSPACE_ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const WORKSPACE_MEMORY_ROOT = path.join(WORKSPACE_ROOT, 'local', 'workspace', 'memory');
const CLIENT_ROOT = path.join(WORKSPACE_ROOT, 'client');
const STATE_ROOT = path.join(WORKSPACE_ROOT, 'state', 'memory');
const memoryDir = path.resolve(process.env.MEMORY_DIR || WORKSPACE_MEMORY_ROOT);
const SNAPSHOT_DIR = path.resolve(
  process.env.MEMORY_SNAPSHOT_DIR || path.join(STATE_ROOT, '_snapshots')
);
const DELTA_CACHE_PATH = path.resolve(
  process.env.MEMORY_REBUILD_DELTA_CACHE_PATH || path.join(STATE_ROOT, '.rebuild_delta_cache.json')
);
const whitelistRegex = /^\d{4}-\d{2}-\d{2}\.md$/;
const UID_PATTERN = /^[A-Za-z0-9]+$/;
const UID_ENFORCE_SINCE = normalizeDate(process.env.MEMORY_UID_ENFORCE_SINCE || '2026-02-22');

const TOKEN_CAPS = {
  default: 200,
  controlPlane: 200,
  metrics: 250
};

const CONTROL_PLANE_NODES = ['project-registry', 'mode-restrictions', 'query-mode-toggle', 'bloat-safeguards', 'test-noise-quarantine', 'bridge-templates', 'graph-bridge-policy', 'creative-mode-spec', 'node-format-policy', 'snippet-index-policy', 'tagging-policy', 'topic-registry', 'pin-policy', 'hyper-creative-mode-spec', 'capture-policy', 'agent-portability-spec'];

// Snippet scanning configuration
const SNIPPET_WINDOW = 8;
const EXCLUDE_PATHS = ['node_modules', 'dist', 'build', '.git', '.next', 'out', 'coverage'];
const INCLUDE_EXTENSIONS = ['.md', '.txt', '.js', '.ts', '.json', '.yaml', '.yml', '.sh', '.py', '.rb', '.go', '.rs', '.c', '.cpp', '.h', '.java', '.kt', '.swift', '.php', '.pl', '.r', '.scala', '.groovy', '.clj', '.erl', '.ex', '.exs', '.lua', '.vim', '.el', '.lisp', '.scm', '.hs', '.ml', '.sql', '.css', '.scss', '.less', '.html', '.xml', '.toml', '.ini', '.cfg', '.conf', '.dockerfile', '.tf', '.hcl'];

fs.mkdirSync(memoryDir, { recursive: true });

function sha1Text(content) {
  return crypto.createHash('sha1').update(String(content || '')).digest('hex');
}

function normalizeDate(v) {
  const s = String(v || '').trim();
  return /^\d{4}-\d{2}-\d{2}$/.test(s) ? s : '2026-02-22';
}

function requiresUid(nodeDate) {
  const d = normalizeDate(nodeDate);
  return d >= UID_ENFORCE_SINCE;
}

function loadDeltaCache() {
  try {
    if (!fs.existsSync(DELTA_CACHE_PATH)) return { version: 1, files: {} };
    const parsed = JSON.parse(fs.readFileSync(DELTA_CACHE_PATH, 'utf8'));
    if (!parsed || typeof parsed !== 'object') return { version: 1, files: {} };
    if (!parsed.files || typeof parsed.files !== 'object') parsed.files = {};
    return parsed;
  } catch {
    return { version: 1, files: {} };
  }
}

function saveDeltaCache(cache) {
  const payload = cache && typeof cache === 'object' ? cache : { version: 1, files: {} };
  payload.version = 1;
  payload.updated_at = new Date().toISOString();
  if (!payload.files || typeof payload.files !== 'object') payload.files = {};
  fs.mkdirSync(path.dirname(DELTA_CACHE_PATH), { recursive: true });
  fs.writeFileSync(DELTA_CACHE_PATH, JSON.stringify(payload, null, 2));
}

function getProjectRegistryFromNodes() {
  const registry = { 'active-core': [], 'active-test': [], paused: [], retired: [] };
  const owners = {};
  let needsRewrite = false;
  let latestFile = null;

  const dailyFiles = fs.readdirSync(memoryDir)
    .filter(f => whitelistRegex.test(f))
    .sort();

  let latestRegistry = null;
  let latestDate = '';

  for (const file of dailyFiles) {
    const filePath = path.join(memoryDir, file);
    const content = fs.readFileSync(filePath, 'utf8');
    const date = file.replace('.md', '');

    const registryNodePattern = /(---\s*\ndate:\s*\d{4}-\d{2}-\d{2}\s*\nnode_id:\s*project-registry\s*\ntags:[\s\S]*?---\s*\n#\s*project-registry\n)([\s\S]*?)(?=\s*<!-- NODE -->|---\s*\ndate:|\.{3}|$)/;
    const registryMatch = content.match(registryNodePattern);
    if (registryMatch) {
      if (date > latestDate) {
        latestDate = date;
        latestRegistry = registryMatch[2];
        latestFile = filePath;
      }
    }
  }

  if (!latestRegistry) {
    return { registry, owners, needsRewrite, latestFile };
  }

  if (/activeCore:|activeTest:/.test(latestRegistry)) {
    needsRewrite = true;
  }

  function parseListItems(content, hyphenatedKey, camelCaseKey) {
    const items = [];
    const patterns = [
      new RegExp(`${hyphenatedKey}:([\\s\\S]*?)(?=\\n(?:active-core:|active-test:|activeCore:|activeTest:|paused:|retired:|owners:|rules:|$))`),
      new RegExp(`${camelCaseKey}:([\\s\\S]*?)(?=\\n(?:active-core:|active-test:|activeCore:|activeTest:|paused:|retired:|owners:|rules:|$))`)
    ];

    for (const pattern of patterns) {
      const match = content.match(pattern);
      if (match) {
        const lines = match[1].split('\n');
        for (const line of lines) {
          const itemMatch = line.match(/^-\s*([a-zA-Z0-9_-]+)/);
          if (itemMatch && itemMatch[1].trim() && !itemMatch[1].includes('none')) {
            items.push(itemMatch[1].trim());
          }
        }
        break;
      }
    }
    return items;
  }

  registry['active-core'] = parseListItems(latestRegistry, 'active-core', 'activeCore');
  registry['active-test'] = parseListItems(latestRegistry, 'active-test', 'activeTest');

  const pausedMatch = latestRegistry.match(/paused:([\s\S]*?)(?=\n(?:active-core:|active-test:|activeCore:|activeTest:|retired:|owners:|rules:|$))/);
  if (pausedMatch) {
    const lines = pausedMatch[1].split('\n');
    for (const line of lines) {
      const match = line.match(/^-\s*([a-zA-Z0-9_-]+)/);
      if (match && match[1].trim() && !match[1].includes('none')) {
        registry.paused.push(match[1].trim());
      }
    }
  }

  const retiredMatch = latestRegistry.match(/retired:([\s\S]*?)(?=\n(?:active-core:|active-test:|activeCore:|activeTest:|paused:|owners:|rules:|$))/);
  if (retiredMatch) {
    const lines = retiredMatch[1].split('\n');
    for (const line of lines) {
      const match = line.match(/^-\s*([a-zA-Z0-9_-]+)/);
      if (match && match[1].trim() && !match[1].includes('none')) {
        registry.retired.push(match[1].trim());
      }
    }
  }

  const ownersMatch = latestRegistry.match(/owners:([\s\S]*?)(?=\n(?:rules:|$))/);
  if (ownersMatch) {
    const lines = ownersMatch[1].split('\n');
    for (const line of lines) {
      const match = line.match(/^-\s*([a-zA-Z0-9_-]+):\s*(.+)/);
      if (match) {
        owners[match[1].trim()] = match[2].trim();
      }
    }
  }

  return { registry, owners, needsRewrite, latestFile, date: latestDate };
}

// Snapshot backup function
function createSnapshotBackups() {
  const snapshotDir = SNAPSHOT_DIR;
  if (!fs.existsSync(snapshotDir)) {
    fs.mkdirSync(snapshotDir, { recursive: true });
  }

  const now = new Date();
  // America/Denver local time: YYYY-MM-DD-HHMM
  const options = { timeZone: 'America/Denver', year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', hour12: false };
  const formatter = new Intl.DateTimeFormat('en-US', options);
  const parts = formatter.formatToParts(now);
  const year = parts.find(p => p.type === 'year').value;
  const month = parts.find(p => p.type === 'month').value;
  const day = parts.find(p => p.type === 'day').value;
  const hour = parts.find(p => p.type === 'hour').value;
  const minute = parts.find(p => p.type === 'minute').value;
  const timestamp = `${year}-${month}-${day}-${hour}${minute}`;

  const filesToBackup = ['MEMORY_INDEX.md', 'TAGS_INDEX.md', 'SNIPPET_INDEX.md'];
  const backedUp = [];

  for (const file of filesToBackup) {
    const sourcePath = path.join(memoryDir, file);
    if (fs.existsSync(sourcePath)) {
      const backupPath = path.join(snapshotDir, file.replace('.md', `-${timestamp}.md`));
      fs.copyFileSync(sourcePath, backupPath);
      backedUp.push(path.basename(backupPath));
    }
  }

  return backedUp;
}

// Scan for inline tag snippets in workspace files
function scanForSnippets() {
  const snippets = [];
  const tagCounts = new Map();

  const tagPatterns = [
    /<!--\s*TAGS:\s*(#[a-z0-9-]+(?:\s+#[a-z0-9-]+)*)\s*-->/i,
    /#\s*TAGS:\s*(#[a-z0-9-]+(?:\s+#[a-z0-9-]+)*)$/im,
    /\[TAGS:\s*(#[a-z0-9-]+(?:\s+#[a-z0-9-]+)*)\]/i
  ];

  function isExcluded(filePath) {
    return EXCLUDE_PATHS.some(ex => filePath.includes(ex));
  }

  function hasValidExtension(filePath) {
    const ext = path.extname(filePath).toLowerCase();
    const base = path.basename(filePath);
    const hasDockerfile = base.toLowerCase().includes('dockerfile');
    return INCLUDE_EXTENSIONS.includes(ext) || hasDockerfile;
  }

  function scanDirectory(dir, baseDir = WORKSPACE_ROOT) {
    const entries = fs.readdirSync(dir, { withFileTypes: true });

    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);

      if (entry.isDirectory()) {
        if (!isExcluded(fullPath)) {
          scanDirectory(fullPath, baseDir);
        }
      } else if (entry.isFile() && !isExcluded(fullPath) && hasValidExtension(fullPath)) {
        if (whitelistRegex.test(entry.name)) continue;

        try {
          const content = fs.readFileSync(fullPath, 'utf8');
          const lines = content.split('\n');
          const relativePath = path.relative(baseDir, fullPath);

          for (let i = 0; i < lines.length; i++) {
            const line = lines[i];
            let match = null;

            for (const pattern of tagPatterns) {
              match = line.match(pattern);
              if (match) break;
            }

            if (match) {
              const tagList = match[1].split(/\s+/).filter(t => t.startsWith('#'));
              
              const lineStart = Math.max(0, i - SNIPPET_WINDOW);
              const lineEnd = Math.min(lines.length - 1, i + SNIPPET_WINDOW);
              const previewLines = lines.slice(lineStart, lineEnd + 1);
              const preview = previewLines.join(' ').substring(0, 60).replace(/\s+/g, ' ');

              for (const tag of tagList) {
                const cleanTag = tag.toLowerCase();
                snippets.push({
                  tag: cleanTag,
                  file: relativePath,
                  line_start: lineStart + 1,
                  line_end: lineEnd + 1,
                  preview: preview
                });

                tagCounts.set(cleanTag, (tagCounts.get(cleanTag) || 0) + 1);
              }
            }
          }
        } catch (err) {
          // Skip files that can't be read as text
        }
      }
    }
  }

  scanDirectory(WORKSPACE_ROOT);
  return { snippets, tagCounts };
}

// Scan for @tag blocks in all workspace files
function scanForTaggedBlocks() {
  const taggedBlocks = [];
  const tagPattern = /@tags:\s*(#[a-z0-9-]+(?:\s+#[a-z0-9-]+)*)\s+@id:\s*(\S+)\s+@date:\s*(\d{4}-\d{2}-\d{2})\s+@scope:\s*(snippet|node|log|draft)/i;
  const topicTagPattern = /#topic-[a-z0-9-]+/g;

  function isExcluded(filePath) {
    return EXCLUDE_PATHS.some(ex => filePath.includes(ex));
  }

  function hasValidExtension(filePath) {
    const ext = path.extname(filePath).toLowerCase();
    const base = path.basename(filePath);
    const hasDockerfile = base.toLowerCase().includes('dockerfile');
    return INCLUDE_EXTENSIONS.includes(ext) || hasDockerfile;
  }

  function scanDirectory(dir, baseDir = WORKSPACE_ROOT) {
    const entries = fs.readdirSync(dir, { withFileTypes: true });

    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);

      if (entry.isDirectory()) {
        if (!isExcluded(fullPath)) {
          scanDirectory(fullPath, baseDir);
        }
      } else if (entry.isFile() && !isExcluded(fullPath) && hasValidExtension(fullPath)) {
        try {
          const content = fs.readFileSync(fullPath, 'utf8');
          const lines = content.split('\n');
          const relativePath = path.relative(baseDir, fullPath);

          for (let i = 0; i < lines.length; i++) {
            const match = lines[i].match(tagPattern);
            if (match) {
              const tags = match[1].split(/\s+/).filter(t => t.startsWith('#'));
              const id = match[2];
              const date = match[3];
              const scope = match[4].toLowerCase();
              const topicTags = tags.filter(t => t.startsWith('#topic-'));

              const lineStart = Math.max(0, i - 3);
              const lineEnd = Math.min(lines.length - 1, i + 3);

              taggedBlocks.push({
                tags,
                id,
                date,
                scope,
                topicTags,
                file: relativePath,
                line: i + 1,
                context: lines.slice(lineStart, lineEnd + 1).join('\n')
              });
            }
          }
        } catch (err) {
          // Skip files that can't be read
        }
      }
    }
  }

  scanDirectory(WORKSPACE_ROOT);

  const summary = {
    total: taggedBlocks.length,
    byScope: {},
    topicTags: new Set(),
    topicTagCount: 0
  };

  for (const block of taggedBlocks) {
    summary.byScope[block.scope] = (summary.byScope[block.scope] || 0) + 1;
    for (const tt of block.topicTags) {
      summary.topicTags.add(tt);
      summary.topicTagCount++;
    }
  }

  summary.topicTags = Array.from(summary.topicTags);
  return summary;
}

const { registry: projectRegistry, needsRewrite, latestFile } = getProjectRegistryFromNodes();

if (needsRewrite && latestFile) {
  console.log('CANONICALIZATION: Normalizing project-registry keys...');
  let content = fs.readFileSync(latestFile, 'utf8');
  content = content.replace(/activeCore:/g, 'active-core:');
  content = content.replace(/activeTest:/g, 'active-test:');
  fs.writeFileSync(latestFile, content);
}

// Create snapshots BEFORE writing new indices
console.log('╔════════════════════════════════════════════════════════════╗');
console.log('║              CREATING SNAPSHOT BACKUPS                     ║');
console.log('╚════════════════════════════════════════════════════════════╝');
const backedUpFiles = createSnapshotBackups();
console.log(`Backed up: ${backedUpFiles.join(', ') || 'none'}`);
console.log(`Snapshot location: ${path.relative(WORKSPACE_ROOT, SNAPSHOT_DIR).replace(/\\/g, '/')}/`);
console.log();

const allRegisteredProjects = [...projectRegistry['active-core'], ...projectRegistry.paused, ...projectRegistry.retired];
const dailyFiles = fs.readdirSync(memoryDir).filter(f => whitelistRegex.test(f)).sort();

console.log('Whitelisted daily files:', dailyFiles);

const allNodes = [];
const allTags = new Map();
const seenNodes = new Set();
const seenUids = new Set();
const formatViolations = [];
const bloatWarnings = [];
const registryWarnings = []; // NEW: For tag/registry mismatches

let projectsFromRegistry = 0;
let projectsFromTag = 0;
const registryForcedChanges = [];

function estimateTokens(text) {
  const wordCount = text.trim().split(/\s+/).length;
  return Math.round(wordCount / 0.75);
}

function getNodeCap(nodeId) {
  if (CONTROL_PLANE_NODES.includes(nodeId)) return TOKEN_CAPS.controlPlane;
  if (nodeId.includes('metrics') || nodeId.includes('weekly')) return TOKEN_CAPS.metrics;
  return TOKEN_CAPS.default;
}

function getBaselineCategory(node) {
  if (node.node_id === 'project-registry') return 'System';
  if (node.tags.includes('project')) return 'Projects';
