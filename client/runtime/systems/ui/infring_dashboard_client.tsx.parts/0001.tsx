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
