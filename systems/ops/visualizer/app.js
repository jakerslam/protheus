const TYPE_ORDER = ['directive', 'strategy', 'campaign', 'proposal', 'outcome'];
const TYPE_COLORS = {
  directive: '#2a6f97',
  strategy: '#7b2cbf',
  campaign: '#ff6b35',
  proposal: '#1d3557',
  outcome: '#2a9d8f',
  unknown: '#6f6a5d'
};

const state = {
  timer: null,
  refreshMs: 20000
};

function byId(id) {
  return document.getElementById(id);
}

function escapeHtml(text) {
  return String(text == null ? '' : text)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function fmtNum(v) {
  const n = Number(v || 0);
  if (!Number.isFinite(n)) return '0';
  return n.toLocaleString();
}

async function fetchPayload(hours) {
  const h = Number(hours || 24);
  const res = await fetch(`/api/graph?hours=${encodeURIComponent(String(h))}`, { cache: 'no-store' });
  if (!res.ok) {
    throw new Error(`api_http_${res.status}`);
  }
  return res.json();
}

function renderCards(summary) {
  const rows = [
    ['Run Events', summary.run_events],
    ['Executed', summary.executed],
    ['Shipped', summary.shipped],
    ['No Change', summary.no_change],
    ['Reverted', summary.reverted],
    ['Confidence Fallback', summary.confidence_fallback],
    ['Route Blocked', summary.route_blocked],
    ['Policy Holds', summary.policy_holds],
    ['Candidate Audits', summary.candidate_audits]
  ];
  byId('cards').innerHTML = rows.map(([k, v]) => (
    `<article class="card"><div class="k">${escapeHtml(k)}</div><div class="v">${escapeHtml(fmtNum(v))}</div></article>`
  )).join('');
}

function renderPairList(elId, rows) {
  const el = byId(elId);
  const list = Array.isArray(rows) ? rows : [];
  if (!list.length) {
    el.innerHTML = '<li><span>none</span><span>0</span></li>';
    return;
  }
  el.innerHTML = list.map((row) => {
    const key = Array.isArray(row) ? row[0] : '';
    const val = Array.isArray(row) ? row[1] : 0;
    return `<li><span>${escapeHtml(String(key))}</span><strong>${escapeHtml(fmtNum(val))}</strong></li>`;
  }).join('');
}

function nodeTypeOrder(type) {
  const idx = TYPE_ORDER.indexOf(type);
  return idx >= 0 ? idx : TYPE_ORDER.length;
}

function layoutNodes(nodes) {
  const groups = {};
  for (const node of nodes) {
    const t = String(node.type || 'unknown');
    if (!groups[t]) groups[t] = [];
    groups[t].push(node);
  }
  for (const t of Object.keys(groups)) {
    groups[t].sort((a, b) => Number(b.weight || 0) - Number(a.weight || 0) || String(a.label).localeCompare(String(b.label)));
  }

  const sortedTypes = Object.keys(groups).sort((a, b) => nodeTypeOrder(a) - nodeTypeOrder(b));
  const width = 1200;
  const height = 700;
  const left = 90;
  const right = 1110;
  const top = 60;
  const bottom = 650;
  const colGap = sortedTypes.length > 1 ? (right - left) / (sortedTypes.length - 1) : 1;

  const pos = {};
  for (let i = 0; i < sortedTypes.length; i += 1) {
    const type = sortedTypes[i];
    const col = groups[type];
    const x = left + (i * colGap);
    const step = col.length > 1 ? (bottom - top) / (col.length - 1) : 1;
    for (let j = 0; j < col.length; j += 1) {
      const y = col.length === 1 ? (top + bottom) / 2 : (top + (j * step));
      pos[col[j].id] = { x, y };
    }
  }
  return { pos, width, height };
}

function renderGraph(graph) {
  const svg = byId('graph');
  const nodes = Array.isArray(graph && graph.nodes) ? graph.nodes : [];
  const edges = Array.isArray(graph && graph.edges) ? graph.edges : [];
  svg.innerHTML = '';
  if (!nodes.length) return;

  const { pos } = layoutNodes(nodes);
  const ns = 'http://www.w3.org/2000/svg';

  const edgeLayer = document.createElementNS(ns, 'g');
  edgeLayer.setAttribute('opacity', '0.65');
  svg.appendChild(edgeLayer);

  for (const edge of edges) {
    const from = pos[edge.from];
    const to = pos[edge.to];
    if (!from || !to) continue;
    const path = document.createElementNS(ns, 'path');
    const dx = Math.max(40, Math.abs(to.x - from.x) * 0.35);
    const d = `M ${from.x} ${from.y} C ${from.x + dx} ${from.y}, ${to.x - dx} ${to.y}, ${to.x} ${to.y}`;
    path.setAttribute('d', d);
    path.setAttribute('fill', 'none');
    path.setAttribute('stroke', '#7a7062');
    path.setAttribute('stroke-opacity', '0.34');
    path.setAttribute('stroke-width', String(0.8 + Math.log2(Number(edge.count || 1) + 1)));
    const title = document.createElementNS(ns, 'title');
    title.textContent = `${edge.from} -> ${edge.to} (${edge.label || 'edge'}) x${edge.count || 1}`;
    path.appendChild(title);
    edgeLayer.appendChild(path);
  }

  const nodeLayer = document.createElementNS(ns, 'g');
  svg.appendChild(nodeLayer);

  for (const node of nodes) {
    const p = pos[node.id];
    if (!p) continue;
    const group = document.createElementNS(ns, 'g');
    group.setAttribute('transform', `translate(${p.x}, ${p.y})`);

    const circle = document.createElementNS(ns, 'circle');
    circle.setAttribute('r', String(5 + Math.min(9, Math.log2(Number(node.weight || 1) + 1) * 2)));
    circle.setAttribute('fill', TYPE_COLORS[node.type] || TYPE_COLORS.unknown);
    circle.setAttribute('stroke', '#f6f2e8');
    circle.setAttribute('stroke-width', '1.5');

    const text = document.createElementNS(ns, 'text');
    text.setAttribute('x', '11');
    text.setAttribute('y', '3');
    text.setAttribute('font-size', '11');
    text.setAttribute('font-family', 'IBM Plex Mono, monospace');
    text.setAttribute('fill', '#1c1a17');
    text.textContent = String(node.label || node.id).slice(0, 42);

    const title = document.createElementNS(ns, 'title');
    title.textContent = `${node.id}\n${JSON.stringify(node.meta || {}, null, 2)}`;

    group.appendChild(circle);
    group.appendChild(text);
    group.appendChild(title);
    nodeLayer.appendChild(group);
  }
}

async function refresh() {
  const hours = Number(byId('hours').value || 24);
  const meta = byId('meta');
  meta.textContent = 'Loading...';
  try {
    const payload = await fetchPayload(hours);
    renderCards(payload.summary || {});
    renderPairList('results', payload.summary && payload.summary.top_results);
    renderPairList('gates', payload.summary && payload.summary.top_rejected_gates);
    renderGraph(payload.graph || {});
    meta.textContent = `Updated ${new Date(payload.generated_at || Date.now()).toLocaleString()} | window=${hours}h`;
  } catch (err) {
    meta.textContent = `Load failed: ${String(err && err.message || err || 'unknown')}`;
  }
}

function setPolling() {
  if (state.timer) clearInterval(state.timer);
  state.timer = setInterval(refresh, state.refreshMs);
}

function boot() {
  byId('refresh').addEventListener('click', refresh);
  byId('hours').addEventListener('change', refresh);
  refresh();
  setPolling();
}

boot();
