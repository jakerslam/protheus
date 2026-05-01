// Layer ownership: client/runtime/systems/ui (dashboard chat UX surface only; no runtime authority).
function chatCurrentTip(vm) {
  if (localStorage.getItem('of-tips-off') === 'true') return '';
  return vm.tips[vm.tipIndex % vm.tips.length];
}

function chatThinkingEnabled(vm) {
  return vm.thinkingMode !== 'off';
}

function chatWelcomeTipsSeenStorageKey() {
  return 'of-chat-tips-seen';
}

function chatWelcomeTipsText() {
  return '**Welcome to Infring Chat!**\n\n' +
    '- Type `/` to see available commands\n' +
    '- `/help` shows all commands\n' +
    '- `/think on` enables extended reasoning\n' +
    '- `/context` shows context window usage\n' +
    '- `/verbose off` hides tool details\n' +
    '- `Ctrl+Shift+F` toggles focus mode\n' +
    '- `Ctrl+F` opens file picker\n' +
    '- Drag & drop files to attach them\n' +
    '- `Ctrl+/` opens the command palette';
}

function infringChatEarlyDelegateMethods() {
  return {
    dismissTips: function() { localStorage.setItem('of-tips-off', 'true'); },
    startTipCycle: function() {
      var self = this;
      if (this.tipTimer) clearInterval(this.tipTimer);
      this.tipTimer = setInterval(function() {
        self.tipIndex = (self.tipIndex + 1) % self.tips.length;
      }, 30000);
    },

    formatTokenThousands(value) {
      var raw = Number(value || 0);
      if (!Number.isFinite(raw) || raw <= 0) return '0k';
      var k = raw / 1000;
      if (k >= 100) return Math.round(k) + 'k';
      if (k >= 10) return (Math.round(k * 10) / 10).toFixed(1).replace(/\.0$/, '') + 'k';
      return (Math.round(k * 100) / 100).toFixed(2).replace(/0$/, '').replace(/\.$/, '') + 'k';
    },

    // Backward-compat shim for legacy callers during naming migration.
    formatTokenK(value) {
      return this.formatTokenThousands(value);
    },

    normalizeBranchName: function(value) {
      var raw = String(value == null ? '' : value).trim();
      if (!raw) return '';
      var normalized = raw
        .replace(/[^A-Za-z0-9._/-]+/g, '-')
        .replace(/\/+/g, '/')
        .replace(/^[-./]+|[-./]+$/g, '');
      return normalized;
    },

    closeComposerMenus: function(options) {
      var keep = options && typeof options === 'object' ? options : {};
      if (!keep.attach) this.showAttachMenu = false;
      if (!keep.model) this.showModelSwitcher = false;
      if (!keep.git) this.closeGitTreeMenu();
    },

    toggleAttachMenu: function() {
      var nextOpen = !this.showAttachMenu;
      this.closeComposerMenus(nextOpen ? { attach: true } : {});
      this.showAttachMenu = nextOpen;
    },

    closeGitTreeMenu: function() {
      this.showGitTreeMenu = false;
      this.gitTreeMenuError = '';
    },

    async refreshGitTreeMenu(force) {
      if (!this.currentAgent || !this.currentAgent.id) {
        this.gitTreeMenuItems = [];
        this.gitTreeMenuError = '';
        return;
      }
      if (!force && this.gitTreeMenuLoading) return;
      this.gitTreeMenuLoading = true;
      this.gitTreeMenuError = '';
      try {
        var payload = await InfringAPI.get('/api/agents/' + encodeURIComponent(this.currentAgent.id) + '/git-trees');
        var options = Array.isArray(payload && payload.options) ? payload.options : [];
        this.gitTreeMenuItems = options.map(function(row) {
          return {
            branch: String((row && row.branch) || '').trim(),
            current: !!(row && row.current),
            main: !!(row && row.main),
            kind: String((row && row.kind) || '').trim(),
            in_use_by_agents: Number((row && row.in_use_by_agents) || 0) || 0
          };
        }).filter(function(row) { return !!row.branch; });
        this.applyAgentGitTreeState(this.currentAgent, payload && payload.current ? payload.current : {});
      } catch (e) {
        this.gitTreeMenuItems = [];
        this.gitTreeMenuError = (e && e.message) ? String(e.message) : 'failed_to_load_git_trees';
      } finally {
        this.gitTreeMenuLoading = false;
      }
    },

    async toggleGitTreeMenu() {
      if (!this.currentAgent || !this.currentAgent.id) return;
      if (this.showGitTreeMenu) {
        this.closeGitTreeMenu();
        return;
      }
      this.closeComposerMenus({ git: true });
      this.showGitTreeMenu = true;
      await this.refreshGitTreeMenu(true);
    },

    async switchAgentGitTree(branchName, options) {
      if (!this.currentAgent || !this.currentAgent.id || this.gitTreeSwitching) return;
      var branch = this.normalizeBranchName(branchName);
      if (!branch) return;
      var requireNew = !!(options && options.requireNew === true);
      var current = this.normalizeBranchName(this.activeGitBranchLabel);
      if (!requireNew && current && current === branch) {
        this.closeGitTreeMenu();
        return;
      }
      this.gitTreeSwitching = true;
      this.gitTreeMenuError = '';
      try {
        var result = await InfringAPI.post(
          '/api/agents/' + encodeURIComponent(this.currentAgent.id) + '/git-tree/switch',
          {
            branch: branch,
            require_new: requireNew
          }
        );
        this.applyAgentGitTreeState(this.currentAgent, result && result.current ? result.current : {});
        var appStoreBridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
          ? InfringSharedShellServices.appStore
          : null;
        var refreshAgents = appStoreBridge && typeof appStoreBridge.method === 'function'
          ? appStoreBridge.method('refreshAgents')
          : null;
        if (typeof refreshAgents === 'function') {
          await refreshAgents({ force: true });
        }
        await this.refreshGitTreeMenu(true);
        this.closeGitTreeMenu();
        InfringToast.success('Switched to branch ' + branch);
      } catch (e) {
        var message = (e && e.message) ? String(e.message) : 'git_tree_switch_failed';
        this.gitTreeMenuError = message;
        InfringToast.error('Git tree switch failed: ' + message);
      } finally {
        this.gitTreeSwitching = false;
      }
    },

    async createAndCheckoutGitBranch() {
      if (!this.currentAgent || !this.currentAgent.id || this.gitTreeSwitching) return;
      var suggested = this.normalizeBranchName('feature/' + String(this.currentAgent.id || '').trim().toLowerCase());
      var input = prompt('Create and checkout new branch:', suggested || 'feature/new-branch');
      if (input == null) return;
      var branch = this.normalizeBranchName(input);
      if (!branch) {
        InfringToast.error('Enter a valid branch name');
        return;
      }
      await this.switchAgentGitTree(branch, { requireNew: true });
    },

    hasSeenWelcomeTips: function() {
      return !!localStorage.getItem(chatWelcomeTipsSeenStorageKey());
    },

    markWelcomeTipsSeen: function() {
      localStorage.setItem(chatWelcomeTipsSeenStorageKey(), 'true');
    },

    welcomeTipsText: function() {
      return chatWelcomeTipsText();
    }
  };
}
