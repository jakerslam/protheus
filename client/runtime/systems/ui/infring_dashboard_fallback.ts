// Dependency-free fallback UI for environments where external module CDNs are blocked.

type Dict = Record<string, any>;

function esc(value: unknown): string {
  return String(value == null ? '' : value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}

function short(value: unknown, max = 96): string {
  const text = String(value == null ? '' : value).trim();
  if (!text) return 'n/a';
  if (text.length <= max) return text;
  return `${text.slice(0, max)}...`;
}

function rows(value: unknown): Dict[] {
  return Array.isArray(value) ? value : [];
}

async function fetchSnapshot(): Promise<Dict | null> {
  try {
    const res = await fetch('/api/dashboard/snapshot', { cache: 'no-store' });
    if (!res.ok) return null;
    return (await res.json()) as Dict;
  } catch {
    return null;
  }
}

function render(snapshot: Dict | null) {
  const root = document.getElementById('root');
  if (!root) return;
  if (!snapshot) {
    root.innerHTML = `
      <main style="max-width:980px;margin:32px auto;padding:16px;color:#e8f0ff;background:rgba(9,16,30,.72);border:1px solid rgba(122,163,255,.28);border-radius:14px">
        <h1 style="margin:0 0 8px 0">InfRing Dashboard</h1>
        <p style="margin:0 0 8px 0;color:#bfd3f5">Fallback mode active (React bundle unavailable).</p>
        <p style="margin:0;color:#bfd3f5">Snapshot endpoint not reachable yet. Retry in a few seconds.</p>
      </main>
    `;
    return;
  }

  const agents = rows(snapshot?.collab?.dashboard?.agents);
  const receipts = rows(snapshot?.receipts?.recent).slice(0, 12);
  const logs = rows(snapshot?.logs?.recent).slice(0, 12);
  const checks = Object.entries(snapshot?.health?.checks || {}).slice(0, 12);
  const turns = rows(snapshot?.app?.turns).slice(-12);

  root.innerHTML = `
    <main style="max-width:1200px;margin:20px auto;padding:16px;color:#e8f0ff;background:rgba(9,16,30,.72);border:1px solid rgba(122,163,255,.28);border-radius:14px">
      <header style="display:flex;justify-content:space-between;gap:12px;align-items:flex-start;flex-wrap:wrap">
        <div>
          <h1 style="margin:0 0 6px 0">InfRing Dashboard</h1>
          <p style="margin:0;color:#bfd3f5">Fallback mode active (React/ESM dependency blocked). This is still live authority data.</p>
        </div>
        <div style="font-family:ui-monospace,Menlo,monospace;font-size:12px;color:#a9c2ee">
          <div>Updated: ${esc(snapshot.ts || 'n/a')}</div>
          <div>Receipt: ${esc(short(snapshot.receipt_hash || 'n/a', 32))}</div>
        </div>
      </header>

      <section style="margin-top:14px;display:grid;grid-template-columns:repeat(auto-fit,minmax(160px,1fr));gap:10px">
        <article style="padding:10px;border:1px solid rgba(122,163,255,.22);border-radius:10px;background:rgba(20,32,58,.8)">
          <div style="font-size:12px;color:#b9ceef">Active Agents</div>
          <div style="font-size:22px;font-weight:700">${esc(agents.length)}</div>
        </article>
        <article style="padding:10px;border:1px solid rgba(122,163,255,.22);border-radius:10px;background:rgba(20,32,58,.8)">
          <div style="font-size:12px;color:#b9ceef">Open Alerts</div>
          <div style="font-size:22px;font-weight:700">${esc(snapshot?.health?.alerts?.count ?? 0)}</div>
        </article>
        <article style="padding:10px;border:1px solid rgba(122,163,255,.22);border-radius:10px;background:rgba(20,32,58,.8)">
          <div style="font-size:12px;color:#b9ceef">Provider</div>
          <div style="font-size:16px;font-weight:700">${esc(snapshot?.app?.settings?.provider || 'n/a')}</div>
        </article>
        <article style="padding:10px;border:1px solid rgba(122,163,255,.22);border-radius:10px;background:rgba(20,32,58,.8)">
          <div style="font-size:12px;color:#b9ceef">Model</div>
          <div style="font-size:16px;font-weight:700">${esc(snapshot?.app?.settings?.model || 'n/a')}</div>
        </article>
      </section>

      <section style="margin-top:14px;display:grid;grid-template-columns:repeat(auto-fit,minmax(320px,1fr));gap:12px">
        <article style="padding:10px;border:1px solid rgba(122,163,255,.22);border-radius:10px;background:rgba(20,32,58,.8)">
          <h2 style="margin:0 0 8px 0;font-size:14px">Health Checks</h2>
          <ul style="margin:0;padding-left:18px;font-size:12px;color:#d3e1fa">
            ${checks.map(([name, row]) => `<li><b>${esc(name)}</b> — ${esc((row as Dict)?.status || 'unknown')}</li>`).join('')}
          </ul>
        </article>
        <article style="padding:10px;border:1px solid rgba(122,163,255,.22);border-radius:10px;background:rgba(20,32,58,.8)">
          <h2 style="margin:0 0 8px 0;font-size:14px">Recent Receipts</h2>
          <ul style="margin:0;padding-left:18px;font-size:12px;color:#d3e1fa">
            ${receipts.map((row) => `<li>${esc(short(row.path || 'artifact', 72))}</li>`).join('')}
          </ul>
        </article>
      </section>

      <section style="margin-top:14px;padding:10px;border:1px solid rgba(122,163,255,.22);border-radius:10px;background:rgba(20,32,58,.8)">
        <h2 style="margin:0 0 8px 0;font-size:14px">Chat Interface (compatibility mode)</h2>
        <div style="font-size:12px;color:#b9ceef;margin-bottom:8px">Session: ${esc(snapshot?.app?.session_id || 'chat-ui-default')}</div>
        <div style="max-height:220px;overflow:auto;border:1px solid rgba(122,163,255,.2);border-radius:8px;padding:8px;background:rgba(5,10,20,.5)">
          ${
            turns.length === 0
              ? '<div style="font-size:12px;color:#b9ceef">No turns yet.</div>'
              : turns
                  .map(
                    (turn) => `
                      <article style="margin-bottom:8px;padding:6px;border:1px solid rgba(122,163,255,.16);border-radius:8px;background:rgba(10,16,28,.5)">
                        <div style="font-size:11px;color:#95b7e7">${esc(short(turn.ts || 'n/a', 32))} · ${esc(turn.provider || 'unknown')}/${esc(turn.model || 'n/a')}</div>
                        <div style="font-size:12px;color:#8fd0ff;margin-top:4px"><b>User:</b> ${esc(turn.user || '')}</div>
                        <div style="font-size:12px;color:#9ff2cf;margin-top:4px"><b>Assistant:</b> ${esc(turn.assistant || '')}</div>
                      </article>
                    `
                  )
                  .join('')
          }
        </div>
        <div style="display:flex;gap:8px;margin-top:8px">
          <input id="fallback-chat-input" type="text" placeholder="Message chat-ui..." style="flex:1;border:1px solid rgba(122,163,255,.4);border-radius:8px;background:rgba(5,10,20,.9);color:#e6efff;padding:8px;font-size:12px" />
          <button id="fallback-chat-send" type="button" style="border:1px solid rgba(77,226,197,.45);border-radius:8px;background:rgba(77,226,197,.14);color:#e8fff9;padding:8px 10px;font-size:12px;font-weight:700;cursor:pointer">Send</button>
        </div>
      </section>

      <section style="margin-top:14px;padding:10px;border:1px solid rgba(122,163,255,.22);border-radius:10px;background:rgba(20,32,58,.8)">
        <h2 style="margin:0 0 8px 0;font-size:14px">Recent Logs</h2>
        <ul style="margin:0;padding-left:18px;font-size:12px;color:#d3e1fa">
          ${logs.map((row) => `<li>${esc(short(row.ts || 'n/a', 24))} — ${esc(short(row.message || '', 100))}</li>`).join('')}
        </ul>
      </section>
    </main>
  `;

  const sendBtn = root.querySelector('#fallback-chat-send') as HTMLButtonElement | null;
  const inputEl = root.querySelector('#fallback-chat-input') as HTMLInputElement | null;
  if (sendBtn && inputEl) {
    sendBtn.onclick = async () => {
      const text = String(inputEl.value || '').trim();
      if (!text) return;
      sendBtn.disabled = true;
      try {
        await fetch('/api/dashboard/action', {
          method: 'POST',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify({ action: 'app.chat', payload: { input: text } }),
        });
        inputEl.value = '';
        const next = await fetchSnapshot();
        render(next);
      } catch {
        // keep fallback resilient; update loop will retry snapshot anyway
      } finally {
        sendBtn.disabled = false;
      }
    };
  }
}

function bootFallback() {
  const root = document.getElementById('root');
  if (!root) return;
  if (root.getAttribute('data-dashboard-hydrated') === 'react') return;
  if (root.childElementCount > 0) return;
  root.setAttribute('data-dashboard-hydrated', 'fallback');

  const update = async () => {
    if (root.getAttribute('data-dashboard-hydrated') !== 'fallback') return;
    const snapshot = await fetchSnapshot();
    render(snapshot);
  };

  void update();
  window.setInterval(update, 5000);
}

window.addEventListener('DOMContentLoaded', () => {
  window.setTimeout(bootFallback, 900);
});
