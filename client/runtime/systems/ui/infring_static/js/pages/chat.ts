// Canonical Shell source-of-truth: assembled runtime chat surface.
// Decomposition debt lives under ./chat.ts.parts/** and must not count as additive production source.
// Infring Chat Page — Agent chat with markdown + streaming
'use strict';

function chatPage() {
  var msgId = 0;
  return {
    ...infringChatCoreInitialState(),
    ...infringChatCatalogInitialState(),

    // ── Tip Bar ──
    tipIndex: 0,
    tips: ['Type / for commands', '/think on for reasoning', 'Ctrl+Shift+F for focus mode', 'Ctrl+T or Ctrl+\\ for terminal mode', 'Ctrl+F to add files', '/model to switch models', '/context to check usage', '/continuity to see pending work'],
    tipTimer: null,
    ...infringChatEarlyDelegateMethods(),
    get currentTip() {
      return chatCurrentTip(this);
    },

    // Backward compat helper
    get thinkingEnabled() { return chatThinkingEnabled(this); },

    get terminalPromptPath() {
      return chatTerminalPromptPath(this);
    },

    get terminalPromptPrefix() {
      return chatTerminalPromptPrefix(this);
    },

    get terminalPromptChars() {
      return chatTerminalPromptChars(this);
    },

    get terminalCursorIndex() {
      return chatTerminalCursorIndex(this);
    },

    get terminalCursorRow() {
      return chatTerminalCursorRow(this);
    },

    get terminalCursorColumn() {
      return chatTerminalCursorColumn(this);
    },

    get terminalCursorStyle() {
      return chatTerminalCursorStyle(this);
    },


    get contextUsagePercent() {
      return chatContextUsagePercent(this);
    },

    get contextRingArcLength() {
      return chatContextRingArcLength(this);
    },

    get contextRingProgressStyle() {
      return chatContextRingProgressStyle(this);
    },

    get contextRingTooltip() {
      return chatContextRingTooltip(this);
    },

    get contextRingCompactLabel() {
      return chatContextRingCompactLabel(this);
    },

    get activeGitBranchLabel() {
      return chatActiveGitBranchLabel(this);
    },

    get activeGitBranchMenuLabel() {
      return chatActiveGitBranchMenuLabel(this);
    },


    get freshInitCanLaunch() {
      return chatFreshInitCanLaunch(this);
    },

    ...infringChatComposerStateMethods(),

    ...infringChatInputHistoryDelegateMethods(),

    ...infringChatFreshInitPermissionMethods(),

    ...infringChatFreshInitModelMethods(),

    get modelDisplayName() {
      return chatModelDisplayName(this);
    },

    get menuModelLabel() {
      return chatMenuModelLabel(this);
    },

    get switcherViewState() {
      return chatModelSwitcherViewState(this);
    },

    get switcherProviders() {
      return chatModelSwitcherProviders(this);
    },
    get filteredSwitcherModels() {
      return chatFilteredSwitcherModels(this);
    },
    get renderedSwitcherModels() {
      return chatRenderedSwitcherModels(this);
    },
    get modelSwitcherTruncatedCount() {
      return chatModelSwitcherTruncatedCount(this);
    },
    ...infringChatModelCatalogDelegateMethods(),

    get groupedSwitcherModels() {
      return chatGroupedSwitcherModels(this);
    },
    ...infringChatModelVisualMethods(),

    ...infringChatAgentResolutionMethods(),

    ...infringChatConversationCacheDelegateMethods(),

    ...infringChatModelCatalogForwarderMethods(),

    ...infringChatModelGuidanceMethods(),

    ...infringChatConversationCachePersistenceMethods(),

    ...infringChatSessionNoticeDelegateMethods(),

    ...infringChatPasteDelegateMethods(),

    ...infringChatContextTelemetryMethods(),

    ...infringChatAutoModelMethods(),

    ...infringChatPromptSuggestionMethods(),

    ...infringChatPromptQueueMethods(),

    get promptQueueItems() {
      return chatPromptQueueItems(this);
    },

    get hasPromptQueue() {
      return chatHasPromptQueue(this);
    },

    ...infringChatFreshInitFlowMethods(),

    ...infringChatPointerFxMethods(),

    ...infringChatAgentTrailMethods(),

    ...infringChatModelUsageNoticeMethods(),

    ...infringChatMessageNormalizationMethods(),

    ...infringChatLifecycleInitMethods(),

    ...infringChatTerminalComposeMethods(),

    ...infringChatAgentTrailAnchorMethods(),

    get filteredModelPicker() {
      return chatFilteredModelPicker(this);
    },
    ...infringChatModelSwitchMethods(),

    ...infringChatModelFailoverMethods(),

    ...infringChatTypewriterMethods(),

    ...infringChatPendingResponseMethods(),

    ...infringChatSlashCommandMethods(),

    ...infringChatMemprobeMethods(),

    ...infringChatAgentSelectionMethods(),

    ...infringChatMessageVirtualizationMethods(),

    ...infringChatSlashApiKeyMethods(),
    ...infringChatFreshInitSelectionMethods(),

    // Fresh-init launch lives in infringChatFreshInitFlowMethods().

    ...infringChatSessionScopeMethods(),

    ...infringChatSessionLoadMethods(),

    ...infringChatSessionActionMethods(),

    ...infringChatWebSocketConnectionMethods(),

    ...infringChatWebSocketLifecycleEventMethods(),

    ...infringChatWebSocketPhaseEventMethods(),

    ...infringChatWebSocketTextDeltaEventMethods(),

    ...infringChatWebSocketToolEventMethods(),

    ...infringChatWebSocketTerminalEventMethods(),

    ...infringChatWebSocketMiscEventMethods(),

    ...infringChatWebSocketErrorEventMethods(),

    ...infringChatWebSocketResponseEventMethods(),

    ...infringChatAgentLiveStatusMethods(),

    ...infringChatAgentLifecycleMethods(),

    // Backward-compat websocket event entrypoint.
    handleWsMessage(data) {
      var eventAgentId = String(data && data.agent_id ? data.agent_id : '').trim();
      var activeWsAgentId = String(this._wsAgent || '').trim();
      if (eventAgentId && activeWsAgentId && eventAgentId !== activeWsAgentId) {
        return;
      }
      switch (data.type) {
        case 'connected':
          this.handleWsConnectedEvent(data, activeWsAgentId);
          break;

        case 'context_state':
          this.handleWsContextStateEvent(data);
          break;

        // Legacy thinking event (backward compat)
        case 'thinking':
          this.handleWsThinkingEvent(data);
          break;

        // New typing lifecycle
        case 'typing':
          this.handleWsTypingEvent(data);
          break;

        case 'phase':
          this.handleWsPhaseEvent(data);
          break;

        case 'text_delta':
          this.handleWsTextDeltaEvent(data);
          break;
        case 'tool_start':
          this.handleWsToolStartEvent(data);
          break;
        case 'tool_end':
          this.handleWsToolEndEvent(data);
          break;
        case 'tool_result':
          this.handleWsToolResultEvent(data);
          break;
        case 'response':
          this.handleWsResponseEvent(data);
          break;
        case 'silent_complete':
          this.handleWsSilentCompleteEvent(data);
          break;
        case 'error':
          this.handleWsErrorEvent(data);
          break;

        case 'agent_archived':
          this.handleWsAgentArchivedEvent(data);
          break;

        case 'agents_updated':
          this.handleWsAgentsUpdatedEvent(data);
          break;

        case 'command_result':
          this.handleWsCommandResultEvent(data);
          break;

        case 'terminal_output':
          this.handleWsTerminalOutputEvent(data);
          break;

        case 'terminal_error':
          this.handleWsTerminalErrorEvent(data);
          break;
        case 'canvas':
          this.handleWsCanvasEvent(data);
          break;
        case 'pong': break;
      }
      if (data && data.type !== 'connected' && data.type !== 'context_state' && data.type !== 'pong') this.syncActiveChatMessages();
      this.scheduleConversationPersist();
    },

    ...infringChatToolSummaryMethods(),

    ...infringChatMessageMetaMethods(),

    ...infringChatSideResultMethods(),

    ...infringChatNoticeMessageMethods(),

    ...infringChatMessageSourceRunMethods(),

    ...infringChatMessagePreviewMapMethods(),

    ...infringChatAgentMessageDedupeMethods(),

    ...infringChatNoticeActionMethods(),

    ...infringChatActiveMessageStoreMethods(),

    ...infringChatMapInteractionMethods(),

    ...infringChatDrawerIdentityMethods(),

    ...infringChatDrawerLifecycleMethods(),

    ...infringChatDrawerPermissionMethods(),

    ...infringChatDrawerSettingsMethods(),

    ...infringChatToolLabelMethods(),

    ...infringChatToolCardMethods(),

    ...infringChatComposerMotionMethods(),

    ...infringChatMessageAppendMethods(),

    ...infringChatQueueProcessingMethods(),

    ...infringChatSlashTelemetryMethods(),

    ...infringChatSlashAliasMethods(),

    ...infringChatProactiveTelemetryMethods(),

    get filteredSlashCommands() {
      return chatFilteredSlashCommands(this);
    },

    ...infringChatResponseToolPayloadMethods(),

    ...infringChatTerminalSessionMethods(),

    ...infringChatSendMessageMethods(),

    ...infringChatSendPayloadMethods(),

    ...infringChatScrollMethodHelpers(),
    ...infringChatAttachmentMethods(),
    ...infringChatMessageGroupingMethods(),
    ...infringChatAssistantTextSignalMethods(),
    ...infringChatSourceTraceMethods(),
    ...infringChatThinkingDisplayMethods(),
    ...infringChatMessageWorkspaceMetaMethods(),
    ...infringChatMessageMetaActionMethods(),
    ...infringChatArtifactTextMethods(),
    resolveArtifactDirectives: async function(directives) {
      if (!this.currentAgent || !this.currentAgent.id) return;
      var rows = Array.isArray(directives) ? directives : [];
      if (!rows.length) return;
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i] || {};
        var targetPath = String(row.path || '').trim();
        if (!targetPath) continue;
        try {
          if (row.kind === 'file') {
            this.inputText = 'Use the workspace_read tool route to read this file and return a structured receipt: ' + targetPath;
            await this.sendMessage();
          } else if (row.kind === 'folder') {
            this.inputText = 'Use the workspace_export tool route to inspect/export this folder and return a structured receipt: ' + targetPath;
            await this.sendMessage();
          }
        } catch (_) {}
      }
      this.scrollToBottom();
      this.scheduleConversationPersist();
    },

    // Remove disclosure/speaker prefixes injected by model/backend responses.
    // Examples:
    //   "[openai/gpt-5] hello" -> "hello"
    //   "Agent: hello" -> "hello"
    //   "**Assistant:** hello" -> "hello"
    ...infringChatResponseTextFormatMethods(),

    ...infringChatVoiceRecordingMethods(),

    // Voice: handle completed recording — upload and transcribe
    _handleRecordingComplete: async function() {
      var voiceAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
      if (!this._audioChunks.length || !voiceAgent || !voiceAgent.id) return;
      var blob = new Blob(this._audioChunks, { type: this._audioChunks[0].type || 'audio/webm' });
      this._audioChunks = [];
      if (blob.size < 100) return; // too small

      this.addNoticeEvent({
        notice_label: 'Transcribing audio...',
        notice_type: 'info',
        ts: Date.now()
      });
      this.scrollToBottom();

      try {
        // Upload audio file
        var ext = blob.type.includes('webm') ? 'webm' : blob.type.includes('ogg') ? 'ogg' : 'mp3';
        var file = new File([blob], 'voice_' + Date.now() + '.' + ext, { type: blob.type });
        var upload = await InfringAPI.upload(voiceAgent.id, file);

        // Remove the "Transcribing..." message
        this.clearSystemThinkingRows();

        // Use server-side transcription if available, otherwise fall back to placeholder
        var text = (upload.transcription && upload.transcription.trim())
          ? upload.transcription.trim()
          : '[Voice message - audio: ' + upload.filename + ']';
        this._sendPayload(text, [upload], [], { agent_id: voiceAgent.id });
      } catch(e) {
        this.clearSystemThinkingRows();
        if (typeof InfringToast !== 'undefined') InfringToast.error('Failed to upload audio: ' + (e.message || 'unknown error'));
      }
    },

    ...infringChatSearchDisplayMethods(),

    get canExpandDisplayedMessages() {
      return chatCanExpandDisplayedMessages(this);
    },

    get expandRemainingCount() {
      return chatExpandRemainingCount(this);
    },

    // Search: full filtered message set before display-window capping.
    get allFilteredMessages() {
      return chatAllFilteredMessages(this);
    },

    // Search: filter messages by query + apply incremental display capping.
    get filteredMessages() {
      return chatFilteredMessages(this);
    },

    // Search: highlight matched text in a string
    ...infringChatMessageRenderMethods(),

    renderMarkdown: renderMarkdown,
    escapeHtml: escapeHtml
  };
}
