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
  export let issueNote = '';
  export let issueStatus = '';
  export let issueReceiptRef = '';
  export let approvalId = '';
  export let approvalDecision = 'approve';
  export let approvalStatus = '';
  export let approvalReceiptRef = '';
  export let modelSelection = '';
  export let modelStatus = '';
  export let modelReceiptRef = '';
  export let gitTreeSelection = '';
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

<main class="browser-shell-v2" aria-label="Browser Shell V2">
  <section class="browser-shell-v2__topbar" aria-label="Runtime status">
    <div>
      <p class="browser-shell-v2__eyebrow">Infring Shell V2</p>
      <h1>Gateway Projection</h1>
    </div>
    <div class="browser-shell-v2__status" data-state={runtimeState}>
      <span>{runtimeState}</span>
      <small>{runtimeLabel}</small>
    </div>
  </section>

  <section class="browser-shell-v2__workspace" aria-label="Selected session">
    <aside class="browser-shell-v2__rail">
      <p class="browser-shell-v2__label">Agent</p>
      <strong>{selectedAgentId || 'none selected'}</strong>
      <div class="browser-shell-v2__selector-list" aria-label="Agent selector">
        {#each agentRows as agent (agent.id)}
          <button
            type="button"
            class:active={agent.id === selectedAgentId}
            disabled={disabled}
            on:click={() => onSelectAgent(agent.id)}
          >
            <span>{agent.label || agent.id}</span>
            {#if agent.state}<small>{agent.state}</small>{/if}
          </button>
        {/each}
      </div>
      <p class="browser-shell-v2__label">Session</p>
      <strong>{selectedSessionId || 'none selected'}</strong>
      <div class="browser-shell-v2__selector-list" aria-label="Session selector">
        {#each sessionRows as session (session.id)}
          <button
            type="button"
            class:active={session.id === selectedSessionId}
            disabled={disabled}
            on:click={() => onSelectSession(session.id)}
          >
            <span>{session.label || session.id}</span>
            {#if session.message_count}<small>{session.message_count} msgs</small>{/if}
          </button>
        {/each}
      </div>
    </aside>

    <section class="browser-shell-v2__messages" aria-label="Message window">
      {#each messages as message (message.id)}
        <article class:browser-shell-v2__message--user={message.role === 'user'} class="browser-shell-v2__message">
          <header>
            <span>{message.role}</span>
            {#if message.status}<small>{message.status}</small>{/if}
          </header>
          <p>{message.text}</p>
          {#if message.detail_ref}
            <button class="browser-shell-v2__detail-button" type="button" disabled={disabled} on:click={() => onOpenMessageDetail(message.detail_ref)}>
              View detail
            </button>
          {/if}
        </article>
      {:else}
        <article class="browser-shell-v2__empty">
          No bounded message projection loaded yet.
        </article>
      {/each}
    </section>
  </section>

  {#if activeDetailRef}
    <section class="browser-shell-v2__detail" aria-label="Lazy message detail">
      <p class="browser-shell-v2__label">Lazy Detail</p>
      <strong>{activeDetailRef}</strong>
      <p>{activeDetailPreview || 'Detail projection loaded.'}</p>
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

  <form class="browser-shell-v2__input" on:submit|preventDefault={submit}>
    <input bind:value={inputValue} disabled={disabled} placeholder="Send through Shell Socket..." aria-label="Shell input" />
    <button disabled={disabled || !inputValue.trim()} type="submit">Send</button>
  </form>
</main>
