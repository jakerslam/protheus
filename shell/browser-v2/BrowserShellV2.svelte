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

  function submit() {
    const value = String(inputValue || '').trim();
    if (!value || disabled) return;
    onSubmitInput(value);
  }

  function submitSearch(event) {
    event.preventDefault();
    onSearch(String(searchQuery || '').trim());
  }

  function submitIssue(event) {
    event.preventDefault();
    onSubmitIssue(String(issueNote || '').trim());
  }

  function submitApprovalDecision(event) {
    event.preventDefault();
    onSubmitApprovalDecision(String(approvalId || '').trim(), String(approvalDecision || 'approve').trim());
  }

  function submitModel(event) {
    event.preventDefault();
    onSetModel(String(modelSelection || '').trim());
  }

  function submitGitTree(event) {
    event.preventDefault();
    onSetGitTree(String(gitTreeSelection || '').trim());
  }
</script>

<div class="app-layout browser-shell-v2 browser-shell-v2--legacy-surface" aria-label="Browser Shell V2">
  <aside class="sidebar drag-bar overlay-shared-surface chat-sidebar-dynamic" aria-label="Agent conversations">
    <div class="sidebar-nav-shell">
      <div class="sidebar-nav" role="navigation" aria-label="Main navigation">
        <div class="nav-section">
          <button class="nav-item sidebar-tab-item active" type="button" aria-current="page">
            <span class="nav-icon" aria-hidden="true">∞</span>
            <span class="nav-label">Conversations</span>
          </button>
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
                  {#if agent.state}<span class="chat-sidebar-item-preview">{agent.state}</span>{/if}
                </span>
              </button>
            {/each}
          </div>
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
                  {#if session.message_count}<span class="chat-sidebar-item-preview">{session.message_count} messages</span>{/if}
                </span>
              </button>
            {/each}
          </div>
        </div>
      </div>
    </div>
  </aside>

  <main class="main-content" aria-label="Dashboard main surface">
    <div class="global-taskbar is-docked-top" data-shell-primitive="taskbar-dock">
      <div class="global-taskbar-left">
        <div class="taskbar-visual-group taskbar-visual-group-left">
          <button class="taskbar-brand taskbar-brand-trigger" type="button">
            <span class="brand-mark infring-logo" aria-hidden="true">∞</span>
            <span class="taskbar-brand-title">INFRING</span>
          </button>
          <div class="taskbar-reorder-item taskbar-reorder-nav-cluster taskbar-nav-pill">
            <button class="btn btn-ghost btn-sm taskbar-icon-btn taskbar-nav-btn" type="button" aria-label="Back">‹</button>
            <button class="btn btn-ghost btn-sm taskbar-icon-btn taskbar-nav-btn" type="button" aria-label="Forward">›</button>
          </div>
          <button class="taskbar-text-menu-btn" type="button">Help</button>
        </div>
      </div>
      <div class="global-taskbar-right">
        <div class="taskbar-visual-group taskbar-visual-group-right">
          <span class="taskbar-agent-indicator"><span class="taskbar-agent-indicator-text">{runtimeState}</span></span>
          <span class="conn-badge">{runtimeLabel}</span>
        </div>
      </div>
    </div>

    <div class="chat-wrapper">
      <div class="chat-thread-topline">
        <button class="chat-thread-profile chat-thread-profile-disabled" type="button">
          <span class="chat-thread-profile-avatar agent-mark infring-logo">∞</span>
          <span class="chat-thread-profile-copy">
            <span class="chat-thread-profile-name">{selectedAgentId || 'No agent selected'}</span>
            <span class="chat-thread-profile-subtitle">{selectedSessionId || 'No session selected'}</span>
          </span>
        </button>
      </div>

      <section class="messages" aria-label="Message window">
        <div class="chat-reflection-overlay" aria-hidden="true"></div>
        <div class="chat-grid-overlay" aria-hidden="true"></div>
        {#each messages as message (message.id)}
          <article class:user={message.role === 'user'} class:agent={message.role !== 'user'} class="message meta-collapsed">
            <div class="message-avatar agent-mark infring-logo" aria-hidden="true">{message.role === 'user' ? 'Y' : '∞'}</div>
            <div class="message-body">
              <div class="message-bubble markdown-body">
                <span class="message-agent-name">
                  <span class="message-agent-name-label">{message.role === 'user' ? 'You' : selectedAgentId}</span>
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

      <div class="chat-map" aria-label="Message map">
        <div class="chat-map-surface drag-bar overlay-shared-surface">
          <div class="chat-map-rail">
            <button class="chat-map-jump chat-map-jump-up" type="button" aria-label="Previous message">⌃</button>
            <div class="chat-map-items-wrap">
              <div class="chat-map-viewport">
                <div class="chat-map-scroll">
                  {#each messages as message (message.id)}
                    <div class="chat-map-entry">
                      <button class:role-user={message.role === 'user'} class:role-agent={message.role !== 'user'} class="chat-map-item" type="button">
                        <span class="chat-map-item-main"><span class="chat-map-bar"></span></span>
                      </button>
                    </div>
                  {/each}
                </div>
              </div>
            </div>
            <button class="chat-map-jump chat-map-jump-down" type="button" aria-label="Next message">⌄</button>
          </div>
        </div>
      </div>
    </div>

  {#if activeDetailRef}
    <section class="popup-window dashboard-popup-surface browser-shell-v2__detail" aria-label="Lazy message detail">
      <div class="browser-shell-v2__detail-header">
        <div>
          <p class="browser-shell-v2__label">Lazy Detail</p>
          <strong>{activeDetailPanel?.title || activeDetailRef}</strong>
        </div>
        {#if activeDetailPanel?.kind}<small>{activeDetailPanel.kind}</small>{/if}
      </div>
      <p>{activeDetailPanel?.summary || activeDetailPreview || 'Detail projection loaded.'}</p>
      {#if activeDetailPanel?.rows?.length}
        <div class="browser-shell-v2__detail-grid" aria-label="Detail projection rows">
          {#each activeDetailPanel.rows as row (row.id)}
            <article>
              <span>{row.label}</span>
              {#if row.meta}<small>{row.meta}</small>{/if}
            </article>
          {/each}
        </div>
      {/if}
      {#if activeDetailPanel?.refs?.length || activeDetailPanel?.cursor || activeDetailPanel?.receipt_ref}
        <div class="browser-shell-v2__detail-refs" aria-label="Detail refs">
          {#each activeDetailPanel.refs || [] as ref (ref)}
            <code>{ref}</code>
          {/each}
          {#if activeDetailPanel?.cursor}<code>{activeDetailPanel.cursor}</code>{/if}
          {#if activeDetailPanel?.receipt_ref}<code>{activeDetailPanel.receipt_ref}</code>{/if}
        </div>
      {/if}
    </section>
  {/if}

  <section class="browser-shell-v2__events" aria-label="Gateway event projection">
    <div class="browser-shell-v2__events-header">
      <div>
        <p class="browser-shell-v2__label">Event Projection</p>
        <small>{eventCursor || 'no cursor'}</small>
      </div>
      <button type="button" disabled={disabled || !selectedSessionId} on:click={() => onRefreshEvents()}>
        Refresh
      </button>
    </div>
    <div class="browser-shell-v2__event-list">
      {#each eventRows as event (event.id)}
        <article>
          <span>{event.label}</span>
          {#if event.status}<small>{event.status}</small>{/if}
        </article>
      {:else}
        <article>
          <span>No event projection loaded yet.</span>
        </article>
      {/each}
    </div>
  </section>

  <section class="browser-shell-v2__search" aria-label="Bounded Gateway search">
    <form class="browser-shell-v2__search-form" on:submit={submitSearch}>
      <label>
        <span class="browser-shell-v2__label">Bounded Search</span>
        <input bind:value={searchQuery} disabled={disabled} placeholder="Search via Gateway..." aria-label="Search query" />
      </label>
      <button type="submit" disabled={disabled}>Search</button>
    </form>
    <div class="browser-shell-v2__search-results">
      {#each searchRows as result (result.id)}
        <article>
          <strong>{result.label}</strong>
          {#if result.snippet}<p>{result.snippet}</p>{/if}
          {#if result.detail_ref}
            <button type="button" disabled={disabled} on:click={() => onOpenMessageDetail(result.detail_ref)}>
              View detail
            </button>
          {/if}
        </article>
      {:else}
        <article>
          <strong>No search projection loaded.</strong>
        </article>
      {/each}
    </div>
  </section>

  <section class="browser-shell-v2__issue" aria-label="Gateway issue evaluation request">
    <form class="browser-shell-v2__issue-form" on:submit={submitIssue}>
      <label>
        <span class="browser-shell-v2__label">Issue / Eval Request</span>
        <input bind:value={issueNote} disabled={disabled} placeholder="Ask Gateway to inspect this context..." aria-label="Issue note" />
      </label>
      <button type="submit" disabled={disabled || !selectedSessionId}>Submit</button>
    </form>
    {#if issueStatus || issueReceiptRef}
      <p class="browser-shell-v2__issue-status">
        <strong>{issueStatus || 'submitted'}</strong>
        {#if issueReceiptRef}<span>{issueReceiptRef}</span>{/if}
      </p>
    {/if}
  </section>

  <section class="browser-shell-v2__approval" aria-label="Gateway approval decision request">
    <form class="browser-shell-v2__approval-form" on:submit={submitApprovalDecision}>
      <label>
        <span class="browser-shell-v2__label">Approval Decision</span>
        <input bind:value={approvalId} disabled={disabled} placeholder="approval ref..." aria-label="Approval ID" />
      </label>
      <label>
        <span class="browser-shell-v2__label">Decision</span>
        <select bind:value={approvalDecision} disabled={disabled} aria-label="Approval decision">
          <option value="approve">approve</option>
          <option value="deny">deny</option>
          <option value="defer">defer</option>
        </select>
      </label>
      <button type="submit" disabled={disabled || !approvalId.trim()}>Submit</button>
    </form>
    {#if approvalStatus || approvalReceiptRef}
      <p class="browser-shell-v2__approval-status">
        <strong>{approvalStatus || 'submitted'}</strong>
        {#if approvalReceiptRef}<span>{approvalReceiptRef}</span>{/if}
      </p>
    {/if}
  </section>

  <section class="browser-shell-v2__controls" aria-label="Gateway selection requests">
    <form class="browser-shell-v2__control-form" on:submit={submitModel}>
      <label>
        <span class="browser-shell-v2__label">Model Request</span>
        <input bind:value={modelSelection} disabled={disabled} placeholder="auto, gpt-5.4, ..." aria-label="Model selection" />
      </label>
      <button type="submit" disabled={disabled || !selectedAgentId}>Submit</button>
    </form>
    <div class="browser-shell-v2__control-menu" aria-label="Model selector">
      {#each modelRows as model (model.id)}
        <button
          type="button"
          class:active={model.id === modelSelection}
          disabled={disabled || !selectedAgentId}
          on:click={() => onSetModel(model.id)}
        >
          <span>{model.label || model.id}</span>
          {#if model.meta}<small>{model.meta}</small>{/if}
        </button>
      {:else}
        <span class="browser-shell-v2__control-empty">No model projection loaded.</span>
      {/each}
    </div>
    {#if modelStatus || modelReceiptRef}
      <p class="browser-shell-v2__control-status">
        <strong>{modelStatus || 'submitted'}</strong>
        {#if modelReceiptRef}<span>{modelReceiptRef}</span>{/if}
      </p>
    {/if}
    <form class="browser-shell-v2__control-form" on:submit={submitGitTree}>
      <label>
        <span class="browser-shell-v2__label">Git Tree Request</span>
        <input bind:value={gitTreeSelection} disabled={disabled} placeholder="workspace, branch, tree ref..." aria-label="Git tree selection" />
      </label>
      <button type="submit" disabled={disabled || !selectedAgentId}>Submit</button>
    </form>
    <div class="browser-shell-v2__control-menu" aria-label="Git tree selector">
      {#each gitTreeRows as tree (tree.id)}
        <button
          type="button"
          class:active={tree.id === gitTreeSelection}
          disabled={disabled || !selectedAgentId}
          on:click={() => onSetGitTree(tree.id)}
        >
          <span>{tree.label || tree.id}</span>
          {#if tree.meta}<small>{tree.meta}</small>{/if}
        </button>
      {:else}
        <span class="browser-shell-v2__control-empty">No git tree projection loaded.</span>
      {/each}
    </div>
    {#if gitTreeStatus || gitTreeReceiptRef}
      <p class="browser-shell-v2__control-status">
        <strong>{gitTreeStatus || 'submitted'}</strong>
        {#if gitTreeReceiptRef}<span>{gitTreeReceiptRef}</span>{/if}
      </p>
    {/if}
  </section>

  <section class="browser-shell-v2__receipts" aria-label="Gateway audit receipts">
    <div>
      <p class="browser-shell-v2__label">Gateway Receipts</p>
      <small>Bounded proof that this plug is only calling Shell Socket routes.</small>
    </div>
    <div class="browser-shell-v2__receipt-list">
      {#each receiptRefs as receiptRef (receiptRef)}
        <code>{receiptRef}</code>
      {:else}
        <code>No receipt projection loaded yet.</code>
      {/each}
    </div>
  </section>

    <form class="input-area browser-shell-v2__input" on:submit|preventDefault={submit}>
      <div class="chat-input-lane">
        <div class="composer-display-pill">
          <div class="composer-shell">
            <div class="composer-main-row">
              <button class="composer-menu-pill composer-shared-input-pill" type="button" aria-label="Menu">☰</button>
              <div class="composer-input-pill composer-shared-input-pill">
                <input bind:value={inputValue} disabled={disabled} placeholder="Message Infring..." aria-label="Shell input" />
              </div>
              <div class="composer-controls-pill">
                <button class="btn btn-primary btn-send" disabled={disabled || !inputValue.trim()} type="submit">Send</button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </form>
  </main>
</div>
