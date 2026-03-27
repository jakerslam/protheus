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
