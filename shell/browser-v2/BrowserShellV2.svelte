<script>
  export let runtimeState = 'unknown';
  export let runtimeLabel = 'Waiting for Gateway projection.';
  export let selectedAgentId = '';
  export let selectedSessionId = '';
  export let agentRows = [];
  export let sessionRows = [];
  export let messages = [];
  export let eventRows = [];
  export let eventCursor = '';
  export let searchQuery = '';
  export let searchRows = [];
  export let activeDetailRef = '';
  export let activeDetailPreview = '';
  export let activeDetailPanel = null;
  export let issueNote = '';
  export let issueStatus = '';
  export let issueReceiptRef = '';
  export let approvalId = '';
  export let approvalDecision = 'approve';
  export let approvalStatus = '';
  export let approvalReceiptRef = '';
  export let modelSelection = '';
  export let modelRows = [];
  export let modelStatus = '';
  export let modelReceiptRef = '';
  export let gitTreeSelection = '';
  export let gitTreeRows = [];
  export let gitTreeStatus = '';
  export let gitTreeReceiptRef = '';
  export let receiptRefs = [];
  export let inputValue = '';
  export let disabled = false;

  export let onSubmitInput = () => {};
  export let onSelectAgent = () => {};
  export let onSelectSession = () => {};
  export let onOpenMessageDetail = () => {};
  export let onRefreshEvents = () => {};
  export let onSearch = () => {};
  export let onSubmitIssue = () => {};
  export let onSubmitApprovalDecision = () => {};
  export let onSetModel = () => {};
  export let onSetGitTree = () => {};

  $: selectedAgentLabel = selectedAgentId || 'No agent selected';
  $: selectedSessionLabel = selectedSessionId || 'No session selected';
  $: projectionCounts = [
    eventRows.length,
    searchRows.length,
    modelRows.length,
    gitTreeRows.length,
    receiptRefs.length,
  ].join(':');
  $: requestStatuses = [
    eventCursor,
    issueNote,
    issueStatus,
    issueReceiptRef,
    approvalId,
    approvalDecision,
    approvalStatus,
    approvalReceiptRef,
    modelSelection,
    modelStatus,
    modelReceiptRef,
    gitTreeSelection,
    gitTreeStatus,
    gitTreeReceiptRef,
  ].filter(Boolean).join('|');
  $: actionCount = [
    onRefreshEvents,
    onSearch,
    onSubmitIssue,
    onSubmitApprovalDecision,
    onSetModel,
    onSetGitTree,
  ].length;

  function submit() {
    const value = String(inputValue || '').trim();
    if (!value || disabled) return;
    onSubmitInput(value);
  }
</script>

<div
  class="app-layout"
  data-shell-plug="browser-v2"
  data-projection-counts={projectionCounts}
  data-request-statuses={requestStatuses}
  data-action-count={actionCount}
  aria-label="Browser Shell V2"
>
  <div class="main-pointer-fx-layer" aria-hidden="true"></div>

  <infring-sidebar-rail-shell class="sidebar drag-bar overlay-shared-surface chat-sidebar-dynamic" dragbarsurface="chat-sidebar" parentownedmechanics="true" aria-label="Agent conversations">
    <div class="sidebar-nav-shell">
      <div class="sidebar-nav" role="navigation" aria-label="Main navigation">
        <div class="sidebar-top-ghost" aria-hidden="true"></div>
        <div class="nav-section" aria-label="Agent conversations">
          <button class="nav-item sidebar-tab-item active" type="button" aria-current="page">
            <span class="nav-icon" aria-hidden="true">∞</span>
            <span class="nav-label">Conversations</span>
          </button>
          <div class="nav-sub-search-row">
            <div class="nav-sub-search-wrap">
              <span class="nav-sub-search-icon" aria-hidden="true">⌕</span>
              <input class="nav-sub-search-input" type="text" value={searchQuery} placeholder="Search conversations..." aria-label="Search conversations" readonly>
            </div>
          </div>
          <div class="nav-sub-item-controls">
            <div class="nav-sub-sort-group nav-sub-sort-pill toggle-pill" role="group" aria-label="Sort conversations">
              <button type="button" class="nav-sub-sort-btn active" aria-label="Sort by recent activity">◷</button>
              <button type="button" class="nav-sub-sort-btn" aria-label="Sort by topology">≡</button>
            </div>
          </div>
          <infring-sidebar-agent-list-shell>
          <div class="chat-sidebar-list" aria-label="Agent selector">
            {#each agentRows as agent (agent.id)}
              <button
                type="button"
                class:active={agent.id === selectedAgentId}
                class="chat-sidebar-item"
                disabled={disabled}
                on:click={() => onSelectAgent(agent.id)}
              >
                <span class="chat-sidebar-item-avatar agent-mark infring-logo">{(agent.label || agent.id || 'A').slice(0, 1)}</span>
                <span class="chat-sidebar-item-main">
                  <span class="chat-sidebar-item-name">{agent.label || agent.id}</span>
                  <span class="chat-sidebar-item-preview">{agent.state || 'Gateway projection'}</span>
                </span>
              </button>
            {/each}
          </div>
          </infring-sidebar-agent-list-shell>
          <div class="chat-sidebar-list" aria-label="Session selector">
            {#each sessionRows as session (session.id)}
              <button
                type="button"
                class:active={session.id === selectedSessionId}
                class="chat-sidebar-item"
                disabled={disabled}
                on:click={() => onSelectSession(session.id)}
              >
                <span class="chat-sidebar-item-avatar agent-mark infring-logo">S</span>
                <span class="chat-sidebar-item-main">
                  <span class="chat-sidebar-item-name">{session.label || session.id}</span>
                  <span class="chat-sidebar-item-preview">{session.message_count ? `${session.message_count} messages` : 'Window projection'}</span>
                </span>
              </button>
            {/each}
          </div>
          <button type="button" class="nav-item sidebar-tab-item" aria-current="false">
            <span class="nav-icon">
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>
            </span>
            <span class="nav-label">Agents</span>
          </button>
        </div>
        <div class="nav-section sidebar-tab-section" aria-label="Automation">
          <button type="button" class="nav-item sidebar-tab-item" aria-current="false">
            <span class="nav-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg></span>
            <span class="nav-label">Automation</span>
          </button>
        </div>
        <div class="nav-section sidebar-tab-section" aria-label="Apps">
          <button type="button" class="nav-item sidebar-tab-item" aria-current="false">
            <span class="nav-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="4" y="4" width="6" height="6" rx="1.5"></rect><rect x="14" y="4" width="6" height="6" rx="1.5"></rect><rect x="4" y="14" width="6" height="6" rx="1.5"></rect><rect x="14" y="14" width="6" height="6" rx="1.5"></rect></svg>
            </span>
            <span class="nav-label">Apps</span>
          </button>
        </div>
        <div class="nav-section sidebar-tab-section" aria-label="System">
          <button type="button" class="nav-item sidebar-tab-item" aria-current="false">
            <span class="nav-icon"><svg viewBox="0 0 24 24"><rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4"/></svg></span>
            <span class="nav-label">System</span>
          </button>
        </div>
      </div>
    </div>
    <button
      class="overlay-pulltab-object sidebar-pulltab drag-bar drag-bar-pulltab overlay-shared-surface pulltab-border-top-active pulltab-border-right-active pulltab-border-bottom-active pulltab-border-left-inactive"
      data-dragbar-pulltab="chat-sidebar"
      type="button"
      aria-label="Toggle sidebar"
    >
      <span class="overlay-pulltab-object-joint sidebar-pulltab-joint sidebar-pulltab-joint-top-left" aria-hidden="true"></span>
      <span class="overlay-pulltab-object-joint sidebar-pulltab-joint sidebar-pulltab-joint-top-right" aria-hidden="true"></span>
      <span class="overlay-pulltab-object-joint sidebar-pulltab-joint sidebar-pulltab-joint-bottom-left" aria-hidden="true"></span>
      <span class="overlay-pulltab-object-joint sidebar-pulltab-joint sidebar-pulltab-joint-bottom-right" aria-hidden="true"></span>
      <svg class="overlay-pulltab-object-icon sidebar-pulltab-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="m15 6-6 6 6 6"></path>
      </svg>
    </button>
  </infring-sidebar-rail-shell>
  <div class="sidebar-overlay"></div>

  <main class="main-content" aria-label="Dashboard main surface">
    <div class="global-taskbar is-docked-top" data-shell-primitive="taskbar-dock">
      <div class="global-taskbar-left">
        <div class="taskbar-visual-group taskbar-visual-group-left" aria-label="Primary taskbar items">
          <div class="taskbar-hero-menu-anchor">
            <button class="taskbar-brand taskbar-brand-trigger" type="button" title="System actions">
              <span class="brand-mark infring-logo" aria-hidden="true">∞</span>
              <span class="taskbar-brand-title">INFRING</span>
            </button>
          </div>
          <div class="taskbar-reorder-box taskbar-reorder-box-left">
            <div class="taskbar-reorder-item taskbar-reorder-nav-cluster taskbar-nav-pill">
              <button class="btn btn-ghost btn-sm taskbar-icon-btn taskbar-nav-btn taskbar-back-btn" type="button" aria-label="Back">‹</button>
              <button class="btn btn-ghost btn-sm taskbar-icon-btn taskbar-nav-btn taskbar-forward-btn" type="button" aria-label="Forward">›</button>
            </div>
          </div>
          <div class="taskbar-text-menus">
            <button class="taskbar-text-menu-btn" type="button" aria-label="Help menu">Help</button>
          </div>
          <div class="global-taskbar-page-slot"></div>
        </div>
      </div>
      <div class="global-taskbar-right">
        <infring-taskbar-system-items-shell shellprimitive="taskbar-dock" wrapperrole="taskbar-system-items" parentownedmechanics="true">
        <div class="taskbar-visual-group taskbar-visual-group-right" aria-label="System taskbar items">
          <div class="taskbar-reorder-box taskbar-reorder-box-right">
            <div class="taskbar-reorder-item" data-taskbar-item="connectivity">
              <div class="global-taskbar-controls">
                <button class={runtimeState === 'connected' ? 'health-indicator taskbar-agent-indicator health-ok' : 'health-indicator taskbar-agent-indicator health-connecting'} type="button" aria-label="Open agents" title={runtimeLabel}>
                  <span class="taskbar-agent-indicator-icon" aria-hidden="true"><svg viewBox="0 0 24 24"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path><circle cx="9" cy="7" r="4"></circle><path d="M23 21v-2a4 4 0 0 0-3-3.87"></path><path d="M16 3.13a4 4 0 0 1 0 7.75"></path></svg></span>
                  <span class="taskbar-agent-indicator-text">{runtimeState}</span>
                </button>
              </div>
            </div>
            <div class="taskbar-reorder-item" data-taskbar-item="theme">
              <div class="theme-switcher toggle-pill" data-mode="system" data-resolved="light" role="group" aria-label="Theme">
                <button class="theme-opt" type="button" title="Light" aria-label="Light theme"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="4"></circle><path d="M12 2v2"></path><path d="M12 20v2"></path><path d="m4.93 4.93 1.41 1.41"></path><path d="m17.66 17.66 1.41 1.41"></path><path d="M2 12h2"></path><path d="M20 12h2"></path><path d="m6.34 17.66-1.41 1.41"></path><path d="m19.07 4.93-1.41 1.41"></path></svg></button>
                <button class="theme-opt active" type="button" title="System" aria-label="System theme"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="2" y="3" width="20" height="14" rx="2"></rect><path d="M8 21h8"></path><path d="M12 17v4"></path></svg></button>
                <button class="theme-opt" type="button" title="Dark" aria-label="Dark theme"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 12.79A9 9 0 1 1 11.21 3c0 0 0 0 0 0A7 7 0 0 0 21 12.79z"></path></svg></button>
              </div>
            </div>
            <div class="taskbar-reorder-item" data-taskbar-item="notifications">
              <div id="taskbar-notification-menu-anchor" class="notif-wrap">
                <button class="btn btn-ghost btn-sm taskbar-icon-btn notif-btn" type="button" title="Notifications" aria-label="Notifications">
                  <svg class="notif-bell-icon" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><path d="M15 17h5l-1.4-1.4A2 2 0 0 1 18 14.2V11a6 6 0 1 0-12 0v3.2a2 2 0 0 1-.6 1.4L4 17h5"></path><path d="M9 17a3 3 0 0 0 6 0"></path></svg>
                </button>
              </div>
            </div>
            <div class="taskbar-reorder-item" data-taskbar-item="search">
              <button class="btn btn-ghost btn-sm taskbar-icon-btn taskbar-search-btn" type="button" aria-label="Search" aria-disabled="true"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.05" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="11" cy="11" r="6"></circle><path d="m20 20-3.7-3.7"></path></svg></button>
            </div>
            <div class="taskbar-reorder-item" data-taskbar-item="auth">
              <button class="btn btn-ghost btn-sm taskbar-icon-btn auth-key-btn" type="button" aria-label="Authentication"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="5" y="11" width="14" height="10" rx="2"></rect><path d="M8 11V8a4 4 0 0 1 8 0v3"></path><circle cx="12" cy="16" r="1"></circle></svg></button>
            </div>
            <div class="taskbar-reorder-item" data-taskbar-item="clock">
              <span class="taskbar-clock" aria-label="Clock">--:--</span>
            </div>
          </div>
        </div>
        </infring-taskbar-system-items-shell>
      </div>
    </div>

    <div class="chat-wrapper">
      <infring-chat-header-shell>
      <div class="chat-thread-topline">
        <div class="chat-thread-profile-center">
          <div class="chat-thread-profile warped-glass chat-thread-profile-disabled" role="button" tabindex="-1" title="Agent details">
            <div class="chat-thread-profile-avatar">
              <span class="infring-logo infring-logo--agent-default" aria-hidden="true"><span class="infring-logo-glyph" aria-hidden="true">∞</span></span>
            </div>
            <div class="chat-thread-profile-info-pill">
              <div class="chat-thread-profile-meta">
                <span class={runtimeState === 'connected' ? 'agent-status-dot chat-title-status-dot status-connected' : 'agent-status-dot chat-title-status-dot'} aria-hidden="true"></span>
                <div class="chat-thread-profile-name">{selectedAgentLabel}</div>
              </div>
              <div class="chat-thread-heart-meter" title={selectedSessionLabel}>
                <span class="chat-thread-heart" aria-hidden="true">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M12 21s-7-4.2-9-8.4C1.5 9.5 3.3 6 6.4 6c2.2 0 3.4 1.2 3.9 2.1.5-.9 1.7-2.1 3.9-2.1 3.1 0 4.9 3.5 3.4 6.6-2 4.2-9 8.4-9 8.4z"></path>
                  </svg>
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
      </infring-chat-header-shell>

      <infring-messages-surface-shell>
      <section class="messages" aria-label="Message window">
        <div class="chat-reflection-overlay" aria-hidden="true"></div>
        <div class="chat-grid-overlay" aria-hidden="true"></div>
        {#each messages as message (message.id)}
          <article class:user={message.role === 'user'} class:agent={message.role !== 'user'} class="message meta-collapsed">
            <div class="message-avatar agent-mark infring-logo" aria-hidden="true">{message.role === 'user' ? 'Y' : '∞'}</div>
            <div class="message-body">
              <div class="message-bubble markdown-body">
                <span class="message-agent-name">
                  <span class="message-agent-name-label">{message.role === 'user' ? 'You' : selectedAgentLabel}</span>
                </span>
                <p class="message-bubble-content">{message.text}</p>
                {#if message.detail_ref}
                  <button class="message-stat-btn" type="button" disabled={disabled} on:click={() => onOpenMessageDetail(message.detail_ref)}>
                    View detail
                  </button>
                {/if}
                {#if message.status}
                  <div class="message-stats-row"><span class="message-stat-meta">{message.status}</span></div>
                {/if}
              </div>
            </div>
          </article>
        {:else}
          <article class="empty-state">
            No bounded message projection loaded yet.
          </article>
        {/each}
      </section>
      </infring-messages-surface-shell>

      <div class="chat-map" aria-label="Message map">
        <div class="chat-map-surface drag-bar overlay-shared-surface">
          <div class="chat-map-rail">
            <button class="chat-map-jump chat-map-jump-up" type="button" aria-label="Previous message">⌃</button>
            <div class="chat-map-items-wrap">
              <div class="chat-map-viewport">
                <div class="chat-map-scroll">
                  <div class="chat-map-spacer" aria-hidden="true"></div>
                  {#each messages as message (message.id)}
                    <div class="chat-map-entry">
                      <button class:role-user={message.role === 'user'} class:role-agent={message.role !== 'user'} class="chat-map-item" type="button">
                        <span class="chat-map-item-main"><span class="chat-map-bar"></span></span>
                      </button>
                    </div>
                  {/each}
                  <div class="chat-map-spacer" aria-hidden="true"></div>
                </div>
              </div>
            </div>
            <button class="chat-map-jump chat-map-jump-down" type="button" aria-label="Next message">⌄</button>
          </div>
        </div>
      </div>

      <infring-chat-input-footer-shell>
      <form class="input-area" on:submit|preventDefault={submit}>
        <div class="chat-input-lane">
          <div class="composer-stack">
          <div class="input-row">
            <div class="composer-shell">
              <div class="composer-main-row">
                <div class="composer-display-pill" aria-label="Message input controls">
                  <div class="composer-menu-pill composer-shared-input-pill">
                    <div class="composer-plus-wrap composer-icon-left">
                      <button class="composer-icon-btn composer-hamburger-btn" type="button" aria-label="Add files and more">
                        <svg class="composer-hamburger-icon" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><line x1="4" y1="7" x2="20" y2="7"/><line x1="4" y1="12" x2="20" y2="12"/><line x1="4" y1="17" x2="20" y2="17"/></svg>
                      </button>
                    </div>
                  </div>
                  <div class="composer-input-pill composer-shared-input-pill">
                    <textarea bind:value={inputValue} disabled={disabled} id="msg-input" rows="1" placeholder="Message Infring..." aria-label="Shell input"></textarea>
                  </div>
                  <div class="composer-controls-pill composer-shared-input-pill">
                    <div class="composer-actions-right">
                      <div class="toggle-pill toggle-pill--triple input-toggle-wrapper" data-mode="text" role="group" aria-label="Voice and send controls">
                        <button type="button" class="composer-send-voice-opt composer-send-voice-opt-attach" aria-label="Add files">
                          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21.44 11.05-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48"/></svg>
                        </button>
                        <button type="button" class="composer-send-voice-opt composer-send-voice-opt-voice" aria-label="Toggle voice recording">
                          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><line x1="12" x2="12" y1="19" y2="22"/></svg>
                        </button>
                        <button class="composer-send-voice-opt composer-send-voice-opt-send" disabled={disabled || !inputValue.trim()} type="submit" aria-label="Send message">
                          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="19" x2="12" y2="5"></line><polyline points="5 12 12 5 19 12"></polyline></svg>
                        </button>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
          </div>
        </div>
      </form>
      </infring-chat-input-footer-shell>
    </div>

    {#if activeDetailRef}
      <section class="popup-window dashboard-popup-surface" aria-label="Lazy message detail">
        <div class="popup-window-header">
          <h3 class="popup-window-title">{activeDetailPanel?.title || activeDetailRef}</h3>
        </div>
        <div class="popup-window-body">
          <p>{activeDetailPanel?.summary || activeDetailPreview || 'Detail projection loaded.'}</p>
          {#if activeDetailPanel?.rows?.length}
            <div>
              {#each activeDetailPanel.rows as row (row.id)}
                <p>
                  <strong>{row.label}</strong>
                  {#if row.meta}<span> {row.meta}</span>{/if}
                </p>
              {/each}
            </div>
          {/if}
        </div>
      </section>
    {/if}
  </main>

  <svg
    class="dock-icon-defs"
    aria-hidden="true"
    focusable="false"
    width="0"
    height="0"
    style="position:absolute;width:0;height:0;overflow:hidden;pointer-events:none"
  >
    <defs>
      <linearGradient id="dock-home-icon-stroke-grad" gradientUnits="userSpaceOnUse" x1="12" y1="24" x2="12" y2="0"><stop offset="0%" stop-color="#a8bbd6"></stop><stop offset="100%" stop-color="#f6f9fe"></stop></linearGradient>
      <linearGradient id="dock-agents-icon-stroke-grad" gradientUnits="userSpaceOnUse" x1="12" y1="24" x2="12" y2="0"><stop offset="0%" stop-color="#90a9cf"></stop><stop offset="100%" stop-color="#f4f9ff"></stop></linearGradient>
      <linearGradient id="dock-cog-top-stroke-grad" gradientUnits="userSpaceOnUse" x1="12" y1="12.88" x2="12" y2="2.32"><stop offset="0%" stop-color="#7f8ea5"></stop><stop offset="100%" stop-color="#f0f4f9"></stop></linearGradient>
      <linearGradient id="dock-cog-bl-stroke-grad" gradientUnits="userSpaceOnUse" x1="8.4" y1="20.84" x2="8.4" y2="11.96"><stop offset="0%" stop-color="#7f8ea5"></stop><stop offset="100%" stop-color="#f0f4f9"></stop></linearGradient>
      <linearGradient id="dock-cog-br-stroke-grad" gradientUnits="userSpaceOnUse" x1="15.6" y1="19.64" x2="15.6" y2="13.16"><stop offset="0%" stop-color="#7f8ea5"></stop><stop offset="100%" stop-color="#f0f4f9"></stop></linearGradient>
      <linearGradient id="dock-system-icon-stroke-grad" gradientUnits="userSpaceOnUse" x1="12" y1="24" x2="12" y2="0"><stop offset="0%" stop-color="#91a8cf"></stop><stop offset="100%" stop-color="#f7fbff"></stop></linearGradient>
      <linearGradient id="dock-settings-icon-stroke-grad" gradientUnits="userSpaceOnUse" x1="12" y1="22" x2="12" y2="2"><stop offset="0%" stop-color="#7f8ea5"></stop><stop offset="100%" stop-color="#f0f4f9"></stop></linearGradient>
      <linearGradient id="dock-apps-cell-overlay-grad" x1="1" y1="1" x2="0" y2="0"><stop offset="0%" stop-color="#000000" stop-opacity="0.16"></stop><stop offset="100%" stop-color="#ffffff" stop-opacity="0.24"></stop></linearGradient>
      <symbol id="dock-icon-gear" viewBox="0 0 24 24"><circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06A1.65 1.65 0 0 0 15 19.4a1.65 1.65 0 0 0-1 .6 1.65 1.65 0 0 0-.33 1V21a2 2 0 1 1-4 0v-.09a1.65 1.65 0 0 0-.33-1 1.65 1.65 0 0 0-1-.6 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.6 15a1.65 1.65 0 0 0-.6-1 1.65 1.65 0 0 0-1-.33H3a2 2 0 1 1 0-4h.09a1.65 1.65 0 0 0 1-.33 1.65 1.65 0 0 0 .6-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06A2 2 0 1 1 7.13 3.6l.06.06A1.65 1.65 0 0 0 9 4.6a1.65 1.65 0 0 0 1-.6 1.65 1.65 0 0 0 .33-1V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 .33 1 1.65 1.65 0 0 0 1 .6 1.65 1.65 0 0 0 1.82-.33l.06-.06A2 2 0 1 1 20.4 7.13l-.06.06A1.65 1.65 0 0 0 19.4 9c0 .38.13.74.36 1.03.23.29.57.5.94.57H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1 .4c-.29.24-.5.57-.57.94z"></path></symbol>
      <symbol id="dock-icon-settings" viewBox="0 0 24 24"><use href="#dock-icon-gear"></use></symbol>
    </defs>
  </svg>
  <infring-bottom-dock-shell shellprimitive="taskbar-dock" parentownedmechanics="true"></infring-bottom-dock-shell>
</div>
