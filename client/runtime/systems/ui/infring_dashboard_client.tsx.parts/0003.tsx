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

