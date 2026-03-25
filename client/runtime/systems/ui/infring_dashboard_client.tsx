import React, { useEffect, useMemo, useRef, useState } from 'https://esm.sh/react@18.2.0';
import { createRoot } from 'https://esm.sh/react-dom@18.2.0/client';

type Dict = Record<string, unknown>;

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

type ActionResponse = Dict & {
  ok?: boolean;
  error?: unknown;
  type?: unknown;
  snapshot?: Snapshot;
};

type Tone = 'ok' | 'warn' | 'bad';

type ControlPane = {
  id: string;
  label: string;
};

const CONTROL_PANES: ControlPane[] = [
  { id: 'chat', label: 'Chat' },
  { id: 'swarm', label: 'Swarm / Agent Management' },
  { id: 'health', label: 'Runtime Health' },
  { id: 'receipts', label: 'Receipts & Audit' },
  { id: 'logs', label: 'Logs' },
  { id: 'settings', label: 'Settings' },
];

const THEME_KEY = 'infring_dashboard_theme_v2';
const CONTROLS_OPEN_KEY = 'infring_dashboard_controls_open_v2';
const PANES_KEY = 'infring_dashboard_controls_panes_v1';

function cls(...parts: Array<string | false | null | undefined>): string {
  return parts.filter(Boolean).join(' ');
}

function isRecord(value: unknown): value is Dict {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

function safeParseJson(raw: string): unknown {
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

async function readJsonObject(res: Response): Promise<Dict> {
  const raw = await res.text();
  const parsed = safeParseJson(raw);
  if (!isRecord(parsed)) return {};
  return parsed;
}

function normalizeSnapshot(value: unknown): Snapshot | null {
  if (!isRecord(value)) return null;
  return value as Snapshot;
}

function normalizeSnapshotEnvelope(value: unknown): SnapshotEnvelope | null {
  if (!isRecord(value)) return null;
  const type = asText(value.type || '').trim();
  if (!type) return null;
  const snapshot = normalizeSnapshot(value.snapshot);
  return snapshot ? { type, snapshot } : { type };
}

function asText(value: unknown, fallback = ''): string {
  if (value == null) return fallback;
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}

function shortText(value: unknown, size = 96): string {
  const text = asText(value).trim();
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
  const value = asText(status).trim().toLowerCase();
  if (['pass', 'ok', 'running', 'active', 'success', 'complete', 'live'].includes(value)) return 'ok';
  if (['warn', 'warning', 'pending', 'paused', 'thinking', 'tool_call', 'reconnecting'].includes(value)) return 'warn';
  return 'bad';
}

function iconTone(tone: Tone): string {
  if (tone === 'ok') return 'bg-emerald-400';
  if (tone === 'warn') return 'bg-amber-400';
  return 'bg-rose-400';
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
      {asText(status, 'unknown')}
    </span>
  );
}

function wsUrl(): string {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${protocol}//${window.location.host}/ws`;
}

async function fetchSnapshot(): Promise<Snapshot> {
  const res = await fetch('/api/dashboard/snapshot', { cache: 'no-store' });
  if (!res.ok) throw new Error(`snapshot_http_${res.status}`);
  const payload = await readJsonObject(res);
  const snapshot = normalizeSnapshot(payload);
  if (!snapshot) throw new Error('snapshot_payload_invalid');
  return snapshot;
}

async function postAction(action: string, payload: Dict): Promise<ActionResponse> {
  const res = await fetch('/api/dashboard/action', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ action, payload }),
  });
  const data = await readJsonObject(res);
  const actionData = data as ActionResponse;
  if (!res.ok || actionData.ok === false) {
    throw new Error(asText(actionData.error || actionData.type || `action_http_${res.status}`));
  }
  return actionData;
}

function readControlsOpen(): boolean {
  try {
    if (window.localStorage.getItem(CONTROLS_OPEN_KEY) === '1') return true;
    return window.localStorage.getItem('infring_dashboard_controls_open') === '1';
  } catch {
    return false;
  }
}

function readTheme(): 'dark' | 'light' {
  try {
    const next = window.localStorage.getItem(THEME_KEY);
    if (next === 'light') return 'light';
    if (next === 'dark') return 'dark';
    const legacy = window.localStorage.getItem('infring_dashboard_theme_v1');
    return legacy === 'light' ? 'light' : 'dark';
  } catch {
    return 'dark';
  }
}

function readPaneState(): Record<string, boolean> {
  const seed: Record<string, boolean> = {};
  for (const pane of CONTROL_PANES) {
    seed[pane.id] = pane.id === 'chat' || pane.id === 'swarm';
  }
  try {
    const raw = window.localStorage.getItem(PANES_KEY);
    if (!raw) return seed;
    const parsedUnknown = safeParseJson(raw);
    const parsed = isRecord(parsedUnknown) ? parsedUnknown : {};
    const normalized: Record<string, boolean> = {};
    for (const pane of CONTROL_PANES) normalized[pane.id] = parsed[pane.id] === true;
    return normalized;
  } catch {
    return seed;
  }
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
        setError(asText((err as Error).message || err));
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
          const parsed = safeParseJson(asText(event.data));
          const envelope = normalizeSnapshotEnvelope(parsed);
          if (envelope && envelope.type === 'snapshot' && envelope.snapshot) setSnapshot(envelope.snapshot);
        } catch {
          // ignore malformed envelope
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
        if (!stop) setError(asText((err as Error).message || err));
      })
      .finally(() => connectWs());

    return () => {
      stop = true;
      if (reconnectTimer != null) window.clearTimeout(reconnectTimer);
      if (socket) {
        try {
          socket.close();
        } catch {
          // ignore close race
        }
      }
    };
  }, []);

  return { snapshot, setSnapshot, connected, error, setError };
}

function DrawerAccordion(props: {
  id: string;
  label: string;
  open: boolean;
  onToggle: (id: string) => void;
  children: React.ReactNode;
}) {
  return (
    <section className="drawer-section">
      <button className="drawer-toggle" onClick={() => props.onToggle(props.id)}>
        <span>{props.label}</span>
        <span className="mono text-[11px]">{props.open ? '−' : '+'}</span>
      </button>
      {props.open ? <div className="drawer-body">{props.children}</div> : null}
    </section>
  );
}

function App() {
  const { snapshot, setSnapshot, connected, error, setError } = useDashboardState();
  const chatInputRef = useRef<HTMLInputElement | null>(null);
  const [provider, setProvider] = useState('openai');
  const [model, setModel] = useState('gpt-5');
  const [team, setTeam] = useState('ops');
  const [role, setRole] = useState('analyst');
  const [shadow, setShadow] = useState('ops-analyst');
  const [chatInput, setChatInput] = useState('');
  const [drawerChatInput, setDrawerChatInput] = useState('');
  const [chatTurns, setChatTurns] = useState<Dict[]>([]);
  const [sending, setSending] = useState(false);
  const [controlsOpen, setControlsOpen] = useState<boolean>(() => readControlsOpen());
  const [theme, setTheme] = useState<'dark' | 'light'>(() => readTheme());
  const [openPanes, setOpenPanes] = useState<Record<string, boolean>>(() => readPaneState());

  useEffect(() => {
    const root = document.getElementById('root');
    if (root) root.setAttribute('data-dashboard-hydrated', 'react');
  }, []);

  useEffect(() => {
    if (!snapshot?.app?.settings) return;
    const settings = snapshot.app.settings;
    setProvider(asText(settings.provider || 'openai'));
    setModel(asText(settings.model || 'gpt-5'));
  }, [snapshot?.app?.settings]);

  useEffect(() => {
    const turns = Array.isArray(snapshot?.app?.turns) ? snapshot.app.turns : [];
    if (turns.length > 0) setChatTurns(turns);
  }, [snapshot?.app?.turn_count, snapshot?.app?.receipt_hash]);

  useEffect(() => {
    try {
      window.localStorage.setItem(CONTROLS_OPEN_KEY, controlsOpen ? '1' : '0');
      window.localStorage.setItem('infring_dashboard_controls_open', controlsOpen ? '1' : '0');
    } catch {
      // ignore storage failures
    }
  }, [controlsOpen]);

  useEffect(() => {
    try {
      window.localStorage.setItem(THEME_KEY, theme);
      window.localStorage.setItem('infring_dashboard_theme_v1', theme);
    } catch {
      // ignore storage failures
    }
    document.documentElement.setAttribute('data-infring-theme', theme);
  }, [theme]);

  useEffect(() => {
    try {
      window.localStorage.setItem(PANES_KEY, JSON.stringify(openPanes));
    } catch {
      // ignore storage failures
    }
  }, [openPanes]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const metaOrCtrl = event.metaKey || event.ctrlKey;
      if (metaOrCtrl && event.key.toLowerCase() === 'k') {
        event.preventDefault();
        chatInputRef.current?.focus();
        return;
      }
      if (event.key === 'Escape' && controlsOpen) {
        void toggleControls(false);
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [controlsOpen]);

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
      setError(asText((err as Error).message || err));
      return null;
    }
  };

  const agents = useMemo(
    () => (Array.isArray(snapshot?.collab?.dashboard?.agents) ? snapshot.collab.dashboard.agents : []),
    [snapshot?.collab]
  );
  const checks = useMemo(() => (snapshot?.health?.checks ? Object.entries(snapshot.health.checks) : []), [snapshot?.health]);
  const receipts = useMemo(() => (Array.isArray(snapshot?.receipts?.recent) ? snapshot.receipts.recent : []), [snapshot?.receipts]);
  const logs = useMemo(() => (Array.isArray(snapshot?.logs?.recent) ? snapshot.logs.recent : []), [snapshot?.logs]);
  const alertsCount = Number(snapshot?.health?.alerts?.count || 0);
  const queueDepth = Number(snapshot?.attention_queue?.queue_depth || 0);
  const syncMode = asText(snapshot?.attention_queue?.backpressure?.sync_mode || 'live_sync');
  const backpressureLevel = asText(snapshot?.attention_queue?.backpressure?.level || 'normal');
  const criticalAttention = Number(snapshot?.attention_queue?.priority_counts?.critical || 0);
  const criticalAttentionTotal = Number(snapshot?.attention_queue?.critical_total_count || criticalAttention);
  const criticalEventsFull = useMemo(
    () => (Array.isArray(snapshot?.attention_queue?.critical_events_full) ? snapshot.attention_queue.critical_events_full : []),
    [snapshot?.attention_queue]
  );
  const conduitSignals = Number(snapshot?.cockpit?.metrics?.conduit_signals || 0);
  const conduitChannels = Number(snapshot?.cockpit?.metrics?.conduit_channels_observed || conduitSignals);
  const conduitTargetSignals = Number(snapshot?.attention_queue?.backpressure?.target_conduit_signals || 4);
  const conduitScaleRequired = !!snapshot?.attention_queue?.backpressure?.scale_required;
  const benchmarkCheck = (snapshot?.health?.checks?.benchmark_sanity || {}) as Dict;
  const benchmarkStatus = asText(benchmarkCheck.status || 'unknown');
  const benchmarkAgeSec = Number(benchmarkCheck.age_seconds ?? -1);
  const memoryStream = (snapshot?.memory?.stream || {}) as Dict;
  const ingestControl = (snapshot?.memory?.ingest_control || {}) as Dict;
  const healthCoverage = (snapshot?.health?.coverage || {}) as Dict;
  const runtimeRecommendation = (snapshot?.runtime_recommendation || {}) as Dict;
  const runtimeRolePlan = useMemo(
    () => (Array.isArray(runtimeRecommendation.role_plan) ? runtimeRecommendation.role_plan : []),
    [runtimeRecommendation]
  );

  const toggleControls = async (next?: boolean) => {
    const open = typeof next === 'boolean' ? next : !controlsOpen;
    setControlsOpen(open);
    await runAction('dashboard.ui.toggleControls', { open });
    if (open) {
      await runAction('dashboard.ui.switchControlsTab', { tab: 'swarm' });
    }
  };

  const togglePane = (id: string) => {
    setOpenPanes((prev) => {
      const nextOpen = !prev[id];
      void runAction('dashboard.ui.toggleSection', { section: id, open: nextOpen });
      return { ...prev, [id]: nextOpen };
    });
  };

  const refreshSnapshot = async () => {
    try {
      setError('');
      const fresh = await fetchSnapshot();
      setSnapshot(fresh);
    } catch (err) {
      setError(asText((err as Error).message || err));
    }
  };

  const sendChat = async (input: string) => {
    const text = input.trim();
    if (!text) return;
    setSending(true);
    const response = await runAction('app.chat', { input: text });
    const turn = response && response.lane && response.lane.turn ? response.lane.turn : null;
    if (turn && typeof turn === 'object') setChatTurns((prev) => [...prev, turn]);
    setSending(false);
  };

  const quickAction = async (kind: 'new_agent' | 'new_swarm' | 'assimilate' | 'benchmark' | 'open_controls' | 'swarm' | 'runtime_swarm') => {
    if (kind === 'new_agent') {
      await runAction('collab.launchRole', { team, role: 'analyst', shadow: `${team}-analyst` });
      return;
    }
    if (kind === 'new_swarm') {
      await runAction('collab.launchRole', { team, role: 'orchestrator', shadow: `${team}-orchestrator` });
      return;
    }
    if (kind === 'assimilate') {
      await runAction('dashboard.assimilate', { target: 'codex' });
      return;
    }
    if (kind === 'benchmark') {
      await runAction('dashboard.benchmark', {});
      return;
    }
    if (kind === 'runtime_swarm') {
      await runAction('dashboard.runtime.executeSwarmRecommendation', {});
      return;
    }
    await toggleControls(true);
    if (kind === 'swarm') {
      setOpenPanes((prev) => ({ ...prev, swarm: true }));
      await runAction('dashboard.ui.switchControlsTab', { tab: 'swarm' });
    }
  };

  const recentTurns = chatTurns.slice(-40);
  const recentReceipts = receipts.slice(0, 18);
  const recentLogs = logs.slice(0, 18);
  const recentChecks = useMemo(() => {
    const sorted = checks.slice().sort((a, b) => {
      if (a[0] === 'benchmark_sanity') return -1;
      if (b[0] === 'benchmark_sanity') return 1;
      return String(a[0]).localeCompare(String(b[0]));
    });
    return sorted.slice(0, 16);
  }, [checks]);

  return (
    <div className="dash-root min-h-screen bg-transparent text-slate-100">
      <header className="dash-topbar sticky top-0 z-40">
        <div className="top-left-cluster">
          <div className="top-brand">
            <h1 className="text-[15px] font-semibold tracking-[.01em]">InfRing Chat</h1>
            <p className="text-[11px] text-slate-300">Simple default chat. Open Controls only when needed.</p>
          </div>
          <div className="top-controls">
            <StatusPill status={connected ? 'live' : 'reconnecting'} />
            <button className="btn" onClick={() => toggleControls()}>
              {controlsOpen ? 'Close Controls' : 'Open Controls'}
            </button>
            <button className="micro-btn" onClick={refreshSnapshot}>
              Refresh
            </button>
          </div>
        </div>
        <div className="top-right-cluster">
          <div className="avatar-chip" title="Operator">
            <span>J</span>
          </div>
          <button
            className={cls('theme-switch', theme === 'light' && 'light')}
            onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
            title="Toggle light or dark mode"
            aria-label="Toggle light or dark mode"
            role="switch"
            aria-checked={theme === 'light'}
          >
            <span className="theme-switch-track">
              <span className="theme-switch-thumb" />
            </span>
            <span className="theme-switch-label">{theme === 'dark' ? 'Dark' : 'Light'}</span>
          </button>
        </div>
      </header>

      <main className="dash-main">
        <section className="chat-panel">
          <header className="chat-panel-head">
            <div>
              <h2>Chat</h2>
              <p>
                Session <span className="mono">{asText(snapshot?.app?.session_id || 'chat-ui-default')}</span>
              </p>
            </div>
            <div className="chat-head-stats">
              <span>Queue {fmtNumber(queueDepth)}</span>
              <span>Sync {syncMode === 'batch_sync' ? 'batch' : 'live'}</span>
              <span>Critical {fmtNumber(criticalAttention)} / {fmtNumber(criticalAttentionTotal)}</span>
              <span>Turns {fmtNumber(snapshot?.app?.turn_count || 0)}</span>
              <span>Alerts {fmtNumber(alertsCount)}</span>
              <span>Benchmark {benchmarkStatus}</span>
              <span>Receipt {shortText(snapshot?.receipt_hash || 'n/a', 16)}</span>
            </div>
          </header>

          <div className="chat-scroll">
            {error ? <div className="error-banner">{asText(error)}</div> : null}

            {recentTurns.length === 0 ? (
              <div className="chat-empty">No messages yet. Ask anything or type "new agent" to begin.</div>
            ) : (
              <div className="chat-list">
                {recentTurns.map((turn: Dict, idx: number) => {
                  const userText = asText(turn.user ?? turn.input ?? '');
                  const assistantText = asText(turn.assistant ?? turn.response ?? turn.output ?? '');
                  return (
                    <article key={`${asText(turn.turn_id || 'turn')}-${idx}`} className="chat-turn">
                      <div className="chat-turn-meta">
                        <span>{asText(turn.ts || 'n/a')}</span>
                        <StatusPill status={sending && idx === recentTurns.length - 1 ? 'thinking' : turn.status || 'complete'} />
                      </div>
                      <div className="chat-bubble user">
                        <div className="bubble-label">You</div>
                        <div>{userText || ' '}</div>
                      </div>
                      <div className="chat-bubble assistant">
                        <div className="bubble-label">Agent</div>
                        <div>{assistantText || ' '}</div>
                      </div>
                    </article>
                  );
                })}
              </div>
            )}
            {sending ? (
              <div className="typing-indicator">
                <span className="typing-dot" />
                <span className="typing-dot" />
                <span className="typing-dot" />
                <span>Agent is thinking...</span>
              </div>
            ) : null}
          </div>

          <section className="quick-actions-row">
            <button className="chip-btn" onClick={() => quickAction('new_agent')}>
              New Agent
            </button>
            <button className="chip-btn" onClick={() => quickAction('new_swarm')}>
              New Swarm
            </button>
            <button className="chip-btn" onClick={() => quickAction('assimilate')}>
              Assimilate Codex
            </button>
            <button className="chip-btn" onClick={() => quickAction('benchmark')}>
              Run Benchmark
            </button>
            <button className="chip-btn" onClick={() => quickAction('open_controls')}>
              Open Controls
            </button>
            <button className="chip-btn" onClick={() => quickAction('swarm')}>
              Swarm Tab
            </button>
            <button className="chip-btn" onClick={() => quickAction('runtime_swarm')}>
              Runtime Swarm
            </button>
          </section>

          <form
            className="chat-input-row"
            onSubmit={async (event) => {
              event.preventDefault();
              const text = chatInput.trim();
              if (!text) return;
              await sendChat(text);
              setChatInput('');
            }}
          >
            <input
              ref={chatInputRef}
              className="input"
              value={chatInput}
              onChange={(event) => setChatInput(event.target.value)}
              placeholder="Ask anything or type 'new agent' to begin..."
            />
            <button className="btn" type="submit">
              Send
            </button>
          </form>
        </section>
      </main>

      <div className={cls('drawer-backdrop', controlsOpen && 'open')} onClick={() => toggleControls(false)} />
      <aside className={cls('controls-drawer', controlsOpen && 'open')}>
        <header className="drawer-head">
          <div>
            <h2>Controls</h2>
            <p>Chat stays simple. Open only the panes you need.</p>
          </div>
          <button className="micro-btn" onClick={() => toggleControls(false)}>
            Close
          </button>
        </header>

        <div className="drawer-content">
          {CONTROL_PANES.map((pane) => (
            <DrawerAccordion key={pane.id} id={pane.id} label={pane.label} open={!!openPanes[pane.id]} onToggle={togglePane}>
              {pane.id === 'chat' ? (
                <div className="space-y-2">
                  <p className="text-xs text-slate-300">Quick send from controls.</p>
                  <form
                    className="chat-input-row"
                    onSubmit={async (event) => {
                      event.preventDefault();
                      const text = drawerChatInput.trim();
                      if (!text) return;
                      await sendChat(text);
                      setDrawerChatInput('');
                    }}
                  >
                    <input
                      className="input"
                      value={drawerChatInput}
                      onChange={(event) => setDrawerChatInput(event.target.value)}
                      placeholder="Send a message..."
                    />
                    <button className="micro-btn" type="submit">
                      Send
                    </button>
                  </form>
                </div>
              ) : null}

              {pane.id === 'swarm' ? (
                <div className="grid gap-2">
                  <article className="tile compact">
                    <div className="flex items-center justify-between gap-2">
                      <h3 className="font-semibold">chat-ui</h3>
                      <StatusPill status="active" />
                    </div>
                    <div className="text-xs text-slate-300 mt-1">
                      {asText(snapshot?.app?.settings?.provider || 'n/a')} / {asText(snapshot?.app?.settings?.model || 'n/a')}
                    </div>
                  </article>
                  {agents.map((row: Dict, idx: number) => (
                    <article key={`${asText(row.shadow || 'shadow')}-${idx}`} className="tile compact">
                      <div className="flex items-center justify-between gap-2">
                        <h3 className="font-semibold">{asText(row.shadow || 'shadow')}</h3>
                        <StatusPill status={row.status || 'unknown'} />
                      </div>
                      <div className="text-xs text-slate-300 mt-1">Role {asText(row.role || 'unknown')}</div>
                      <div className="mt-2 flex flex-wrap gap-1">
                        <button
                          className="micro-btn"
                          onClick={() => {
                            setChatInput(`@${asText(row.shadow || 'agent')} `);
                            chatInputRef.current?.focus();
                          }}
                        >
                          Chat
                        </button>
                        <button
                          className="micro-btn"
                          onClick={() =>
                            runAction('collab.launchRole', {
                              team,
                              role: asText(row.role || 'analyst'),
                              shadow: asText(row.shadow || `${team}-analyst`),
                            })
                          }
                        >
                          Respawn
                        </button>
                      </div>
                    </article>
                  ))}
                </div>
              ) : null}

              {pane.id === 'health' ? (
                <div className="space-y-2">
                  <article className="tile compact">
                    <div className="flex items-center justify-between gap-2">
                      <h3 className="font-semibold">Runtime Link</h3>
                      <StatusPill status={syncMode === 'batch_sync' ? 'warning' : 'live'} />
                    </div>
                    <div className="mt-1 text-xs text-slate-300">
                      Queue {fmtNumber(queueDepth)} · Backpressure {backpressureLevel}
                    </div>
                    <div className="mt-1 text-xs text-slate-300">
                      Conduit {fmtNumber(conduitSignals)} signals / {fmtNumber(conduitChannels)} channels
                    </div>
                    <div className="mt-1 text-xs text-slate-300">
                      Target channels {fmtNumber(conduitTargetSignals)}
                      {conduitScaleRequired ? ' · scale-up recommended' : ''}
                    </div>
                    <div className="mt-1 text-xs text-slate-300">
                      Critical attention {fmtNumber(criticalAttention)} visible / {fmtNumber(criticalAttentionTotal)} total
                    </div>
                  </article>
                  <article className="tile compact">
                    <div className="flex items-center justify-between gap-2">
                      <h3 className="font-semibold">Health Coverage</h3>
                      <StatusPill status={Number(healthCoverage.gap_count || 0) > 0 ? 'warning' : 'stable'} />
                    </div>
                    <div className="mt-1 text-xs text-slate-300">
                      Checks {fmtNumber(healthCoverage.count || 0)} (prev {fmtNumber(healthCoverage.previous_count || 0)})
                    </div>
                    <div className="mt-1 text-xs text-slate-300">
                      Gaps {fmtNumber(healthCoverage.gap_count || 0)}
                    </div>
                    {Array.isArray(healthCoverage.retired_checks) && healthCoverage.retired_checks.length > 0 ? (
                      <div className="mono mt-1 text-[11px] text-slate-300">
                        Retired: {shortText((healthCoverage.retired_checks as string[]).join(', '), 140)}
                      </div>
                    ) : null}
                  </article>
                  <article className="tile compact">
                    <div className="flex items-center justify-between gap-2">
                      <h3 className="font-semibold">Critical Queue</h3>
                      <StatusPill status={criticalEventsFull.length > 0 ? 'warning' : 'ok'} />
                    </div>
                    <div className="mt-1 text-xs text-slate-300">
                      Full critical queue: {fmtNumber(criticalEventsFull.length)}
                    </div>
                    <div className="mt-2 space-y-1 max-h-40 overflow-auto pr-1">
                      {criticalEventsFull.slice(0, 20).map((row: Dict, idx: number) => (
                        <div key={`critical-${idx}`} className="rounded-md border border-rose-900/45 bg-rose-950/30 px-2 py-1 text-[11px]">
                          <div className="mono text-rose-200">
                            {shortText(row.ts || 'n/a', 22)} · {shortText(row.severity || 'info', 12)} · {shortText(row.band || 'p4', 6)}
                          </div>
                          <div className="text-slate-100">{shortText(row.summary || '', 120)}</div>
                        </div>
                      ))}
                    </div>
                  </article>
                  <article className="tile compact">
                    <div className="flex items-center justify-between gap-2">
                      <h3 className="font-semibold">Swarm Recommendation</h3>
                      <StatusPill status={runtimeRecommendation.recommended ? 'warning' : 'ok'} />
                    </div>
                    <div className="mt-1 text-xs text-slate-300">
                      {runtimeRecommendation.recommended
                        ? 'Telemetry remediation loop recommended'
                        : 'No swarm telemetry intervention required'}
                    </div>
                    {runtimeRolePlan.length > 0 ? (
                      <div className="mono mt-1 text-[11px] text-slate-300">
                        Roles: {shortText(runtimeRolePlan.map((row: Dict) => asText(row.role || 'agent')).join(', '), 140)}
                      </div>
                    ) : null}
                    {runtimeRecommendation.throttle_required ? (
                      <div className="mono mt-1 text-[11px] text-slate-300">
                        Throttle: {shortText(runtimeRecommendation.throttle_command || 'collab-plane throttle', 140)}
                      </div>
                    ) : null}
                    {runtimeRecommendation.recommended ? (
                      <button className="micro-btn mt-2" onClick={() => runAction('dashboard.runtime.executeSwarmRecommendation', {})}>
                        Run Telemetry Remediation
                      </button>
                    ) : null}
                  </article>
                  <article className="tile compact">
                    <div className="flex items-center justify-between gap-2">
                      <h3 className="font-semibold">Benchmark Sanity</h3>
                      <StatusPill status={benchmarkStatus} />
                    </div>
                    <div className="mt-1 text-xs text-slate-300">
                      {benchmarkAgeSec >= 0 ? `Age ${fmtNumber(benchmarkAgeSec)}s` : 'Age n/a'}
                    </div>
                    <div className="mono mt-1 text-[11px] text-slate-300">{asText(benchmarkCheck.source || 'n/a')}</div>
                  </article>
                  {recentChecks.map(([name, row]: [string, any]) => (
                    <div key={name} className="rounded-lg border border-slate-700/60 bg-slate-900/50 p-2 text-xs">
                      <div className="flex items-center justify-between gap-2">
                        <div className="font-semibold text-slate-100">{name}</div>
                        <StatusPill status={row?.status || 'unknown'} />
                      </div>
                      <div className="mono mt-1 text-[11px] text-slate-300">{asText(row?.source || 'n/a')}</div>
                    </div>
                  ))}
                </div>
              ) : null}

              {pane.id === 'receipts' ? (
                <div className="space-y-2">
                  {recentReceipts.map((row: Dict, idx: number) => (
                    <div key={`${asText(row.path || 'receipt')}-${idx}`} className="rounded-md border border-slate-700/60 bg-slate-900/50 px-2 py-1 text-[11px]">
                      <div className="font-semibold text-slate-100">{asText(row.kind || 'artifact')}</div>
                      <div className="mono text-slate-300">{shortText(row.path || '', 80)}</div>
                    </div>
                  ))}
                </div>
              ) : null}

              {pane.id === 'logs' ? (
                <div className="space-y-2">
                  <article className="tile compact">
                    <div className="flex items-center justify-between gap-2">
                      <h3 className="font-semibold">Memory Stream</h3>
                      <StatusPill status={memoryStream.changed ? 'warning' : 'live'} />
                    </div>
                    <div className="mt-1 text-xs text-slate-300">
                      Seq {fmtNumber(memoryStream.seq || 0)} · Delta {fmtNumber(memoryStream.change_count || 0)}
                    </div>
                    <div className="mono mt-1 text-[11px] text-slate-300">
                      {shortText(
                        Array.isArray(memoryStream.latest_paths) ? memoryStream.latest_paths.join(', ') : 'no recent diffs',
                        120
                      )}
                    </div>
                    <div className="mt-1 text-xs text-slate-300">
                      Ingest {ingestControl.paused ? 'paused (non-critical)' : 'live'} · dropped {fmtNumber(ingestControl.dropped_count || 0)}
                    </div>
                  </article>
                  {recentLogs.map((row: Dict, idx: number) => (
                    <div key={`${asText(row.source || 'log')}-${idx}`} className="rounded-md border border-slate-700/60 bg-slate-900/50 px-2 py-1 text-[11px]">
                      <div className="mono text-slate-300">
                        {shortText(row.ts || 'n/a', 24)} · {shortText(row.source || '', 26)}
                      </div>
                      <div className="text-slate-100">{shortText(row.message || '', 96)}</div>
                    </div>
                  ))}
                </div>
              ) : null}

              {pane.id === 'settings' ? (
                <div className="space-y-2">
                  <div>
                    <label className="text-xs text-slate-300">Provider</label>
                    <input className="input" value={provider} onChange={(event) => setProvider(event.target.value)} />
                  </div>
                  <div>
                    <label className="text-xs text-slate-300">Model</label>
                    <input className="input" value={model} onChange={(event) => setModel(event.target.value)} />
                  </div>
                  <button className="btn" onClick={() => runAction('app.switchProvider', { provider, model })}>
                    Switch Provider
                  </button>
                  <div>
                    <label className="text-xs text-slate-300">Team</label>
                    <input className="input" value={team} onChange={(event) => setTeam(event.target.value)} />
                  </div>
                  <div>
                    <label className="text-xs text-slate-300">Role</label>
                    <input className="input" value={role} onChange={(event) => setRole(event.target.value)} />
                  </div>
                  <div>
                    <label className="text-xs text-slate-300">Shadow</label>
                    <input className="input" value={shadow} onChange={(event) => setShadow(event.target.value)} />
                  </div>
                  <button className="btn" onClick={() => runAction('collab.launchRole', { team, role, shadow })}>
                    Launch Role
                  </button>
                </div>
              ) : null}
            </DrawerAccordion>
          ))}
        </div>
      </aside>
    </div>
  );
}

const rootNode = document.getElementById('root');
if (!rootNode) throw new Error('dashboard_root_missing');
createRoot(rootNode).render(<App />);
