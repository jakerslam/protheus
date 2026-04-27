        case '/context':
          // Visual-only update for context ring; no chat message noise.
          if (self.currentAgent && InfringAPI.isWsConnected()) {
            InfringAPI.wsSend({ type: 'command', command: 'context', args: '', silent: true });
          } else {
            self.recomputeContextEstimate();
            self.setContextWindowFromCurrentAgent();
          }
          break;
        case '/verbose':
          if (self.currentAgent && InfringAPI.isWsConnected()) {
            InfringAPI.wsSend({ type: 'command', command: 'verbose', args: cmdArgs });
          } else {
            self.pushSystemMessage({ id: ++msgId, role: 'system', text: 'Not connected. Connect to an agent first.', meta: '', tools: [], system_origin: 'slash:verbose' });
          }
          break;
        case '/queue':
          if (self.currentAgent && InfringAPI.isWsConnected()) {
            InfringAPI.wsSend({ type: 'command', command: 'queue', args: '' });
          } else {
            self.pushSystemMessage({ id: ++msgId, role: 'system', text: 'Not connected.', meta: '', tools: [], system_origin: 'slash:queue' });
          }
          break;
        case '/status':
          InfringAPI.get('/api/status').then(function(s) {
            self.pushSystemMessage({ id: ++msgId, role: 'system', text: '**System Status**\n- Agents: ' + (s.agent_count || 0) + '\n- Uptime: ' + (s.uptime_seconds || 0) + 's\n- Version: ' + (s.version || '?'), meta: '', tools: [], system_origin: 'slash:status' });
          }).catch(function() {});
          break;
        case '/alerts':
          await self.runSlashAlerts();
          break;
        case '/next':
          await self.runSlashNextActions();
          break;
        case '/memory':
          await self.runSlashMemoryHygiene();
          break;
        case '/continuity':
          await self.runSlashContinuity();
          break;
        case '/aliases':
          self.executeSlashAliases();
          break;
        case '/alias':
          self.executeSlashAliasCommand(cmdArgs);
          break;
        case '/opt':
          await self.runSlashOptimizeWorkers();
          break;
        case '/model':
          if (self.currentAgent) {
            if (cmdArgs) {
              var resolvedSlashModel = typeof self.resolveModelCatalogOption === 'function'
                ? self.resolveModelCatalogOption(
                  cmdArgs,
                  String((self.currentAgent && self.currentAgent.model_provider) || '').trim(),
                  typeof self.modelCatalogRows === 'function' ? self.modelCatalogRows() : []
                )
                : null;
              self.switchAgentModelWithGuards(resolvedSlashModel || { id: cmdArgs }, {
                agent_id: self.currentAgent.id
              }).catch(function(e) {
                InfringToast.error('Model switch failed: ' + e.message);
              });
            } else {
              var catalogRows = typeof self.modelCatalogRows === 'function' ? self.modelCatalogRows() : [];
              var selectedModelRef = typeof self.normalizeQualifiedModelRef === 'function'
                ? self.normalizeQualifiedModelRef(
                  String((self.currentAgent && (self.currentAgent.model_name || self.currentAgent.runtime_model)) || ''),
                  String((self.currentAgent && self.currentAgent.model_provider) || '').trim(),
                  catalogRows
                )
                : String((self.currentAgent && (self.currentAgent.model_name || self.currentAgent.runtime_model)) || '').trim();
              var runtimeModelRef = typeof self.normalizeQualifiedModelRef === 'function'
                ? self.normalizeQualifiedModelRef(
                  String((self.currentAgent && (self.currentAgent.runtime_model || self.currentAgent.model_name)) || ''),
                  String((self.currentAgent && self.currentAgent.model_provider) || '').trim(),
                  catalogRows
                )
                : String((self.currentAgent && (self.currentAgent.runtime_model || self.currentAgent.model_name)) || '').trim();
              var selectedDisplay = typeof self.formatQualifiedModelDisplay === 'function'
                ? self.formatQualifiedModelDisplay(selectedModelRef)
                : selectedModelRef;
              var runtimeDisplay = typeof self.formatQualifiedModelDisplay === 'function'
                ? self.formatQualifiedModelDisplay(runtimeModelRef)
                : runtimeModelRef;
              var availableCount = Array.isArray(catalogRows)
                ? catalogRows.filter(function(row) { return row && row.available !== false; }).length
                : 0;
              self.pushSystemMessage({
                id: ++msgId,
                role: 'system',
                text: '**Current Model**\n' +
                  '- Provider: `' + (self.currentAgent.model_provider || '?') + '`\n' +
                  '- Selected: `' + (selectedDisplay || selectedModelRef || '?') + '`\n' +
                  '- Runtime: `' + (runtimeDisplay || runtimeModelRef || '?') + '`\n' +
                  '- Available catalog models: ' + availableCount + '\n' +
                  '- Usage: `/model <provider/model>` or `/model <model>`',
                meta: '',
                tools: [],
                system_origin: 'slash:model'
              });
            }
          } else {
            self.pushSystemMessage({ id: ++msgId, role: 'system', text: 'No agent selected.', meta: '', tools: [], system_origin: 'slash:model' });
          }
          break;
        case '/apikey':
          await self.runSlashApiKeyDiscovery(cmdArgs);
          break;
        case '/file':
          if (!self.currentAgent) {
            self.pushSystemMessage({ id: ++msgId, role: 'system', text: 'No agent selected.', meta: '', tools: [], system_origin: 'slash:file' });
            break;
          }
          if (!cmdArgs || !String(cmdArgs).trim()) {
            self.pushSystemMessage({ id: ++msgId, role: 'system', text: 'Usage: `/file <path>`', meta: '', tools: [], system_origin: 'slash:file' });
            break;
          }
          try {
            var fileRes = await InfringAPI.post('/api/agents/' + self.currentAgent.id + '/file/read', {
              path: String(cmdArgs || '').trim()
            });
            var fileMeta = fileRes && fileRes.file ? fileRes.file : null;
            if (!fileMeta || !fileMeta.ok) {
              self.pushSystemMessage({ id: ++msgId, role: 'system', text: 'Error: failed to read file output.', meta: '', tools: [], system_origin: 'slash:file', ts: Date.now() });
            } else {
              var bytes = Number(fileMeta.bytes || 0);
              var fileMetaText = (bytes > 0 ? (bytes + ' bytes') : '');
              if (fileMeta.truncated) {
                var maxBytes = Number(fileMeta.max_bytes || 0);
                fileMetaText += (fileMetaText ? ' | ' : '') + 'truncated to ' + (maxBytes > 0 ? maxBytes : 'limit') + ' bytes';
              }
              self.messages.push({
                id: ++msgId, role: 'agent', text: '', meta: fileMetaText, tools: [], ts: Date.now(),
                file_output: { path: String(fileMeta.path || cmdArgs || ''), content: String(fileMeta.content || ''), truncated: !!fileMeta.truncated, bytes: bytes }
              });
            }
            self.scrollToBottom();
          } catch (e) {
            self.pushSystemMessage({ id: ++msgId, role: 'system', text: 'Error: ' + (e && e.message ? e.message : 'file read failed'), meta: '', tools: [], system_origin: 'slash:file', ts: Date.now() });
          }
          break;
        case '/folder':
          if (!self.currentAgent) {
            self.pushSystemMessage({ id: ++msgId, role: 'system', text: 'No agent selected.', meta: '', tools: [], system_origin: 'slash:folder' });
            break;
          }
          if (!cmdArgs || !String(cmdArgs).trim()) {
            self.pushSystemMessage({ id: ++msgId, role: 'system', text: 'Usage: `/folder <path>`', meta: '', tools: [], system_origin: 'slash:folder' });
            break;
          }
          try {
            var folderRes = await InfringAPI.post('/api/agents/' + self.currentAgent.id + '/folder/export', {
              path: String(cmdArgs || '').trim()
            });
            var folderMeta = folderRes && folderRes.folder ? folderRes.folder : null;
            var archiveMeta = folderRes && folderRes.archive ? folderRes.archive : null;
            if (!folderMeta || !folderMeta.ok) {
              self.pushSystemMessage({ id: ++msgId, role: 'system', text: 'Error: failed to export folder output.', meta: '', tools: [], system_origin: 'slash:folder', ts: Date.now() });
            } else {
              var entryCount = Number(folderMeta.entries || 0);
              var folderMetaText = (entryCount > 0 ? (entryCount + ' entries') : '');
              if (folderMeta.truncated) folderMetaText += (folderMetaText ? ' | ' : '') + 'tree truncated';
              if (archiveMeta && archiveMeta.file_name) folderMetaText += (folderMetaText ? ' | ' : '') + archiveMeta.file_name;
              self.messages.push({
                id: ++msgId, role: 'agent', text: '', meta: folderMetaText, tools: [], ts: Date.now(),
                folder_output: {
                  path: String(folderMeta.path || cmdArgs || ''), tree: String(folderMeta.tree || ''), entries: entryCount, truncated: !!folderMeta.truncated,
                  download_url: archiveMeta && archiveMeta.download_url ? String(archiveMeta.download_url) : '', archive_name: archiveMeta && archiveMeta.file_name ? String(archiveMeta.file_name) : '',
                  archive_bytes: Number(archiveMeta && archiveMeta.bytes ? archiveMeta.bytes : 0)
                }
              });
            }
            self.scrollToBottom();
          } catch (e2) {
            self.pushSystemMessage({ id: ++msgId, role: 'system', text: 'Error: ' + (e2 && e2.message ? e2.message : 'folder export failed'), meta: '', tools: [], system_origin: 'slash:folder', ts: Date.now() });
          }
          break;
        case '/clear':
          self.messages = [];
          break;
        case '/exit':
          InfringAPI.wsDisconnect();
          self._wsAgent = null;
          self.currentAgent = null;
          self.setStoreActiveAgentId(null);
          self.messages = [];
          window.dispatchEvent(new Event('close-chat'));
          break;
        case '/budget':
          InfringAPI.get('/api/budget').then(function(b) {
            var fmt = function(v) { return v > 0 ? '$' + v.toFixed(2) : 'unlimited'; };
            self.pushSystemMessage({ id: ++msgId, role: 'system', text: '**Budget Status**\n' +
              '- Hourly: $' + (b.hourly_spend||0).toFixed(4) + ' / ' + fmt(b.hourly_limit) + '\n' +
              '- Daily: $' + (b.daily_spend||0).toFixed(4) + ' / ' + fmt(b.daily_limit) + '\n' +
              '- Monthly: $' + (b.monthly_spend||0).toFixed(4) + ' / ' + fmt(b.monthly_limit), meta: '', tools: [], system_origin: 'slash:budget' });
          }).catch(function() {});
          break;
        case '/peers':
          InfringAPI.get('/api/network/status').then(function(ns) {
            self.pushSystemMessage({ id: ++msgId, role: 'system', text: '**OFP Network**\n' +
              '- Status: ' + (ns.enabled ? 'Enabled' : 'Disabled') + '\n' +
              '- Connected peers: ' + (ns.connected_peers||0) + ' / ' + (ns.total_peers||0), meta: '', tools: [], system_origin: 'slash:peers' });
          }).catch(function() {});
          break;
        case '/a2a':
          InfringAPI.get('/api/a2a/agents').then(function(res) {
            var agents = res.agents || [];
            if (!agents.length) {
              self.pushSystemMessage({ id: ++msgId, role: 'system', text: 'No external A2A agents discovered.', meta: '', tools: [], system_origin: 'slash:a2a' });
            } else {
              var lines = agents.map(function(a) { return '- **' + a.name + '** — ' + a.url; });
              self.pushSystemMessage({ id: ++msgId, role: 'system', text: '**A2A Agents (' + agents.length + ')**\n' + lines.join('\n'), meta: '', tools: [], system_origin: 'slash:a2a' });
            }
          }).catch(function() {});
          break;
      }
      this.scheduleConversationPersist();
    },
    maybeDiscardPendingFreshAgent: function(nextAgentId) {
      var store = Alpine.store('app');
      if (!store) return;
      var pendingId = String(store.pendingFreshAgentId || '').trim();
      if (!pendingId) return;
      var targetId = String(nextAgentId || '').trim();
      if (!targetId || targetId === pendingId) return;
      store.pendingFreshAgentId = null;
      store.pendingAgent = null;
      InfringAPI.del('/api/agents/' + encodeURIComponent(pendingId)).catch(function() {});
      if (typeof store.refreshAgents === 'function') {
        setTimeout(function() { store.refreshAgents({ force: true }).catch(function() {}); }, 0);
      }
    },
    selectAgent(agent) {
      var resolved = this.resolveAgent(agent);
      if (!resolved) return;
      var selectingSystemThread = this.isSystemThreadAgent(resolved);
      this.closeGitTreeMenu();
      var currentAgentId = this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : '';
      var nextAgentId = String((resolved && resolved.id) || '');
      this.maybeDiscardPendingFreshAgent(nextAgentId);
      if (currentAgentId !== nextAgentId) {
        var activeSearch = String(this.searchQuery || '').trim();
        if (activeSearch) {
          this.searchQuery = '';
          this.searchOpen = false;
        }
      }
      this._markAgentPreviewUnread(resolved.id, false);
      var store = Alpine.store('app');
      var pendingFreshId = store && store.pendingFreshAgentId ? String(store.pendingFreshAgentId) : '';
      var forceFreshSession = pendingFreshId && String(resolved.id) === pendingFreshId;
      this.clearHoveredMessageHard();
      if (this.currentAgent && this.currentAgent.id && this.currentAgent.id !== resolved.id) {
        var switchingFrom = String(this.currentAgent.id || '');
        if (
          this.sending &&
          this._pendingWsRequest &&
          String(this._pendingWsRequest.agent_id || '') === switchingFrom
        ) {
          this._clearTypingTimeout();
          this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
          this.sending = false;
          this._responseStartedAt = 0;
          this.setAgentLiveActivity(switchingFrom, 'working');
          this._recoverPendingWsRequest('agent_switch');
        }
        if (typeof this.captureConversationDraft === 'function') {
          this.captureConversationDraft(this.currentAgent.id);
        }
        this.cacheAgentConversation(this.currentAgent.id);
      }
      if (this.currentAgent && this.currentAgent.id === resolved.id) {
        if (selectingSystemThread) {
          this.activateSystemThread({ preserve_if_empty: true });
          return;
        }
        this.currentAgent = this.applyAgentGitTreeState(resolved, resolved) || resolved;
        this.recordModelUsageTimestamp(resolved.model_name || resolved.runtime_model || '');
        if (forceFreshSession) {
          this.applyConversationInputMode(resolved.id, { force_terminal: false });
          this.messages = [];
          this.inputText = '';
          this.contextApproxTokens = 0;
          this.refreshContextPressure();
          this.resetFreshInitStateForAgent(resolved);
          if (this.conversationCache) {
            delete this.conversationCache[String(resolved.id)];
            this.persistConversationCache();
          }
          InfringAPI.post('/api/agents/' + resolved.id + '/session/reset', {}).catch(function() {});
          this.connectWs(resolved.id);
          this.loadSessions(resolved.id);
          this.requestContextTelemetry(true);
          this.clearPromptSuggestions();
          this.startFreshInitSequence(resolved);
          if (typeof this.restoreConversationDraft === 'function') {
            this.restoreConversationDraft(resolved.id, 'chat');
          }
          var selfFreshCurrent = this;
          this.$nextTick(function() {
            selfFreshCurrent.scrollToBottomImmediate();
            selfFreshCurrent.stabilizeBottomScroll();
            selfFreshCurrent.pinToLatestOnOpen(null, { maxFrames: 20 });
            selfFreshCurrent.installChatMapWheelLock();
            selfFreshCurrent.scheduleMessageRenderWindowUpdate();
          });
        } else {
          this.loadSession(resolved.id, false);
        }
        if (!(this.isSystemThreadAgent && this.isSystemThreadAgent(resolved))) {
          this.refreshModelCatalogAndGuidance({ discover: true, guidance: true }).catch(function() {});
        }
        return;
      }
      if (selectingSystemThread) {
        this.activateSystemThread({ preserve_if_empty: false });
        return;
      }
      this.currentAgent = this.applyAgentGitTreeState(resolved, resolved) || resolved;
      if (store) this.setStoreActiveAgentId(resolved.id || null);
      this.recordModelUsageTimestamp(resolved.model_name || resolved.runtime_model || '');
      // Reset context meter on agent switch to avoid stale carry-over from prior threads.
      this.contextApproxTokens = 0;
      this.contextPressure = 'low';
      this.setContextWindowFromCurrentAgent();
      if (forceFreshSession) this.applyConversationInputMode(resolved.id, { force_terminal: false });
      else this.applyConversationInputMode(resolved.id);
      if (forceFreshSession && this.conversationCache) {
        delete this.conversationCache[String(resolved.id)];
        this.persistConversationCache();
        InfringAPI.post('/api/agents/' + resolved.id + '/session/reset', {}).catch(function() {});
      }
      var restored = forceFreshSession ? false : this.restoreAgentConversation(resolved.id);
      if (!restored) {
        this.messages = [];
        this.inputText = '';
        this.contextApproxTokens = 0;
        this.refreshContextPressure();
      }
      if (typeof this.restoreConversationDraft === 'function') {
        this.restoreConversationDraft(resolved.id);
      }
      this.showFreshArchetypeTiles = false;
      this.freshInitRevealMenu = false;
      if (forceFreshSession) {
        this.resetFreshInitStateForAgent(resolved);
        this.clearPromptSuggestions();
        this.startFreshInitSequence(resolved);
      } else {
        this.freshInitStageToken = Number(this.freshInitStageToken || 0) + 1;
        this._freshInitThreadShownFor = '';
      }
      this._reconcileSendingState();
      this.connectWs(resolved.id);
      // Show welcome tips on first use
      if (!restored && !this.showFreshArchetypeTiles && !localStorage.getItem('of-chat-tips-seen')) {
        this.messages.push({
          id: ++msgId,
          role: 'system',
          text: '**Welcome to Infring Chat!**\n\n' +
            '- Type `/` to see available commands\n' +
            '- `/help` shows all commands\n' +
            '- `/think on` enables extended reasoning\n' +
            '- `/context` shows context window usage\n' +
            '- `/verbose off` hides tool details\n' +
            '- `Ctrl+Shift+F` toggles focus mode\n' +
            '- `Ctrl+F` opens file picker\n' +
            '- Drag & drop files to attach them\n' +
            '- `Ctrl+/` opens the command palette',
          meta: '',
          tools: [],
          system_origin: 'chat:welcome'
        });
        localStorage.setItem('of-chat-tips-seen', 'true');
      }
      if (!forceFreshSession) {
        this.loadSession(resolved.id, false);
      }
      this.loadSessions(resolved.id);
      this.requestContextTelemetry(true);
      this.refreshModelCatalogAndGuidance({ discover: true, guidance: true }).catch(function() {});
      if (!forceFreshSession) {
        this.refreshPromptSuggestions(false);
      }
      if (this.showAgentDrawer) {
        this.openAgentDrawer();
      }
      // Focus input after agent selection
      var self = this;
      this.$nextTick(function() {
        var el = document.getElementById('msg-input');
        if (el) el.focus();
        self.scrollToBottomImmediate();
        self.stabilizeBottomScroll();
        self.pinToLatestOnOpen(null, { maxFrames: 20 });
        self.installChatMapWheelLock();
        self.scheduleMessageRenderWindowUpdate();
      });
    },
    isMessageVirtualizationActive(list) {
      var rows = Array.isArray(list) ? list : this.messages;
      return Array.isArray(rows) && rows.length > 80;
    },
    messageRenderMetrics(msg) {
      if (!msg || typeof msg !== 'object') return null;
      var metrics = msg._renderMetrics;
      if (!metrics || typeof metrics !== 'object') {
        metrics = {};
        msg._renderMetrics = metrics;
      }
      return metrics;
    },
    resolveMessageByDomId(domId) {
      var target = String(domId || '').trim();
      if (!target) return null;
      var rows = Array.isArray(this.messages) ? this.messages : [];
      for (var i = 0; i < rows.length; i++) {
        if (this.messageDomId(rows[i], i) === target) return rows[i];
      }
      return null;
    },
    trackRenderedMessageMetrics(blockEl) {
      if (!blockEl || typeof blockEl.querySelector !== 'function') return;
      var metricRoot = blockEl.classList && blockEl.classList.contains('chat-message-block') ? blockEl : ((typeof blockEl.closest === 'function' && blockEl.closest('.chat-message-block')) || blockEl), bubble = metricRoot.querySelector('.message:not(.message-placeholder) .message-bubble:not(.message-placeholder-bubble)');
      if (!bubble) return;
      var msg = this.resolveMessageByDomId(String(metricRoot.id || blockEl.id || '').trim());
      if (!msg) return;
      var styles = window.getComputedStyle(bubble);
      var paddingTop = parseFloat(styles.paddingTop || '0');
      var paddingBottom = parseFloat(styles.paddingBottom || '0');
      var lineHeightRaw = parseFloat(styles.lineHeight || '0');
      var fontSizeRaw = parseFloat(styles.fontSize || '14');
      var lineHeight = Number.isFinite(lineHeightRaw) && lineHeightRaw > 0
        ? lineHeightRaw
        : Math.max(20, Math.round(fontSizeRaw * 1.6));
      var bubbleHeight = Math.max(0, Math.round(bubble.getBoundingClientRect().height));
      var bubbleWidth = Math.max(0, Math.round(bubble.getBoundingClientRect().width));
      var contentHeight = Math.max(0, bubbleHeight - Math.round(paddingTop + paddingBottom));
      var lineCount = Math.max(1, Math.ceil(contentHeight / Math.max(lineHeight, 1)));
      var metrics = this.messageRenderMetrics(msg);
      if (!metrics) return;
      metrics.lineCount = lineCount;
      metrics.lineHeight = Math.max(18, Math.round(lineHeight));
      metrics.bubbleHeight = Math.max(Math.round(lineHeight + paddingTop + paddingBottom), bubbleHeight);
      metrics.bubbleWidth = bubbleWidth;
      metrics.updatedAt = Date.now();
    },
    shouldRenderMessage(msg, idx, list) { void msg; void idx; void list; return true; },
    // Gate the heavyweight bubble content on the render window. When this returns
    // false, Alpine's x-if branch in index_body.html.parts unmounts the
    // <infring-chat-bubble-render> element (markdown + code blocks + media) and
    // mounts the lightweight <infring-message-placeholder-shell> instead, which
    // is a sized stack of <span class="message-placeholder-line"> elements
    // dimensioned from msg._renderMetrics so scroll position is preserved.
    //
    // Previously this was hardcoded to `return true`, which meant the heavy
    // bubble never unmounted; the .message-text-skeletonized CSS class was used
    // as a visual fallback (transparent text + repeating-linear-gradient gray
    // lines) but every DOM node was still rendered, so markdown parsing, code
    // tokenization, and layout cost all stayed in the hot path. Flipping this
    // gate to delegate to isMessageTextInRenderWindow turns the existing
    // placeholder infrastructure into a real DOM-level virtualization.
    shouldRenderMessageContent(msg, idx, list) {
      var rows = Array.isArray(list) ? list : this.messages;
      // Virtualization only kicks in once the chat passes the threshold
      // (currently > 80 messages, see isMessageVirtualizationActive). Below
      // that, render everything to keep the small-chat path simple.
      if (typeof this.isMessageVirtualizationActive === 'function'
        && !this.isMessageVirtualizationActive(rows)) return true;
      // Always keep streaming / thinking / typing-visual / thought-streaming
      // messages fully rendered. The user is actively watching them and any
      // visual flicker from unmount/remount destroys the live-text experience.
      if (msg && (msg.streaming || msg.thinking || msg._typingVisual || msg.thoughtStreaming)) return true;
      // Forced hydration overrides: scheduleMessageRenderWindowUpdate's
      // forceMessageRender path keeps a message rendered for ttlMs after focus,
      // and the messageHydration map carries selected/hovered/recently-focused
      // dom IDs as a viewport-aware allowlist. If either says yes, render.
      var domId = typeof this.messageDomId === 'function'
        ? this.messageDomId(msg, idx)
        : null;
      if (domId) {
        var hydration = this.messageHydration && typeof this.messageHydration === 'object'
          ? this.messageHydration
          : null;
        if (hydration && hydration[domId] === true) return true;
        var forced = this._forcedHydrateById && typeof this._forcedHydrateById === 'object'
          ? this._forcedHydrateById
          : null;
        if (forced && Number(forced[domId] || 0) > Date.now()) return true;
      }
      // Fall through to the existing render-window logic (±messageTextRenderWindowRadius
      // around the active scroll position, default 20). Returns true for messages
      // close to the user's current scroll focus, false for distant history.
      if (typeof this.isMessageTextInRenderWindow === 'function') {
        return !!this.isMessageTextInRenderWindow(msg, idx, rows);
      }
      // Conservative fallback: if the gate plumbing is missing, render.
      return true;
    },
    isMessageTextInRenderWindow(msg, idx, list) {
      var rows = Array.isArray(list) ? list : this.messages, active = Number(this.mapStepIndex), selected = String(this.selectedMessageDomId || this.hoveredMessageDomId || this.directHoveredMessageDomId || '').trim(), windowRows = Number(this.messageTextRenderWindowRadius || 20);
      if (!this.isMessageVirtualizationActive(rows)) return true;
      if (!Number.isFinite(active) || active < 0 || active >= rows.length) active = Math.max(0, rows.length - 1);
      for (var i = 0; selected && i < rows.length; i++) if (this.messageDomId(rows[i], i) === selected) { active = i; break; }
      return Math.abs(Number(idx || 0) - active) <= (Number.isFinite(windowRows) && windowRows > 0 ? windowRows : 20) || !!(msg && (msg.streaming || msg.thinking || msg._typingVisual));
    },
    messageEstimatedLineCount(msg) {
      var metrics = this.messageRenderMetrics(msg);
      if (metrics && Number.isFinite(Number(metrics.lineCount)) && Number(metrics.lineCount) > 0) {
        return Math.max(1, Math.round(Number(metrics.lineCount)));
      }
      if (!msg || typeof msg !== 'object') return 1;
      var preview = '';
      if (typeof this.messageVisiblePreviewText === 'function') {
        preview = String(this.messageVisiblePreviewText(msg) || '');
      }
      if (!preview && typeof msg.text === 'string') preview = String(msg.text || '');
      var logicalLines = preview ? preview.split(/\r?\n/) : [''];
      var charsPerLine = msg.terminal ? 72 : (String(msg.role || '').toLowerCase() === 'user' ? 46 : 54);
      var lineCount = 0;
      for (var i = 0; i < logicalLines.length; i++) {
        var segment = String(logicalLines[i] || '');
        lineCount += Math.max(1, Math.ceil(segment.length / Math.max(charsPerLine, 1)));
      }
      if (Array.isArray(msg.tools) && msg.tools.length) lineCount += Math.max(2, msg.tools.length * 2);
      if (msg.file_output && msg.file_output.path) lineCount += 4;
      if (msg.folder_output && msg.folder_output.path) lineCount += 5;
      if (Array.isArray(msg.images) && msg.images.length) lineCount += Math.max(2, msg.images.length * 2);
      if (typeof this.messageProgress === 'function' && this.messageProgress(msg)) lineCount += 2;
      if (typeof this.messageToolTraceSummary === 'function' && this.messageToolTraceSummary(msg).visible) lineCount += 1;
      return Math.max(1, Math.min(48, lineCount));
    },
    messagePlaceholderResolvedLineCount(msg, idx, list) {
      void idx;
      void list;
      return this.messageEstimatedLineCount(msg);
    },
    messagePlaceholderResolvedLineHeight(msg, idx, list) {
      void idx;
      void list;
      var metrics = this.messageRenderMetrics(msg);
      if (metrics && Number.isFinite(Number(metrics.lineHeight)) && Number(metrics.lineHeight) > 0) {
        return Math.max(18, Math.round(Number(metrics.lineHeight)));
      }
      return msg && msg.terminal ? 20 : 24;
    },
    messagePlaceholderStyle(msg, idx, list) {
      var lineCount = this.messagePlaceholderResolvedLineCount(msg, idx, list);
      var lineHeight = this.messagePlaceholderResolvedLineHeight(msg, idx, list);
      var metrics = this.messageRenderMetrics(msg);
      var bubbleHeight = metrics && Number.isFinite(Number(metrics.bubbleHeight)) && Number(metrics.bubbleHeight) > 0
        ? Math.round(Number(metrics.bubbleHeight))
        : Math.round((lineCount * lineHeight) + (msg && msg.terminal ? 20 : 28));
      var trackedWidth = metrics && Number.isFinite(Number(metrics.bubbleWidth)) ? Math.round(Number(metrics.bubbleWidth)) : 0;
      var widthValue = 'var(--message-bubble-readable-width)';
      if (msg && msg.terminal) {
        widthValue = trackedWidth > 0 ? (trackedWidth + 'px') : 'min(84ch, 90%)';
      } else if (lineCount > 1 && trackedWidth > 0) {
        widthValue = Math.max(180, trackedWidth) + 'px';
      }
      return '--message-placeholder-line-count:' + String(lineCount) + ';' +
        '--message-placeholder-line-height:' + String(lineHeight) + 'px;' +
        '--message-placeholder-bubble-height:' + String(bubbleHeight) + 'px;' +
        '--message-placeholder-width:' + widthValue + ';';
    },
    messagePlaceholderLineIndices(msg, idx, list) {
      var count = this.messagePlaceholderResolvedLineCount(msg, idx, list);
      var indices = [];
      for (var i = 0; i < count; i++) indices.push(i);
      return indices;
    },
    forceMessageRender(msg, idx, ttlMs) {
      if (!msg) return;
      if (!this._forcedHydrateById || typeof this._forcedHydrateById !== 'object') this._forcedHydrateById = {};
      var domId = this.messageDomId(msg, idx);
      if (!domId) return;
      var ttl = Number(ttlMs || 0);
      if (!Number.isFinite(ttl) || ttl < 250) ttl = 2500;
      this._forcedHydrateById[domId] = Date.now() + ttl;
      this.scheduleMessageRenderWindowUpdate();
    },
    scheduleMessageRenderWindowUpdate(container) {
      var root = container && typeof container.querySelectorAll === 'function' ? container : null;
      if (this._renderWindowRaf) window.cancelAnimationFrame(this._renderWindowRaf);
      var self = this;
      this._renderWindowRaf = window.requestAnimationFrame(function() {
        self._renderWindowRaf = 0;
        self.updateMessageRenderWindow(root);
      });
    },
    updateMessageRenderWindow(container) {
      var root = container && typeof container.querySelectorAll === 'function'
        ? container
        : (this.$refs && this.$refs.messagesEl ? this.$refs.messagesEl : document.getElementById('messages'));
      if (!root) {
        this.messageHydration = {};
        this.messageHydrationReady = false;
        return;
      }
      var blocks = Array.prototype.slice.call(root.querySelectorAll('.chat-message-block[id]')); if (!blocks.length) blocks = Array.prototype.slice.call(root.querySelectorAll('.chat-message-block .message[id]'));
      if (!blocks.length) {
        this.messageHydration = {};
        this.messageHydrationReady = false;
        return;
      }
      for (var i = 0; i < blocks.length; i++) this.trackRenderedMessageMetrics(blocks[i]);
      if (!this.isMessageVirtualizationActive(blocks)) {
        this.messageHydration = {};
        this.messageHydrationReady = false;
        return;
      }
      var scrollTop = Number(root.scrollTop || 0);
      var viewportHeight = Number(root.clientHeight || 0);
      var bufferPx = Math.max(viewportHeight, 320);
      var firstVisible = -1;
      var lastVisible = -1;
      for (var j = 0; j < blocks.length; j++) {
        var block = blocks[j];
        var top = Number(block.offsetTop || 0);
        var height = Number(block.offsetHeight || 0);
        var bottom = top + Math.max(height, 1);
        if (bottom >= (scrollTop - bufferPx) && top <= (scrollTop + viewportHeight + bufferPx)) {
          if (firstVisible < 0) firstVisible = j;
          lastVisible = j;
        }
      }
      if (firstVisible < 0 || lastVisible < 0) {
        firstVisible = Math.max(0, blocks.length - 20);
        lastVisible = blocks.length - 1;
      }
      var extraRows = 10;
      var start = Math.max(0, firstVisible - extraRows);
      var end = Math.min(blocks.length - 1, lastVisible + extraRows);
      var nextHydration = {};
      for (var k = start; k <= end; k++) {
        nextHydration[blocks[k].id] = true;
      }
      if (blocks.length > 0) {
        nextHydration[blocks[0].id] = true;
        nextHydration[blocks[blocks.length - 1].id] = true;
      }
      if (this.selectedMessageDomId) nextHydration[String(this.selectedMessageDomId)] = true;
      if (this.hoveredMessageDomId) nextHydration[String(this.hoveredMessageDomId)] = true;
      if (this.directHoveredMessageDomId) nextHydration[String(this.directHoveredMessageDomId)] = true;
      var retainedForced = {};
      var now = Date.now();
      var forced = this._forcedHydrateById && typeof this._forcedHydrateById === 'object' ? this._forcedHydrateById : {};
      Object.keys(forced).forEach(function(domId) {
        var expiresAt = Number(forced[domId] || 0);
        if (!Number.isFinite(expiresAt) || expiresAt <= now) return;
        retainedForced[domId] = expiresAt;
        nextHydration[domId] = true;
      });
      this._forcedHydrateById = retainedForced;
      this.messageHydration = nextHydration;
      this.messageHydrationReady = true;
    },
