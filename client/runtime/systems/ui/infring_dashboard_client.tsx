import React, { useDeferredValue, useEffect, useMemo, useState } from 'https://esm.sh/react@18.2.0';
import { createRoot } from 'https://esm.sh/react-dom@18.2.0/client';

type Dict = Record<string, any>;

type Snapshot = {
  ts: string;
  receipt_hash?: string;
  metadata?: Dict;
  health?: Dict;
  app?: Dict;
  collab?: Dict;
  skills?: Dict;
  memory?: Dict;
  receipts?: Dict;
  logs?: Dict;
  apm?: Dict;
};

type SnapshotEnvelope = {
  type: string;
  snapshot?: Snapshot;
};

type Tone = 'ok' | 'warn' | 'bad';

const SECTIONS = [
  { id: 'command', label: 'Command Deck' },
  { id: 'fleet', label: 'Agent Fleet' },
  { id: 'graph', label: 'Activity Graph' },
  { id: 'explorer', label: 'Explorer' },
  { id: 'telemetry', label: 'Telemetry' },
  { id: 'governance', label: 'Governance' },
];

const TOUR_STEPS = [
  {
    title: 'Command Deck',
    body: 'Start in Command Deck to chat with chat-ui, run top actions, and inspect live session state.',
    focus: 'command',
  },
  {
    title: 'Fleet + Graph',
    body: 'Use Agent Fleet and Activity Graph together to trace handoffs and quickly respawn/update shadows.',
    focus: 'fleet',
  },
  {
    title: 'Explorer + Governance',
    body: 'Search receipts/logs/memory from Explorer, then apply guarded model/role/skill controls in Governance.',
    focus: 'explorer',
  },
] as const;

function cls(...parts: Array<string | false | null | undefined>): string {
  return parts.filter(Boolean).join(' ');
}

function shortHash(value: unknown, size = 14): string {
  const text = String(value ?? '').trim();
  if (!text) return 'n/a';
  return text.length <= size ? text : `${text.slice(0, size)}...`;
}

function fmtNumber(value: unknown): string {
  const num = Number(value);
  if (!Number.isFinite(num)) return 'n/a';
  if (Math.abs(num) >= 1000) return num.toLocaleString('en-US');
  if (Math.abs(num) >= 100) return num.toFixed(0);
  if (Math.abs(num) >= 10) return num.toFixed(1);
  return num.toFixed(2);
}

function statusTone(status: unknown): Tone {
  const value = String(status ?? '').trim().toLowerCase();
  if (['pass', 'ok', 'running', 'active', 'success'].includes(value)) return 'ok';
  if (['warn', 'warning', 'pending', 'paused'].includes(value)) return 'warn';
  return 'bad';
}

function iconTone(tone: Tone): string {
  if (tone === 'ok') return 'bg-emerald-400 shadow-[0_0_16px_rgba(52,211,153,.65)]';
  if (tone === 'warn') return 'bg-amber-400 shadow-[0_0_16px_rgba(251,191,36,.65)]';
  return 'bg-rose-400 shadow-[0_0_16px_rgba(251,113,133,.65)]';
}

function StatusPill({ status }: { status: unknown }) {
  const tone = statusTone(status);
  return (
    <span
      className={cls(
        'inline-flex items-center gap-1 rounded-full px-2 py-1 text-[10px] font-bold uppercase tracking-[.11em]',
        tone === 'ok' && 'bg-emerald-500/25 text-emerald-100',
        tone === 'warn' && 'bg-amber-500/20 text-amber-100',
        tone === 'bad' && 'bg-rose-500/22 text-rose-100'
      )}
    >
      <i className={cls('inline-block h-2 w-2 rounded-full', iconTone(tone))} />
      {String(status ?? 'unknown')}
    </span>
  );
}

function wsUrl(): string {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${protocol}//${window.location.host}/ws`;
}

async function fetchSnapshot(): Promise<Snapshot> {
  const res = await fetch('/api/dashboard/snapshot', { cache: 'no-store' });
  if (!res.ok) {
    throw new Error(`snapshot_http_${res.status}`);
  }
  return (await res.json()) as Snapshot;
}

async function postAction(action: string, payload: Dict): Promise<Dict> {
  const res = await fetch('/api/dashboard/action', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ action, payload }),
  });
  const data = await res.json();
  if (!res.ok || data.ok === false) {
    throw new Error(String(data.error || data.type || `action_http_${res.status}`));
  }
  return data;
}

function useDashboardState() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    let stop = false;
    let socket: WebSocket | null = null;
    let reconnectTimer: number | null = null;
    let backoffMs = 1000;

    const scheduleReconnect = () => {
      if (reconnectTimer != null || stop) return;
      reconnectTimer = window.setTimeout(() => {
        reconnectTimer = null;
        connectWs();
      }, backoffMs);
      backoffMs = Math.min(12000, Math.floor(backoffMs * 1.8));
    };

    const connectWs = () => {
      try {
        socket = new WebSocket(wsUrl());
      } catch (err) {
        setConnected(false);
        setError(String((err as Error).message || err));
        scheduleReconnect();
        return;
      }
      socket.addEventListener('open', () => {
        if (stop) return;
        backoffMs = 1000;
        setConnected(true);
      });
      socket.addEventListener('message', (event) => {
        if (stop) return;
        try {
          const envelope = JSON.parse(String(event.data)) as SnapshotEnvelope;
          if (envelope.type === 'snapshot' && envelope.snapshot) {
            setSnapshot(envelope.snapshot);
          }
        } catch {
          // ignore malformed envelope, stream will continue
        }
      });
      socket.addEventListener('close', () => {
        if (stop) return;
        setConnected(false);
        scheduleReconnect();
      });
      socket.addEventListener('error', () => {
        if (stop) return;
        setConnected(false);
        scheduleReconnect();
      });
    };

    fetchSnapshot()
      .then((row) => {
        if (!stop) setSnapshot(row);
      })
      .catch((err) => {
        if (!stop) setError(String((err as Error).message || err));
      })
      .finally(() => connectWs());

    return () => {
      stop = true;
      if (reconnectTimer != null) window.clearTimeout(reconnectTimer);
      if (socket) {
        try {
          socket.close();
        } catch {}
      }
    };
  }, []);

  return { snapshot, setSnapshot, connected, error, setError };
}

function containsQuery(query: string, fields: unknown[]): boolean {
  if (!query) return true;
  const haystack = fields
    .map((value) => String(value == null ? '' : value).toLowerCase())
    .join(' ');
  return haystack.includes(query);
}

function SectionCard(props: { id: string; title: string; subtitle?: string; children: React.ReactNode; action?: React.ReactNode }) {
  return (
    <section id={props.id} className="panel space-y-3">
      <header className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <h2 className="text-[13px] font-semibold uppercase tracking-[.12em] text-sky-100">{props.title}</h2>
          {props.subtitle ? <p className="text-xs text-slate-300">{props.subtitle}</p> : null}
        </div>
        {props.action ? <div>{props.action}</div> : null}
      </header>
      {props.children}
    </section>
  );
}

function WindowedList<T>(props: {
  items: T[];
  rowHeight: number;
  height: number;
  overscan?: number;
  emptyLabel?: string;
  keyFor: (item: T, index: number) => string;
  renderRow: (item: T, index: number) => React.ReactNode;
}) {
  const { items, rowHeight, height } = props;
  const overscan = props.overscan ?? 6;
  const [scrollTop, setScrollTop] = useState(0);

  const totalHeight = items.length * rowHeight;
  const start = Math.max(0, Math.floor(scrollTop / rowHeight) - overscan);
  const end = Math.min(items.length, Math.ceil((scrollTop + height) / rowHeight) + overscan);
  const visible = items.slice(start, end);

  if (items.length === 0) {
    return <div className="rounded-lg border border-slate-700/60 bg-slate-900/45 p-3 text-xs text-slate-400">{props.emptyLabel || 'No records'}</div>;
  }

  return (
    <div
      style={{ height }}
      className="overflow-y-auto rounded-lg border border-slate-700/60 bg-slate-950/55"
      onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
    >
      <div style={{ height: totalHeight, position: 'relative' }}>
        {visible.map((item, offset) => {
          const index = start + offset;
          return (
            <div
              key={props.keyFor(item, index)}
              style={{
                position: 'absolute',
                top: index * rowHeight,
                left: 0,
                right: 0,
                height: rowHeight,
              }}
              className="px-2"
            >
              {props.renderRow(item, index)}
            </div>
          );
        })}
      </div>
    </div>
  );
}

function App() {
  const { snapshot, setSnapshot, connected, error, setError } = useDashboardState();
  const [provider, setProvider] = useState('openai');
  const [model, setModel] = useState('gpt-5');
  const [team, setTeam] = useState('ops');
  const [role, setRole] = useState('analyst');
  const [shadow, setShadow] = useState('ops-analyst');
  const [skill, setSkill] = useState('');
  const [skillInput, setSkillInput] = useState('');
  const [chatInput, setChatInput] = useState('');
  const [chatTurns, setChatTurns] = useState<Dict[]>([]);
  const [search, setSearch] = useState('');
  const [focusView, setFocusView] = useState<string>('all');
  const [tourOpen, setTourOpen] = useState(false);
  const [tourStep, setTourStep] = useState(0);
  const deferredSearch = useDeferredValue(search.trim().toLowerCase());

  useEffect(() => {
    if (!snapshot?.app?.settings) return;
    const settings = snapshot.app.settings;
    setProvider(String(settings.provider || 'openai'));
    setModel(String(settings.model || 'gpt-5'));
  }, [snapshot?.app?.settings]);

  useEffect(() => {
    const turns = Array.isArray(snapshot?.app?.turns) ? snapshot.app.turns : [];
    if (turns.length > 0) setChatTurns(turns);
  }, [snapshot?.app?.turn_count, snapshot?.app?.receipt_hash]);

  useEffect(() => {
    const root = document.getElementById('root');
    if (root) root.setAttribute('data-dashboard-hydrated', 'react');
  }, []);

  useEffect(() => {
    try {
      const done = window.localStorage.getItem('infring_dashboard_tour_v1');
      if (!done) {
        setTourOpen(true);
      }
    } catch {
      // ignore localStorage failures
    }
  }, []);

  const runAction = async (action: string, payload: Dict): Promise<Dict | null> => {
    try {
      setError('');
      const response = await postAction(action, payload);
      if (response.snapshot) {
        setSnapshot(response.snapshot as Snapshot);
      } else {
        const fresh = await fetchSnapshot();
        setSnapshot(fresh);
      }
      return response;
    } catch (err) {
      setError(String((err as Error).message || err));
      return null;
    }
  };

  const receipts = useMemo(() => (Array.isArray(snapshot?.receipts?.recent) ? snapshot!.receipts.recent : []), [snapshot?.receipts]);
  const logs = useMemo(() => (Array.isArray(snapshot?.logs?.recent) ? snapshot!.logs.recent : []), [snapshot?.logs]);
  const memories = useMemo(() => (Array.isArray(snapshot?.memory?.entries) ? snapshot!.memory.entries : []), [snapshot?.memory]);
  const checks = useMemo(() => (snapshot?.health?.checks ? Object.entries(snapshot.health.checks) : []), [snapshot?.health]);
  const apmRows = useMemo(() => (Array.isArray(snapshot?.apm?.metrics) ? snapshot!.apm.metrics : []), [snapshot?.apm]);
  const agents = useMemo(() => (Array.isArray(snapshot?.collab?.dashboard?.agents) ? snapshot!.collab.dashboard.agents : []), [snapshot?.collab]);
  const hotspots = useMemo(() => (Array.isArray(snapshot?.skills?.metrics?.run_hotspots) ? snapshot!.skills.metrics.run_hotspots : []), [snapshot?.skills]);
  const handoffs = useMemo(() => (Array.isArray(snapshot?.collab?.dashboard?.handoff_history) ? snapshot!.collab.dashboard.handoff_history : []), [snapshot?.collab]);

  const filteredReceipts = useMemo(
    () => receipts.filter((row: Dict) => containsQuery(deferredSearch, [row.kind, row.path, row.mtime, row.size_bytes])),
    [receipts, deferredSearch]
  );
  const filteredLogs = useMemo(
    () => logs.filter((row: Dict) => containsQuery(deferredSearch, [row.ts, row.source, row.message])),
    [logs, deferredSearch]
  );
  const filteredMemories = useMemo(
    () => memories.filter((row: Dict) => containsQuery(deferredSearch, [row.scope, row.kind, row.path, row.mtime])),
    [memories, deferredSearch]
  );

  const kpis = useMemo(() => {
    const health = snapshot?.health || {};
    const metrics = health.dashboard_metrics || {};
    return {
      agents: agents.length,
      alerts: Number(health.alerts?.count || 0),
      burn: Number(metrics.token_burn_cost_attribution?.latest_day_tokens || 0),
      latency: metrics.vbrowser_session_surface?.stream_latency_ms,
      turns: Number(snapshot?.app?.turn_count || 0),
    };
  }, [snapshot, agents.length]);

  const graphNodes = useMemo(() => {
    const nodes: Array<{ id: string; label: string; x: number; y: number; tone: Tone }> = [
      { id: 'chat-ui', label: 'chat-ui', x: 88, y: 72, tone: 'ok' },
    ];
    handoffs.slice(0, 8).forEach((row: Dict, idx: number) => {
      nodes.push({
        id: String(row.shadow || `shadow-${idx}`),
        label: shortHash(row.shadow || `shadow-${idx}`, 14),
        x: 250 + idx * 106,
        y: idx % 2 === 0 ? 56 : 120,
        tone: statusTone(row.status || 'unknown'),
      });
    });
    chatTurns.slice(-4).forEach((turn: Dict, idx: number) => {
      nodes.push({
        id: String(turn.turn_id || `turn-${idx}`),
        label: shortHash(turn.turn_id || `turn-${idx}`, 10),
        x: 170 + idx * 170,
        y: 242,
        tone: 'warn',
      });
    });
    return nodes;
  }, [handoffs, chatTurns]);

  const nodeMap = useMemo(() => {
    const map = new Map<string, { x: number; y: number }>();
    for (const row of graphNodes) map.set(row.id, { x: row.x, y: row.y });
    return map;
  }, [graphNodes]);

  const graphEdges = useMemo(() => {
    const edges: Array<{ from: string; to: string; label: string }> = [];
    handoffs.slice(0, 8).forEach((row: Dict, idx: number) => {
      edges.push({
        from: 'chat-ui',
        to: String(row.shadow || `shadow-${idx}`),
        label: shortHash(row.job_id || 'handoff', 10),
      });
    });
    chatTurns.slice(-4).forEach((turn: Dict, idx: number) => {
      edges.push({
        from: 'chat-ui',
        to: String(turn.turn_id || `turn-${idx}`),
        label: shortHash(turn.provider || 'turn', 8),
      });
    });
    return edges;
  }, [handoffs, chatTurns]);

  const jumpTo = (id: string) => {
    setFocusView(id);
    window.setTimeout(() => {
      const node = document.getElementById(id);
      if (node) node.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }, 40);
  };

  const show = (id: string): boolean => focusView === 'all' || focusView === id;

  const finishTour = () => {
    setTourOpen(false);
    try {
      window.localStorage.setItem('infring_dashboard_tour_v1', '1');
    } catch {
      // ignore localStorage failures
    }
  };

  const stepMeta = TOUR_STEPS[Math.max(0, Math.min(tourStep, TOUR_STEPS.length - 1))];

  return (
    <div className="min-h-screen bg-transparent text-slate-100">
      <div className="mx-auto max-w-[1580px] px-3 pb-10 pt-3">
        <header className="dashboard-head sticky top-3 z-40">
          <div>
            <h1 className="text-xl font-bold tracking-wide">InfRing Control Plane</h1>
            <p className="text-xs text-slate-300">Fast command deck over Rust-core lanes with receipted actions.</p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <input
              className="input h-9 min-w-[260px]"
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder="Search receipts, logs, memory, checks..."
            />
            <button className="btn" onClick={() => fetchSnapshot().then(setSnapshot).catch((err) => setError(String((err as Error).message || err)))}>
              Refresh
            </button>
          </div>
        </header>

        <div className="mt-3 grid gap-3 lg:grid-cols-[260px_1fr]">
          <aside className="panel h-fit lg:sticky lg:top-[88px]">
            <h2 className="text-xs font-bold uppercase tracking-[.14em] text-slate-200">Navigation</h2>
            <div className="mt-2 space-y-1">
              <button className={cls('nav-chip', focusView === 'all' && 'nav-chip-active')} onClick={() => setFocusView('all')}>
                All Surfaces
              </button>
              {SECTIONS.map((section) => (
                <button
                  key={section.id}
                  className={cls('nav-chip', focusView === section.id && 'nav-chip-active')}
                  onClick={() => jumpTo(section.id)}
                >
                  {section.label}
                </button>
              ))}
            </div>
            <div className="mt-3 rounded-xl border border-slate-700/60 bg-slate-900/60 p-2 text-xs">
              <div className="flex items-center gap-2">
                <i className={cls('inline-block h-2.5 w-2.5 rounded-full', connected ? iconTone('ok') : iconTone('bad'), connected && 'pulse-ok')} />
                {connected ? 'Realtime stream online' : 'Realtime reconnecting'}
              </div>
              <div className="mono mt-1 text-[11px] text-slate-300">receipt {shortHash(snapshot?.receipt_hash, 24)}</div>
            </div>
            <button className="btn mt-2 w-full" onClick={() => { setTourStep(0); setTourOpen(true); }}>
              Show Guided Tour
            </button>
          </aside>

          <main className="space-y-3">
            {error ? <div className="rounded-xl border border-rose-400/40 bg-rose-500/10 px-3 py-2 text-xs text-rose-100">{error}</div> : null}

            {show('command') ? (
            <SectionCard id="command" title="Command Deck" subtitle="Chat, top-level controls, and immediate action lanes.">
              <div className="grid gap-3 xl:grid-cols-[1.2fr_.8fr]">
                <article className="tile">
                  <div className="mb-2 text-xs text-slate-300">chat-ui session <span className="mono">{String(snapshot?.app?.session_id || 'chat-ui-default')}</span></div>
                  <div className="max-h-[300px] overflow-auto rounded-lg border border-slate-700/60 bg-slate-950/60 p-2">
                    {chatTurns.length === 0 ? (
                      <div className="text-xs text-slate-400">No turns yet. Send a message to start.</div>
                    ) : (
                      <div className="space-y-2">
                        {chatTurns.slice(-20).map((turn: Dict, idx: number) => (
                          <article key={`${turn.turn_id || 'turn'}-${idx}`} className="rounded-lg border border-slate-700/60 bg-slate-900/50 p-2">
                            <div className="text-[11px] text-slate-400">{String(turn.ts || 'n/a')} · {String(turn.provider || 'unknown')}/{String(turn.model || 'n/a')}</div>
                            <p className="mt-1 text-xs text-sky-100"><span className="font-semibold text-sky-300">User</span> {String(turn.user || '')}</p>
                            <p className="mt-1 text-xs text-emerald-100"><span className="font-semibold text-emerald-300">Assistant</span> {String(turn.assistant || '')}</p>
                          </article>
                        ))}
                      </div>
                    )}
                  </div>
                  <form
                    className="mt-2 flex gap-2"
                    onSubmit={async (event) => {
                      event.preventDefault();
                      const text = chatInput.trim();
                      if (!text) return;
                      const response = await runAction('app.chat', { input: text });
                      const turn = response && response.lane && response.lane.turn ? response.lane.turn : null;
                      if (turn && typeof turn === 'object') setChatTurns((prev) => [...prev, turn]);
                      setChatInput('');
                    }}
                  >
                    <input className="input" value={chatInput} onChange={(e) => setChatInput(e.target.value)} placeholder="Message chat-ui..." />
                    <button className="btn" type="submit">Send</button>
                  </form>
                </article>

                <article className="tile grid gap-2 sm:grid-cols-2 xl:grid-cols-1">
                  <button className="action-btn" onClick={() => runAction('dashboard.assimilate', { target: 'codex' })}>Assimilate Target</button>
                  <button className="action-btn" onClick={() => runAction('dashboard.benchmark', {})}>Benchmark Surface</button>
                  <button className="action-btn" onClick={() => runAction('app.switchProvider', { provider, model })}>Reapply Provider/Model</button>
                  <button className="action-btn" onClick={() => jumpTo('governance')}>Open Governance</button>

                  <div className="mt-1 grid gap-2 sm:grid-cols-2 xl:grid-cols-1">
                    <div className="kpi-card"><div className="kpi-label">Active Agents</div><div className="kpi-value">{fmtNumber(kpis.agents)}</div></div>
                    <div className="kpi-card"><div className="kpi-label">Open Alerts</div><div className="kpi-value">{fmtNumber(kpis.alerts)}</div></div>
                    <div className="kpi-card"><div className="kpi-label">Daily Token Burn</div><div className="kpi-value">{fmtNumber(kpis.burn)}</div></div>
                    <div className="kpi-card"><div className="kpi-label">Turns</div><div className="kpi-value">{fmtNumber(kpis.turns)}</div></div>
                  </div>
                </article>
              </div>
            </SectionCard>
            ) : null}

            {show('fleet') ? (
            <SectionCard id="fleet" title="Agent Fleet" subtitle="Card-first fleet view with direct quick actions.">
              <div className="grid gap-2 md:grid-cols-2 xl:grid-cols-3">
                <article className="tile">
                  <header className="flex items-center justify-between">
                    <h3 className="font-semibold">chat-ui</h3>
                    <StatusPill status="active" />
                  </header>
                  <div className="mt-2 text-xs text-slate-300">Provider {String(snapshot?.app?.settings?.provider || 'n/a')}</div>
                  <div className="text-xs text-slate-300">Model {String(snapshot?.app?.settings?.model || 'n/a')}</div>
                  <div className="text-xs text-slate-300">Turns {fmtNumber(snapshot?.app?.turn_count || 0)}</div>
                  <div className="mt-2 flex flex-wrap gap-1">
                    <button className="micro-btn" onClick={() => jumpTo('command')}>Open Chat</button>
                    <button className="micro-btn" onClick={() => jumpTo('explorer')}>View Receipts</button>
                  </div>
                </article>
                {agents.map((row: Dict, idx: number) => (
                  <article key={`${row.shadow || 'shadow'}-${idx}`} className="tile">
                    <header className="flex items-center justify-between">
                      <h3 className="font-semibold">{String(row.shadow || 'shadow')}</h3>
                      <StatusPill status={row.status || 'unknown'} />
                    </header>
                    <div className="mt-2 text-xs text-slate-300">Role {String(row.role || 'unknown')}</div>
                    <div className="text-xs text-slate-300">Activated {String(row.activated_at || 'n/a')}</div>
                    <div className="mt-2 flex flex-wrap gap-1">
                      <button className="micro-btn" onClick={() => jumpTo('graph')}>Trace Flow</button>
                      <button className="micro-btn" onClick={() => jumpTo('explorer')}>Memory</button>
                      <button className="micro-btn" onClick={() => runAction('collab.launchRole', { team, role, shadow: String(row.shadow || `${team}-${role}`) })}>Respawn</button>
                    </div>
                  </article>
                ))}
              </div>
            </SectionCard>
            ) : null}

            {show('graph') ? (
            <SectionCard id="graph" title="Activity Graph" subtitle="Live handoff topology and turn edges.">
              <div className="rounded-xl border border-slate-700/60 bg-slate-950/60 p-2">
                <svg viewBox="0 0 1140 320" className="h-[300px] w-full">
                  {graphEdges.map((edge, idx) => {
                    const a = nodeMap.get(edge.from);
                    const b = nodeMap.get(edge.to);
                    if (!a || !b) return null;
                    return (
                      <g key={`edge-${idx}`}>
                        <line x1={a.x} y1={a.y} x2={b.x} y2={b.y} stroke="#4c79a6" strokeWidth="1.6" strokeDasharray="5 4" />
                        <text x={(a.x + b.x) / 2} y={(a.y + b.y) / 2 - 6} fill="#9ec6ef" fontSize="9" textAnchor="middle">{edge.label}</text>
                      </g>
                    );
                  })}
                  {graphNodes.map((node) => {
                    const tone = node.tone;
                    return (
                      <g key={node.id}>
                        <circle
                          cx={node.x}
                          cy={node.y}
                          r={node.id === 'chat-ui' ? 20 : 14}
                          fill={tone === 'ok' ? '#163042' : tone === 'warn' ? '#3a3118' : '#3a1c26'}
                          stroke={tone === 'ok' ? '#4de2c5' : tone === 'warn' ? '#ffb347' : '#fb7185'}
                          strokeWidth="1.7"
                        />
                        <text x={node.x} y={node.y + 26} fill="#e8f0ff" fontSize="10" textAnchor="middle">{node.label}</text>
                      </g>
                    );
                  })}
                </svg>
              </div>
            </SectionCard>
            ) : null}

            {show('explorer') ? (
            <SectionCard id="explorer" title="Explorer" subtitle="Virtualized receipts, logs, and memory with global filter.">
              <div className="grid gap-3 xl:grid-cols-3">
                <article className="tile">
                  <h3 className="mb-2 text-xs font-bold uppercase tracking-[.1em] text-slate-300">Receipts</h3>
                  <WindowedList
                    items={filteredReceipts}
                    rowHeight={44}
                    height={300}
                    emptyLabel="No receipts"
                    keyFor={(row: Dict, idx) => `${row.path || 'receipt'}-${idx}`}
                    renderRow={(row: Dict) => (
                      <div className="mt-1 rounded-md border border-slate-700/60 bg-slate-900/50 px-2 py-1 text-[11px]">
                        <div className="font-semibold text-slate-100">{String(row.kind || 'artifact')}</div>
                        <div className="mono text-slate-300">{shortHash(row.path || '', 48)}</div>
                      </div>
                    )}
                  />
                </article>

                <article className="tile">
                  <h3 className="mb-2 text-xs font-bold uppercase tracking-[.1em] text-slate-300">Logs</h3>
                  <WindowedList
                    items={filteredLogs}
                    rowHeight={56}
                    height={300}
                    emptyLabel="No logs"
                    keyFor={(row: Dict, idx) => `${row.source || 'log'}-${idx}`}
                    renderRow={(row: Dict) => (
                      <div className="mt-1 rounded-md border border-slate-700/60 bg-slate-900/50 px-2 py-1 text-[11px]">
                        <div className="mono text-slate-300">{shortHash(row.ts || 'n/a', 24)} · {shortHash(row.source || '', 24)}</div>
                        <div className="text-slate-100">{shortHash(row.message || '', 70)}</div>
                      </div>
                    )}
                  />
                </article>

                <article className="tile">
                  <h3 className="mb-2 text-xs font-bold uppercase tracking-[.1em] text-slate-300">Memory</h3>
                  <WindowedList
                    items={filteredMemories}
                    rowHeight={50}
                    height={300}
                    emptyLabel="No memory entries"
                    keyFor={(row: Dict, idx) => `${row.path || 'memory'}-${idx}`}
                    renderRow={(row: Dict) => (
                      <div className="mt-1 rounded-md border border-slate-700/60 bg-slate-900/50 px-2 py-1 text-[11px]">
                        <div className="text-slate-200">{String(row.scope || 'state')} · {String(row.kind || 'snapshot')}</div>
                        <div className="mono text-slate-300">{shortHash(row.path || '', 48)}</div>
                      </div>
                    )}
                  />
                </article>
              </div>
            </SectionCard>
            ) : null}

            {show('telemetry') ? (
            <SectionCard id="telemetry" title="Telemetry" subtitle="APM and channel checks with visual status at-a-glance.">
              <div className="grid gap-3 xl:grid-cols-[1.2fr_.8fr]">
                <article className="tile">
                  <h3 className="mb-2 text-xs font-bold uppercase tracking-[.1em] text-slate-300">APM metrics</h3>
                  <div className="grid gap-2 sm:grid-cols-2">
                    {apmRows.slice(0, 12).map((row: Dict) => (
                      <div key={String(row.name || 'metric')} className="rounded-lg border border-slate-700/60 bg-slate-900/50 p-2">
                        <div className="text-xs font-semibold text-slate-100">{String(row.name || 'metric')}</div>
                        <div className="mt-1 text-[11px] text-slate-300">Value {fmtNumber(row.value)}</div>
                        <div className="text-[11px] text-slate-300">Target {String(row.target || 'n/a')}</div>
                        <div className="mt-1"><StatusPill status={row.status || 'unknown'} /></div>
                      </div>
                    ))}
                  </div>
                </article>
                <article className="tile">
                  <h3 className="mb-2 text-xs font-bold uppercase tracking-[.1em] text-slate-300">Channel checks</h3>
                  <div className="max-h-[340px] overflow-auto space-y-2">
                    {checks.slice(0, 14).map(([name, row]: [string, any]) => (
                      <div key={name} className="rounded-lg border border-slate-700/60 bg-slate-900/50 p-2 text-xs">
                        <div className="flex items-center justify-between gap-2">
                          <div className="font-semibold text-slate-100">{name}</div>
                          <StatusPill status={row?.status || 'unknown'} />
                        </div>
                        <div className="mono mt-1 text-[11px] text-slate-300">{String(row?.source || 'n/a')}</div>
                      </div>
                    ))}
                  </div>
                </article>
              </div>
            </SectionCard>
            ) : null}

            {show('governance') ? (
            <SectionCard id="governance" title="Governance Controls" subtitle="Model, role, and skill actions with strict lane routing.">
              <div className="grid gap-3 xl:grid-cols-3">
                <form
                  className="tile space-y-2"
                  onSubmit={(event) => {
                    event.preventDefault();
                    runAction('app.switchProvider', { provider, model });
                  }}
                >
                  <h3 className="font-semibold">Provider / Model</h3>
                  <div>
                    <label className="text-xs text-slate-300">Provider</label>
                    <input className="input" value={provider} onChange={(e) => setProvider(e.target.value)} />
                  </div>
                  <div>
                    <label className="text-xs text-slate-300">Model</label>
                    <input className="input" value={model} onChange={(e) => setModel(e.target.value)} />
                  </div>
                  <button className="btn">Switch</button>
                </form>

                <form
                  className="tile space-y-2"
                  onSubmit={(event) => {
                    event.preventDefault();
                    runAction('collab.launchRole', { team, role, shadow });
                  }}
                >
                  <h3 className="font-semibold">Launch Team Role</h3>
                  <div>
                    <label className="text-xs text-slate-300">Team</label>
                    <input className="input" value={team} onChange={(e) => setTeam(e.target.value)} />
                  </div>
                  <div>
                    <label className="text-xs text-slate-300">Role</label>
                    <input className="input" value={role} onChange={(e) => setRole(e.target.value)} />
                  </div>
                  <div>
                    <label className="text-xs text-slate-300">Shadow</label>
                    <input className="input" value={shadow} onChange={(e) => setShadow(e.target.value)} />
                  </div>
                  <button className="btn">Launch</button>
                </form>

                <form
                  className="tile space-y-2"
                  onSubmit={(event) => {
                    event.preventDefault();
                    if (!skill.trim()) return;
                    runAction('skills.run', { skill, input: skillInput });
                  }}
                >
                  <h3 className="font-semibold">Run Skill</h3>
                  <div>
                    <label className="text-xs text-slate-300">Skill</label>
                    <input className="input" value={skill} onChange={(e) => setSkill(e.target.value)} placeholder="compat_skill" />
                  </div>
                  <div>
                    <label className="text-xs text-slate-300">Input</label>
                    <input className="input" value={skillInput} onChange={(e) => setSkillInput(e.target.value)} placeholder="optional payload" />
                  </div>
                  <button className="btn">Run</button>
                </form>
              </div>

              <div className="grid gap-2 md:grid-cols-2 xl:grid-cols-4">
                {hotspots.slice(0, 8).map((row: Dict, idx: number) => (
                  <article key={`${row.skill || row.name || idx}`} className="tile">
                    <h4 className="font-semibold">{String(row.skill || row.name || 'skill')}</h4>
                    <div className="mt-1 text-xs text-slate-300">Runs {fmtNumber(row.runs)}</div>
                  </article>
                ))}
              </div>
            </SectionCard>
            ) : null}
          </main>
        </div>
      </div>
      {tourOpen ? (
        <div className="fixed inset-0 z-50 flex items-end justify-center bg-black/65 p-3 md:items-center">
          <article className="w-full max-w-[640px] rounded-2xl border border-sky-300/40 bg-slate-950/95 p-4 shadow-[0_0_45px_rgba(0,240,255,.24)]">
            <div className="text-[11px] uppercase tracking-[.14em] text-slate-400">
              Guided Tour {tourStep + 1} / {TOUR_STEPS.length}
            </div>
            <h3 className="mt-1 text-lg font-semibold text-sky-100">{stepMeta.title}</h3>
            <p className="mt-1 text-sm text-slate-200">{stepMeta.body}</p>
            <div className="mt-3 flex flex-wrap gap-2">
              <button
                className="micro-btn"
                onClick={() => {
                  setFocusView(stepMeta.focus);
                  window.setTimeout(() => jumpTo(stepMeta.focus), 20);
                }}
              >
                Focus This Section
              </button>
              <button className="micro-btn" onClick={finishTour}>
                Skip Tour
              </button>
            </div>
            <div className="mt-4 flex justify-between">
              <button
                className="btn"
                onClick={() => setTourStep((prev) => Math.max(0, prev - 1))}
                disabled={tourStep === 0}
              >
                Back
              </button>
              {tourStep < TOUR_STEPS.length - 1 ? (
                <button className="btn" onClick={() => setTourStep((prev) => Math.min(TOUR_STEPS.length - 1, prev + 1))}>
                  Next
                </button>
              ) : (
                <button className="btn" onClick={finishTour}>
                  Finish
                </button>
              )}
            </div>
          </article>
        </div>
      ) : null}
    </div>
  );
}

const rootNode = document.getElementById('root');
if (!rootNode) {
  throw new Error('dashboard_root_missing');
}
createRoot(rootNode).render(<App />);
