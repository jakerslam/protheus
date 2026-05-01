    updateSidebarScrollIndicators() {
      var refs = this.$refs || {};
      var navState = this._computeScrollHintState(refs.sidebarNav);
      this.sidebarHasOverflowAbove = !!navState.above;
      this.sidebarHasOverflowBelow = !!navState.below;
      var chatState = this._computeScrollHintState(refs.chatSidebarList);
      this.chatSidebarHasOverflowAbove = !!chatState.above;
      this.chatSidebarHasOverflowBelow = !!chatState.below;
    },
    scheduleSidebarScrollIndicators() {
      if (this._sidebarScrollIndicatorRaf) return;
      var self = this;
      this._sidebarScrollIndicatorRaf = requestAnimationFrame(function() {
        self._sidebarScrollIndicatorRaf = 0;
        self.updateSidebarScrollIndicators();
        if (typeof self.maybeAnimateChatSidebarRows === 'function') {
          self.maybeAnimateChatSidebarRows();
        }
      });
    },
    shellAppStoreBridge() {
      return infringShellAppStoreBridge();
    },
    notifyShellAppStore(reason) {
      var bridge = this.shellAppStoreBridge();
      if (bridge && typeof bridge.notify === 'function') bridge.notify(reason || 'shell_root_changed');
    },
    getAppStore() {
      var bridge = this.shellAppStoreBridge();
      if (bridge && typeof bridge.current === 'function') {
        var bridgedStore = bridge.current();
        if (bridgedStore && typeof bridgedStore === 'object') return bridgedStore;
      }
      return (typeof window !== 'undefined' && window.InfringApp && typeof window.InfringApp === 'object')
        ? window.InfringApp
        : null;
    },
    get agents() {
      var store = this.getAppStore();
      return store && Array.isArray(store.agents) ? store.agents : [];
    },
    isSystemSidebarThread(agent) {
      if (!agent || typeof agent !== 'object') return false;
      if (agent.is_system_thread === true) return true;
      var id = String(agent.id || '').trim().toLowerCase();
      if (id === 'system') return true;
      var role = String(agent.role || '').trim().toLowerCase();
      return role === 'system';
    },
    isSidebarArchivedAgent(agent) {
      if (!agent || typeof agent !== 'object') return false;
      var store = this.getAppStore();
      if (store && typeof store.isArchivedLikeAgent === 'function') return store.isArchivedLikeAgent(agent);
      if (Object.prototype.hasOwnProperty.call(agent, 'sidebar_archived')) return !!agent.sidebar_archived;
      return !!agent.archived;
    },
    isReservedSystemEmoji(rawEmoji) {
      var normalized = String(rawEmoji || '').replace(/\uFE0F/g, '').trim();
      return normalized === '⚙';
    },
    sanitizeSidebarAgentRow(agent) {
      if (!agent || typeof agent !== 'object') return agent;
      var row = Object.assign({}, agent);
      var identity = Object.assign({}, (row.identity && typeof row.identity === 'object') ? row.identity : {});
      if (this.isSystemSidebarThread(row)) {
        row.id = 'system';
        row.name = 'System';
        row.is_system_thread = true;
        row.role = 'system';
        identity.emoji = '\u2699\ufe0f';
        row.identity = identity;
        return row;
      }
      if (this.isReservedSystemEmoji(identity.emoji)) {
        identity.emoji = '';
      }
      row.identity = identity;
      return row;
    },
    persistChatSidebarTopologyOrder() {
      var seen = {};
      var out = [];
      (this.chatSidebarTopologyOrder || []).forEach(function(id) {
        var key = String(id || '').trim();
        if (!key || seen[key]) return;
        seen[key] = true;
        out.push(key);
      });
      this.chatSidebarTopologyOrder = out;
      try {
        localStorage.setItem('infring-chat-sidebar-topology-order', JSON.stringify(out));
      } catch(_) {}
    },
    chatSidebarCanReorderTopology() {
      return String(this.chatSidebarSortMode || '').toLowerCase() === 'topology';
    },
    startChatSidebarTopologyDrag(agent, ev) {
      if (!this.chatSidebarCanReorderTopology() || !agent || !agent.id) return;
      this.syncChatSidebarTopologyOrderFromAgents();
      this.chatSidebarDragAgentId = String(agent.id);
      this.chatSidebarDropTargetId = '';
      this.chatSidebarDropAfter = false;
      if (ev && ev.dataTransfer) {
        ev.dataTransfer.effectAllowed = 'move';
        ev.dataTransfer.setData('text/plain', this.chatSidebarDragAgentId);
      }
    },
    handleChatSidebarTopologyDragOver(agent, ev) {
      if (!this.chatSidebarCanReorderTopology() || !this.chatSidebarDragAgentId || !agent || !agent.id) return;
      if (ev) {
        ev.preventDefault();
        if (ev.dataTransfer) ev.dataTransfer.dropEffect = 'move';
      }
      var targetId = String(agent.id);
      var dropAfter = false;
      if (ev && ev.currentTarget && typeof ev.clientY === 'number' && typeof ev.currentTarget.getBoundingClientRect === 'function') {
        var rect = ev.currentTarget.getBoundingClientRect();
        dropAfter = ev.clientY > (rect.top + (rect.height / 2));
      }
      this.chatSidebarDropAfter = !!dropAfter;
      this.chatSidebarDropTargetId = targetId === this.chatSidebarDragAgentId ? '' : targetId;
    },
    handleChatSidebarTopologyDrop(agent, ev) {
      if (ev) ev.preventDefault();
      if (!this.chatSidebarCanReorderTopology() || !agent || !agent.id) return this.endChatSidebarTopologyDrag();
      var dragId = String(this.chatSidebarDragAgentId || '').trim();
      if (!dragId && ev && ev.dataTransfer) dragId = String(ev.dataTransfer.getData('text/plain') || '').trim();
      var targetId = String(agent.id).trim();
      if (!dragId || !targetId || dragId === targetId) return this.endChatSidebarTopologyDrag();
      this.syncChatSidebarTopologyOrderFromAgents();
      var order = (this.chatSidebarTopologyOrder || []).slice();
      var fromIndex = order.indexOf(dragId);
      var targetIndex = order.indexOf(targetId);
      if (fromIndex < 0 || targetIndex < 0) return this.endChatSidebarTopologyDrag();
      var dropAfter = false;
      if (ev && ev.currentTarget && typeof ev.clientY === 'number' && typeof ev.currentTarget.getBoundingClientRect === 'function') {
        var rect = ev.currentTarget.getBoundingClientRect();
        dropAfter = ev.clientY > (rect.top + (rect.height / 2));
      }
      order.splice(fromIndex, 1);
      if (fromIndex < targetIndex) targetIndex -= 1;
      if (dropAfter) targetIndex += 1;
      if (targetIndex < 0) targetIndex = 0;
      if (targetIndex > order.length) targetIndex = order.length;
      order.splice(targetIndex, 0, dragId);
      this.chatSidebarTopologyOrder = order;
      this.persistChatSidebarTopologyOrder();
      this.endChatSidebarTopologyDrag();
      this.scheduleSidebarScrollIndicators();
    },
    endChatSidebarTopologyDrag() {
      this.chatSidebarDragAgentId = '';
      this.chatSidebarDropTargetId = '';
      this.chatSidebarDropAfter = false;
    },
    get chatSidebarAgents() {
      var list = (this.agents || []).slice();
      var self = this;
      var pendingFreshId = String((this.getAppStore() && this.getAppStore().pendingFreshAgentId) || '').trim();
      list = list.filter(function(agent) {
        if (!agent || !agent.id) return false;
        if (pendingFreshId && String(agent.id || '') === pendingFreshId) return false;
        if (self.isSidebarArchivedAgent(agent)) return false;
        return true;
      });
      list.sort(function(a, b) {
        return self.chatSidebarSortComparator(a, b);
      });
      if (this.chatSidebarCanReorderTopology() && Array.isArray(this.chatSidebarTopologyOrder) && this.chatSidebarTopologyOrder.length) {
        var rank = {};
        this.chatSidebarTopologyOrder.forEach(function(id, idx) {
          var key = String(id || '').trim();
          if (!key || rank[key] != null) return;
          rank[key] = idx;
        });
        list.sort(function(a, b) {
          var aId = String((a && a.id) || '');
          var bId = String((b && b.id) || '');
          var hasA = Object.prototype.hasOwnProperty.call(rank, aId);
          var hasB = Object.prototype.hasOwnProperty.call(rank, bId);
          if (hasA && hasB && rank[aId] !== rank[bId]) return rank[aId] - rank[bId];
          if (hasA && !hasB) return -1;
          if (!hasA && hasB) return 1;
          return self.chatSidebarSortComparator(a, b);
        });
      }
      return list.map(function(agent) {
        return self.sanitizeSidebarAgentRow(agent);
      });
    },
    get chatSidebarRows() {
      if (this.chatSidebarDragActive && Array.isArray(this._chatSidebarDragRowsCache)) {
        return this._chatSidebarDragRowsCache;
      }
      var query = String(this.chatSidebarQuery || '').trim();
      var rows;
      if (!query) rows = this.chatSidebarAgents || [];
      else if (Array.isArray(this.chatSidebarSearchResults) && this.chatSidebarSearchResults.length) rows = this.chatSidebarSearchResults;
      else rows = [];
      if (this.chatSidebarDragActive) {
        this._chatSidebarDragRowsCache = Array.isArray(rows) ? rows.slice() : [];
      } else {
        this._chatSidebarDragRowsCache = null;
      }
      return rows;
    },
    chatSidebarDragRenderWindow(rows) {
      var sourceRows = Array.isArray(rows) ? rows : [];
      var total = sourceRows.length;
      var maxRows = Math.max(1, Math.floor(Number(this._chatSidebarDragRenderMaxRows || 10)));
      if (!this.chatSidebarDragActive || total <= maxRows) {
        return { virtualized: false, start: 0, end: total, padTop: 0, padBottom: 0 };
      }
      var refs = this.$refs || {};
      var nav = refs.sidebarNav || null;
      var rowHeight = Math.max(1, Math.floor(Number(this._chatSidebarDragRenderRowHeight || 56)));
      var scrollTop = nav ? Math.max(0, Number(nav.scrollTop || 0)) : 0;
      var start = Math.max(0, Math.floor(scrollTop / rowHeight));
      if (start > (total - maxRows)) start = Math.max(0, total - maxRows);
      var end = Math.min(total, start + maxRows);
      return {
        virtualized: true,
        start: start,
        end: end,
        padTop: start * rowHeight,
        padBottom: Math.max(0, (total - end) * rowHeight)
      };
    },
    get chatSidebarVirtualized() {
      var rows = Array.isArray(this.chatSidebarRows) ? this.chatSidebarRows : [];
      return this.chatSidebarDragRenderWindow(rows).virtualized;
    },
    get chatSidebarVirtualPadTop() {
      var rows = Array.isArray(this.chatSidebarRows) ? this.chatSidebarRows : [];
      return this.chatSidebarDragRenderWindow(rows).padTop;
    },
    get chatSidebarVirtualPadBottom() {
      var rows = Array.isArray(this.chatSidebarRows) ? this.chatSidebarRows : [];
      return this.chatSidebarDragRenderWindow(rows).padBottom;
    },
    get chatSidebarVisibleRows() {
      var rows = Array.isArray(this.chatSidebarRows) ? this.chatSidebarRows : [];
      var window = this.chatSidebarDragRenderWindow(rows);
      if (!window.virtualized) return rows;
      return rows.slice(window.start, window.end);
    },
    chatSidebarHasMoreRows() { return false; },
    showMoreChatSidebarRows() { this.scheduleSidebarScrollIndicators(); },
    init() {
      var self = this;
      var appStoreBridge = typeof this.shellAppStoreBridge === 'function' ? this.shellAppStoreBridge() : null;
      if (appStoreBridge && typeof appStoreBridge.registerShellRoot === 'function') {
        appStoreBridge.registerShellRoot(this);
      }
      this._bootSplashStartedAt = Date.now();
      this.bootSplashVisible = true;
      this.applyOverlayGlassTemplate('simple-glass', true);
      if (typeof this.resetBootProgress === 'function') this.resetBootProgress();
      if (typeof this.setBootProgressEvent === 'function') this.setBootProgressEvent('splash_visible');
      if (typeof this.hideDashboardPopupBySource === 'function') this.hideDashboardPopupBySource('sidebar');
      if (this._bootSplashMaxTimer) {
        clearTimeout(this._bootSplashMaxTimer);
        this._bootSplashMaxTimer = 0;
      }
      this._bootSplashMaxTimer = window.setTimeout(function() {
        self.releaseBootSplash(true);
      }, Number(this._bootSplashMaxMs || 5000));
      window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', function(e) {
        if (self.themeMode === 'system') {
          self.beginInstantThemeFlip();
          self.theme = e.matches ? 'dark' : 'light';
        }
      });
      var validPages = ['chat','agents','sessions','approvals','comms','workflows','scheduler','channels','eyes','skills','hands','overview','analytics','logs','runtime','settings','wizard'];
      var pageRedirects = {
        'automation': 'scheduler',
        'templates': 'agents',
        'triggers': 'workflows',
        'cron': 'scheduler',
        'schedules': 'scheduler',
        'memory': 'sessions',
        'audit': 'logs',
        'security': 'settings',
        'peers': 'settings',
        'migration': 'settings',
        'usage': 'analytics',
        'approval': 'approvals'
      };
      this.syncAgentChatsSectionForPage = function() {
        this.agentChatsSectionCollapsed = false;
      };
      this.toggleAgentChatsSection = function() {
        this.agentChatsSectionCollapsed = false;
      };
      var searchParams = new URLSearchParams(window.location.search || '');
      var embeddedDashboardMode = searchParams.get('embed') === '1';
      var embeddedPage = String(searchParams.get('page') || '').trim().toLowerCase();
      var pathnamePage = '';
      try {
        var pathname = String(window.location.pathname || '').trim();
        if (pathname.indexOf('/dashboard/') === 0) {
          pathnamePage = pathname.slice('/dashboard/'.length).split('/')[0].trim().toLowerCase();
        }
      } catch (_) {}
      if (embeddedDashboardMode && document && document.body && document.body.classList) {
        document.body.classList.add('dashboard-embedded-shell');
      }
      function handleHash() {
        var hash = window.location.hash.replace('#', '') || embeddedPage || pathnamePage || 'chat';
        if (pageRedirects[hash]) {
          hash = pageRedirects[hash];
          window.location.hash = hash;
        }
        if (validPages.indexOf(hash) >= 0) {
          self.page = hash;
          self.syncAgentChatsSectionForPage(hash);
          if (typeof self.syncPageHistory === 'function') self.syncPageHistory(hash);
          if (typeof self.notifyShellAppStore === 'function') self.notifyShellAppStore('route_changed');
        }
      }
      window.addEventListener('hashchange', handleHash);
      handleHash();

      document.addEventListener('keydown', function(e) {
        if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
          e.preventDefault();
          self.navigate('agents');
        }
        if ((e.ctrlKey || e.metaKey) && e.key === 'n' && !e.shiftKey) {
          e.preventDefault();
          self.createSidebarAgentChat();
        }
        if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'F') {
          e.preventDefault();
          var keyStore = self.getAppStore();
          if (keyStore && typeof keyStore.toggleFocusMode === 'function') {
            keyStore.toggleFocusMode();
          }
        }
        if (e.key === 'Escape') {
          self.mobileMenuOpen = false;
        }
      });

      InfringAPI.onConnectionChange(function(state) {
        var connStore = self.getAppStore();
        if (connStore) connStore.connectionState = state;
        self.connectionState = state;
        self.queueConnectionIndicatorState(state);
      });

      if (!window.__infringToastCaptureInstalled) {
        window.addEventListener('infring:toast', function(ev) {
          var detail = (ev && ev.detail) ? ev.detail : {};
          var store = self.getAppStore();
          if (store && typeof store.addNotification === 'function') {
            store.addNotification(detail);
          }
        });
        window.__infringToastCaptureInstalled = true;
      }

      this.pollStatus();
      var initStore = this.getAppStore();
      if (initStore && typeof initStore.checkOnboarding === 'function') initStore.checkOnboarding();
      if (initStore && typeof initStore.checkAuth === 'function') initStore.checkAuth();
      if (!this._dashboardClockTimer) this._dashboardClockTimer = setInterval(function() { self.clockTick = Date.now(); }, 1000);
      if (!this._dashboardStatusTimer) this._dashboardStatusTimer = setInterval(function() {
        if (document && document.hidden) return;
        self.pollStatus();
      }, 10000);
      if (!this._dashboardVisibilityHandler && document) {
        this._dashboardVisibilityHandler = function() { if (!document.hidden) self.pollStatus(); };
        document.addEventListener('visibilitychange', this._dashboardVisibilityHandler);
      }
      window.addEventListener('resize', function() {
        self.scheduleSidebarScrollIndicators();
      });
      this.$nextTick(function() {
        self.scheduleSidebarScrollIndicators();
      });
    },
    releaseBootSplash(force) {
      if (!this.bootSplashVisible) return;
      var now = Date.now();
      var elapsed = Math.max(0, now - Number(this._bootSplashStartedAt || now));
      var minRemain = Math.max(0, Number(this._bootSplashMinMs || 0) - elapsed);
      var store = this.getAppStore();
      var ready = !!force || !store || store.booting === false;
      if (!ready) return;
      if (typeof this.setBootProgressEvent === 'function') this.setBootProgressEvent('releasing', { bootStage: store && store.bootStage });
      if (this._bootSplashHideTimer) {
        clearTimeout(this._bootSplashHideTimer);
        this._bootSplashHideTimer = 0;
      }
      var self = this;
      var progressNow = typeof this.bootProgressClamped === 'function'
        ? this.bootProgressClamped(this.bootProgressPercent)
        : Math.max(0, Math.min(100, Number(this.bootProgressPercent || 0)));
      var completionAnimationDelayMs = progressNow < 100 ? 500 : 0;
      var hideDelayMs = Math.max(minRemain, completionAnimationDelayMs);
      if (typeof this.setBootProgressEvent === 'function') this.setBootProgressEvent('complete', { bootStage: store && store.bootStage });
      if (hideDelayMs <= 0) {
        this.bootSplashVisible = false;
        if (this._bootSplashMaxTimer) {
          clearTimeout(this._bootSplashMaxTimer);
          this._bootSplashMaxTimer = 0;
        }
        return;
      }
      this._bootSplashHideTimer = window.setTimeout(function() {
        self.bootSplashVisible = false;
        self._bootSplashHideTimer = 0;
        if (self._bootSplashMaxTimer) {
          clearTimeout(self._bootSplashMaxTimer);
          self._bootSplashMaxTimer = 0;
        }
      }, hideDelayMs);
    },
    normalizeNavigablePage(pageId) {
      var raw = String(pageId || '').trim().toLowerCase();
      if (!raw) return 'chat';
      var aliases = {
        'automation': 'scheduler',
        'templates': 'agents',
        'triggers': 'workflows',
        'cron': 'scheduler',
        'schedules': 'scheduler',
        'memory': 'sessions',
        'audit': 'logs',
        'security': 'settings',
        'peers': 'settings',
        'migration': 'settings',
        'usage': 'analytics',
        'approval': 'approvals'
      };
      return aliases[raw] || raw;
    },
    isKnownNavigablePage(pageId) {
      var normalized = this.normalizeNavigablePage(pageId);
      return ['chat','agents','sessions','approvals','comms','workflows','scheduler','channels','eyes','skills','hands','overview','analytics','logs','runtime','settings','wizard']
        .indexOf(normalized) >= 0;
    },
    syncPageHistory(nextPage) {
      var next = this.normalizeNavigablePage(nextPage);
      if (!this.isKnownNavigablePage(next)) return;
      var current = this.normalizeNavigablePage(this._navCurrentPage || this.page || '');
      var action = String(this._navHistoryAction || '').trim().toLowerCase();
      var back = Array.isArray(this.navBackStack) ? this.navBackStack.slice() : [];
      var forward = Array.isArray(this.navForwardStack) ? this.navForwardStack.slice() : [];
      var cap = Number(this._navHistoryCap || 48);
      if (!Number.isFinite(cap) || cap < 8) cap = 48;
      var trim = function(list) {
        return list.length > cap ? list.slice(list.length - cap) : list;
      };
      if (!current || !this.isKnownNavigablePage(current)) {
        this._navCurrentPage = next;
        this._navHistoryAction = '';
        return;
      }
      if (next === current) {
        this._navCurrentPage = next;
        this._navHistoryAction = '';
        return;
      }
      if (action === 'back') {
        if (forward.length === 0 || forward[forward.length - 1] !== current) forward.push(current);
      } else if (action === 'forward') {
        if (back.length === 0 || back[back.length - 1] !== current) back.push(current);
      } else if (back.length > 0 && back[back.length - 1] === next) {
        back.pop();
        if (forward.length === 0 || forward[forward.length - 1] !== current) forward.push(current);
      } else if (forward.length > 0 && forward[forward.length - 1] === next) {
        forward.pop();
        if (back.length === 0 || back[back.length - 1] !== current) back.push(current);
      } else {
        if (back.length === 0 || back[back.length - 1] !== current) back.push(current);
        forward = [];
      }
      this.navBackStack = trim(back);
      this.navForwardStack = trim(forward);
      this._navCurrentPage = next;
      this._navHistoryAction = '';
    },
    canNavigateBack() {
      return Array.isArray(this.navBackStack) && this.navBackStack.length > 0;
    },
    canNavigateForward() {
      return Array.isArray(this.navForwardStack) && this.navForwardStack.length > 0;
    },
    navigateBackPage() {
      if (!this.canNavigateBack()) return;
      var back = this.navBackStack.slice();
      var target = this.normalizeNavigablePage(back.pop());
      this.navBackStack = back;
      this._navHistoryAction = 'back';
      if (!target || target === this.normalizeNavigablePage(this.page)) {
        this._navHistoryAction = '';
        return;
      }
      this.navigate(target);
    },
    navigateForwardPage() {
      if (!this.canNavigateForward()) return;
      var forward = this.navForwardStack.slice();
      var target = this.normalizeNavigablePage(forward.pop());
      this.navForwardStack = forward;
      this._navHistoryAction = 'forward';
      if (!target || target === this.normalizeNavigablePage(this.page)) {
        this._navHistoryAction = '';
        return;
      }
      this.navigate(target);
    },
    navigate(p) {
      if (typeof this.hideDashboardPopupBySource === 'function') this.hideDashboardPopupBySource('sidebar');
      if (String(p || '') !== 'chat') {
        var store = this.getAppStore();
        var pendingId = String((store && store.pendingFreshAgentId) || '').trim();
        var activeId = String((store && store.activeAgentId) || '').trim();
        if (pendingId) {
          if (store) {
            store.pendingFreshAgentId = null;
            store.pendingAgent = null;
            if (pendingId === activeId) {
              if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(null);
              else store.activeAgentId = null;
            }
          }
          this.chatSidebarTopologyOrder = (this.chatSidebarTopologyOrder || []).filter(function(id) {
            return String(id || '').trim() !== pendingId;
          });
          this.persistChatSidebarTopologyOrder();
          InfringAPI.del('/api/agents/' + encodeURIComponent(pendingId)).catch(function() {});
          if (store && typeof store.refreshAgents === 'function') setTimeout(function() { store.refreshAgents({ force: true }).catch(function() {}); }, 0);
        }
      }
      this.page = p;
      if (typeof this.syncAgentChatsSectionForPage === 'function') {
        this.syncAgentChatsSectionForPage(p);
      }
      if (typeof this.notifyShellAppStore === 'function') this.notifyShellAppStore('navigate');
      window.location.hash = p;
