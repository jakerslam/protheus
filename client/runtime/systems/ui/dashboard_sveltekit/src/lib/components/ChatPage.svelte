<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { onDestroy, onMount } from 'svelte';
  import ChatComposer from '$lib/components/ChatComposer.svelte';
  import ChatDrawer from '$lib/components/ChatDrawer.svelte';
  import ChatSidebar from '$lib/components/ChatSidebar.svelte';
  import ChatStatusHeader from '$lib/components/ChatStatusHeader.svelte';
  import ChatTranscript from '$lib/components/ChatTranscript.svelte';
  import type {
    DashboardAgentRow,
    DashboardAgentStreamController,
    DashboardChatMessage,
    DashboardChatToolRow,
    DashboardModelRow,
    DashboardUploadedFile,
  } from '$lib/chat';
  import {
    compactAgentSession,
    connectAgentStream,
    createDraftAgent,
    readAgentSession,
    readModels,
    readSidebarAgents,
    resetAgentSession,
    sendAgentMessage,
    stopAgent,
    updateAgentConfig,
    updateAgentModel,
    uploadAgentFile,
  } from '$lib/chat';

  let agents: DashboardAgentRow[] = [];
  let activeAgentId = '';
  let messages: DashboardChatMessage[] = [];
  let models: DashboardModelRow[] = [];
  let composer = '';
  let pendingFiles: File[] = [];
  let error = '';
  let notice = '';
  let loadingAgents = true;
  let loadingSession = false;
  let loadingModels = false;
  let creatingAgent = false;
  let sending = false;
  let drawerOpen = false;
  let drawerBusy = '';
  let drawerName = '';
  let drawerModel = '';
  let streamState = 'disconnected';
  let streamController: DashboardAgentStreamController | null = null;

  $: activeAgent = agents.find((row) => row.id === activeAgentId) || null;
  $: activeModel = activeAgent ? String(activeAgent.runtime_model || activeAgent.model_name || 'Server default') : 'Select a conversation';

  onMount(async () => {
    await Promise.all([refreshAgents({ syncQuery: false }), loadModels()]);
  });

  onDestroy(() => {
    streamController?.disconnect();
    streamController = null;
  });

  function resetDrawerDrafts(): void {
    drawerName = String(activeAgent?.name || '').trim();
    drawerModel = String(activeAgent?.runtime_model || activeAgent?.model_name || '').trim();
  }

  function setNotice(text: string): void { notice = String(text || '').trim(); error = ''; }

  function setErrorMessage(text: string): void { error = String(text || '').trim(); notice = ''; }

  function preferredAgentId(rows: DashboardAgentRow[], requestedId: string): string {
    const cleanRequested = String(requestedId || '').trim();
    return cleanRequested && rows.some((row) => row.id === cleanRequested) ? cleanRequested : (rows[0]?.id || '');
  }

  async function syncAgentQuery(agentId: string): Promise<void> {
    const url = new URL($page.url);
    if (agentId) url.searchParams.set('agent', agentId);
    else url.searchParams.delete('agent');
    await goto(`${url.pathname}${url.search}`, {
      replaceState: true,
      noScroll: true,
      keepFocus: true,
      invalidateAll: false,
    });
  }

  function appendSystemMessage(text: string, meta = 'System'): void {
    messages = [...messages, { id: `system-${Date.now()}`, role: 'system', text, meta, ts: Date.now(), tools: [] }];
  }

  function ensureStreamingAssistant(): DashboardChatMessage {
    const existing = messages[messages.length - 1];
    if (existing && existing.role === 'agent' && existing.pending) return existing;
    const row: DashboardChatMessage = { id: `stream-${Date.now()}`, role: 'agent', text: '', meta: streamState === 'connected' ? 'Streaming…' : 'Waiting for response…', ts: Date.now(), tools: [], pending: true };
    messages = [...messages, row];
    return row;
  }

  function updatePendingAssistant(mutator: (row: DashboardChatMessage) => void): void {
    const row = ensureStreamingAssistant();
    const next = { ...row, tools: row.tools.map((tool) => ({ ...tool })) };
    mutator(next);
    messages = [...messages.slice(0, -1), next];
  }

  function upsertPendingTool(name: string, mutator: (tool: DashboardChatToolRow) => void): void {
    updatePendingAssistant((row) => {
      const toolName = String(name || 'tool').trim() || 'tool';
      const existingIndex = row.tools.findIndex((tool) => tool.name === toolName);
      const current = existingIndex >= 0
        ? { ...row.tools[existingIndex] }
        : {
            id: `tool-${toolName}-${row.tools.length + 1}`,
            name: toolName,
            input: '',
            result: '',
            status: 'running',
            isError: false,
            blocked: false,
          };
      mutator(current);
      if (existingIndex >= 0) row.tools = row.tools.map((tool, index) => (index === existingIndex ? current : tool));
      else row.tools = [...row.tools, current];
    });
  }

  function bindStream(agentId: string): void {
    streamController?.disconnect();
    streamController = null;
    streamState = agentId ? 'connecting' : 'disconnected';
    if (!agentId) return;
    streamController = connectAgentStream(agentId, {
      onOpen: () => {
        if (agentId !== activeAgentId) return;
        streamState = 'connected';
      },
      onReconnect: () => {
        if (agentId !== activeAgentId) return;
        streamState = 'reconnecting';
        updatePendingAssistant((row) => {
          row.meta = 'Reconnecting…';
        });
      },
      onClose: () => {
        if (agentId !== activeAgentId) return;
        streamState = 'disconnected';
      },
      onError: () => {
        if (agentId !== activeAgentId) return;
        streamState = 'reconnecting';
      },
      onMessage: async (event) => {
        if (agentId !== activeAgentId) return;
        const type = String(event.type || '').trim().toLowerCase();
        if (type === 'text_delta') {
          updatePendingAssistant((row) => {
            row.text = `${String(row.text || '')}${String(event.content || '')}`;
            row.meta = 'Streaming…';
          });
          return;
        }
        if (type === 'tool_start') {
          upsertPendingTool(String(event.tool || 'tool'), (tool) => {
            tool.input = String(event.input || tool.input || '');
            tool.status = 'running';
          });
          return;
        }
        if (type === 'tool_result') {
          upsertPendingTool(String(event.tool || 'tool'), (tool) => {
            tool.input = String(event.input || tool.input || '');
            tool.result = String(event.result || '');
            tool.status = String(event.is_error ? 'error' : 'done');
            tool.isError = Boolean(event.is_error);
          });
          return;
        }
        if (type === 'tool_end') {
          upsertPendingTool(String(event.tool || 'tool'), (tool) => {
            tool.status = 'done';
          });
          return;
        }
        if (type === 'response') {
          sending = false;
          await loadSession(agentId);
          return;
        }
        if (type === 'error' || type === 'response_error' || type === 'terminal_error') {
          sending = false;
          setErrorMessage(String(event.message || 'stream_error'));
          appendSystemMessage(`Stream error: ${String(event.message || 'unknown failure')}`, 'Runtime');
        }
      },
    });
  }

  async function loadModels(): Promise<void> {
    loadingModels = true;
    try {
      models = await readModels();
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'models_unavailable'));
    } finally {
      loadingModels = false;
    }
  }

  async function loadSession(agentId: string): Promise<void> {
    if (!agentId) {
      messages = [];
      return;
    }
    loadingSession = true;
    try {
      const session = await readAgentSession(agentId);
      if (agentId !== activeAgentId) return;
      messages = session.messages;
    } catch (cause) {
      if (agentId === activeAgentId) {
        setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'session_unavailable'));
      }
    } finally {
      if (agentId === activeAgentId) loadingSession = false;
    }
  }

  async function activateAgent(agentId: string, options: { syncQuery?: boolean; forceReload?: boolean } = {}): Promise<void> {
    if (!agentId) return;
    const changed = agentId !== activeAgentId;
    activeAgentId = agentId;
    resetDrawerDrafts();
    if (changed) bindStream(agentId);
    if (options.syncQuery !== false) await syncAgentQuery(agentId);
    if (changed || options.forceReload) await loadSession(agentId);
  }

  async function refreshAgents(options: { syncQuery?: boolean; preferredId?: string } = {}): Promise<void> {
    loadingAgents = true;
    try {
      const rows = await readSidebarAgents();
      agents = rows;
      const requestedId = String(options.preferredId || $page.url.searchParams.get('agent') || activeAgentId || '').trim();
      const nextId = preferredAgentId(rows, requestedId);
      if (!nextId) {
        activeAgentId = '';
        messages = [];
        streamController?.disconnect();
        streamController = null;
        streamState = 'disconnected';
        if (options.syncQuery !== false) await syncAgentQuery('');
        return;
      }
      await activateAgent(nextId, { syncQuery: options.syncQuery !== false && nextId !== requestedId, forceReload: true });
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'agent_roster_unavailable'));
    } finally {
      loadingAgents = false;
    }
  }

  async function handleCreateAgent(): Promise<void> {
    creatingAgent = true;
    try {
      const created = await createDraftAgent();
      setNotice('Draft chat created.');
      await refreshAgents({ preferredId: created.id });
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'spawn_failed'));
    } finally {
      creatingAgent = false;
    }
  }

  async function uploadPendingFiles(agentId: string, files: File[]): Promise<{ uploaded: DashboardUploadedFile[]; refs: string[] }> {
    const uploaded: DashboardUploadedFile[] = [];
    const refs: string[] = [];
    for (const file of files) {
      try {
        const result = await uploadAgentFile(agentId, file);
        uploaded.push(result);
        refs.push(`[File: ${result.filename}]`);
      } catch (cause) {
        refs.push(`[File: ${file.name} (upload failed)]`);
        appendSystemMessage(
          `Failed to upload ${file.name}: ${cause instanceof Error ? cause.message : String(cause || 'upload_failed')}`,
          'Upload'
        );
      }
    }
    return { uploaded, refs };
  }

  async function handleSend(): Promise<void> {
    const raw = String(composer || '');
    const files = pendingFiles.slice();
    const text = raw.trim();
    if ((!text && files.length === 0) || !activeAgentId || sending) return;
    composer = '';
    pendingFiles = [];
    const uploadSummary = files.length ? await uploadPendingFiles(activeAgentId, files) : { uploaded: [], refs: [] };
    const finalText = uploadSummary.refs.length
      ? `${text ? `${text}\n` : ''}${uploadSummary.refs.join('\n')}`
      : raw;
    if (!String(finalText || '').trim()) return;
    messages = [
      ...messages,
      {
        id: `pending-user-${Date.now()}`,
        role: 'user',
        text: finalText,
        meta: uploadSummary.uploaded.length ? `${uploadSummary.uploaded.length} attachment(s)` : '',
        ts: Date.now(),
        tools: [],
      },
    ];
    sending = true;
    notice = '';
    error = '';
    if (streamController && streamController.isConnected() && streamController.sendMessage(finalText, uploadSummary.uploaded)) {
      ensureStreamingAssistant();
      return;
    }
    try {
      await sendAgentMessage(activeAgentId, finalText, uploadSummary.uploaded);
      await loadSession(activeAgentId);
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'send_failed'));
      appendSystemMessage(`Message failed: ${error || 'send_failed'}`, 'Runtime');
    } finally {
      sending = false;
    }
  }

  async function handleSaveName(): Promise<void> {
    if (!activeAgentId || !drawerName.trim() || drawerBusy) return;
    drawerBusy = 'name';
    try {
      await updateAgentConfig(activeAgentId, { name: drawerName.trim() });
      setNotice('Conversation name updated.');
      await refreshAgents({ preferredId: activeAgentId, syncQuery: false });
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'config_update_failed'));
    } finally {
      drawerBusy = '';
    }
  }

  async function handleSaveModel(): Promise<void> {
    if (!activeAgentId || !drawerModel.trim() || drawerBusy) return;
    drawerBusy = 'model';
    try {
      await updateAgentModel(activeAgentId, drawerModel.trim());
      setNotice('Model switched. Session memory was reset by the backend contract.');
      await refreshAgents({ preferredId: activeAgentId, syncQuery: false });
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'model_update_failed'));
    } finally {
      drawerBusy = '';
    }
  }

  async function handleCompact(): Promise<void> {
    if (!activeAgentId || drawerBusy) return;
    drawerBusy = 'compact';
    try {
      const summary = await compactAgentSession(activeAgentId);
      await loadSession(activeAgentId);
      setNotice(summary);
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'compact_failed'));
    } finally {
      drawerBusy = '';
    }
  }

  async function handleReset(): Promise<void> {
    if (!activeAgentId || drawerBusy) return;
    drawerBusy = 'reset';
    try {
      const summary = await resetAgentSession(activeAgentId);
      await loadSession(activeAgentId);
      setNotice(summary);
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'reset_failed'));
    } finally {
      drawerBusy = '';
    }
  }

  async function handleStop(): Promise<void> {
    if (!activeAgentId || drawerBusy) return;
    drawerBusy = 'stop';
    try {
      const summary = await stopAgent(activeAgentId);
      setNotice(summary);
      await refreshAgents({ preferredId: activeAgentId, syncQuery: false });
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'stop_failed'));
    } finally {
      drawerBusy = '';
    }
  }
</script>

<section class:drawer-open={drawerOpen} class="chat-page">
  <ChatSidebar {agents} {activeAgentId} loading={loadingAgents} creating={creatingAgent} on:refresh={() => void refreshAgents()} on:create={() => void handleCreateAgent()} on:select={(event) => void activateAgent(event.detail.id, { syncQuery: true, forceReload: event.detail.id !== activeAgentId || messages.length === 0 })} />

  <div class="chat-workbench">
    <ChatStatusHeader {activeAgent} {activeModel} {streamState} {drawerOpen} {error} {notice} on:toggledrawer={() => { drawerOpen = !drawerOpen; if (drawerOpen && models.length === 0) void loadModels(); }} />

    <ChatTranscript {activeAgentId} loading={loadingSession} {messages} />
    <ChatComposer bind:value={composer} bind:files={pendingFiles} disabled={!activeAgentId || sending} {sending} on:submit={() => void handleSend()} />
  </div>

  <ChatDrawer
    open={drawerOpen}
    agent={activeAgent}
    models={models}
    loadingModels={loadingModels}
    connectionState={streamState}
    busyKey={drawerBusy}
    bind:nameDraft={drawerName}
    bind:modelDraft={drawerModel}
    on:close={() => { drawerOpen = false; }}
    on:refreshmodels={() => void loadModels()}
    on:savename={() => void handleSaveName()}
    on:savemodel={() => void handleSaveModel()}
    on:compact={() => void handleCompact()}
    on:reset={() => void handleReset()}
    on:stop={() => void handleStop()}
  />
</section>

<style>
  .chat-page {
    display: grid;
    grid-template-columns: 300px minmax(0, 1fr);
    gap: 18px;
    min-height: calc(100vh - 170px);
  }

  .chat-page.drawer-open {
    grid-template-columns: 300px minmax(0, 1fr) 320px;
  }

  .chat-workbench {
    min-width: 0;
    grid-template-rows: auto auto minmax(0, 1fr) auto;
    display: grid;
    gap: 12px;
  }

  @media (max-width: 1320px) {
    .chat-page,
    .chat-page.drawer-open {
      grid-template-columns: 300px minmax(0, 1fr);
    }
  }

  @media (max-width: 1080px) {
    .chat-page,
    .chat-page.drawer-open {
      grid-template-columns: 1fr;
    }
  }
</style>
